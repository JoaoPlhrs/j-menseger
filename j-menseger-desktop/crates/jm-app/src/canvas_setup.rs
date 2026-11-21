//! Desenhos animados do instalador: fundo com luzes passeando, barra de
//! progresso com brilho e o "certo" que se desenha sozinho.

use crate::theme as t;
use iced::mouse;
use iced::widget::canvas::{self, gradient, Fill, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

/// Fundo escuro com três luzes na cor de destaque passeando devagar.
pub struct Background {
    pub t: f32,
    pub accent: Color,
}

impl<M> canvas::Program<M> for Background {
    type State = ();
    fn draw(&self, _: &(), r: &Renderer, _: &Theme, b: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let grad = gradient::Linear::new(Point::new(0.0, 0.0), Point::new(0.0, b.height))
            .add_stop(0.0, t::rgb(0x14, 0x16, 0x1c))
            .add_stop(1.0, t::rgb(0x0a, 0x0b, 0x0f));
        f.fill_rectangle(Point::ORIGIN, b.size(), Fill::from(grad));
        let luzes = [(0.25, 0.22, 0.9, 0.0), (0.8, 0.35, 0.7, 2.1), (0.5, 0.85, 1.1, 4.2)];
        for (i, (x, y, vel, fase)) in luzes.iter().enumerate() {
            let tt = self.t * 0.35 * vel + fase;
            let cx = b.width * (x + 0.12 * tt.sin());
            let cy = b.height * (y + 0.08 * (tt * 1.3).cos());
            let raio = b.width * (0.42 + 0.05 * (tt * 0.7 + i as f32).sin());
            // Brilho macio: círculos um dentro do outro, cada vez mais fracos.
            for k in 0..14 {
                let p = k as f32 / 14.0;
                let a = 0.012 + 0.010 * (1.0 - p);
                f.fill(&Path::circle(Point::new(cx, cy), raio * (1.0 - p * 0.92)), t::with_alpha(self.accent, a));
            }
        }
        vec![f.into_geometry()]
    }
}

/// Barra de progresso arredondada com um brilho que corre por cima.
pub struct Bar {
    pub p: f32,
    pub t: f32,
    pub color: Color,
}

impl<M> canvas::Program<M> for Bar {
    type State = ();
    fn draw(&self, _: &(), r: &Renderer, _: &Theme, b: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let raio = b.height / 2.0;
        f.fill(&Path::rounded_rectangle(Point::ORIGIN, b.size(), raio.into()), t::with_alpha(Color::WHITE, 0.08));
        let w = (b.width * self.p.clamp(0.0, 1.0)).max(b.height);
        f.fill(&Path::rounded_rectangle(Point::ORIGIN, Size::new(w, b.height), raio.into()), self.color);
        // Brilho: uma faixa clara andando dentro da parte cheia.
        let x = ((self.t * 0.8).fract() * (w + 80.0)) - 60.0;
        let brilho = gradient::Linear::new(Point::new(x, 0.0), Point::new(x + 60.0, 0.0))
            .add_stop(0.0, t::with_alpha(Color::WHITE, 0.0))
            .add_stop(0.5, t::with_alpha(Color::WHITE, 0.35))
            .add_stop(1.0, t::with_alpha(Color::WHITE, 0.0));
        let ini = x.max(0.0);
        let fim = (x + 60.0).min(w);
        if fim > ini {
            f.fill(&Path::rounded_rectangle(Point::new(ini, 0.0), Size::new(fim - ini, b.height), raio.into()), Fill::from(brilho));
        }
        vec![f.into_geometry()]
    }
}

/// Círculo que se fecha e o "certo" desenhado por dentro. `p` de 0 a 1.
pub struct Check {
    pub p: f32,
    pub color: Color,
}

impl<M> canvas::Program<M> for Check {
    type State = ();
    fn draw(&self, _: &(), r: &Renderer, _: &Theme, b: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(r, b.size());
        let c = Point::new(b.width / 2.0, b.height / 2.0);
        let raio = b.width.min(b.height) / 2.0 - 3.0;
        let pc = crate::motion::ease_out_cubic((self.p / 0.6).min(1.0));
        f.fill(&Path::circle(c, raio * pc), t::with_alpha(self.color, 0.18 * pc));
        let arco = Path::new(|pb| {
            pb.arc(canvas::path::Arc {
                center: c,
                radius: raio,
                start_angle: iced::Radians(-std::f32::consts::FRAC_PI_2),
                end_angle: iced::Radians(-std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * pc),
            });
        });
        f.stroke(&arco, Stroke::default().with_width(3.5).with_color(self.color).with_line_cap(canvas::LineCap::Round));
        let pk = ((self.p - 0.45) / 0.55).clamp(0.0, 1.0);
        if pk > 0.0 {
            let s = raio / 24.0;
            let a = Point::new(c.x - 10.0 * s, c.y + 1.0 * s);
            let m = Point::new(c.x - 3.0 * s, c.y + 8.0 * s);
            let z = Point::new(c.x + 11.0 * s, c.y - 7.0 * s);
            let l1 = (m.x - a.x).hypot(m.y - a.y);
            let l2 = (z.x - m.x).hypot(z.y - m.y);
            let feito = crate::motion::ease_out_cubic(pk) * (l1 + l2);
            let traço = Path::new(|pb| {
                pb.move_to(a);
                if feito <= l1 {
                    let k = feito / l1;
                    pb.line_to(Point::new(a.x + (m.x - a.x) * k, a.y + (m.y - a.y) * k));
                } else {
                    pb.line_to(m);
                    let k = (feito - l1) / l2;
                    pb.line_to(Point::new(m.x + (z.x - m.x) * k, m.y + (z.y - m.y) * k));
                }
            });
            f.stroke(&traço, Stroke::default().with_width(4.0).with_color(self.color).with_line_cap(canvas::LineCap::Round).with_line_join(canvas::LineJoin::Round));
        }
        vec![f.into_geometry()]
    }
}
