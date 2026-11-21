//! Captura de tela: monitores e janelas.
//!
//! Cada sistema tem a sua API, e a biblioteca `xcap` usa a nativa de cada um:
//! Windows Graphics Capture no Windows, ScreenCaptureKit no macOS e
//! PipeWire/X11 no Linux. Isso importa porque capturar pela API certa é o que
//! permite pegar a tela sem copiar quadro a quadro pela CPU.

use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum ScreenError {
    #[error("não foi possível listar o que há para capturar: {0}")]
    List(String),
    #[error("a captura falhou: {0}")]
    Capture(String),
    #[error("essa tela ou janela não existe mais")]
    Gone,
}

pub type Result<T> = std::result::Result<T, ScreenError>;

/// O que a pessoa pode escolher transmitir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    /// Um monitor inteiro.
    Monitor,
    /// Uma janela específica de um aplicativo.
    Window,
    /// A câmera do computador (`id` é o número da câmera).
    Camera,
}

/// Uma opção de transmissão, como aparece na tela de escolha.
#[derive(Debug, Clone)]
pub struct Source {
    pub id: u32,
    pub kind: SourceKind,
    /// Nome do monitor, ou título da janela.
    pub title: String,
    /// Aplicativo dono da janela.
    pub app: String,
    pub width: u32,
    pub height: u32,
    /// Monitor principal do sistema.
    pub primary: bool,
}

impl Source {
    pub fn label(&self) -> String {
        match self.kind {
            SourceKind::Monitor => {
                if self.primary {
                    format!("{} (principal) · {}×{}", self.title, self.width, self.height)
                } else {
                    format!("{} · {}×{}", self.title, self.width, self.height)
                }
            }
            SourceKind::Camera => format!("Câmera: {}", self.title),
            SourceKind::Window => {
                if self.app.is_empty() || self.app == self.title {
                    self.title.clone()
                } else {
                    format!("{} — {}", self.app, self.title)
                }
            }
        }
    }
}

/// Um quadro capturado, em RGBA.
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub at: Instant,
}

impl Frame {
    pub fn pixels(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Reduz o quadro para caber em uma altura máxima, mantendo a proporção.
    ///
    /// Transmitir 1440p para quem está assistindo em uma janela pequena só
    /// gasta banda, então a resolução acompanha a qualidade escolhida.
    pub fn scale_to_height(&self, max_height: u32) -> Frame {
        if self.height <= max_height || self.height == 0 {
            return self.clone();
        }
        let escala = max_height as f32 / self.height as f32;
        // Codificadores de vídeo exigem dimensões pares.
        let nw = (((self.width as f32 * escala) as u32).max(2) + 1) & !1;
        let nh = (max_height.max(2) + 1) & !1;

        let mut saida = vec![0u8; (nw as usize) * (nh as usize) * 4];
        for y in 0..nh {
            let sy = ((y as f32 / nh as f32) * self.height as f32) as u32;
            let sy = sy.min(self.height - 1);
            for x in 0..nw {
                let sx = ((x as f32 / nw as f32) * self.width as f32) as u32;
                let sx = sx.min(self.width - 1);
                let de = ((sy * self.width + sx) * 4) as usize;
                let para = ((y * nw + x) * 4) as usize;
                saida[para..para + 4].copy_from_slice(&self.rgba[de..de + 4]);
            }
        }
        Frame {
            width: nw,
            height: nh,
            rgba: saida,
            at: self.at,
        }
    }
}

/// Lista os monitores disponíveis.
pub fn monitors() -> Result<Vec<Source>> {
    let lista = xcap::Monitor::all().map_err(|e| ScreenError::List(e.to_string()))?;
    Ok(lista
        .into_iter()
        .filter_map(|m| {
            Some(Source {
                id: m.id().ok()?,
                kind: SourceKind::Monitor,
                title: m.name().unwrap_or_else(|_| "Monitor".into()),
                app: String::new(),
                width: m.width().ok()?,
                height: m.height().ok()?,
                primary: m.is_primary().unwrap_or(false),
            })
        })
        .collect())
}

/// Lista as janelas que podem ser transmitidas.
///
/// Janelas minimizadas ficam de fora: o sistema não tem o conteúdo delas para
/// entregar, e transmitir um retângulo preto não serve para nada.
pub fn windows() -> Result<Vec<Source>> {
    let lista = xcap::Window::all().map_err(|e| ScreenError::List(e.to_string()))?;
    Ok(lista
        .into_iter()
        .filter_map(|w| {
            if w.is_minimized().unwrap_or(false) {
                return None;
            }
            let largura = w.width().ok()?;
            let altura = w.height().ok()?;
            if largura < 32 || altura < 32 {
                return None;
            }
            let titulo = w.title().unwrap_or_default();
            let app = w.app_name().unwrap_or_default();
            if titulo.is_empty() && app.is_empty() {
                return None;
            }
            Some(Source {
                id: w.id().ok()?,
                kind: SourceKind::Window,
                title: titulo,
                app,
                width: largura,
                height: altura,
                primary: false,
            })
        })
        .collect())
}

/// Tudo o que pode ser transmitido, monitores primeiro.
pub fn sources() -> Result<Vec<Source>> {
    let mut v = monitors()?;
    v.extend(windows().unwrap_or_default());
    Ok(v)
}

/// Captura um quadro avulso de uma fonte.
///
/// Para transmitir, use [`Capturer`]: esta função reabre a conexão com o
/// sistema a cada chamada, o que é aceitável para uma prévia mas não para
/// dezenas de quadros por segundo.
pub fn capture(source: &Source) -> Result<Frame> {
    Capturer::open(source)?.grab()
}

/// Captura repetida da mesma fonte.
///
/// Guardar a referência da tela entre os quadros não é só mais rápido: abrir
/// uma conexão nova a cada quadro esgota os recursos do servidor gráfico em
/// poucos segundos a 30 ou 60 quadros por segundo, e a transmissão morre.
pub enum Capturer {
    Monitor(Box<xcap::Monitor>),
    Window(Box<xcap::Window>),
    Camera(Box<crate::camera::CameraCapture>),
}

impl Capturer {
    pub fn open(source: &Source) -> Result<Self> {
        match source.kind {
            SourceKind::Camera => crate::camera::CameraCapture::open(source.id).map(|c| Capturer::Camera(Box::new(c))),
            SourceKind::Monitor => {
                let m = xcap::Monitor::all()
                    .map_err(|e| ScreenError::List(e.to_string()))?
                    .into_iter()
                    .find(|m| m.id().map(|i| i == source.id).unwrap_or(false))
                    .ok_or(ScreenError::Gone)?;
                Ok(Capturer::Monitor(Box::new(m)))
            }
            SourceKind::Window => {
                let w = xcap::Window::all()
                    .map_err(|e| ScreenError::List(e.to_string()))?
                    .into_iter()
                    .find(|w| w.id().map(|i| i == source.id).unwrap_or(false))
                    .ok_or(ScreenError::Gone)?;
                Ok(Capturer::Window(Box::new(w)))
            }
        }
    }

