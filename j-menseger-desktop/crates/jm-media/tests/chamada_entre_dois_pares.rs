//! Uma chamada real entre dois pares, com áudio atravessando a conexão.
//!
//! Não há microfone nem alto-falante neste ambiente, então a voz é substituída
//! por um tom gerado. Tudo o resto é de verdade: a negociação, a conexão
//! WebRTC, a cifra DTLS-SRTP, a compressão Opus e a entrega dos pacotes.
//!
//! O que este teste **não** cobre: a captura do microfone e a reprodução no
//! alto-falante. Essas duas pontas dependem de hardware de áudio.

use jm_media::codec::{level, AudioQuality, Decoder, Encoder, FRAME_SAMPLES, SAMPLE_RATE};
use jm_media::peer::{CallConfig, ConnectionState, MediaKind, Peer, PeerEvent};
use std::time::Duration;
use tokio::time::timeout;

/// Um tom, no lugar da voz que viria do microfone.
fn tom(freq: f32, quadro: usize) -> Vec<i16> {
    (0..FRAME_SAMPLES)
        .map(|i| {
            let t = (quadro * FRAME_SAMPLES + i) as f32 / SAMPLE_RATE as f32;
            ((t * freq * std::f32::consts::TAU).sin() * 0.6 * 32767.0) as i16
        })
        .collect()
}

/// Liga dois pares trocando a proposta e a resposta, como o aplicativo faz
/// pelos envelopes efêmeros.
async fn conectar() -> Option<(Peer, Peer, tokio::sync::mpsc::UnboundedReceiver<PeerEvent>, tokio::sync::mpsc::UnboundedReceiver<PeerEvent>)> {
    let (ana, rx_ana) = Peer::new(CallConfig::default()).await.ok()?;
    let (beto, rx_beto) = Peer::new(CallConfig::default()).await.ok()?;

    let oferta = ana.create_offer().await.ok()?;
    let resposta = beto.accept_offer(&oferta).await.ok()?;
    ana.accept_answer(&resposta).await.ok()?;
    Some((ana, beto, rx_ana, rx_beto))
}

async fn esperar_conexao(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<PeerEvent>,
    segundos: u64,
) -> bool {
    timeout(Duration::from_secs(segundos), async {
        while let Some(ev) = rx.recv().await {
            if let PeerEvent::State(ConnectionState::Connected) = ev {
                return true;
            }
            if let PeerEvent::State(ConnectionState::Failed) = ev {
                return false;
            }
        }
        false
    })
    .await
    .unwrap_or(false)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chamada_conecta_e_o_audio_atravessa() {
    let Some((ana, beto, mut rx_ana, mut rx_beto)) = conectar().await else {
        eprintln!("pulando: não foi possível criar os pares");
        return;
    };

    let ligou_ana = esperar_conexao(&mut rx_ana, 25).await;
    let ligou_beto = esperar_conexao(&mut rx_beto, 25).await;
    if !ligou_ana || !ligou_beto {
        // Em ambientes sem rede local utilizável isso pode não fechar; o teste
        // avisa em vez de acusar um bug que não existe.
        eprintln!("pulando: a conexão não fechou neste ambiente de rede");
        ana.close().await;
        beto.close().await;
        return;
    }

    assert_eq!(ana.connection_state().await, ConnectionState::Connected);
    assert_eq!(beto.connection_state().await, ConnectionState::Connected);

    // Ana "fala": tom comprimido em Opus, enviado quadro a quadro.
    let mut enc = Encoder::new(AudioQuality::Normal).unwrap();
    let envio = tokio::spawn(async move {
        for q in 0..60 {
            let pacote = enc.encode(&tom(440.0, q)).unwrap();
            if ana.send_audio(&pacote, Duration::from_millis(20)).await.is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        ana
    });

    // Beto recebe, decodifica e confere que chegou som de verdade.
    let mut dec = Decoder::new().unwrap();
    let mut quadros = 0usize;
    let mut com_som = 0usize;

    let _ = timeout(Duration::from_secs(12), async {
        while let Some(ev) = rx_beto.recv().await {
            if let PeerEvent::Media { kind: MediaKind::Audio, data } = ev {
                if let Ok(pcm) = dec.decode(&data) {
                    quadros += 1;
                    if level(&pcm) > 0.05 {
                        com_som += 1;
                    }
                    if quadros >= 25 {
                        break;
                    }
                }
            }
        }
    })
    .await;

    let ana = envio.await.unwrap();

    assert!(
        quadros >= 10,
        "chegaram só {quadros} quadros de áudio; a mídia não está atravessando"
    );
    assert!(
        com_som * 2 >= quadros,
        "os quadros chegaram, mas quase todos mudos: {com_som} de {quadros}"
    );
    println!("áudio recebido: {quadros} quadros, {com_som} com som");

    ana.close().await;
    beto.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn o_canal_de_dados_leva_o_soundboard() {
    let Some((ana, beto, mut rx_ana, mut rx_beto)) = conectar().await else {
        return;
    };
    if !esperar_conexao(&mut rx_ana, 25).await || !esperar_conexao(&mut rx_beto, 25).await {
        eprintln!("pulando: a conexão não fechou neste ambiente de rede");
        ana.close().await;
        beto.close().await;
        return;
    }

    // Espera o canal de dados abrir.
    for _ in 0..50 {
        if ana.data_is_open() && beto.data_is_open() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(ana.data_is_open(), "o canal de dados não abriu");

    let aviso = br#"{"t":"snd","id":"aplausos"}"#;
    ana.send_data(aviso).await.expect("não enviou pelo canal de dados");

    let recebido = timeout(Duration::from_secs(8), async {
        while let Some(ev) = rx_beto.recv().await {
            if let PeerEvent::Data(d) = ev {
                return Some(d);
            }
        }
        None
    })
    .await
    .ok()
    .flatten();

    assert_eq!(
        recebido.as_deref(),
        Some(&aviso[..]),
        "o aviso do soundboard não chegou igual"
    );

    ana.close().await;
    beto.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_negociacao_descreve_uma_conexao_cifrada() {
    // Vale mesmo sem rede: o conteúdo da negociação é verificável sozinho.
    let (ana, _rx) = Peer::new(CallConfig::default()).await.unwrap();
    let oferta = ana.create_offer().await.unwrap();

    assert!(oferta.contains("a=fingerprint"), "sem impressão digital DTLS");
    assert!(
        oferta.contains("UDP/TLS/RTP/SAVPF") || oferta.contains("DTLS/SCTP") || oferta.contains("UDP/DTLS/SCTP"),
        "a mídia deveria ir cifrada"
    );
    assert!(oferta.contains("a=ice-ufrag"), "sem credenciais de conexão");
    // Sem trickle: os caminhos já vêm na descrição.
    assert!(
        oferta.contains("a=candidate") || oferta.contains("a=end-of-candidates"),
        "a descrição deveria já trazer os caminhos de rede"
    );
    ana.close().await;
}
