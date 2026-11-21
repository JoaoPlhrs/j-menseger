//! Fotos: preparar para enviar, gerar foto de perfil e abrir para mostrar.
//!
//! Os números são os do celular: lado maior até 1600 px, arquivo até
//! 320 KB, miniatura de 24 px embutida no cabeçalho, foto de perfil de
//! 192 × 192 que cabe em 22 000 caracteres.
//!
//! **Metadados:** a foto é decodificada e codificada de novo a partir dos
//! pixels. O que estava no arquivo original — localização do GPS, modelo da
//! câmera, data — não passa para o arquivo novo. A rotação do EXIF é aplicada
//! antes, para a foto não chegar deitada.

use base64::Engine as _;
use image::{DynamicImage, ImageDecoder, ImageReader};

pub const MAX_SIDE: u32 = 1600;
pub const TARGET_BYTES: usize = 320_000;
pub const AVATAR_SIDE: u32 = 192;
pub const AVATAR_MAX_LEN: usize = 22_000;

#[derive(Debug, thiserror::Error)]
pub enum PhotoError {
    #[error("não consegui abrir essa imagem: {0}")]
    Decode(String),
    #[error("não consegui preparar a imagem: {0}")]
    Encode(String),
}

type Result<T> = std::result::Result<T, PhotoError>;

/// Foto pronta para mandar.
#[derive(Debug, Clone)]
pub struct Photo {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub w: u32,
    pub h: u32,
    /// Miniatura em data URL, para aparecer antes da foto chegar inteira.
    pub th: String,
}

/// Abre a imagem já com a rotação do EXIF aplicada.
pub fn open(bytes: &[u8]) -> Result<DynamicImage> {
    let reader = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| PhotoError::Decode(e.to_string()))?;
    let mut decoder = reader.into_decoder().map_err(|e| PhotoError::Decode(e.to_string()))?;
    let orientacao = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| PhotoError::Decode(e.to_string()))?;
    if let Some(o) = orientacao {
        img.apply_orientation(o);
    }
    Ok(img)
}

fn jpeg(img: &DynamicImage, q: u8) -> Result<Vec<u8>> {
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q)
        .encode_image(&rgb)
        .map_err(|e| PhotoError::Encode(e.to_string()))?;
    Ok(out)
}

/// Prepara uma foto para enviar numa conversa.
///
/// Sai em JPEG: o celular aceita JPEG e WebP, e o codificador de WebP com
/// perdas não existe em Rust puro. O próprio celular cai para JPEG quando o
/// aparelho não gera WebP.
pub fn prepare(bytes: &[u8]) -> Result<Photo> {
    let img = open(bytes)?;
    let (w0, h0) = (img.width().max(1), img.height().max(1));
    let mut escala = (MAX_SIDE as f64 / w0.max(h0) as f64).min(1.0);
    let mut melhor: Option<(Vec<u8>, u32, u32)> = None;
    for _ in 0..6 {
        let w = ((w0 as f64 * escala).round() as u32).max(1);
        let h = ((h0 as f64 * escala).round() as u32).max(1);
        let redim = if (w, h) == (w0, h0) { img.clone() } else { img.resize_exact(w, h, image::imageops::FilterType::Lanczos3) };
        let mut q = 82u8;
        let mut dados = jpeg(&redim, q)?;
        while dados.len() > TARGET_BYTES && q > 45 {
            q -= 12;
            dados = jpeg(&redim, q)?;
        }
        let cabe = dados.len() <= TARGET_BYTES;
        melhor = Some((dados, w, h));
        if cabe {
            break;
        }
        escala *= 0.78;
    }
    let (dados, w, h) = melhor.ok_or_else(|| PhotoError::Encode("imagem vazia".into()))?;
    Ok(Photo { bytes: dados, mime: "image/jpeg", w, h, th: thumbnail(&img)? })
}

