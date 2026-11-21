//! Áudio gravado do jeito que o celular grava.
//!
//! O arquivo de teste foi gravado pelo `MediaRecorder` do Chromium — o mesmo
//! motor da WebView do Android em que o aplicativo de celular roda — com as
//! mesmas opções de `media.js` (`audio/webm;codecs=opus`, 24 kbps, pedaços de
//! 250 ms). O script que gerou está ao lado: `gravar-como-o-celular.cjs`.

use jm_media::{audiofile, container};

fn fixture(nome: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(nome)).unwrap()
}

#[test]
fn mensagem_de_voz_do_celular_abre_no_pc() {
    let webm = fixture("mediarecorder-chromium.webm");
    let s = container::read_webm_opus(&webm).unwrap_or_else(|e| panic!("o WebM do MediaRecorder não abriu: {e:?}"));
    // O MediaRecorder do Chromium a 24 kbps usa quadros de 60 ms: 2,5 s dão
    // uns 41 pacotes. (O decodificador de chamadas, de 20 ms, recusaria todos;
    // por isso arquivos usam um decodificador com espaço para 120 ms.)
    assert!((35..=50).contains(&s.packets.len()), "pacotes: {}", s.packets.len());
    let pcm = audiofile::decode(&webm).expect("não decodificou");
    let d = pcm.duration();
    assert!((2.2..=2.8).contains(&d), "duração {d:.2} s (gravou 2,5 s)");
    // O dispositivo falso do Chromium toca bipes: precisa haver som de verdade.
    let pico = pcm.samples.iter().fold(0f32, |m, v| m.max(v.abs()));
    assert!(pico > 0.05, "áudio veio mudo (pico {pico})");
}

#[test]
fn o_que_o_pc_grava_volta_igual() {
    // Mensagem de voz feita no PC: Ogg Opus a 24 kbps, como o celular espera.
    let tom: Vec<f32> = (0..48_000 * 2).map(|i| ((i as f32) * 0.0575).sin() * 0.4).collect();
    let ogg = audiofile::encode_ogg_opus(&tom, 24_000, false).unwrap();
    assert!(ogg.len() < 9_000, "2 s a 24 kbps: {} bytes", ogg.len());
    let volta = audiofile::decode(&ogg).unwrap();
    assert!((volta.duration() - 2.0).abs() < 0.03);
}
