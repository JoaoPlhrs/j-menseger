//! Animações da interface.
//!
//! Cada animação é um número que desliza de um valor a outro com uma curva
//! de suavização. Enquanto alguma está andando, o app pede um quadro novo a
//! cada atualização da tela (`window::frames`); paradas, não gastam nada.
//!
//! O iced desenha sem transparência de grupo, então as entradas são feitas
//! com deslocamento (deslizar) e com o fundo escurecido aparecendo aos
//! poucos, e não com "fade" do conteúdo.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Curva: começa rápido e assenta devagar.
pub fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}

/// Curva com um leve passar do ponto e voltar (para "pulos").
pub fn ease_out_back(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2)
}

#[derive(Debug, Clone, Copy)]
pub struct Tween {
    from: f32,
    to: f32,
    start: Instant,
    dur: Duration,
    ease: fn(f32) -> f32,
}

impl Tween {
    pub fn new(v: f32) -> Self {
        Tween { from: v, to: v, start: Instant::now(), dur: Duration::ZERO, ease: ease_out_cubic }
    }

    pub fn at(&self, now: Instant) -> f32 {
        if self.dur.is_zero() {
            return self.to;
        }
        let p = now.saturating_duration_since(self.start).as_secs_f32() / self.dur.as_secs_f32();
        if p >= 1.0 {
            return self.to;
        }
        self.from + (self.to - self.from) * (self.ease)(p)
    }

    pub fn running(&self, now: Instant) -> bool {
        !self.dur.is_zero() && now.saturating_duration_since(self.start) < self.dur
    }

    /// Vai para `to` a partir de onde está agora.
    pub fn go(&mut self, to: f32, now: Instant, dur: Duration, ease: fn(f32) -> f32) {
        if (self.to - to).abs() < f32::EPSILON && !self.running(now) {
            return;
        }
        self.from = self.at(now);
        self.to = to;
        self.start = now;
        self.dur = dur;
        self.ease = ease;
    }

    /// Recomeça do `from` (para entradas: sempre do zero).
    pub fn restart(&mut self, from: f32, to: f32, now: Instant, dur: Duration, ease: fn(f32) -> f32) {
        self.from = from;
        self.to = to;
        self.start = now;
        self.dur = dur;
        self.ease = ease;
    }
}

/// O que estava na tela antes de uma mensagem, para saber o que entrou.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub modal: Option<std::mem::Discriminant<crate::state::Modal>>,
    pub modal_key: String,
    pub popup: Option<crate::state::Popup>,
    pub menu: bool,
    pub toast: Option<String>,
    pub page: String,
    pub rail_sel: String,
    pub ringing: bool,
}

pub struct Motion {
    pub enabled: bool,
    pub now: Instant,
    pub modal: Tween,
    pub popup: Tween,
    pub menu: Tween,
    pub toast: Tween,
    pub page: Tween,
    pub ring: Tween,
    /// Barra da esquerda: (passando o mouse, selecionado) por item.
    pub rail: HashMap<String, (Tween, Tween)>,
    pub rail_hover: Option<String>,
    pub rail_sel: String,
}

pub const ENTRADA: Duration = Duration::from_millis(220);
pub const RAPIDA: Duration = Duration::from_millis(160);

impl Default for Motion {
    fn default() -> Self {
        Motion {
            enabled: true,
            now: Instant::now(),
            modal: Tween::new(1.0),
            popup: Tween::new(1.0),
            menu: Tween::new(1.0),
            toast: Tween::new(1.0),
            page: Tween::new(1.0),
            ring: Tween::new(1.0),
            rail: HashMap::new(),
            rail_hover: None,
            rail_sel: String::new(),
        }
    }
}

impl Motion {
    fn dur(&self, d: Duration) -> Duration {
        if self.enabled {
            d
        } else {
            Duration::ZERO
        }
    }

    /// Alguma animação andando agora.
    pub fn running(&self) -> bool {
        let n = Instant::now();
        [&self.modal, &self.popup, &self.menu, &self.toast, &self.page, &self.ring].iter().any(|t| t.running(n))
            || self.rail.values().any(|(a, b)| a.running(n) || b.running(n))
    }