/// Miniatura da prévia de um link: até 200 px no lado maior, em data URL
/// JPEG de no máximo ~24 KB.
pub fn link_thumb(bytes: &[u8]) -> Result<String> {
    let img = open(bytes)?;
    let (w, h) = (img.width().max(1), img.height().max(1));
    let mut lado = 200.0f64;
    loop {
        let e = (lado / w.max(h) as f64).min(1.0);
        let p = img.resize_exact(((w as f64 * e).round() as u32).max(1), ((h as f64 * e).round() as u32).max(1), image::imageops::FilterType::Triangle);
        for q in [70u8, 55, 40] {
            let u = data_url("image/jpeg", &jpeg(&p, q)?);
            if u.len() <= 24_000 {
                return Ok(u);
            }
        }
        lado *= 0.7;
        if lado < 40.0 {
            return Err(PhotoError::Encode("miniatura grande demais".into()));
        }
    }
}

/// Miniatura de 24 px no lado maior, em data URL.
pub fn thumbnail(img: &DynamicImage) -> Result<String> {
    let (w, h) = (img.width().max(1), img.height().max(1));
    let tw = ((w as f64 * 24.0 / w.max(h) as f64).round() as u32).max(1);
    let th = ((h as f64 * 24.0 / w.max(h) as f64).round() as u32).max(1);
    let p = img.resize_exact(tw, th, image::imageops::FilterType::Triangle);
    Ok(data_url("image/jpeg", &jpeg(&p, 50)?))
}

/// Foto de perfil (ou de grupo): quadrada, 192 px, em data URL pequena.
///
/// `crop` é o quadrado escolhido na imagem original, em pixels
/// (x, y, lado). Sem ele, pega o quadrado do centro.
pub fn avatar(bytes: &[u8], crop: Option<(u32, u32, u32)>) -> Result<String> {
    let img = open(bytes)?;
    let (w, h) = (img.width(), img.height());
    let (x, y, lado) = match crop {
        Some((x, y, l)) => {
            let l = l.clamp(1, w.min(h));
            (x.min(w - l), y.min(h - l), l)
        }
        None => {
            let l = w.min(h).max(1);
            ((w - l) / 2, (h - l) / 2, l)
        }
    };
    let quadrado = img.crop_imm(x, y, lado, lado);
    let mut alvo = AVATAR_SIDE;
    loop {
        let p = quadrado.resize_exact(alvo, alvo, image::imageops::FilterType::Lanczos3);
        let mut q = 86i32;
        while q >= 40 {
            let u = data_url("image/jpeg", &jpeg(&p, q as u8)?);
            if u.len() <= AVATAR_MAX_LEN {
                return Ok(u);
            }
            q -= 8;
        }
        if alvo <= 96 {
            return Err(PhotoError::Encode("não coube no tamanho de foto de perfil".into()));
        }
        alvo = 128.min(alvo - 32);
    }
}

/// Banner do perfil: retângulo 3:1, 600×200, em data URL pequena (cabe no
/// limite do banner, 24 000 caracteres). `crop` é (x, y, largura, altura)
/// na imagem original; sem ele, pega a faixa do meio.
pub const BANNER_W: u32 = 600;
pub const BANNER_H: u32 = 200;
pub const BANNER_MAX_LEN: usize = 24_000;

pub fn banner(bytes: &[u8], crop: Option<(u32, u32, u32, u32)>) -> Result<String> {
    let img = open(bytes)?;
    let (w, h) = (img.width().max(1), img.height().max(1));
    let (x, y, cw, ch) = match crop {
        Some((x, y, cw, ch)) => {
            let cw = cw.clamp(1, w);
            let ch = ch.clamp(1, h);
            (x.min(w - cw), y.min(h - ch), cw, ch)
        }
        None => {
            let cw = w.min(h * 3);
            let ch = (cw / 3).max(1);
            ((w - cw) / 2, (h - ch) / 2, cw, ch)
        }
    };
    let faixa = img.crop_imm(x, y, cw, ch);
    let (mut bw, mut bh) = (BANNER_W, BANNER_H);
    loop {
        let p = faixa.resize_exact(bw, bh, image::imageops::FilterType::Lanczos3);
        let mut q = 80i32;
        while q >= 30 {
            let u = data_url("image/jpeg", &jpeg(&p, q as u8)?);
            if u.len() <= BANNER_MAX_LEN {
                return Ok(u);
            }
            q -= 8;
        }
        if bw <= 300 {
            return Err(PhotoError::Encode("não coube no tamanho do banner".into()));
        }
        bw -= 100;
        bh = bw / 3;
    }
}

