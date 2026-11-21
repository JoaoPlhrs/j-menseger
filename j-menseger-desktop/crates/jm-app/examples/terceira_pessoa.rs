//! Ferramenta de teste: uma terceira pessoa ("Caio") que manda, para o código
//! dado, um texto, uma foto, uma mensagem de voz e um som próprio do
//! soundboard. Serve para conferir na interface como chega cada tipo de
//! mensagem, inclusive o pedido de conversa de quem não é contato.
//!
//! Uso: `cargo run --example terceira_pessoa -p jm-app -- <pasta> <relay> <npub>`

use jm_core::engine::{Engine, MediaMeta, Settings};
use jm_core::messages::{MediaKind, SoundMeta};
use jm_core::store::Store;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let (pasta, relay, alvo) = match (args.next(), args.next(), args.next()) {
        (Some(p), Some(r), Some(a)) => (p, r, a),
        _ => {
            eprintln!("uso: terceira_pessoa <pasta> <relay> <npub>");
            std::process::exit(2);
        }
    };
    std::fs::create_dir_all(&pasta).expect("pasta");
    let db = std::path::Path::new(&pasta).join("jm.db");
    let store = Store::open(&db).expect("banco");
    let keys = match store.get_kv::<String>("account_sk").ok().flatten() {
        Some(sk) => jm_core::identity::keys_from_any(&sk).expect("chave"),
        None => {
            let w = jm_core::identity::new_words().expect("palavras");
            Engine::create_identity(&store, &w, "Caio Teste").expect("conta")
        }
    };
    let mut s: Settings = store.get_kv("settings").ok().flatten().unwrap_or_default();
    s.relays = vec![relay];
    store.set_kv("settings", &s).expect("ajustes");
    let (e, _rx) = Engine::open(keys, store).expect("motor");
    e.start().await.expect("conectar");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let pk = jm_core::identity::parse_pubkey(&alvo).expect("código").to_hex();
    let chat = format!("dm:{pk}");
    e.add_contact(&alvo).await.expect("contato");
    e.send_text(&chat, "Oi! Sou o Caio, peguei seu código com o Beto.").await.expect("texto");

    // Foto: o próprio ícone do app, preparado como o app prepara.
    let foto = jm_media::photo::prepare(include_bytes!("../recursos/icone.png")).expect("foto");
    let meta = MediaMeta { kind: Some(MediaKind::Image), mime: foto.mime.into(), w: Some(foto.w), h: Some(foto.h), th: Some(foto.th.clone()), ..Default::default() };
    e.send_media(&chat, foto.bytes, meta).await.expect("foto");

    // Voz: 3 s de uma melodia simples, em Ogg Opus a 24 kbps (como o celular).
    let pcm: Vec<f32> = (0..48_000 * 3)
        .map(|i| {
            let t = i as f32 / 48_000.0;
            let f = [330.0, 392.0, 494.0][((t * 2.0) as usize) % 3];
            (t * f * std::f32::consts::TAU).sin() * 0.4 * (1.0 - (t * 2.0).fract() * 0.6)
        })
        .collect();
    let ogg = jm_media::audiofile::encode_ogg_opus(&pcm, 24_000, false).expect("opus");
    let wf = jm_media::audiofile::waveform_of(&pcm, 48);
    let meta = MediaMeta { kind: Some(MediaKind::Audio), mime: "audio/ogg".into(), dur: Some(3.0), wf: Some(wf), ..Default::default() };
    e.send_media(&chat, ogg, meta).await.expect("voz");

    // Som próprio do soundboard (vai como áudio com nome e emoji).
    let som: Vec<f32> = (0..48_000).map(|i| ((i as f32 / 48_000.0) * 880.0 * std::f32::consts::TAU).sin() * 0.3).collect();
    let ogg = jm_media::audiofile::encode_ogg_opus(&som, 64_000, true).expect("opus");
    let meta = MediaMeta { kind: Some(MediaKind::Audio), mime: "audio/ogg".into(), dur: Some(1.0), sb: Some(SoundMeta { n: "Apito".into(), e: "📯".into() }), ..Default::default() };
    e.send_media(&chat, ogg, meta).await.expect("som");

    // Os blocos saem em segundo plano: espera terminarem.
    tokio::time::sleep(Duration::from_secs(8)).await;
    println!("enviado");
    e.shutdown().await;
}
