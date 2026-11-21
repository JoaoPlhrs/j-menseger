//! Desenhos interativos: o editor de corte dos sons e o recorte de foto.

use crate::app::Msg;
use crate::state::{TRIM_MAX, TRIM_MIN};
use crate::theme as t;
use iced::mouse;
use iced::widget::canvas::{self, Action, Event, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

/* ------------------------------------------------------------ corte */

/// A forma de onda da janela visível, com o trecho escolhido.
pub struct Trim {
    pub peaks: Vec<f32>,
    pub a: f64,
    pub b: f64,
    pub v0: f64,
    pub vlen: f64,
    pub playhead: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub enum Drag {
    A,
    B,
    Move { off: f64, len: f64 },
}

impl Trim {
    fn x_to_t(&self, x: f32, w: f32) -> f64 {
        self.v0 + (x / w.max(1.0)).clamp(0.0, 1.0) as f64 * self.vlen
    }

    fn t_to_x(&self, t: f64, w: f32) -> f32 {
        (((t - self.v0) / self.vlen.max(0.001)) as f32 * w).clamp(0.0, w)
    }

    /// As mesmas regras do editor do celular.
    fn apply(&self, d: Drag, t: f64) -> (f64, f64) {
        let (mut a, mut b) = (self.a, self.b);
        match d {
            Drag::A => {
                a = t.min(b - TRIM_MIN);
                if b - a > TRIM_MAX {
                    b = a + TRIM_MAX;
                }
            }
            Drag::B => {
                b = t.max(a + TRIM_MIN);
                if b - a > TRIM_MAX {
                    a = b - TRIM_MAX;
                }
            }
            Drag::Move { off, len } => {
                a = (t - off).clamp(self.v0, (self.v0 + self.vlen - len).max(self.v0));
                b = a + len;
            }
        }
        a = a.max(self.v0);
        b = b.min(self.v0 + self.vlen);
        (a, b)
    }
}

impl canvas::Program<Msg> for Trim {
    type State = Option<Drag>;

    fn update(&self, state: &mut Option<Drag>, event: &Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<Action<Msg>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                let t = self.x_to_t(p.x, bounds.width);
                let perto = 14.0 / bounds.width.max(1.0) as f64 * self.vlen;
                let d = if (t - self.a).abs() <= perto && (t - self.a).abs() <= (t - self.b).abs() {
                    Drag::A
                } else if (t - self.b).abs() <= perto {
                    Drag::B
                } else if t > self.a && t < self.b {
                    Drag::Move { off: t - self.a, len: self.b - self.a }
                } else if t < self.a {
                    Drag::A
                } else {
                    Drag::B
                };
                *state = Some(d);
                let (a, b) = self.apply(d, t);
                Some(Action::publish(Msg::TrimSet(a, b)).and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let d = (*state)?;
                let t = self.x_to_t(position.x - bounds.x, bounds.width);
                let (a, b) = self.apply(d, t);
                Some(Action::publish(Msg::TrimSet(a, b)).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.take()?;
                Some(Action::capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _state: &Option<Drag>, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        f.fill(&Path::rounded_rectangle(Point::ORIGIN, bounds.size(), 10.0.into()), t::mix(t::surface(), Color::BLACK, 0.25));
        let n = self.peaks.len().max(1);
        let maior = self.peaks.iter().copied().fold(0.0001f32, f32::max);
        let bw = w / n as f32;
        let xa = self.t_to_x(self.a, w);
        let xb = self.t_to_x(self.b, w);
        for (i, v) in self.peaks.iter().enumerate() {
            let x = i as f32 * bw;
            let bh = ((v / maior) * h * 0.86).max(2.0);
            let dentro = x + bw / 2.0 >= xa && x + bw / 2.0 <= xb;
            let cor = if dentro { t::accent() } else { t::with_alpha(t::FG_DIM, 0.45) };
            f.fill_rectangle(Point::new(x + bw * 0.18, (h - bh) / 2.0), Size::new((bw * 0.64).max(1.0), bh), cor);
        }
        // Fora do trecho fica escuro.
        f.fill_rectangle(Point::ORIGIN, Size::new(xa, h), t::with_alpha(Color::BLACK, 0.45));
        f.fill_rectangle(Point::new(xb, 0.0), Size::new((w - xb).max(0.0), h), t::with_alpha(Color::BLACK, 0.45));
        f.stroke(
            &Path::rounded_rectangle(Point::new(xa, 1.0), Size::new((xb - xa).max(2.0), h - 2.0), 6.0.into()),
            Stroke::default().with_color(t::accent()).with_width(2.0),
        );
        for x in [xa, xb] {
            f.fill(&Path::rounded_rectangle(Point::new(x - 4.0, h / 2.0 - 18.0), Size::new(8.0, 36.0), 4.0.into()), t::accent());
        }
        if let Some(p) = self.playhead {
            let x = self.t_to_x(p, w);
            f.stroke(&Path::line(Point::new(x, 0.0), Point::new(x, h)), Stroke::default().with_color(t::FG).with_width(2.0));
        }
        vec![f.into_geometry()]
    }

    fn mouse_interaction(&self, state: &Option<Drag>, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if state.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

/// Visão do arquivo inteiro (áudios longos): escolhe a janela de 30 s.
pub struct Overview {
    pub peaks: Vec<f32>,
    pub dur: f64,
    pub v0: f64,
    pub vlen: f64,
}

impl canvas::Program<Msg> for Overview {
    type State = bool;

    fn update(&self, state: &mut bool, event: &Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<Action<Msg>> {
        let para = |x: f32| -> f64 { (x / bounds.width.max(1.0)).clamp(0.0, 1.0) as f64 * self.dur - self.vlen / 2.0 };
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                *state = true;
                Some(Action::publish(Msg::TrimView(para(p.x))).and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) if *state => {
                Some(Action::publish(Msg::TrimView(para(position.x - bounds.x))).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if *state => {
                *state = false;
                Some(Action::capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _state: &bool, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        f.fill(&Path::rounded_rectangle(Point::ORIGIN, bounds.size(), 6.0.into()), t::mix(t::surface(), Color::BLACK, 0.3));
        let n = self.peaks.len().max(1);
        let maior = self.peaks.iter().copied().fold(0.0001f32, f32::max);
        let bw = w / n as f32;
        for (i, v) in self.peaks.iter().enumerate() {
            let bh = ((v / maior) * h * 0.8).max(1.0);
            f.fill_rectangle(Point::new(i as f32 * bw, (h - bh) / 2.0), Size::new(bw.max(1.0) * 0.7, bh), t::with_alpha(t::FG_DIM, 0.6));
        }
        let x0 = (self.v0 / self.dur.max(0.001)) as f32 * w;
        let x1 = ((self.v0 + self.vlen) / self.dur.max(0.001)) as f32 * w;
        f.fill_rectangle(Point::new(x0, 0.0), Size::new((x1 - x0).max(2.0), h), t::with_alpha(t::accent(), 0.22));
        f.stroke(
            &Path::rectangle(Point::new(x0, 1.0), Size::new((x1 - x0).max(2.0), h - 2.0)),
            Stroke::default().with_color(t::accent()).with_width(1.5),
        );
        vec![f.into_geometry()]
    }

    fn mouse_interaction(&self, _state: &bool, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

/* ------------------------------------------------------------ recorte */

/// Quadrado de recorte por cima da foto (que está desenhada com "caber").
pub struct Crop {
    pub w: f32,
    pub h: f32,
    pub x: f32,
    pub y: f32,
    /// Largura do recorte; a altura é `side / ratio`.
    pub side: f32,
    /// Largura ÷ altura: 1 para foto, 3 para banner.
    pub ratio: f32,
}

#[derive(Default)]
pub struct CropDrag {
    modo: Option<CropMode>,
}

#[derive(Clone, Copy)]
enum CropMode {
    Move { dx: f32, dy: f32 },
    Resize,
}

impl Crop {
    /// Escala e deslocamento da foto dentro da área.
    fn fit(&self, b: Size) -> (f32, f32, f32) {
        let k = (b.width / self.w.max(1.0)).min(b.height / self.h.max(1.0));
        let ox = (b.width - self.w * k) / 2.0;
        let oy = (b.height - self.h * k) / 2.0;
        (k, ox, oy)
    }

    fn to_img(&self, p: Point, b: Size) -> (f32, f32) {
        let (k, ox, oy) = self.fit(b);
        ((p.x - ox) / k, (p.y - oy) / k)
    }
}

impl canvas::Program<Msg> for Crop {
    type State = CropDrag;

    fn update(&self, state: &mut CropDrag, event: &Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<Action<Msg>> {
        let tam = bounds.size();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                let (ix, iy) = self.to_img(p, tam);
                let (k, _, _) = self.fit(tam);
                let alca = 16.0 / k;
                let canto = (ix - (self.x + self.side)).abs() < alca && (iy - (self.y + self.side / self.ratio)).abs() < alca;
                state.modo = Some(if canto {
                    CropMode::Resize
                } else {
                    CropMode::Move { dx: ix - self.x, dy: iy - self.y }
                });
                Some(Action::capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let modo = state.modo?;
                let p = Point::new(position.x - bounds.x, position.y - bounds.y);
                let (ix, iy) = self.to_img(p, tam);
                let msg = match modo {
                    CropMode::Move { dx, dy } => Msg::CropSet(ix - dx, iy - dy, self.side),
                    CropMode::Resize => {
                        let lado = ((ix - self.x).max((iy - self.y) * self.ratio)).min(self.w - self.x).min((self.h - self.y) * self.ratio);
                        Msg::CropSet(self.x, self.y, lado)
                    }
                };
                Some(Action::publish(msg).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.modo.take()?;
                Some(Action::capture())
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                cursor.position_in(bounds)?;
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                let novo = (self.side * (1.0 + dy * 0.06)).clamp(16.0, self.w.min(self.h * self.ratio));
                let c = (self.x + self.side / 2.0, self.y + self.side / self.ratio / 2.0);
                Some(Action::publish(Msg::CropSet(c.0 - novo / 2.0, c.1 - novo / self.ratio / 2.0, novo)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _state: &CropDrag, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let (k, ox, oy) = self.fit(bounds.size());
        let (x, y, s) = (ox + self.x * k, oy + self.y * k, self.side * k);
        let sh = s / self.ratio;
        let (iw, ih) = (self.w * k, self.h * k);
        let escuro = t::with_alpha(Color::BLACK, 0.55);
        f.fill_rectangle(Point::new(ox, oy), Size::new(iw, (y - oy).max(0.0)), escuro);
        f.fill_rectangle(Point::new(ox, y + sh), Size::new(iw, (oy + ih - y - sh).max(0.0)), escuro);
        f.fill_rectangle(Point::new(ox, y), Size::new((x - ox).max(0.0), sh), escuro);
        f.fill_rectangle(Point::new(x + s, y), Size::new((ox + iw - x - s).max(0.0), sh), escuro);
        f.stroke(&Path::rectangle(Point::new(x, y), Size::new(s, sh)), Stroke::default().with_color(t::accent()).with_width(2.0));
        if self.ratio == 1.0 {
            // Prévia do recorte redondo do avatar.
            f.stroke(&Path::circle(Point::new(x + s / 2.0, y + s / 2.0), s / 2.0), Stroke::default().with_color(t::with_alpha(Color::WHITE, 0.6)).with_width(1.0));
        }
        f.fill(&Path::circle(Point::new(x + s, y + sh), 7.0), t::accent());
        vec![f.into_geometry()]
    }

    fn mouse_interaction(&self, state: &CropDrag, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        match state.modo {
            Some(CropMode::Resize) => mouse::Interaction::ResizingDiagonallyDown,
            Some(_) => mouse::Interaction::Grabbing,
            None if cursor.is_over(bounds) => mouse::Interaction::Grab,
            None => mouse::Interaction::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trim() -> Trim {
        Trim { peaks: vec![], a: 1.0, b: 3.0, v0: 0.0, vlen: 30.0, playhead: None }
    }

    #[test]
    fn corte_respeita_minimo_e_maximo() {
        let t = trim();
        // Puxar o começo para depois do fim não inverte o trecho.
        let (a, b) = t.apply(Drag::A, 5.0);
        assert!(b - a >= TRIM_MIN - 1e-9 && a < b);
        // Esticar o fim além de 8 s arrasta o começo junto.
        let (a, b) = t.apply(Drag::B, 20.0);
        assert!((b - a - TRIM_MAX).abs() < 1e-9 && (b - 20.0).abs() < 1e-9);
        // Mover mantém o tamanho e não sai da janela.
        let (a, b) = t.apply(Drag::Move { off: 1.0, len: 2.0 }, 100.0);
        assert!((b - a - 2.0).abs() < 1e-9 && b <= 30.0 + 1e-9);
    }

    #[test]
    fn recorte_converte_tela_para_foto() {
        let c = Crop { w: 1000.0, h: 500.0, x: 0.0, y: 0.0, side: 400.0, ratio: 1.0 };
        // Área 500×500: a foto (2:1) ocupa 500×250 centralizada na vertical.
        let (k, ox, oy) = c.fit(Size::new(500.0, 500.0));
        assert!((k - 0.5).abs() < 1e-6 && ox.abs() < 1e-6 && (oy - 125.0).abs() < 1e-6);
        let (ix, iy) = c.to_img(Point::new(250.0, 250.0), Size::new(500.0, 500.0));
        assert!((ix - 500.0).abs() < 1e-3 && (iy - 250.0).abs() < 1e-3);
    }
}

/* ------------------------------------------------------------ animações */

/// Arco girando (carregando).
pub struct Spinner {
    pub t: f32,
    pub color: Color,
}

impl canvas::Program<Msg> for Spinner {
    type State = ();
    fn draw(&self, _: &(), r: &Renderer, _: &Theme, b: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let c = Point::new(b.width / 2.0, b.height / 2.0);
        let raio = b.width.min(b.height) / 2.0 - 2.0;
        let ini = self.t * 6.0;
        let fundo = Path::circle(c, raio);
        f.stroke(&fundo, Stroke::default().with_width(2.5).with_color(t::with_alpha(self.color, 0.2)));
        let arco = Path::new(|p| {
            p.arc(canvas::path::Arc { center: c, radius: raio, start_angle: iced::Radians(ini), end_angle: iced::Radians(ini + 4.2) });
        });
        f.stroke(&arco, Stroke::default().with_width(2.5).with_color(self.color).with_line_cap(canvas::LineCap::Round));
        vec![f.into_geometry()]
    }
}

/// Ondas saindo do avatar (chamada chegando).
pub struct Pulse {
    pub t: f32,
    pub color: Color,
    /// Raio do avatar no centro.
    pub inner: f32,
}

impl canvas::Program<Msg> for Pulse {
    type State = ();
    fn draw(&self, _: &(), r: &Renderer, _: &Theme, b: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let c = Point::new(b.width / 2.0, b.height / 2.0);
        let max = b.width.min(b.height) / 2.0 - 1.0;
        for k in 0..3 {
            let p = ((self.t / 1.6) + k as f32 / 3.0).fract();
            let raio = self.inner + (max - self.inner) * p;
            let a = (1.0 - p).powf(1.6) * 0.55;
            f.stroke(&Path::circle(c, raio), Stroke::default().with_width(2.0 + 2.0 * (1.0 - p)).with_color(t::with_alpha(self.color, a)));
        }
        vec![f.into_geometry()]
    }
}