    /// Pega o quadro atual.
    pub fn grab(&self) -> Result<Frame> {
        if let Capturer::Camera(c) = self {
            return c.grab();
        }
        let imagem = match self {
            Capturer::Camera(_) => unreachable!(),
            Capturer::Monitor(m) => m
                .capture_image()
                .map_err(|e| ScreenError::Capture(e.to_string()))?,
            Capturer::Window(w) => w
                .capture_image()
                .map_err(|e| ScreenError::Capture(e.to_string()))?,
        };
        Ok(Frame {
            width: imagem.width(),
            height: imagem.height(),
            rgba: imagem.into_raw(),
            at: Instant::now(),
        })
    }
}

/// Quadros por segundo da transmissão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fps {
    Fps30,
    Fps60,
}

impl Fps {
    pub fn value(&self) -> u32 {
        match self {
            Fps::Fps30 => 30,
            Fps::Fps60 => 60,
        }
    }
    pub fn interval(&self) -> Duration {
        Duration::from_nanos(1_000_000_000 / self.value() as u64)
    }
}

/// Resolução da transmissão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    P720,
    P1080,
    P1440,
}

impl Quality {
    pub fn height(&self) -> u32 {
        match self {
            Quality::P720 => 720,
            Quality::P1080 => 1080,
            Quality::P1440 => 1440,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Quality::P720 => "720p",
            Quality::P1080 => "1080p",
            Quality::P1440 => "1440p",
        }
    }

    /// Banda alvo, em bits por segundo.
    ///
    /// 60 quadros custam mais que 30, mas não o dobro: quadros seguidos são
    /// parecidos, e o codificador manda só a diferença.
    pub fn bitrate(&self, fps: Fps) -> u32 {
        let base = match self {
            Quality::P720 => 1_800_000,
            Quality::P1080 => 3_500_000,
            Quality::P1440 => 6_000_000,
        };
        match fps {
            Fps::Fps30 => base,
            Fps::Fps60 => base * 3 / 2,
        }
    }
}

