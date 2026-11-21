//! Conexão direta entre dois aparelhos (WebRTC).
//!
//! O áudio e o vídeo de uma chamada **não passam pelos servidores**: vão
//! direto de um computador para o outro, cifrados com DTLS-SRTP. Os servidores
//! só carregam a negociação inicial, que também vai cifrada dentro de
//! envelopes efêmeros.
//!
//! A negociação aqui é "sem trickle": o aparelho junta todos os caminhos de
//! rede possíveis e manda uma descrição só. É mais lento para começar que o
//! trickle, mas combina com o transporte que temos — cada troca de mensagem
//! passa por um relay, então mandar vinte candidatos separados sairia pior.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_OPUS, MIME_TYPE_VP8};
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecParameters, RTPCodecType};
use webrtc::rtp_transceiver::RTCPFeedback;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::policy::ice_transport_policy::RTCIceTransportPolicy;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;
use webrtc::rtp::codecs::vp8::Vp8Packet;
use webrtc::rtp::packetizer::Depacketizer;
use webrtc::track::track_remote::TrackRemote;

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("webrtc: {0}")]
    Rtc(String),
    #[error("a negociação demorou demais")]
    Timeout,
    #[error("a conexão foi fechada")]
    Closed,
}

type Result<T> = std::result::Result<T, PeerError>;

fn rtc<E: std::fmt::Display>(e: E) -> PeerError {
    PeerError::Rtc(e.to_string())
}

/// Servidor que ajuda dois aparelhos atrás de roteadores a se encontrarem.
#[derive(Debug, Clone)]
pub struct TurnServer {
    pub urls: Vec<String>,
    pub username: String,
    pub password: String,
}

/// Como a chamada deve se conectar.
#[derive(Debug, Clone, Default)]
pub struct CallConfig {
    /// Servidor próprio, opcional. Sem ele, redes muito fechadas podem não
    /// conseguir fechar a conexão direta.
    pub turn: Option<TurnServer>,
    /// Forçar todo o tráfego pelo TURN, escondendo o seu endereço de internet
    /// da outra pessoa. Custa banda e um pouco de atraso.
    pub hide_ip: bool,
}

/// O que acontece na conexão, para a interface mostrar.
#[derive(Debug, Clone)]
pub enum PeerEvent {
    /// Mudou o estado: conectando, conectado, caiu.
    State(ConnectionState),
    /// Chegou áudio ou vídeo do outro lado.
    TrackOpened { kind: MediaKind },
    /// Chegou um pacote de mídia já decodificável.
    Media { kind: MediaKind, data: Vec<u8> },
    /// Mensagem pelo canal de dados (soundboard, avisos de câmera).
    Data(Vec<u8>),
    /// O canal de dados abriu: hora de contar se a câmera e o microfone
    /// estão ligados, como o celular faz.
    DataOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Audio,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    /// Caiu, mas pode voltar sozinha.
    Reconnecting,
    Failed,
    Closed,
}

impl ConnectionState {
    pub fn label(&self) -> &'static str {
        match self {
            ConnectionState::Connecting => "conectando…",
            ConnectionState::Connected => "conectado",
            ConnectionState::Reconnecting => "reconectando…",
            ConnectionState::Failed => "a conexão caiu",
            ConnectionState::Closed => "encerrada",
        }
    }
}

/// Uma conexão com outro aparelho.
pub struct Peer {
    pc: Arc<RTCPeerConnection>,
    data: Arc<RTCDataChannel>,
    audio: Arc<TrackLocalStaticSample>,
    video: Arc<TrackLocalStaticSample>,
    events: Arc<Mutex<Option<mpsc::UnboundedSender<PeerEvent>>>>,
    /// O outro lado pediu uma imagem inteira (PLI/FIR): perdeu um pedaço ou
    /// acabou de começar a ver. Sem atender, a imagem fica congelada até o
    /// próximo quadro-chave natural, que pode demorar segundos.
    keyframe_wanted: Arc<std::sync::atomic::AtomicBool>,
}

