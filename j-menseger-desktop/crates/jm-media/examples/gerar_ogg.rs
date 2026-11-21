//! Gera um Ogg Opus como o PC manda (voz a 24 kbps e som a 64 kbps), para
//! conferir num navegador que o celular consegue tocar.
fn main() {
    let dir = std::env::args().nth(1).expect("diretório de saída");
    let tom: Vec<f32> = (0..48_000 * 3).map(|i| ((i as f32) * 0.0575).sin() * 0.4).collect();
    let voz = jm_media::audiofile::encode_ogg_opus(&tom, 24_000, false).unwrap();
    let som = jm_media::audiofile::encode_ogg_opus(&tom[..48_000 * 2], 64_000, true).unwrap();
    std::fs::write(format!("{dir}/pc-voz.ogg"), voz).unwrap();
    std::fs::write(format!("{dir}/pc-som.ogg"), som).unwrap();
}