    /// Compara o antes e o depois de uma mensagem e começa as entradas.
    pub fn react(&mut self, antes: &Snapshot, depois: &Snapshot) {
        let now = Instant::now();
        self.now = now;
        if depois.modal.is_some() && (antes.modal != depois.modal || antes.modal_key != depois.modal_key) {
            self.modal.restart(0.0, 1.0, now, self.dur(ENTRADA), ease_out_cubic);
        }
        if depois.popup.is_some() && antes.popup != depois.popup {
            self.popup.restart(0.0, 1.0, now, self.dur(RAPIDA), ease_out_cubic);
        }
        if depois.menu && !antes.menu {
            self.menu.restart(0.0, 1.0, now, self.dur(Duration::from_millis(130)), ease_out_cubic);
        }
        if depois.toast.is_some() && antes.toast != depois.toast {
            self.toast.restart(0.0, 1.0, now, self.dur(Duration::from_millis(260)), ease_out_back);
        }
        if depois.page != antes.page {
            self.page.restart(0.0, 1.0, now, self.dur(ENTRADA), ease_out_cubic);
        }
        if depois.ringing && !antes.ringing {
            self.ring.restart(0.0, 1.0, now, self.dur(Duration::from_millis(320)), ease_out_back);
        }
        if depois.rail_sel != self.rail_sel {
            self.rail_sel = depois.rail_sel.clone();
            let d = self.dur(Duration::from_millis(200));
            let sel = self.rail_sel.clone();
            for (k, (_, s)) in self.rail.iter_mut() {
                s.go(if *k == sel { 1.0 } else { 0.0 }, now, d, ease_out_cubic);
            }
            let e = self.rail.entry(sel).or_insert((Tween::new(0.0), Tween::new(0.0)));
            e.1.go(1.0, now, d, ease_out_cubic);
        }
    }

    pub fn hover(&mut self, key: Option<String>) {
        if self.rail_hover == key {
            return;
        }
        let now = Instant::now();
        self.now = now;
        let d = self.dur(Duration::from_millis(150));
        if let Some(antigo) = self.rail_hover.take() {
            if let Some((h, _)) = self.rail.get_mut(&antigo) {
                h.go(0.0, now, d, ease_out_cubic);
            }
        }
        if let Some(k) = &key {
            let sel = *k == self.rail_sel;
            let e = self.rail.entry(k.clone()).or_insert((Tween::new(0.0), Tween::new(if sel { 1.0 } else { 0.0 })));
            e.0.go(1.0, now, d, ease_out_cubic);
        }
        self.rail_hover = key;
    }

    /// (passando o mouse, selecionado) de um item da barra, de 0 a 1.
    pub fn rail_of(&self, key: &str, sel: bool) -> (f32, f32) {
        match self.rail.get(key) {
            Some((h, s)) => (h.at(self.now), s.at(self.now)),
            None => (0.0, if sel { 1.0 } else { 0.0 }),
        }
    }

    pub fn v(&self, t: &Tween) -> f32 {
        t.at(self.now)
    }

    /// Segundos desde o começo, para o que gira ou pulsa sem parar.
    pub fn clock(&self) -> f32 {
        static INICIO: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        self.now.saturating_duration_since(*INICIO.get_or_init(Instant::now)).as_secs_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desliza_e_assenta() {
        let t0 = Instant::now();
        let mut t = Tween::new(0.0);
        t.go(1.0, t0, Duration::from_millis(200), ease_out_cubic);
        assert_eq!(t.at(t0), 0.0);
        let meio = t.at(t0 + Duration::from_millis(100));
        assert!(meio > 0.5 && meio < 1.0, "{meio}");
        assert_eq!(t.at(t0 + Duration::from_millis(250)), 1.0);
        assert!(!t.running(t0 + Duration::from_millis(250)));
        // Mudar de ideia no meio continua de onde estava, sem pular.
        t.go(0.0, t0 + Duration::from_millis(100), Duration::from_millis(200), ease_out_cubic);
        assert!((t.at(t0 + Duration::from_millis(100)) - meio).abs() < 1e-4);
    }

    #[test]
    fn curvas_comecam_em_0_e_terminam_em_1() {
        for f in [ease_out_cubic, ease_out_back] {
            assert!(f(0.0).abs() < 1e-5);
            assert!((f(1.0) - 1.0).abs() < 1e-5);
        }
        assert!(ease_out_back(0.8) > 1.0);
    }

    #[test]
    fn sem_animacoes_vai_direto() {
        let mut m = Motion { enabled: false, ..Default::default() };
        let antes = Snapshot::default();
        let depois = Snapshot { menu: true, ..Default::default() };
        m.react(&antes, &depois);
        assert!(!m.running());
        assert_eq!(m.v(&m.menu), 1.0);
    }
}