impl Peer {
    /// Abre uma conexão nova e devolve o canal de eventos.
    pub async fn new(cfg: CallConfig) -> Result<(Self, mpsc::UnboundedReceiver<PeerEvent>)> {
        // Só os codecs que este lado sabe tratar: Opus e VP8. Anunciar outros
        // (H.264, VP9, AV1) deixaria o outro lado escolher um deles e a
        // chamada conectaria sem imagem. VP8 e Opus são obrigatórios em
        // WebRTC, então qualquer navegador (e o celular) aceita.
        let mut m = MediaEngine::default();
        m.register_codec(
            RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_OPUS.to_owned(),
                    clock_rate: 48_000,
                    channels: 2,
                    sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
                    rtcp_feedback: vec![],
                },
                payload_type: 111,
                ..Default::default()
            },
            RTPCodecType::Audio,
        )
        .map_err(rtc)?;
        let feedback = vec![
            RTCPFeedback { typ: "goog-remb".to_owned(), parameter: "".to_owned() },
            RTCPFeedback { typ: "ccm".to_owned(), parameter: "fir".to_owned() },
            RTCPFeedback { typ: "nack".to_owned(), parameter: "".to_owned() },
            RTCPFeedback { typ: "nack".to_owned(), parameter: "pli".to_owned() },
        ];
        m.register_codec(
            RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_VP8.to_owned(),
                    clock_rate: 90_000,
                    channels: 0,
                    sdp_fmtp_line: "".to_owned(),
                    rtcp_feedback: feedback,
                },
                payload_type: 96,
                ..Default::default()
            },
            RTPCodecType::Video,
        )
        .map_err(rtc)?;
        let registry = register_default_interceptors(Registry::new(), &mut m).map_err(rtc)?;
        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();

        // Servidores públicos de descoberta de endereço. Eles não veem nem
        // transportam a conversa: só respondem "o seu endereço visto de fora é
        // este".
        let mut ice = vec![RTCIceServer {
            urls: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun.cloudflare.com:3478".to_string(),
            ],
            ..Default::default()
        }];
        if let Some(t) = &cfg.turn {
            ice.push(RTCIceServer {
                urls: t.urls.clone(),
                username: t.username.clone(),
                credential: t.password.clone(),
            });
        }

        let config = RTCConfiguration {
            ice_servers: ice,
            ice_transport_policy: if cfg.hide_ip && cfg.turn.is_some() {
                RTCIceTransportPolicy::Relay
            } else {
                RTCIceTransportPolicy::All
            },
            ..Default::default()
        };

        let pc = Arc::new(api.new_peer_connection(config).await.map_err(rtc)?);
        let (tx, rx) = mpsc::unbounded_channel();
        let events = Arc::new(Mutex::new(Some(tx)));

        // Canal de dados combinado dos dois lados com o mesmo número, para não
        // depender de uma negociação extra depois que a chamada já começou.
        let data = pc
            .create_data_channel(
                "jm",
                Some(RTCDataChannelInit {
                    negotiated: Some(0),
                    ordered: Some(true),
                    ..Default::default()
                }),
            )
            .await
            .map_err(rtc)?;

        let audio = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 48_000,
                channels: 1,
                ..Default::default()
            },
            "audio".to_owned(),
            "jm".to_owned(),
        ));
        let video = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_owned(),
                clock_rate: 90_000,
                ..Default::default()
            },
            "video".to_owned(),
            "jm".to_owned(),
        ));

        let envio_audio = pc
            .add_track(Arc::clone(&audio) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(rtc)?;
        let envio_video = pc
            .add_track(Arc::clone(&video) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(rtc)?;

        // Ler o RTCP que volta é o que faz os interceptadores funcionarem
        // (reenvio de pacote perdido, NACK) e é por onde chega o pedido de
        // quadro-chave do outro lado.
        let keyframe_wanted = Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let kf = Arc::clone(&keyframe_wanted);
            tokio::spawn(async move {
                while let Ok((pacotes, _)) = envio_video.read_rtcp().await {
                    for p in pacotes {
                        let a = p.as_any();
                        if a.is::<webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication>()
                            || a.is::<webrtc::rtcp::payload_feedbacks::full_intra_request::FullIntraRequest>()
                        {
                            kf.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            });
            tokio::spawn(async move { while envio_audio.read_rtcp().await.is_ok() {} });
        }

        // Avisos de mudança de estado.
        {
            let ev = Arc::clone(&events);
            pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                let ev = Arc::clone(&ev);
                Box::pin(async move {
                    let estado = match s {
                        RTCPeerConnectionState::New | RTCPeerConnectionState::Connecting => {
                            ConnectionState::Connecting
                        }
                        RTCPeerConnectionState::Connected => ConnectionState::Connected,
                        RTCPeerConnectionState::Disconnected => ConnectionState::Reconnecting,
                        RTCPeerConnectionState::Failed => ConnectionState::Failed,
                        RTCPeerConnectionState::Closed => ConnectionState::Closed,
                        _ => ConnectionState::Connecting,
                    };
                    if let Some(tx) = ev.lock().await.as_ref() {
                        let _ = tx.send(PeerEvent::State(estado));
                    }
                })
            }));
        }

        // Mídia que chega do outro lado.
        {
            let ev = Arc::clone(&events);
            pc.on_track(Box::new(move |track: Arc<TrackRemote>, _, _| {
                let ev = Arc::clone(&ev);
                Box::pin(async move {
                    let kind = if track.kind() == webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio {
                        MediaKind::Audio
                    } else {
                        MediaKind::Video
                    };
                    if let Some(tx) = ev.lock().await.as_ref() {
                        let _ = tx.send(PeerEvent::TrackOpened { kind });
                    }
                    // A leitura roda numa tarefa própria: a biblioteca espera
                    // este tratador terminar antes de anunciar a próxima faixa.
                    // Com o laço aqui dentro, a faixa de vídeo nunca era
                    // anunciada depois que o áudio começava — a câmera do
                    // celular chegava ao PC e ficava sem ninguém lendo.
                    tokio::spawn(async move {
                    // Um quadro de vídeo quase nunca cabe em um pacote só: ele
                    // é picado para caber na rede e precisa ser remontado antes
                    // de ir para o decodificador. O áudio cabe em um pacote por
                    // quadro, então passa direto.
                    let mut vp8 = Vp8Packet::default();
                    let mut montando: Vec<u8> = Vec::new();

                    while let Ok((pacote, _)) = track.read_rtp().await {
                        if pacote.payload.is_empty() {
                            continue;
                        }
                        let pronto: Option<Vec<u8>> = if kind == MediaKind::Audio {
                            Some(pacote.payload.to_vec())
                        } else {
                            match vp8.depacketize(&pacote.payload) {
                                Ok(pedaco) => {
                                    // Começo de quadro novo: o que estava pela
                                    // metade se perdeu e não serve mais.
                                    if vp8.s == 1 && vp8.pid == 0 {
                                        montando.clear();
                                    }
                                    montando.extend_from_slice(&pedaco);
                                    // A marca do RTP indica o último pedaço.
                                    if pacote.header.marker && !montando.is_empty() {
                                        Some(std::mem::take(&mut montando))
                                    } else {
                                        None
                                    }
                                }
                                Err(_) => {
                                    montando.clear();
                                    None
                                }
                            }
                        };

                        let Some(dados) = pronto else { continue };
                        match ev.lock().await.as_ref() {
                            Some(tx) => {
                                if tx.send(PeerEvent::Media { kind, data: dados }).is_err() {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    });
                })
            }));
        }

        // Canal de dados.
        {
            let ev = Arc::clone(&events);
            data.on_open(Box::new(move || {
                let ev = Arc::clone(&ev);
                Box::pin(async move {
                    if let Some(tx) = ev.lock().await.as_ref() {
                        let _ = tx.send(PeerEvent::DataOpen);
                    }
                })
            }));
        }
        {
            let ev = Arc::clone(&events);
            data.on_message(Box::new(move |msg| {
                let ev = Arc::clone(&ev);
                Box::pin(async move {
                    if let Some(tx) = ev.lock().await.as_ref() {
                        let _ = tx.send(PeerEvent::Data(msg.data.to_vec()));
                    }
                })
            }));
        }

        Ok((
            Self { pc, data, audio, video, events, keyframe_wanted },
            rx,
        ))
    }

    /// Monta a proposta de conexão para mandar ao outro lado.
    pub async fn create_offer(&self) -> Result<String> {
        let oferta = self.pc.create_offer(None).await.map_err(rtc)?;
        self.pc.set_local_description(oferta).await.map_err(rtc)?;
        self.wait_gathering().await?;
        self.local_sdp().await
    }

    /// Responde a uma proposta recebida.
    pub async fn accept_offer(&self, sdp: &str) -> Result<String> {
        let oferta = RTCSessionDescription::offer(sdp.to_string()).map_err(rtc)?;
        self.pc.set_remote_description(oferta).await.map_err(rtc)?;
        let resposta = self.pc.create_answer(None).await.map_err(rtc)?;
        self.pc.set_local_description(resposta).await.map_err(rtc)?;
        self.wait_gathering().await?;
        self.local_sdp().await
    }

    /// Recebe a resposta de quem foi chamado.
    pub async fn accept_answer(&self, sdp: &str) -> Result<()> {
        let resposta = RTCSessionDescription::answer(sdp.to_string()).map_err(rtc)?;
        self.pc.set_remote_description(resposta).await.map_err(rtc)
    }

    /// Espera o aparelho terminar de juntar os caminhos de rede possíveis.
    ///
    /// Com limite de tempo: em redes com muitas interfaces isso pode demorar,
    /// e é melhor tentar com os caminhos que já temos do que travar a chamada.
    async fn wait_gathering(&self) -> Result<()> {
        let mut pronto = self.pc.gathering_complete_promise().await;
        match tokio::time::timeout(Duration::from_secs(5), pronto.recv()).await {
            Ok(_) => Ok(()),
            Err(_) => {
                tracing::warn!("a busca de caminhos de rede demorou; seguindo com o que já temos");
                Ok(())
            }
        }
    }

    async fn local_sdp(&self) -> Result<String> {
        self.pc
            .local_description()
            .await
            .map(|d| d.sdp)
            .ok_or(PeerError::Closed)
    }

    /// Envia um quadro de áudio já comprimido.
    pub async fn send_audio(&self, opus: &[u8], duration: Duration) -> Result<()> {
        self.audio
            .write_sample(&Sample {
                data: bytes::Bytes::copy_from_slice(opus),
                duration,
                ..Default::default()
            })
            .await
            .map_err(rtc)
    }

    /// Envia um quadro de vídeo já comprimido.
    pub async fn send_video(&self, frame: &[u8], duration: Duration) -> Result<()> {
        self.video
            .write_sample(&Sample {
                data: bytes::Bytes::copy_from_slice(frame),
                duration,
                ..Default::default()
            })
            .await
            .map_err(rtc)
    }

    /// Manda uma mensagem de texto pelo canal de dados.
    ///
    /// Tem que ser **texto**: o celular faz `JSON.parse(ev.data)`, e uma
    /// mensagem binária chega lá como `ArrayBuffer` e é descartada em
    /// silêncio — o soundboard e o aviso de câmera do PC sumiriam no caminho.
    pub async fn send_text(&self, text: &str) -> Result<()> {
        self.data.send_text(text.to_string()).await.map(|_| ()).map_err(rtc)
    }

    /// Manda bytes pelo canal de dados (só entre computadores; o celular
    /// ignora mensagens binárias).
    pub async fn send_data(&self, bytes: &[u8]) -> Result<()> {
        self.data
            .send(&bytes::Bytes::copy_from_slice(bytes))
            .await
            .map(|_| ())
            .map_err(rtc)
    }

    /// O outro lado pediu um quadro-chave desde a última vez que perguntamos?
    pub fn take_keyframe_request(&self) -> bool {
        self.keyframe_wanted.swap(false, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn data_is_open(&self) -> bool {
        self.data.ready_state() == webrtc::data_channel::data_channel_state::RTCDataChannelState::Open
    }

    pub async fn connection_state(&self) -> ConnectionState {
        match self.pc.connection_state() {
            RTCPeerConnectionState::Connected => ConnectionState::Connected,
            RTCPeerConnectionState::Failed => ConnectionState::Failed,
            RTCPeerConnectionState::Closed => ConnectionState::Closed,
            RTCPeerConnectionState::Disconnected => ConnectionState::Reconnecting,
            _ => ConnectionState::Connecting,
        }
    }

    /// Encerra a conexão e libera o microfone e a câmera.
    pub async fn close(&self) {
        *self.events.lock().await = None;
        let _ = self.pc.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estados_tem_texto_para_a_tela() {
        for e in [
            ConnectionState::Connecting,
            ConnectionState::Connected,
            ConnectionState::Reconnecting,
            ConnectionState::Failed,
            ConnectionState::Closed,
        ] {
            assert!(!e.label().is_empty());
        }
    }

    #[tokio::test]
    async fn a_proposta_de_conexao_descreve_audio_video_e_dados() {
        let (p, _rx) = Peer::new(CallConfig::default()).await.unwrap();
        let sdp = p.create_offer().await.unwrap();

        assert!(sdp.starts_with("v=0"), "não parece uma descrição de sessão");
        assert!(sdp.contains("m=audio"), "faltou o áudio");
        assert!(sdp.contains("m=video"), "faltou o vídeo");
        assert!(sdp.contains("m=application"), "faltou o canal de dados");
        assert!(sdp.to_lowercase().contains("opus"), "o áudio deveria ser Opus");
        // DTLS: é o que cifra a mídia ponta a ponta.
        assert!(sdp.contains("a=fingerprint"), "sem impressão digital DTLS");
        p.close().await;
    }

    #[tokio::test]
    async fn esconder_o_ip_exige_servidor_proprio() {
        // Sem TURN configurado, pedir para esconder o IP não pode quebrar a
        // chamada: ela continua, só que sem essa proteção.
        let cfg = CallConfig { turn: None, hide_ip: true };
        let (p, _rx) = Peer::new(cfg).await.unwrap();
        assert!(p.create_offer().await.is_ok());
        p.close().await;
    }
}
