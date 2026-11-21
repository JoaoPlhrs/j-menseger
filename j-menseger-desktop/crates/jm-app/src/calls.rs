//! Chamadas e salas de voz, no protocolo do celular (`calls.js`).
//!
//! **Chamada de duas pessoas**
//! 1. Quem liga monta a conexão e manda a proposta por envelope efêmero,
//!    repetindo a cada 4 s enquanto toca (um envelope perdido não pode
//!    deixar o outro lado sem tocar). Depois de 45 s vira chamada perdida.
//! 2. Quem recebe vê a chamada e, ao atender, devolve a resposta.
//! 3. O áudio e o vídeo vão **direto** entre os aparelhos (DTLS-SRTP).
//!
//! **Canal de dados** (texto JSON, igual ao celular): `{t:'cam',on}`,
//! `{t:'mic',on}`, `{t:'snd',id,n,e,d}` e `{t:'sndd',id,i,n,b}` — este último
//! leva, em blocos de 12 000 bytes, um som próprio que o outro lado ainda não
//! tem, antes do `snd` que manda tocar.
//!
//! **Salas de voz**: malha de até 6 pessoas. Cada um avisa presença a cada
//! 8 s; entre dois participantes, quem tem a chave pública menor manda a
//! proposta, então nunca as duas pontas ofertam ao mesmo tempo.

use crate::sounds::{emoji_ok, SoundBank};
use base64::Engine as _;
use jm_core::engine::{CallsFrom, ContactStatus, Engine};
use jm_core::messages::{CallAction, CallSignal, RoomRef};
use jm_core::store::CallLog;
use jm_media::audio::{Capture, Playback};
use jm_media::codec::{AudioQuality, Decoder, Encoder, VoiceDetector};
use jm_media::peer::{CallConfig, ConnectionState, MediaKind, Peer, PeerEvent, TurnServer};
use jm_media::player::Tone;
use jm_media::screen::{self, Capturer, Fps, Quality, Source};
use jm_media::video::{DecodedFrame, VideoDecoder, VideoEncoder};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Quanto tempo o telefone toca antes de desistir.
const RING_TIMEOUT: Duration = Duration::from_secs(45);
/// Intervalo de repetição da proposta enquanto toca.
const OFFER_RESEND: Duration = Duration::from_secs(4);
/// Maior sala de voz.
pub const MAX_ROOM: usize = 6;
/// Blocos de som próprio pelo canal de dados.
const SND_CHUNK: usize = 12_000;

/// Em que ponto a chamada está.
#[derive(Debug, Clone, PartialEq)]
pub enum CallState {
    /// Chamando, esperando a outra pessoa atender.
    Calling,
    /// Alguém está ligando para você.
    Ringing { video: bool },
    /// Atendeu, montando a conexão.
    Connecting,
    /// Falando.
    Active { since: Instant },
    /// Caiu e está tentando voltar.
    Reconnecting,
    /// Acabou.
    Ended { reason: EndReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    HungUp,
    Declined,
    NoAnswer,
    Busy,
    Failed,
}

impl EndReason {
    pub fn label(&self) -> &'static str {
        match self {
            EndReason::HungUp => "Chamada encerrada",
            EndReason::Declined => "Chamada recusada",
            EndReason::NoAnswer => "Não atendeu",
            EndReason::Busy => "A pessoa está em outra chamada",
            EndReason::Failed => "Não foi possível conectar. Redes muito restritas podem precisar de um servidor TURN (Ajustes → Rede e servidores).",
        }
    }
}

/// "Fulano tocou 🎉 Tcharam!" — aparece na tela da chamada.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundBanner {
    pub who: String,
    pub emoji: String,
    pub name: String,
    /// Som próprio de outra pessoa que dá para guardar no seu soundboard.
    pub save: Option<(String, f32)>,
    pub until: Instant,
}

/// O que a interface precisa saber sobre a chamada.
#[derive(Debug, Clone)]
pub struct CallInfo {
    pub peer: String,
    pub state: CallState,
    /// Chamada de vídeo (a outra pessoa pode ligar a câmera).
    pub video: bool,
    pub muted: bool,
    pub mic_level: f32,
    pub remote_speaking: bool,
    /// A outra pessoa desligou o microfone.
    pub remote_muted: bool,
    pub sem_microfone: bool,
    /// O que você está transmitindo, se estiver.
    pub sharing: Option<String>,
    /// A outra pessoa está mandando imagem (câmera ou tela).
    pub receiving_video: bool,
    pub frame_version: u64,
    pub banner: Option<SoundBanner>,
}

/// Um participante da sala de voz.
#[derive(Debug, Clone)]
pub struct RoomPeerInfo {
    pub pk: String,
    pub connected: bool,
    pub speaking: bool,
    pub mic: bool,
}

#[derive(Debug, Clone)]
pub struct RoomInfo {
    pub lid: String,
    pub ch: String,
    pub since: Instant,
    pub muted: bool,
    pub sem_microfone: bool,
    pub mic_level: f32,
    pub me_speaking: bool,
    pub peers: Vec<RoomPeerInfo>,
    pub banner: Option<SoundBanner>,
}

/// Uma conexão com alguém (chamada ou sala), com o que chegou por ela.
struct Link {
    pk: String,
    conn: Arc<Peer>,
    speaking: Arc<AtomicBool>,
    remote_mic: Arc<AtomicBool>,
    remote_cam: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    frame: Arc<Mutex<Option<(u64, DecodedFrame)>>>,
    /// Sons próprios já enviados para esta pessoa (não manda duas vezes).
    sent_sounds: std::sync::Mutex<HashSet<String>>,
    /// Quando chegou a última imagem.
    frame_at: std::sync::Mutex<Option<Instant>>,
}

/// Espera antes do primeiro aviso pelo canal de dados.
///
/// Os dois lados abrem o mesmo canal combinado (id 0). Se a nossa mensagem
/// chega antes de o outro lado terminar de abrir a ponta dele, a biblioteca
/// de WebRTC de lá registra o canal como "vindo de fora" e a ponta dele
/// nunca abre: aviso de câmera, microfone e soundboard somem na chamada.
const PRIMEIRA_FALA: Duration = Duration::from_millis(1200);

