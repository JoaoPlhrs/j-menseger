//! Lado PC de uma chamada de teste contra o navegador (o mesmo WebRTC do
//! celular). Fala por linhas JSON na entrada e na saída padrão.
//!
//! `ponte_webrtc resposta` — lê a proposta do navegador e devolve a resposta.
//! `ponte_webrtc oferta`   — manda a proposta e lê a resposta do navegador.
//!
//! Depois de conectar: manda `{t:'cam'}` e `{t:'mic'}` pelo canal de dados
//! (em texto, como o celular espera), um tom em Opus e imagem em VP8, e
//! relata o que chegou do outro lado.

use jm_media::codec::{level, AudioQuality, Decoder, Encoder, FRAME_SAMPLES, SAMPLE_RATE};
use jm_media::peer::{CallConfig, ConnectionState, MediaKind, Peer, PeerEvent};
use jm_media::screen::{Fps, Quality};
use jm_media::video::{VideoDecoder, VideoEncoder};
use std::io::BufRead;
use std::time::Duration;

fn saida(v: serde_json::Value) {
    println!("{v}");
}

fn ler_linha() -> serde_json::Value {
    let mut l = String::new();
    std::io::stdin().lock().read_line(&mut l).expect("entrada fechada");
    serde_json::from_str(&l).expect("linha inválida")
}

#[tokio::main]
async fn main() {
    let modo = std::env::args().nth(1).unwrap_or_else(|| "resposta".into());
    // Sem STUN: o teste é local e servidores de fora não são alcançáveis aqui.
    let (peer, mut ev) = Peer::new(CallConfig::default()).await.expect("par");
    if modo == "oferta" {
        let o = peer.create_offer().await.expect("oferta");
        saida(serde_json::json!({ "offer": o }));
        let r = ler_linha();
        peer.accept_answer(r["sdp"].as_str().unwrap()).await.expect("resposta");
    } else {
        let o = ler_linha();
        let r = peer.accept_offer(o["sdp"].as_str().unwrap()).await.expect("aceitar oferta");
        saida(serde_json::json!({ "answer": r }));
    }

    let peer = std::sync::Arc::new(peer);
    let fim = tokio::time::Instant::now() + Duration::from_secs(14);
    let mut dec = Decoder::new().unwrap();
    let mut vdec = VideoDecoder::new().unwrap();
    let mut decodificados = 0u32;
    let mut tamanho = (0u32, 0u32);
    let (mut pacotes_audio, mut pico, mut quadros_video) = (0u32, 0f32, 0u32);
    let mut conectado = false;

    // Tom e imagem, mandados em paralelo quando conectar.
    let p2 = std::sync::Arc::clone(&peer);
    let mandar = tokio::spawn(async move {
        let mut enc = Encoder::new(AudioQuality::Normal).unwrap();
        let mut venc = VideoEncoder::new(320, 240, Quality::P720, Fps::Fps30).unwrap();
        let mut rgba = vec![0u8; 320 * 240 * 4];
        for n in 0u32.. {
            let pcm: Vec<i16> = (0..FRAME_SAMPLES)
                .map(|i| {
                    let t = (n as usize * FRAME_SAMPLES + i) as f32 / SAMPLE_RATE as f32;
                    ((t * 440.0 * std::f32::consts::TAU).sin() * 0.5 * 32767.0) as i16
                })
                .collect();
            if let Ok(p) = enc.encode(&pcm) {
                let _ = p2.send_audio(&p, Duration::from_millis(20)).await;
            }
            if n % 3 == 0 {
                // Barras que andam: o navegador precisa decodificar quadros novos.
                for (i, px) in rgba.chunks_mut(4).enumerate() {
                    let x = (i % 320) as u32;
                    let v = if ((x + n) / 20) % 2 == 0 { 230 } else { 30 };
                    px.copy_from_slice(&[v, 120, 255 - v, 255]);
                }
                if let Ok(pkts) = venc.encode(&rgba, n == 0 || p2.take_keyframe_request()) {
                    for p in pkts {
                        let _ = p2.send_video(&p.data, Duration::from_millis(60)).await;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    });

    while tokio::time::Instant::now() < fim {
        let Ok(Some(e)) = tokio::time::timeout(Duration::from_millis(500), ev.recv()).await else { continue };
        match e {
            PeerEvent::State(ConnectionState::Connected) if !conectado => {
                conectado = true;
                saida(serde_json::json!({ "ev": "connected" }));
            }
            PeerEvent::State(ConnectionState::Failed) => {
                saida(serde_json::json!({ "ev": "failed" }));
                break;
            }
            PeerEvent::DataOpen => {
                peer.send_text(r#"{"t":"cam","on":true}"#).await.ok();
                peer.send_text(r#"{"t":"mic","on":true}"#).await.ok();
                saida(serde_json::json!({ "ev": "data_open" }));
            }
            PeerEvent::Data(b) => saida(serde_json::json!({ "ev": "data", "text": String::from_utf8_lossy(&b) })),
            PeerEvent::Media { kind: MediaKind::Audio, data } => {
                if let Ok(pcm) = dec.decode(&data) {
                    pacotes_audio += 1;
                    pico = pico.max(level(&pcm));
                }
            }
            PeerEvent::Media { kind: MediaKind::Video, data } => {
                quadros_video += 1;
                if let Ok(Some(f)) = vdec.decode(&data) {
                    decodificados += 1;
                    tamanho = (f.width, f.height);
                }
            }
            _ => {}
        }
    }
    mandar.abort();
    saida(serde_json::json!({ "ev": "fim", "audio_pacotes": pacotes_audio, "audio_nivel": pico, "video_pacotes": quadros_video, "video_decodificados": decodificados, "video_tamanho": [tamanho.0, tamanho.1] }));
    peer.close().await;
}
