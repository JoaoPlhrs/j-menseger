//! Chamada de verdade entre o PC e o WebRTC do celular.
//!
//! O outro lado é o Chromium (o motor da WebView do Android onde o
//! aplicativo de celular roda) executando a classe `Peer` de `calls.js`:
//! canal de dados negociado com id 0, ICE sem trickle, transceptores de áudio
//! e vídeo. Precisa de Node e Playwright; sem eles, o teste é pulado.
//!
//! O que é conferido, nos três sentidos (celular liga com vídeo, PC liga,
//! celular liga só voz): a conexão fecha dos dois lados, a voz atravessa nos
//! dois sentidos, a imagem do PC é decodificada no celular, a câmera do
//! celular é decodificada no PC, e as mensagens do canal de dados chegam como
//! texto JSON — o celular descarta mensagens binárias em silêncio.

use std::path::PathBuf;
use std::process::Command;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rodar(cenario: &str) -> Option<serde_json::Value> {
    let ponte = raiz().join("target/debug/examples/ponte_webrtc");
    if !ponte.exists() {
        eprintln!("pulando: compile antes com `cargo build -p jm-media --example ponte_webrtc`");
        return None;
    }
    let npm_root = Command::new("npm").args(["root", "-g"]).output().ok()?;
    let modulos = String::from_utf8_lossy(&npm_root.stdout).trim().to_string();
    if !PathBuf::from(&modulos).join("playwright").exists() {
        eprintln!("pulando: Playwright não está instalado");
        return None;
    }
    let out = Command::new("node")
        .arg(raiz().join("tools/webrtc/chamada-com-o-celular.cjs"))
        .arg(&ponte)
        .arg(cenario)
        .env("NODE_PATH", &modulos)
        .output()
        .ok()?;
    let texto = String::from_utf8_lossy(&out.stdout);
    let json = &texto[texto.find('{')?..];
    serde_json::from_str(json).ok()
}

fn conferir_basico(r: &serde_json::Value) {
    assert_eq!(r["navegador_conectou"], "connected", "{r}");
    assert_eq!(r["pc_conectou"], true);
    assert!(r["pc_recebeu_audio_pacotes"].as_u64().unwrap_or(0) > 200, "a voz do celular não chegou ao PC: {r}");
    assert!(r["pc_nivel_audio"].as_f64().unwrap_or(0.0) > 0.05, "a voz chegou muda");
    assert!(r["navegador_recebeu_audio"].as_u64().unwrap_or(0) > 100, "a voz do PC não chegou ao celular");
    assert!(r["navegador_quadros_video"].as_u64().unwrap_or(0) > 20, "a imagem do PC não apareceu no celular");
    assert_eq!(r["tipos_das_mensagens"], serde_json::json!(["string"]), "o celular só entende texto no canal de dados");
    assert_eq!(r["mensagens_descartadas"], 0);
    let do_pc = r["navegador_recebeu_do_pc"].as_array().unwrap();
    assert!(do_pc.iter().any(|m| m["t"] == "cam" && m["on"] == true), "aviso de imagem do PC não chegou");
    let do_cel = r["pc_recebeu_do_navegador"].as_array().unwrap();
    assert!(do_cel.iter().any(|m| m["t"] == "snd" && m["id"] == "aplausos"), "som do soundboard do celular não chegou");
    assert!(do_cel.iter().any(|m| m["t"] == "mic" && m["on"] == false), "aviso de microfone do celular não chegou");
}

#[test]
fn celular_liga_em_video_para_o_pc() {
    let Some(r) = rodar("celular-video") else { return };
    conferir_basico(&r);
    assert!(r["pc_video_decodificado"].as_u64().unwrap_or(0) > 50, "a câmera do celular não apareceu no PC: {r}");
    assert_eq!(r["pc_video_tamanho"], serde_json::json!([640, 480]));
}

#[test]
fn pc_liga_para_o_celular() {
    let Some(r) = rodar("pc-oferta") else { return };
    conferir_basico(&r);
}

#[test]
fn celular_liga_so_voz_para_o_pc() {
    let Some(r) = rodar("celular-oferta") else { return };
    conferir_basico(&r);
}