impl Link {
    /// A imagem que chega deve aparecer: o outro lado avisou que ligou a
    /// câmera/tela, ou — se o canal de avisos não abriu deste lado — as
    /// imagens continuam chegando.
    fn showing_video(&self) -> bool {
        let recente = self.frame_at.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_secs(2));
        if self.conn.data_is_open() {
            recente && self.remote_cam.load(Ordering::Relaxed)
        } else {
            recente
        }
    }

    fn new(pk: &str, conn: Arc<Peer>) -> Arc<Self> {
        Arc::new(Self {
            pk: pk.into(),
            conn,
            speaking: Arc::new(AtomicBool::new(false)),
            remote_mic: Arc::new(AtomicBool::new(true)),
            remote_cam: Arc::new(AtomicBool::new(false)),
            connected: Arc::new(AtomicBool::new(false)),
            frame: Arc::new(Mutex::new(None)),
            sent_sounds: std::sync::Mutex::new(HashSet::new()),
            frame_at: std::sync::Mutex::new(None),
        })
    }

    async fn send(&self, v: serde_json::Value) {
        if self.conn.data_is_open() {
            let _ = self.conn.send_text(&v.to_string()).await;
        }
    }
}

struct Active {
    link: Arc<Link>,
    cid: String,
    outgoing: bool,
    video: bool,
    capture: Option<Capture>,
    playback: Option<Playback>,
    muted: Arc<AtomicBool>,
    state: CallState,
    pending_offer: Option<String>,
    sem_microfone: bool,
    sharing: Option<(String, Arc<AtomicBool>)>,
    /// Quando atendeu (para a duração e o histórico).
    t0: Option<Instant>,
    /// Quando começou (ms), para o histórico.
    ts: i64,
}

struct Room {
    lid: String,
    ch: String,
    links: HashMap<String, Arc<Link>>,
    playbacks: HashMap<String, Playback>,
    capture: Option<Capture>,
    muted: Arc<AtomicBool>,
    me_speaking: Arc<AtomicBool>,
    sem_microfone: bool,
    since: Instant,
    stop: Arc<AtomicBool>,
    /// Pacotes do meu microfone, para mandar a todos.
    audio_tx: tokio::sync::broadcast::Sender<Arc<Vec<u8>>>,
}

/// Cuida da chamada atual e da sala de voz. Só uma das duas por vez.
pub struct CallManager {
    engine: Arc<Engine>,
    sounds: Arc<SoundBank>,
    active: Arc<Mutex<Option<Active>>>,
    room: Arc<Mutex<Option<Room>>>,
    ending: Arc<Mutex<Option<(String, EndReason, Instant)>>>,
    banner: std::sync::Mutex<Option<SoundBanner>>,
    inbound: std::sync::Mutex<HashMap<String, (u32, Vec<Option<Vec<u8>>>)>>,
    last_snd: std::sync::Mutex<HashMap<String, Instant>>,
}

/// Quanto tempo o aviso de chamada encerrada fica na tela.
const AVISO_DE_FIM: Duration = Duration::from_secs(5);

impl CallManager {
    pub fn new(engine: Arc<Engine>, sounds: Arc<SoundBank>) -> Arc<Self> {
        Arc::new(Self {
            engine,
            sounds,
            active: Arc::new(Mutex::new(None)),
            room: Arc::new(Mutex::new(None)),
            ending: Arc::new(Mutex::new(None)),
            banner: std::sync::Mutex::new(None),
            inbound: std::sync::Mutex::new(HashMap::new()),
            last_snd: std::sync::Mutex::new(HashMap::new()),
        })
    }

