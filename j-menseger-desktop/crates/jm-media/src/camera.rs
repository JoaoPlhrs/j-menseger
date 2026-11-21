//! Câmera do computador, para as chamadas de vídeo.
//!
//! Usa a câmera pelo sistema (Media Foundation no Windows, V4L2 no Linux),
//! pedindo 1280×720 a 30 quadros, ou o mais próximo disso. Os quadros saem
//! no mesmo formato da captura de tela, então vão pelo mesmo caminho de
//! compressão e envio da chamada.
//!
//! Para testes sem câmera: `J_MENSEGER_CAMERA_TESTE=1` acrescenta uma
//! "câmera de teste" que desenha um quadro animado.

use crate::screen::{Frame, Result, ScreenError, Source, SourceKind};
use std::cell::RefCell;
use std::time::Instant;

/// Número reservado para a câmera de teste.
pub const TESTE_ID: u32 = u32::MAX;

/// Câmeras disponíveis.
pub fn cameras() -> Vec<Source> {
    let mut v: Vec<Source> = Vec::new();
    match nokhwa::query(nokhwa::utils::ApiBackend::Auto) {
        Ok(lista) => {
            for c in lista {
                let id = match c.index() {
                    nokhwa::utils::CameraIndex::Index(i) => *i,
                    nokhwa::utils::CameraIndex::String(_) => v.len() as u32,
                };
                v.push(Source { id, kind: SourceKind::Camera, title: c.human_name(), app: String::new(), width: 1280, height: 720, primary: v.is_empty() });
            }
        }
        Err(e) => tracing::info!("nenhuma câmera listada: {e}"),
    }
    if std::env::var("J_MENSEGER_CAMERA_TESTE").is_ok() {
        v.push(Source { id: TESTE_ID, kind: SourceKind::Camera, title: "Câmera de teste".into(), app: String::new(), width: 640, height: 480, primary: v.is_empty() });
    }
    v
}

pub enum CameraCapture {
    Real(RefCell<nokhwa::Camera>),
    Teste { inicio: Instant },
}

impl CameraCapture {
    pub fn open(id: u32) -> Result<Self> {
        if id == TESTE_ID {
            return Ok(CameraCapture::Teste { inicio: Instant::now() });
        }
        use nokhwa::pixel_format::RgbAFormat;
        use nokhwa::utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType, Resolution};
        let pedido = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::Closest(CameraFormat::new(Resolution::new(1280, 720), FrameFormat::MJPEG, 30)));
        let mut cam = nokhwa::Camera::new(CameraIndex::Index(id), pedido)
            .or_else(|_| nokhwa::Camera::new(CameraIndex::Index(id), RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestFrameRate)))
            .map_err(|e| ScreenError::Capture(format!("não deu para abrir a câmera: {e}")))?;
        cam.open_stream().map_err(|e| ScreenError::Capture(format!("a câmera não começou: {e}")))?;
        Ok(CameraCapture::Real(RefCell::new(cam)))
    }

    pub fn grab(&self) -> Result<Frame> {
        match self {
            CameraCapture::Real(c) => {
                use nokhwa::pixel_format::RgbAFormat;
                let mut cam = c.borrow_mut();
                let buf = cam.frame().map_err(|e| ScreenError::Capture(e.to_string()))?;
                let img = buf.decode_image::<RgbAFormat>().map_err(|e| ScreenError::Capture(e.to_string()))?;
                // Espelhada não: quem recebe vê como uma câmera normal.
                let (w, h) = img.dimensions();
                Ok(Frame { width: w & !1, height: h & !1, rgba: recorta_par(img.into_raw(), w, h), at: Instant::now() })
            }
            CameraCapture::Teste { inicio } => Ok(quadro_teste(inicio.elapsed().as_secs_f32())),
        }
    }
}

impl Drop for CameraCapture {
    fn drop(&mut self) {
        if let CameraCapture::Real(c) = self {
            let _ = c.borrow_mut().stop_stream();
        }
    }
}

/// O codificador de vídeo exige largura e altura pares.
fn recorta_par(rgba: Vec<u8>, w: u32, h: u32) -> Vec<u8> {
    if w % 2 == 0 && h % 2 == 0 {
        return rgba;
    }
    let (nw, nh) = (w & !1, h & !1);
    let mut out = Vec::with_capacity((nw * nh * 4) as usize);
    for y in 0..nh {
        let i = (y * w * 4) as usize;
        out.extend_from_slice(&rgba[i..i + (nw * 4) as usize]);
    }
    out
}

/// Quadro animado da câmera de teste: fundo em degradê e um círculo andando.
pub fn quadro_teste(t: f32) -> Frame {
    let (w, h) = (640u32, 480u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let cx = (w as f32 / 2.0) + (t * 1.7).sin() * 200.0;
    let cy = (h as f32 / 2.0) + (t * 1.3).cos() * 120.0;
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let dentro = d < 70.0;
            rgba[i] = if dentro { 47 } else { (20 + x * 40 / w) as u8 };
            rgba[i + 1] = if dentro { 212 } else { (24 + y * 30 / h) as u8 };
            rgba[i + 2] = if dentro { 124 } else { 40 };
            rgba[i + 3] = 255;
        }
    }
    Frame { width: w, height: h, rgba, at: Instant::now() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_de_teste_anda_e_tem_tamanho_par() {
        let a = quadro_teste(0.0);
        let b = quadro_teste(1.0);
        assert_eq!((a.width, a.height), (640, 480));
        assert_ne!(a.rgba, b.rgba);
        assert_eq!(recorta_par(vec![1; 3 * 3 * 4], 3, 3).len(), 2 * 2 * 4);
    }
}