/// Escolhe sozinho a qualidade que cabe na banda disponível.
///
/// Sem isto, a transmissão trava em vez de piorar a imagem: é melhor cair para
/// 720p e continuar fluida do que insistir em 1440p picotado.
pub fn fit_quality(available_bps: u32, desired: Quality, fps: Fps) -> (Quality, Fps) {
    let ordem = [Quality::P1440, Quality::P1080, Quality::P720];
    for q in ordem.into_iter().filter(|q| q.height() <= desired.height()) {
        if q.bitrate(fps) <= available_bps {
            return (q, fps);
        }
    }
    // Nem o menor cabe com 60 quadros: tenta 30.
    if fps == Fps::Fps60 {
        return fit_quality(available_bps, desired, Fps::Fps30);
    }
    (Quality::P720, Fps::Fps30)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadro(w: u32, h: u32) -> Frame {
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        // Um degradê, para dar para ver se a redução embaralhou a imagem.
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                rgba[i] = (x * 255 / w.max(1)) as u8;
                rgba[i + 1] = (y * 255 / h.max(1)) as u8;
                rgba[i + 2] = 128;
                rgba[i + 3] = 255;
            }
        }
        Frame { width: w, height: h, rgba, at: Instant::now() }
    }

    #[test]
    fn reduzir_mantem_a_proporcao_e_da_lados_pares() {
        let f = quadro(1920, 1080);
        let p = f.scale_to_height(720);
        assert_eq!(p.height, 720);
        assert_eq!(p.width, 1280, "16:9 tem que continuar 16:9");
        assert_eq!(p.width % 2, 0);
        assert_eq!(p.height % 2, 0);
        assert_eq!(p.rgba.len(), (p.width * p.height * 4) as usize);
    }

    #[test]
    fn quadro_pequeno_nao_e_esticado() {
        let f = quadro(800, 600);
        let p = f.scale_to_height(1080);
        assert_eq!((p.width, p.height), (800, 600), "não faz sentido aumentar");
    }

    #[test]
    fn reducao_preserva_o_desenho() {
        let f = quadro(400, 400);
        let p = f.scale_to_height(100);
        // Canto superior esquerdo continua escuro, inferior direito continua claro.
        let topo = p.rgba[0] as u32 + p.rgba[1] as u32;
        let fim = ((p.height - 1) * p.width + (p.width - 1)) * 4;
        let base = p.rgba[fim as usize] as u32 + p.rgba[fim as usize + 1] as u32;
        assert!(base > topo + 200, "o degradê sumiu: {topo} contra {base}");
    }

    #[test]
    fn banda_cresce_com_resolucao_e_com_quadros() {
        assert!(Quality::P720.bitrate(Fps::Fps30) < Quality::P1080.bitrate(Fps::Fps30));
        assert!(Quality::P1080.bitrate(Fps::Fps30) < Quality::P1440.bitrate(Fps::Fps30));
        assert!(Quality::P1080.bitrate(Fps::Fps30) < Quality::P1080.bitrate(Fps::Fps60));
        // 60 quadros não custam o dobro.
        assert!(Quality::P1080.bitrate(Fps::Fps60) < Quality::P1080.bitrate(Fps::Fps30) * 2);
    }

    #[test]
    fn qualidade_cai_sozinha_quando_a_banda_aperta() {
        // Banda de sobra: fica no que foi pedido.
        let (q, f) = fit_quality(20_000_000, Quality::P1440, Fps::Fps60);
        assert_eq!((q, f), (Quality::P1440, Fps::Fps60));

        // Banda média: desce a resolução, mantém a fluidez.
        let (q, f) = fit_quality(4_000_000, Quality::P1440, Fps::Fps60);
        assert_eq!(q, Quality::P720);
        assert_eq!(f, Fps::Fps60);

        // Banda apertada: sacrifica os quadros também.
        let (q, f) = fit_quality(2_000_000, Quality::P1440, Fps::Fps60);
        assert_eq!((q, f), (Quality::P720, Fps::Fps30));

        // Nunca sobe acima do que a pessoa pediu.
        let (q, _) = fit_quality(50_000_000, Quality::P720, Fps::Fps30);
        assert_eq!(q, Quality::P720);
    }

    #[test]
    fn intervalo_entre_quadros_bate_com_a_taxa() {
        assert_eq!(Fps::Fps30.value(), 30);
        assert!((Fps::Fps30.interval().as_micros() as i64 - 33_333).abs() < 50);
        assert!((Fps::Fps60.interval().as_micros() as i64 - 16_666).abs() < 50);
    }

    #[test]
    fn rotulo_da_fonte_e_legivel() {
        let m = Source {
            id: 1,
            kind: SourceKind::Monitor,
            title: "Tela 1".into(),
            app: String::new(),
            width: 1920,
            height: 1080,
            primary: true,
        };
        assert_eq!(m.label(), "Tela 1 (principal) · 1920×1080");

        let j = Source {
            id: 2,
            kind: SourceKind::Window,
            title: "documento.txt".into(),
            app: "Editor".into(),
            width: 800,
            height: 600,
            primary: false,
        };
        assert_eq!(j.label(), "Editor — documento.txt");
    }
}