    /// Configuração da conexão a partir dos ajustes (TURN, esconder o IP).
    fn config(&self) -> CallConfig {
        let s = self.engine.settings();
        let urls: Vec<String> = s
            .turn
            .url
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|u| !u.is_empty())
            .map(|u| u.to_string())
            .collect();
        let turn = (!urls.is_empty()).then(|| TurnServer { urls, username: s.turn.user.clone(), password: s.turn.pass.clone() });
        CallConfig { hide_ip: s.force_relay && turn.is_some(), turn }
    }

    fn tone(&self, t: Option<Tone>) {
        if let Some(p) = self.sounds.player() {
            match t {
                Some(t) => p.start_loop(t),
                None => p.stop_loop(),
            }
        }
    }

    fn current_banner(&self) -> Option<SoundBanner> {
        let mut b = self.banner.lock().unwrap();
        if b.as_ref().is_some_and(|x| x.until < Instant::now()) {
            *b = None;
        }
        b.clone()
    }

    /* ------------------------------------------------------------ resumo */

    /// Resumo da chamada atual, para desenhar a tela.
    pub async fn info(&self) -> Option<CallInfo> {
        if self.active.lock().await.is_none() {
            let mut fim = self.ending.lock().await;
            let (peer, reason, quando) = fim.as_ref()?.clone();
            if quando.elapsed() > AVISO_DE_FIM {
                *fim = None;
                return None;
            }
            return Some(CallInfo {
                peer,
                state: CallState::Ended { reason },
                video: false,
                muted: false,
                mic_level: 0.0,
                remote_speaking: false,
                remote_muted: false,
                sem_microfone: false,
                sharing: None,
                receiving_video: false,
                frame_version: 0,
                banner: None,
            });
        }
        let frame = {
            let g = self.active.lock().await;
            Arc::clone(&g.as_ref()?.link.frame)
        };
        let versao = frame.lock().await.as_ref().map(|(v, _)| *v).unwrap_or(0);
        let banner = self.current_banner();
        let g = self.active.lock().await;
        let a = g.as_ref()?;
        Some(CallInfo {
            peer: a.link.pk.clone(),
            state: a.state.clone(),
            video: a.video,
            muted: a.muted.load(Ordering::Relaxed),
            mic_level: a.capture.as_ref().map(|c| c.level()).unwrap_or(0.0),
            remote_speaking: a.link.speaking.load(Ordering::Relaxed),
            remote_muted: !a.link.remote_mic.load(Ordering::Relaxed),
            sem_microfone: a.sem_microfone,
            sharing: a.sharing.as_ref().map(|(n, _)| n.clone()),
            receiving_video: versao > 0 && a.link.showing_video(),
            frame_version: versao,
            banner,
        })
    }

    /// Última imagem recebida, se for mais nova que a `desde`.
    pub async fn take_frame(&self, desde: u64) -> Option<(u64, DecodedFrame)> {
        let destino = {
            let g = self.active.lock().await;
            Arc::clone(&g.as_ref()?.link.frame)
        };
        let slot = destino.lock().await;
        match slot.as_ref() {
            Some((v, img)) if *v > desde => Some((*v, img.clone())),
            _ => None,
        }
    }

    pub async fn room_info(&self) -> Option<RoomInfo> {
        let banner = self.current_banner();
        let g = self.room.lock().await;
        let r = g.as_ref()?;
        let mut peers: Vec<RoomPeerInfo> = r
            .links
            .values()
            .map(|l| RoomPeerInfo {
                pk: l.pk.clone(),
                connected: l.connected.load(Ordering::Relaxed),
                speaking: l.speaking.load(Ordering::Relaxed),
                mic: l.remote_mic.load(Ordering::Relaxed),
            })
            .collect();
        peers.sort_by(|a, b| a.pk.cmp(&b.pk));
        Some(RoomInfo {
            lid: r.lid.clone(),
            ch: r.ch.clone(),
            since: r.since,
            muted: r.muted.load(Ordering::Relaxed),
            sem_microfone: r.sem_microfone,
            mic_level: r.capture.as_ref().map(|c| c.level()).unwrap_or(0.0),
            me_speaking: r.me_speaking.load(Ordering::Relaxed),
            peers,
            banner,
        })
    }

    pub async fn is_busy(&self) -> bool {
        self.active.lock().await.is_some() || self.room.lock().await.is_some()
    }

    pub async fn in_room(&self, lid: &str, ch: &str) -> bool {
        self.room.lock().await.as_ref().is_some_and(|r| r.lid == lid && r.ch == ch)
    }

    /* ------------------------------------------------------ chamada 1:1 */

    /// Liga para alguém.
    pub async fn start(self: &Arc<Self>, peer_pk: &str, video: bool) -> Result<(), String> {
        *self.ending.lock().await = None;
        if self.room.lock().await.is_some() {
            return Err("Saia da sala de voz para ligar".into());
        }
        if self.active.lock().await.is_some() {
            return Err("Você já está em uma chamada".into());
        }
        if self.engine.contact(peer_pk).map(|c| c.status) != Some(ContactStatus::Ok) {
            return Err("Aceite o contato antes de ligar.".into());
        }
        if self.engine.open_relays().await == 0 {
            return Err("Sem conexão com os servidores. Tente quando a internet voltar.".into());
        }
        let cid = random_hex(8);
        let (conn, eventos) = Peer::new(self.config()).await.map_err(|e| format!("não deu para preparar a chamada: {e}"))?;
        let conn = Arc::new(conn);
        let oferta = conn.create_offer().await.map_err(|e| format!("não deu para preparar a chamada: {e}"))?;
        let link = Link::new(peer_pk, Arc::clone(&conn));
        {
            let mut g = self.active.lock().await;
            *g = Some(Active {
                link: Arc::clone(&link),
                cid: cid.clone(),
                outgoing: true,
                video,
                capture: None,
                playback: None,
                muted: Arc::new(AtomicBool::new(false)),
                state: CallState::Calling,
                pending_offer: None,
                sem_microfone: false,
                sharing: None,
                t0: None,
                ts: jm_core::engine::now_ms(),
            });
        }
        self.tone(Some(Tone::Ringback));
        self.pump_call(link, eventos);

        // Proposta repetida a cada 4 s enquanto toca, e desistência em 45 s.
        let eu = Arc::clone(self);
        let alvo = peer_pk.to_string();
        tokio::spawn(async move {
            let inicio = Instant::now();
            loop {
                let ainda = {
                    let g = eu.active.lock().await;
                    matches!(g.as_ref(), Some(a) if a.cid == cid && a.state == CallState::Calling)
                };
                if !ainda {
                    return;
                }
                if inicio.elapsed() >= RING_TIMEOUT {
                    let _ = eu.engine.report_missed_call(&alvo, video).await;
                    let _ = eu.engine.send_call_signal(&alvo, CallSignal::new(&cid, CallAction::End)).await;
                    eu.end(EndReason::NoAnswer).await;
                    return;
                }
                let sig = CallSignal { sdp: Some(oferta.clone()), v: Some(video as u8), ..CallSignal::new(&cid, CallAction::Offer) };
                let _ = eu.engine.send_call_signal(&alvo, sig).await;
                tokio::time::sleep(OFFER_RESEND).await;
            }
        });
        Ok(())
    }

    /// Trata a sinalização que chega de alguém (chamada ou sala).
    pub async fn on_signal(self: &Arc<Self>, from: &str, sig: CallSignal) {
        if sig.is_room() {
            self.on_room_signal(from, sig).await;
            return;
        }
        match sig.a {
            CallAction::Offer => self.on_offer(from, sig).await,
            CallAction::Answer => {
                let g = self.active.lock().await;
                if let Some(a) = g.as_ref() {
                    if a.link.pk == from && a.cid == sig.cid {
                        if let Some(sdp) = &sig.sdp {
                            let _ = a.link.conn.accept_answer(sdp).await;
                        }
                    }
                }
            }
            CallAction::Decline => {
                if self.call_matches(from, &sig.cid).await {
                    self.end(EndReason::Declined).await;
                }
            }
            CallAction::Busy => {
                if self.call_matches(from, &sig.cid).await {
                    self.end(EndReason::Busy).await;
                }
            }
            CallAction::End => {
                if self.call_matches(from, &sig.cid).await {
                    self.end(EndReason::HungUp).await;
                }
            }
            CallAction::Missed => {}
        }
    }

    async fn on_offer(self: &Arc<Self>, from: &str, sig: CallSignal) {
        let Some(sdp) = sig.sdp.clone() else { return };
        // A mesma proposta reenviada não é uma chamada nova.
        if self.active.lock().await.as_ref().is_some_and(|a| a.link.pk == from && a.cid == sig.cid) {
            return;
        }
        // "Quem pode me ligar": ninguém, ou só contatos aceitos.
        if self.engine.settings().calls_from == CallsFrom::Nobody {
            return;
        }
        if self.engine.contact(from).map(|c| c.status) != Some(ContactStatus::Ok) {
            return;
        }
        // Ocupado: avisa quem liga em vez de deixar tocando.
        if self.is_busy().await {
            let _ = self.engine.send_call_signal(from, CallSignal::new(&sig.cid, CallAction::Busy)).await;
            return;
        }
        let video = sig.v.unwrap_or(0) == 1;
        let Ok((conn, eventos)) = Peer::new(self.config()).await else { return };
        let link = Link::new(from, Arc::new(conn));
        {
            let mut g = self.active.lock().await;
            *g = Some(Active {
                link: Arc::clone(&link),
                cid: sig.cid.clone(),
                outgoing: false,
                video,
                capture: None,
                playback: None,
                muted: Arc::new(AtomicBool::new(false)),
                state: CallState::Ringing { video },
                pending_offer: Some(sdp),
                sem_microfone: false,
                sharing: None,
                t0: None,
                ts: jm_core::engine::now_ms(),
            });
        }
        *self.ending.lock().await = None;
        self.tone(Some(Tone::Ring));
        self.pump_call(link, eventos);
        // Segurança: se quem liga sumir sem avisar, para de tocar.
        let eu = Arc::clone(self);
        let cid = sig.cid;
        tokio::spawn(async move {
            tokio::time::sleep(RING_TIMEOUT + Duration::from_secs(5)).await;
            let tocando = eu.active.lock().await.as_ref().is_some_and(|a| a.cid == cid && matches!(a.state, CallState::Ringing { .. }));
            if tocando {
                eu.end(EndReason::NoAnswer).await;
            }
        });
    }

    /// Atende a chamada que está tocando.
    pub async fn accept(self: &Arc<Self>) -> Result<(), String> {
        let (conn, sdp, peer_pk, cid) = {
            let mut g = self.active.lock().await;
            let a = g.as_mut().ok_or("a chamada já acabou")?;
            let sdp = a.pending_offer.take().ok_or("a chamada já foi atendida")?;
            a.state = CallState::Connecting;
            (Arc::clone(&a.link.conn), sdp, a.link.pk.clone(), a.cid.clone())
        };
        self.tone(None);
        let resposta = match conn.accept_offer(&sdp).await {
            Ok(r) => r,
            Err(e) => {
                let _ = self.engine.send_call_signal(&peer_pk, CallSignal::new(&cid, CallAction::Decline)).await;
                self.end(EndReason::Failed).await;
                return Err(format!("não deu para atender: {e}"));
            }
        };
        let _ = self
            .engine
            .send_call_signal(&peer_pk, CallSignal { sdp: Some(resposta), ..CallSignal::new(&cid, CallAction::Answer) })
            .await;
        Ok(())
    }

    /// Recusa a chamada que está tocando.
    pub async fn decline(self: &Arc<Self>) {
        let alvo = self.active.lock().await.as_ref().map(|a| (a.link.pk.clone(), a.cid.clone()));
        if let Some((pk, cid)) = alvo {
            let _ = self.engine.send_call_signal(&pk, CallSignal::new(&cid, CallAction::Decline)).await;
        }
        self.end(EndReason::Declined).await;
    }

    /// Desliga.
    pub async fn hang_up(self: &Arc<Self>) {
        let alvo = self
            .active
            .lock()
            .await
            .as_ref()
            .map(|a| (a.link.pk.clone(), a.cid.clone(), a.outgoing, a.state.clone(), a.video));
        if let Some((pk, cid, saindo, estado, video)) = alvo {
            let _ = self.engine.send_call_signal(&pk, CallSignal::new(&cid, CallAction::End)).await;
            // Desligar antes de a pessoa atender é chamada perdida para ela.
            if saindo && estado == CallState::Calling {
                let _ = self.engine.report_missed_call(&pk, video).await;
            }
        }
        self.end(EndReason::HungUp).await;
    }

    /// Liga e desliga o microfone (e avisa o outro lado, como o celular).
    pub async fn set_muted(&self, mudo: bool) {
        let links: Vec<Arc<Link>> = {
            let g = self.active.lock().await;
            if let Some(a) = g.as_ref() {
                a.muted.store(mudo, Ordering::Relaxed);
                if let Some(c) = &a.capture {
                    c.set_muted(mudo);
                }
                vec![Arc::clone(&a.link)]
            } else {
                drop(g);
                let r = self.room.lock().await;
                match r.as_ref() {
                    Some(r) => {
                        r.muted.store(mudo, Ordering::Relaxed);
                        if let Some(c) = &r.capture {
                            c.set_muted(mudo);
                        }
                        r.links.values().cloned().collect()
                    }
                    None => Vec::new(),
                }
            }
        };
        for l in links {
            l.send(serde_json::json!({ "t": "mic", "on": !mudo })).await;
        }
    }

    async fn call_matches(&self, from: &str, cid: &str) -> bool {
        self.active.lock().await.as_ref().is_some_and(|a| a.link.pk == from && (cid.is_empty() || a.cid == cid))
    }

    /// Encerra a chamada, solta microfone e alto-falante e grava no histórico.
    pub async fn end(self: &Arc<Self>, reason: EndReason) {
        let anterior = self.active.lock().await.take();
        self.tone(None);
        let Some(a) = anterior else { return };
        *self.ending.lock().await = Some((a.link.pk.clone(), reason, Instant::now()));
        if let Some((_, parar)) = &a.sharing {
            parar.store(true, Ordering::Relaxed);
        }
        a.link.conn.close().await;
        drop(a.capture);
        drop(a.playback);
        if a.t0.is_some() || a.outgoing {
            if let Some(p) = self.sounds.player() {
                p.beep(Tone::Hangup);
            }
        }
        // Histórico, com os mesmos estados do celular.
        let status = match (a.t0, reason) {
            (Some(_), _) => "ok",
            (None, EndReason::Declined) => "declined",
            (None, EndReason::Busy) => "busy",
            (None, EndReason::HungUp) if a.outgoing => "cancelled",
            _ => "missed",
        };
        let _ = self.engine.log_call(&CallLog {
            id: random_hex(8),
            peer: a.link.pk.clone(),
            dir: if a.outgoing { "out" } else { "in" }.into(),
            video: a.video,
            ts: a.ts,
            dur: a.t0.map(|t| t.elapsed().as_secs() as i64).unwrap_or(0),
            status: status.into(),
        });
        tracing::info!("chamada encerrada: {}", reason.label());
    }

    /// Escuta a conexão da chamada e cuida do áudio, do vídeo e dos dados.
    fn pump_call(self: &Arc<Self>, link: Arc<Link>, mut eventos: tokio::sync::mpsc::UnboundedReceiver<PeerEvent>) {
        let eu = Arc::clone(self);
        tokio::spawn(async move {
            let Ok(mut dec) = Decoder::new() else { return };
            let mut vad = VoiceDetector::default();
            let mut dec_video: Option<VideoDecoder> = None;
            while let Some(ev) = eventos.recv().await {
                match ev {
                    PeerEvent::State(ConnectionState::Connected) => {
                        link.connected.store(true, Ordering::Relaxed);
                        eu.on_call_connected().await;
                    }
                    PeerEvent::State(ConnectionState::Reconnecting) => {
                        if let Some(a) = eu.active.lock().await.as_mut() {
                            if matches!(a.state, CallState::Active { .. }) {
                                a.state = CallState::Reconnecting;
                            }
                        }
                        // Se não voltar em 10 s, a chamada caiu.
                        let eu2 = Arc::clone(&eu);
                        let l2 = Arc::clone(&link);
                        tokio::spawn(async move {
                            tokio::time::sleep(Duration::from_secs(10)).await;
                            let caiu = eu2.active.lock().await.as_ref().is_some_and(|a| Arc::ptr_eq(&a.link, &l2) && a.state == CallState::Reconnecting);
                            if caiu {
                                eu2.end(EndReason::Failed).await;
                            }
                        });
                    }
                    PeerEvent::State(ConnectionState::Failed) => {
                        eu.end(EndReason::Failed).await;
                        break;
                    }
                    PeerEvent::State(ConnectionState::Closed) => break,
                    PeerEvent::DataOpen => {
                        let (eu2, l2) = (Arc::clone(&eu), Arc::clone(&link));
                        tokio::spawn(async move {
                            tokio::time::sleep(PRIMEIRA_FALA).await;
                            let (cam, mic) = {
                                let g = eu2.active.lock().await;
                                match g.as_ref() {
                                    Some(a) if Arc::ptr_eq(&a.link, &l2) => (a.sharing.is_some(), !a.muted.load(Ordering::Relaxed)),
                                    _ => return,
                                }
                            };
                            l2.send(serde_json::json!({ "t": "cam", "on": cam })).await;
                            l2.send(serde_json::json!({ "t": "mic", "on": mic })).await;
                        });
                    }
                    PeerEvent::Data(bytes) => eu.on_data(&link, &bytes).await,
                    PeerEvent::Media { kind: MediaKind::Video, data } => {
                        if dec_video.is_none() {
                            dec_video = VideoDecoder::new().ok();
                        }
                        if let Some(d) = dec_video.as_mut() {
                            if let Ok(Some(img)) = d.decode(&data) {
                                let mut slot = link.frame.lock().await;
                                let n = slot.as_ref().map(|(v, _)| v + 1).unwrap_or(1);
                                *slot = Some((n, img));
                                *link.frame_at.lock().unwrap() = Some(Instant::now());
                            }
                        }
                    }
                    PeerEvent::Media { kind: MediaKind::Audio, data } => {
                        let pcm = if data.is_empty() { dec.decode_lost() } else { dec.decode(&data) };
                        if let Ok(pcm) = pcm {
                            link.speaking.store(vad.push(&pcm), Ordering::Relaxed);
                            let g = eu.active.lock().await;
                            match g.as_ref() {
                                Some(a) => {
                                    if let Some(p) = &a.playback {
                                        p.push(&pcm);
                                    }
                                }
                                None => break,
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    /// A conexão fechou: abre microfone e alto-falante.
    async fn on_call_connected(self: &Arc<Self>) {
        let ja = self.active.lock().await.as_ref().is_some_and(|a| a.t0.is_some());
        if ja {
            if let Some(a) = self.active.lock().await.as_mut() {
                if a.state == CallState::Reconnecting {
                    a.state = CallState::Active { since: a.t0.unwrap_or_else(Instant::now) };
                }
            }
            return;
        }
        self.tone(None);
        let saida = Playback::start(None).ok();
        let entrada = Capture::start(None);
        let (conn, mudo) = {
            let mut g = self.active.lock().await;
            let Some(a) = g.as_mut() else { return };
            let agora = Instant::now();
            a.t0 = Some(agora);
            a.state = CallState::Active { since: agora };
            a.playback = saida;
            (Arc::clone(&a.link.conn), Arc::clone(&a.muted))
        };
        match entrada {
            Ok((cap, mut quadros)) => {
                cap.set_muted(mudo.load(Ordering::Relaxed));
                if let Some(a) = self.active.lock().await.as_mut() {
                    a.capture = Some(cap);
                }
                tokio::spawn(async move {
                    let Ok(mut enc) = Encoder::new(AudioQuality::Normal) else { return };
                    while let Some(pcm) = quadros.recv().await {
                        let Ok(pacote) = enc.encode(&pcm) else { continue };
                        if conn.send_audio(&pacote, Duration::from_millis(20)).await.is_err() {
                            break;
                        }
                    }
                });
            }
            Err(e) => {
                // Sem microfone a chamada ainda serve para ouvir; a tela avisa.
                tracing::warn!("sem microfone nesta chamada: {e}");
                if let Some(a) = self.active.lock().await.as_mut() {
                    a.sem_microfone = true;
                }
            }
        }
    }

    /* ---------------------------------------------- transmissão de tela */

    pub fn share_sources(&self) -> Vec<Source> {
        screen::sources().unwrap_or_default()
    }

    /// Começa a transmitir uma tela ou janela para quem está na chamada.
    pub async fn start_sharing(self: &Arc<Self>, source: Source, quality: Quality, fps: Fps) -> Result<(), String> {
        let link = {
            let g = self.active.lock().await;
            let a = g.as_ref().ok_or("não há chamada em andamento")?;
            if a.sharing.is_some() {
                return Err("você já está transmitindo".into());
            }
            Arc::clone(&a.link)
        };
        let conn = Arc::clone(&link.conn);
        let rotulo = source.label();
        let parar = Arc::new(AtomicBool::new(false));
        let intervalo = fps.interval();
        let altura = quality.height();

        // Captura e compressão numa thread do sistema: é trabalho pesado e
        // bloqueante, e no Windows o capturador não pode atravessar threads.
        let (avisa_inicio, inicio_pronto) = tokio::sync::oneshot::channel();
        let (manda, mut recebe) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
        let p = Arc::clone(&parar);
        let pedido = Arc::clone(&conn);
        std::thread::Builder::new()
            .name("captura-de-tela".into())
            .spawn(move || {
                let preparar = || -> Result<(Capturer, VideoEncoder), String> {
                    let cap = Capturer::open(&source).map_err(|e| format!("não deu para abrir a captura: {e}"))?;
                    let primeiro = cap.grab().map_err(|e| format!("não deu para capturar a tela: {e}"))?;
                    let ajustado = primeiro.scale_to_height(altura);
                    let enc = VideoEncoder::new(ajustado.width, ajustado.height, quality, fps)
                        .map_err(|e| format!("não deu para preparar o vídeo: {e}"))?;
                    Ok((cap, enc))
                };
                let (capturador, mut enc) = match preparar() {
                    Ok(v) => {
                        let _ = avisa_inicio.send(Ok(()));
                        v
                    }
                    Err(e) => {
                        let _ = avisa_inicio.send(Err(e));
                        return;
                    }
                };
                let mut n: u64 = 0;
                while !p.load(Ordering::Relaxed) {
                    let inicio = Instant::now();
                    let Ok(quadro) = capturador.grab() else { break };
                    let pronto = quadro.scale_to_height(altura);
                    // Quadro inteiro no começo e sempre que o outro lado pedir.
                    match enc.encode(&pronto.rgba, n == 0 || pedido.take_keyframe_request()) {
                        Ok(pacotes) => {
                            for pkt in pacotes {
                                if manda.blocking_send(pkt.data).is_err() {
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("falha ao comprimir a tela: {e}");
                            break;
                        }
                    }
                    n += 1;
                    let gasto = inicio.elapsed();
                    if gasto < intervalo {
                        std::thread::sleep(intervalo - gasto);
                    }
                }
            })
            .map_err(|e| format!("não deu para iniciar a captura: {e}"))?;

        match inicio_pronto.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err("a captura de tela encerrou antes de começar".into()),
        }
        if let Some(a) = self.active.lock().await.as_mut() {
            a.sharing = Some((rotulo, Arc::clone(&parar)));
        }
        // O celular só mostra imagem depois deste aviso (é o mesmo da câmera).
        link.send(serde_json::json!({ "t": "cam", "on": true })).await;
        tokio::task::spawn(async move {
            while let Some(dados) = recebe.recv().await {
                if conn.send_video(&dados, intervalo).await.is_err() {
                    break;
                }
            }
        });
        Ok(())
    }

    /// Para a transmissão de tela.
    pub async fn stop_sharing(self: &Arc<Self>) {
        let link = {
            let mut g = self.active.lock().await;
            match g.as_mut() {
                Some(a) => {
                    if let Some((_, parar)) = a.sharing.take() {
                        parar.store(true, Ordering::Relaxed);
                    }
                    Some(Arc::clone(&a.link))
                }
                None => None,
            }
        };
        if let Some(l) = link {
            l.send(serde_json::json!({ "t": "cam", "on": false })).await;
        }
    }

    /* ---------------------------------------------- soundboard na chamada */

    /// Toca um som para todos na chamada (ou na sala).
    pub async fn call_sound(self: &Arc<Self>, id: &str) {
        let Some(s) = self.sounds.info(id) else { return };
        self.sounds.play(&s.id);
        self.set_banner("Você", &s.emoji, &s.name, s.dur, None);
        let links: Vec<Arc<Link>> = match self.active.lock().await.as_ref() {
            Some(a) => vec![Arc::clone(&a.link)],
            None => self.room.lock().await.as_ref().map(|r| r.links.values().cloned().collect()).unwrap_or_default(),
        };
        for l in links {
            if !s.builtin {
                let ja = l.sent_sounds.lock().unwrap().contains(&s.id);
                if !ja {
                    let Some(bytes) = self.sounds.bytes(&s.id) else { continue };
                    if bytes.len() > jm_core::media::MAX_SOUND_BYTES {
                        continue;
                    }
                    let n = bytes.len().div_ceil(SND_CHUNK);
                    for (i, pedaco) in bytes.chunks(SND_CHUNK).enumerate() {
                        let b = base64::engine::general_purpose::STANDARD.encode(pedaco);
                        l.send(serde_json::json!({ "t": "sndd", "id": s.id, "i": i, "n": n, "b": b })).await;
                    }
                    l.sent_sounds.lock().unwrap().insert(s.id.clone());
                }
            }
            l.send(serde_json::json!({ "t": "snd", "id": s.id, "n": s.name, "e": s.emoji, "d": s.dur })).await;
        }
    }

    fn set_banner(&self, who: &str, emoji: &str, name: &str, dur: f32, save: Option<(String, f32)>) {
        let ms = (dur * 1000.0) as u64 + 600;
        let ms = if save.is_some() { ms.max(5000) } else { ms };
        *self.banner.lock().unwrap() = Some(SoundBanner {
            who: who.into(),
            emoji: emoji.into(),
            name: name.into(),
            save,
            until: Instant::now() + Duration::from_millis(ms),
        });
    }

    /// Guarda no seu soundboard um som que chegou na chamada.
    pub fn save_banner_sound(&self) -> Option<String> {
        let b = self.banner.lock().unwrap().clone()?;
        let (id, dur) = b.save?;
        let bytes = self.sounds.remote_bytes(&id)?;
        let mime = if bytes.starts_with(b"OggS") { "audio/ogg" } else { "audio/webm" };
        self.engine.save_custom_sound(&bytes, mime, &b.name, &b.emoji, dur as f64).ok()?;
        if let Some(x) = self.banner.lock().unwrap().as_mut() {
            x.save = None;
        }
        Some(b.name)
    }

    /// Mensagem que chegou pelo canal de dados.
    async fn on_data(self: &Arc<Self>, link: &Arc<Link>, bytes: &[u8]) {
        let Ok(d) = serde_json::from_slice::<serde_json::Value>(bytes) else { return };
        match d["t"].as_str() {
            Some("cam") => link.remote_cam.store(d["on"].as_bool().unwrap_or(false), Ordering::Relaxed),
            Some("mic") => link.remote_mic.store(d["on"].as_bool().unwrap_or(true), Ordering::Relaxed),
            Some("sndd") => self.on_sound_data(&link.pk, &d),
            Some("snd") => self.on_sound_play(&link.pk, &d),
            _ => {}
        }
    }

    fn on_sound_data(&self, pk: &str, d: &serde_json::Value) {
        let (Some(id), Some(i), Some(n), Some(b)) = (d["id"].as_str(), d["i"].as_u64(), d["n"].as_u64(), d["b"].as_str()) else {
            return;
        };
        if !jm_core::media::is_custom_sound_id(id) || n == 0 || n > 16 || i >= n || b.len() > 17_000 {
            return;
        }
        let Ok(parte) = base64::engine::general_purpose::STANDARD.decode(b) else { return };
        let chave = format!("{pk}:{id}");
        let completo = {
            let mut m = self.inbound.lock().unwrap();
            let e = m.entry(chave.clone()).or_insert_with(|| (n as u32, vec![None; n as usize]));
            if e.0 != n as u32 || e.1[i as usize].is_some() {
                return;
            }
            e.1[i as usize] = Some(parte);
            if e.1.iter().all(|p| p.is_some()) {
                let partes = m.remove(&chave).map(|(_, v)| v).unwrap_or_default();
                Some(partes.into_iter().flatten().flatten().collect::<Vec<u8>>())
            } else {
                if m.len() > 20 {
                    if let Some(k) = m.keys().next().cloned() {
                        m.remove(&k);
                    }
                }
                None
            }
        };
        // Só guarda se o conteúdo bate com o id (o id é o hash dos bytes).
        if let Some(todos) = completo {
            if todos.len() <= jm_core::media::MAX_SOUND_BYTES && jm_core::media::sound_id(&todos) == id {
                self.sounds.add_remote(id, todos);
            }
        }
    }

    fn on_sound_play(&self, pk: &str, d: &serde_json::Value) {
        let Some(id) = d["id"].as_str().filter(|s| s.len() <= 20) else { return };
        {
            // Anti-repetição: no máximo um som a cada 300 ms por pessoa.
            let mut m = self.last_snd.lock().unwrap();
            if m.get(pk).is_some_and(|t| t.elapsed() < Duration::from_millis(300)) {
                return;
            }
            m.insert(pk.into(), Instant::now());
        }
        let conhecido = self.sounds.info(id);
        let (nome, emoji, dur) = match &conhecido {
            Some(s) => (s.name.clone(), s.emoji.clone(), s.dur),
            None => {
                let n: String = d["n"].as_str().unwrap_or("").trim().chars().take(24).collect();
                let e = d["e"].as_str().filter(|e| emoji_ok(e)).unwrap_or("🔊").to_string();
                let dur = d["d"].as_f64().unwrap_or(2.0).clamp(0.5, 10.0) as f32;
                (if n.is_empty() { "Som".into() } else { n }, e, dur)
            }
        };
        self.sounds.play(id);
        let save = (conhecido.is_none() && self.sounds.remote_bytes(id).is_some()).then(|| (id.to_string(), dur));
        let quem = first_name(&self.engine.name_of(pk));
        self.set_banner(&quem, &emoji, &nome, dur, save);
    }

    /* ------------------------------------------------------ salas de voz */

    /// Entra numa sala de voz.
    pub async fn join_room(self: &Arc<Self>, lid: &str, ch: &str) -> Result<(), String> {
        if self.active.lock().await.is_some() {
            return Err("Termine a chamada atual primeiro".into());
        }
        if self.in_room(lid, ch).await {
            return Ok(());
        }
        if self.room.lock().await.is_some() {
            self.leave_room().await;
        }
        let g = self.engine.group(lid).ok_or("grupo não encontrado")?;
        let eu = self.engine.pk_hex();
        let presentes: Vec<String> = self
            .engine
            .room_members(lid, ch)
            .into_iter()
            .filter(|pk| *pk != eu && g.meta.members.contains(pk))
            .collect();
        if presentes.len() >= MAX_ROOM {
            return Err(format!("Sala cheia (até {MAX_ROOM} pessoas)"));
        }
        let (audio_tx, _) = tokio::sync::broadcast::channel::<Arc<Vec<u8>>>(64);
        let muted = Arc::new(AtomicBool::new(false));
        let me_speaking = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (capture, sem_microfone) = match Capture::start(None) {
            Ok((cap, mut quadros)) => {
                // Microfone → Opus, uma vez só, para todos da sala.
                let tx = audio_tx.clone();
                let falando = Arc::clone(&me_speaking);
                tokio::spawn(async move {
                    let Ok(mut enc) = Encoder::new(AudioQuality::Normal) else { return };
                    let mut vad = VoiceDetector::default();
                    while let Some(pcm) = quadros.recv().await {
                        falando.store(vad.push(&pcm), Ordering::Relaxed);
                        if let Ok(p) = enc.encode(&pcm) {
                            let _ = tx.send(Arc::new(p));
                        }
                    }
                });
                (Some(cap), false)
            }
            Err(e) => {
                tracing::warn!("sala de voz sem microfone: {e}");
                (None, true)
            }
        };
        *self.room.lock().await = Some(Room {
            lid: lid.into(),
            ch: ch.into(),
            links: HashMap::new(),
            playbacks: HashMap::new(),
            capture,
            muted,
            me_speaking,
            sem_microfone,
            since: Instant::now(),
            stop: Arc::clone(&stop),
            audio_tx,
        });
        let _ = self.engine.room_presence(lid, ch, true).await;
        // Presença a cada 8 s: quem some por mais de 26 s sai da lista.
        let eng = Arc::clone(&self.engine);
        let (l2, c2) = (lid.to_string(), ch.to_string());
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(8)).await;
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let _ = eng.room_presence(&l2, &c2, true).await;
            }
        });
        for pk in presentes {
            self.connect_room_peer(&pk).await;
        }
        Ok(())
    }

    /// Sai da sala de voz.
    pub async fn leave_room(self: &Arc<Self>) {
        let Some(r) = self.room.lock().await.take() else { return };
        r.stop.store(true, Ordering::Relaxed);
        let sala = RoomRef { lid: r.lid.clone(), ch: r.ch.clone() };
        for (pk, l) in &r.links {
            let sig = CallSignal { room: Some(sala.clone()), ..CallSignal::new("room", CallAction::End) };
            let _ = self.engine.send_call_signal(pk, sig).await;
            l.conn.close().await;
        }
        let _ = self.engine.room_presence(&r.lid, &r.ch, false).await;
        drop(r.capture);
    }

    /// A lista de presença da sala mudou (vem do motor).
    pub async fn on_room_update(self: &Arc<Self>, lid: &str, ch: &str, members: &[String]) {
        let (novos, sumidos) = {
            let g = self.room.lock().await;
            let Some(r) = g.as_ref().filter(|r| r.lid == lid && r.ch == ch) else { return };
            let eu = self.engine.pk_hex();
            let novos: Vec<String> = members.iter().filter(|pk| **pk != eu && !r.links.contains_key(*pk)).cloned().collect();
            let sumidos: Vec<String> = r.links.keys().filter(|pk| !members.contains(pk)).cloned().collect();
            (novos, sumidos)
        };
        for pk in sumidos {
            self.drop_room_peer(&pk).await;
        }
        for pk in novos {
            self.connect_room_peer(&pk).await;
        }
    }

    /// Conecta a alguém da sala. Só quem tem a chave menor faz a proposta.
    async fn connect_room_peer(self: &Arc<Self>, pk: &str) {
        let eu = self.engine.pk_hex();
        let (lid, ch, n) = {
            let g = self.room.lock().await;
            let Some(r) = g.as_ref() else { return };
            if r.links.contains_key(pk) {
                return;
            }
            (r.lid.clone(), r.ch.clone(), r.links.len())
        };
        if n + 1 >= MAX_ROOM || eu.as_str() >= pk {
            return; // o outro lado é quem oferta
        }
        let Ok((conn, eventos)) = Peer::new(self.config()).await else { return };
        let conn = Arc::new(conn);
        let Ok(oferta) = conn.create_offer().await else { return };
        let link = Link::new(pk, Arc::clone(&conn));
        if let Some(r) = self.room.lock().await.as_mut() {
            r.links.insert(pk.into(), Arc::clone(&link));
        }
        self.pump_room(link, eventos);
        let sig = CallSignal { sdp: Some(oferta), room: Some(RoomRef { lid, ch }), ..CallSignal::new("room", CallAction::Offer) };
        let _ = self.engine.send_call_signal(pk, sig).await;
    }

    async fn drop_room_peer(&self, pk: &str) {
        let tirado = {
            let mut g = self.room.lock().await;
            match g.as_mut() {
                Some(r) => {
                    r.playbacks.remove(pk);
                    r.links.remove(pk)
                }
                None => None,
            }
        };
        if let Some(l) = tirado {
            l.conn.close().await;
        }
    }

    async fn on_room_signal(self: &Arc<Self>, from: &str, sig: CallSignal) {
        let Some(sala) = sig.room.clone() else { return };
        let ok = {
            let g = self.room.lock().await;
            g.as_ref().is_some_and(|r| r.lid == sala.lid && r.ch == sala.ch)
        };
        let membro = self.engine.group(&sala.lid).is_some_and(|g| g.meta.members.iter().any(|m| m == from));
        if !ok || !membro {
            return;
        }
        match sig.a {
            CallAction::Offer => {
                let Some(sdp) = sig.sdp else { return };
                self.drop_room_peer(from).await;
                let Ok((conn, eventos)) = Peer::new(self.config()).await else { return };
                let conn = Arc::new(conn);
                let Ok(resposta) = conn.accept_offer(&sdp).await else { return };
                let link = Link::new(from, Arc::clone(&conn));
                if let Some(r) = self.room.lock().await.as_mut() {
                    if r.links.len() + 1 >= MAX_ROOM {
                        conn.close().await;
                        return;
                    }
                    r.links.insert(from.into(), Arc::clone(&link));
                }
                self.pump_room(link, eventos);
                let s = CallSignal { sdp: Some(resposta), room: Some(sala), ..CallSignal::new("room", CallAction::Answer) };
                let _ = self.engine.send_call_signal(from, s).await;
            }
            CallAction::Answer => {
                let link = self.room.lock().await.as_ref().and_then(|r| r.links.get(from).cloned());
                if let (Some(l), Some(sdp)) = (link, sig.sdp) {
                    let _ = l.conn.accept_answer(&sdp).await;
                }
            }
            CallAction::End => self.drop_room_peer(from).await,
            _ => {}
        }
    }

    fn pump_room(self: &Arc<Self>, link: Arc<Link>, mut eventos: tokio::sync::mpsc::UnboundedReceiver<PeerEvent>) {
        let eu = Arc::clone(self);
        tokio::spawn(async move {
            let Ok(mut dec) = Decoder::new() else { return };
            let mut vad = VoiceDetector::default();
            while let Some(ev) = eventos.recv().await {
                match ev {
                    PeerEvent::State(ConnectionState::Connected) => {
                        if link.connected.swap(true, Ordering::Relaxed) {
                            continue;
                        }
                        // Alto-falante próprio para esta pessoa; o sistema mistura.
                        let saida = Playback::start(None).ok();
                        let mut rx = {
                            let mut g = eu.room.lock().await;
                            let Some(r) = g.as_mut() else { break };
                            if let Some(p) = saida {
                                r.playbacks.insert(link.pk.clone(), p);
                            }
                            r.audio_tx.subscribe()
                        };
                        let conn = Arc::clone(&link.conn);
                        tokio::spawn(async move {
                            while let Ok(p) = rx.recv().await {
                                if conn.send_audio(&p, Duration::from_millis(20)).await.is_err() {
                                    break;
                                }
                            }
                        });
                    }
                    PeerEvent::State(ConnectionState::Failed) | PeerEvent::State(ConnectionState::Closed) => {
                        eu.drop_room_peer(&link.pk).await;
                        break;
                    }
                    PeerEvent::DataOpen => {
                        let (eu2, l2) = (Arc::clone(&eu), Arc::clone(&link));
                        tokio::spawn(async move {
                            tokio::time::sleep(PRIMEIRA_FALA).await;
                            let mic = !eu2.room.lock().await.as_ref().map(|r| r.muted.load(Ordering::Relaxed)).unwrap_or(false);
                            l2.send(serde_json::json!({ "t": "cam", "on": false })).await;
                            l2.send(serde_json::json!({ "t": "mic", "on": mic })).await;
                        });
                    }
                    PeerEvent::Data(bytes) => eu.on_data(&link, &bytes).await,
                    PeerEvent::Media { kind: MediaKind::Audio, data } => {
                        let pcm = if data.is_empty() { dec.decode_lost() } else { dec.decode(&data) };
                        if let Ok(pcm) = pcm {
                            link.speaking.store(vad.push(&pcm), Ordering::Relaxed);
                            let g = eu.room.lock().await;
                            match g.as_ref() {
                                Some(r) => {
                                    if let Some(p) = r.playbacks.get(&link.pk) {
                                        p.push(&pcm);
                                    }
                                }
                                None => break,
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}

/// Quanto tempo a chamada já dura, formatado para a tela.
pub fn duration_label(since: Instant) -> String {
    let s = since.elapsed().as_secs();
    if s >= 3600 {
        format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

fn first_name(n: &str) -> String {
    n.split_whitespace().next().unwrap_or(n).to_string()
}

fn random_hex(bytes: usize) -> String {
    let mut v = vec![0u8; bytes];
    for b in v.iter_mut() {
        *b = rand_byte();
    }
    v.iter().map(|b| format!("{b:02x}")).collect()
}

fn rand_byte() -> u8 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    h.finish() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motivos_de_encerramento_tem_texto() {
        for r in [EndReason::HungUp, EndReason::Declined, EndReason::NoAnswer, EndReason::Busy, EndReason::Failed] {
            assert!(!r.label().is_empty());
        }
        assert!(EndReason::Failed.label().contains("TURN"), "deve explicar o que fazer");
    }

    #[test]
    fn duracao_e_formatada_para_a_tela() {
        assert_eq!(duration_label(Instant::now()), "00:00");
    }

    #[test]
    fn primeiro_nome() {
        assert_eq!(first_name("Ana Souza"), "Ana");
        assert_eq!(first_name("Beto"), "Beto");
    }
}
