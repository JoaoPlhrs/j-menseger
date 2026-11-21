//! GIF animado: conferir antes de mandar e abrir os quadros para mostrar.
//!
//! O GIF vai **do jeito que veio** (não é recodificado): recodificar um GIF
//! em Rust puro perde qualidade e quase sempre aumenta o arquivo. Ele viaja
//! fatiado e cifrado como qualquer foto, então a outra pessoa nunca acessa o
//! Tenor ou o Giphy.

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage};

/// Maior GIF aceito para mandar (cabe no limite de mídia do protocolo).
pub const GIF_MAX: usize = 2_000_000;
/// Quadros demais travam a tela; o resto do GIF é ignorado ao mostrar.
pub const MAX_FRAMES: usize = 240;
/// Memória máxima dos quadros já abertos de um GIF (RGBA).
const MAX_PIXELS_TOTAL: u64 = 60_000_000;

#[derive(Debug, thiserror::Error)]
pub enum GifError {
    #[error("isso não é um GIF")]
    NotGif,
    #[error("GIF grande demais (até 2 MB)")]
    TooBig,
    #[error("não consegui abrir esse GIF: {0}")]
    Decode(String),
}

type Result<T> = std::result::Result<T, GifError>;

/// GIF pronto para mandar.
#[derive(Debug, Clone)]
pub struct Gif {
    pub bytes: Vec<u8>,
    pub w: u32,
    pub h: u32,
    /// Miniatura do primeiro quadro, em data URL (aparece enquanto chega).
    pub th: String,
    /// Primeiro quadro em PNG, para a prévia antes de mandar.
    pub first_png: Vec<u8>,
}

/// Um quadro aberto: pixels RGBA e quanto tempo fica na tela.
pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
    pub delay_ms: u32,
}

pub fn is_gif(bytes: &[u8]) -> bool {
    bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")
}

/// Confere o GIF e prepara a miniatura. Não mexe nos bytes.
pub fn prepare(bytes: Vec<u8>) -> Result<Gif> {
    if !is_gif(&bytes) {
        return Err(GifError::NotGif);
    }
    if bytes.len() > GIF_MAX {
        return Err(GifError::TooBig);
    }
    let dec = GifDecoder::new(std::io::Cursor::new(&bytes)).map_err(|e| GifError::Decode(e.to_string()))?;
    let primeiro = dec
        .into_frames()
        .next()
        .ok_or_else(|| GifError::Decode("sem quadros".into()))?
        .map_err(|e| GifError::Decode(e.to_string()))?;
    let img = DynamicImage::ImageRgba8(primeiro.into_buffer());
    let (w, h) = (img.width().max(1), img.height().max(1));
    let th = crate::photo::thumbnail(&img).map_err(|e| GifError::Decode(e.to_string()))?;
    let mut first_png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut first_png), image::ImageFormat::Png)
        .map_err(|e| GifError::Decode(e.to_string()))?;
    Ok(Gif { bytes, w, h, th, first_png })
}

/// Abre os quadros para animar na tela, reduzindo para caber em `max_side`
/// e dentro do limite de memória.
pub fn frames(bytes: &[u8], max_side: u32) -> Result<Vec<Frame>> {
    if !is_gif(bytes) {
        return Err(GifError::NotGif);
    }
    let dec = GifDecoder::new(std::io::Cursor::new(bytes)).map_err(|e| GifError::Decode(e.to_string()))?;
    let mut out: Vec<Frame> = Vec::new();
    let mut usado: u64 = 0;
    for q in dec.into_frames().take(MAX_FRAMES) {
        let q = q.map_err(|e| GifError::Decode(e.to_string()))?;
        let (num, den) = q.delay().numer_denom_ms();
        let ms = if den == 0 { 100 } else { num / den };
        // Navegadores tratam atraso 0 ou 10 ms como 100 ms; fazemos igual.
        let delay_ms = if ms <= 10 { 100 } else { ms.min(10_000) };
        let mut img = DynamicImage::ImageRgba8(q.into_buffer());
        let maior = img.width().max(img.height());
        if maior > max_side {
            img = img.resize(max_side, max_side, image::imageops::FilterType::Triangle);
        }
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        usado += w as u64 * h as u64 * 4;
        if usado > MAX_PIXELS_TOTAL && !out.is_empty() {
            break;
        }
        out.push(Frame { w, h, rgba: rgba.into_raw(), delay_ms });
    }
    if out.is_empty() {
        return Err(GifError::Decode("sem quadros".into()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame as IFrame, RgbaImage};

    pub(crate) fn gif_de_teste(n: usize, lado: u32) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = GifEncoder::new(&mut out);
            enc.set_repeat(Repeat::Infinite).unwrap();
            for i in 0..n {
                let cor = [(i * 60 % 255) as u8, 80, 200, 255];
                let img = RgbaImage::from_pixel(lado, lado / 2, image::Rgba(cor));
                enc.encode_frame(IFrame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(80, 1))).unwrap();
            }
        }
        out
    }

    #[test]
    fn prepara_sem_mexer_nos_bytes() {
        let g = gif_de_teste(3, 120);
        let p = prepare(g.clone()).unwrap();
        assert_eq!(p.bytes, g);
        assert_eq!((p.w, p.h), (120, 60));
        assert!(p.th.starts_with("data:image/jpeg;base64,"));
        assert!(p.first_png.starts_with(b"\x89PNG"));
    }

    #[test]
    fn recusa_o_que_nao_e_gif_e_o_grande_demais() {
        assert!(matches!(prepare(b"\x89PNG....".to_vec()), Err(GifError::NotGif)));
        let mut g = gif_de_teste(1, 8);
        g.resize(GIF_MAX + 1, 0);
        assert!(matches!(prepare(g), Err(GifError::TooBig)));
    }

    #[test]
    fn abre_todos_os_quadros_com_o_tempo_certo() {
        let q = frames(&gif_de_teste(4, 400), 200).unwrap();
        assert_eq!(q.len(), 4);
        assert!(q.iter().all(|f| f.delay_ms == 80));
        assert_eq!((q[0].w, q[0].h), (200, 100));
        assert_eq!(q[0].rgba.len(), 200 * 100 * 4);
    }
}