/// Decodifica para mostrar na tela (RGBA), limitando o tamanho para não
/// gastar memória à toa com fotos enormes.
pub fn to_rgba(bytes: &[u8], max_side: u32) -> Result<(u32, u32, Vec<u8>)> {
    let img = open(bytes)?;
    let img = if img.width().max(img.height()) > max_side {
        img.resize(max_side, max_side, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    Ok((rgba.width(), rgba.height(), rgba.into_raw()))
}

/// Bytes de uma data URL `data:image/...;base64,...`.
pub fn from_data_url(u: &str) -> Option<Vec<u8>> {
    let (_, b64) = u.split_once(";base64,")?;
    base64::engine::general_purpose::STANDARD.decode(b64).ok()
}

pub fn data_url(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Imagem de teste com detalhe (gradiente + ruído), que comprime mal.
    fn foto(w: u32, h: u32) -> Vec<u8> {
        let mut img = image::RgbImage::new(w, h);
        let mut s: u32 = 12345;
        for (x, y, p) in img.enumerate_pixels_mut() {
            s = s.wrapping_mul(1_103_515_245).wrapping_add(12345);
            let r = (s >> 16) as u8;
            *p = image::Rgb([(x * 255 / w) as u8 ^ (r & 0x3f), (y * 255 / h) as u8, r]);
        }
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn banner_cabe_no_limite() {
        let u = banner(&foto(1600, 900), None).unwrap();
        assert!(u.starts_with("data:image/jpeg;base64,") && u.len() <= BANNER_MAX_LEN);
        let u = banner(&foto(800, 800), Some((0, 100, 600, 200))).unwrap();
        assert!(u.len() <= BANNER_MAX_LEN);
    }

    #[test]
    fn foto_grande_fica_dentro_dos_limites_do_celular() {
        let p = prepare(&foto(3000, 2000)).unwrap();
        assert!(p.w.max(p.h) <= MAX_SIDE, "{}x{}", p.w, p.h);
        assert!(p.bytes.len() <= TARGET_BYTES, "{} bytes", p.bytes.len());
        assert_eq!(p.mime, "image/jpeg");
        assert!(p.th.starts_with("data:image/jpeg;base64,"));
        assert!(p.th.len() < 3000, "miniatura grande demais: {}", p.th.len());
        // A proporção foi mantida.
        let prop = p.w as f64 / p.h as f64;
        assert!((prop - 1.5).abs() < 0.01);
    }

    #[test]
    fn foto_de_perfil_cabe_no_limite() {
        let u = avatar(&foto(900, 1200), None).unwrap();
        assert!(u.len() <= AVATAR_MAX_LEN, "{} caracteres", u.len());
        let (w, h, _) = to_rgba(&from_data_url(&u).unwrap(), 1000).unwrap();
        assert_eq!((w, h), (AVATAR_SIDE, AVATAR_SIDE));
    }

    #[test]
    fn metadados_nao_passam_adiante() {
        // JPEG com um bloco EXIF que contém "GPS" e o modelo da câmera.
        let mut base = jpeg(&DynamicImage::ImageRgb8(image::RgbImage::new(40, 30)), 90).unwrap();
        let exif = b"Exif\0\0GPSLatitude=-23.55;Model=CameraSecreta";
        let mut app1 = vec![0xFF, 0xE1];
        app1.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        app1.extend_from_slice(exif);
        base.splice(2..2, app1);
        assert!(base.windows(13).any(|w| w == b"CameraSecreta"));

        let p = prepare(&base).unwrap();
        assert!(!p.bytes.windows(13).any(|w| w == b"CameraSecreta"), "o modelo da câmera vazou");
        assert!(!p.bytes.windows(11).any(|w| w == b"GPSLatitude"), "a localização vazou");
    }

    #[test]
    fn imagem_invalida_vira_erro() {
        assert!(prepare(b"nao sou uma foto").is_err());
        assert!(avatar(&[], None).is_err());
    }
}
