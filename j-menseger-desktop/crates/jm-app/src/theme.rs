//! Cores, estilos e formatos da interface.
//!
//! Como no celular: preto e cinza, mais uma cor de destaque que a pessoa
//! escolhe (verde na primeira vez). A paleta vive num lugar só e as funções de
//! estilo leem dela, então trocar a cor nos ajustes muda o aplicativo inteiro
//! na hora.

use iced::widget::{button, container, scrollable, text_editor, text_input};
use iced::{Background, Border, Color, Shadow, Theme, Vector};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
}

pub fn with_alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: 1.0 }
}

pub fn hex(c: Color) -> String {
    format!("#{:02X}{:02X}{:02X}", (c.r * 255.0).round() as u8, (c.g * 255.0).round() as u8, (c.b * 255.0).round() as u8)
}

pub fn from_hex(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// Cor a partir de matiz (0–360), saturação e luminosidade (0–1).
pub fn hsl(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    Color { r: r + m, g: g + m, b: b + m, a: 1.0 }
}

pub fn hue_of(c: Color) -> f32 {
    let max = c.r.max(c.g).max(c.b);
    let min = c.r.min(c.g).min(c.b);
    let d = max - min;
    if d < 1e-6 {
        return 0.0;
    }
    let h = if max == c.r {
        60.0 * (((c.g - c.b) / d) % 6.0)
    } else if max == c.g {
        60.0 * ((c.b - c.r) / d + 2.0)
    } else {
        60.0 * ((c.r - c.g) / d + 4.0)
    };
    h.rem_euclid(360.0)
}

fn luminance(c: Color) -> f32 {
    0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
}

/// Fundos disponíveis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Backdrop {
    #[default]
    Grafite,
    Preto,
    Noite,
}

impl Backdrop {
    pub fn all() -> [Backdrop; 3] {
        [Backdrop::Grafite, Backdrop::Preto, Backdrop::Noite]
    }
    pub fn label(&self) -> &'static str {
        match self {
            Backdrop::Grafite => "Grafite",
            Backdrop::Preto => "Preto",
            Backdrop::Noite => "Azul-noite",
        }
    }
    /// (barra, lateral, principal, superfície)
    fn base(&self) -> [Color; 4] {
        match self {
            Backdrop::Grafite => [rgb(0x0a, 0x0b, 0x0d), rgb(0x11, 0x13, 0x16), rgb(0x16, 0x18, 0x1c), rgb(0x1e, 0x21, 0x26)],
            Backdrop::Preto => [rgb(0x00, 0x00, 0x00), rgb(0x07, 0x07, 0x08), rgb(0x0b, 0x0b, 0x0c), rgb(0x16, 0x16, 0x18)],
            Backdrop::Noite => [rgb(0x08, 0x0b, 0x14), rgb(0x0e, 0x13, 0x20), rgb(0x12, 0x18, 0x28), rgb(0x1b, 0x23, 0x36)],
        }
    }
}

/// O que fica gravado nos ajustes de aparência.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Look {
    pub accent: String,
    #[serde(default)]
    pub backdrop: Backdrop,
}

impl Default for Look {
    fn default() -> Self {
        Self { accent: "#2FD47C".into(), backdrop: Backdrop::Grafite }
    }
}

/// Cores de destaque prontas (a primeira é a de fábrica).
pub const PRESETS: [(&str, &str); 9] = [
    ("Verde", "#2FD47C"),
    ("Menta", "#2DD4BF"),
    ("Azul", "#3B82F6"),
    ("Anil", "#6366F1"),
    ("Roxo", "#A855F7"),
    ("Rosa", "#EC4899"),
    ("Vermelho", "#EF4444"),
    ("Laranja", "#F97316"),
    ("Amarelo", "#EAB308"),
];

#[derive(Clone, Copy)]
struct Palette {
    rail: Color,
    sidebar: Color,
    main: Color,
    surface: Color,
    accent: Color,
}

static PALETA: RwLock<Palette> = RwLock::new(Palette {
    rail: rgb(0x0a, 0x0b, 0x0d),
    sidebar: rgb(0x11, 0x13, 0x16),
    main: rgb(0x16, 0x18, 0x1c),
    surface: rgb(0x1e, 0x21, 0x26),
    accent: rgb(0x2f, 0xd4, 0x7c),
});

/// Aplica a aparência escolhida.
pub fn apply(look: &Look) {
    let [rail, sidebar, main, surface] = look.backdrop.base();
    let accent = from_hex(&look.accent).unwrap_or(rgb(0x2f, 0xd4, 0x7c));
    *PALETA.write().unwrap() = Palette { rail, sidebar, main, surface, accent };
}

fn p() -> Palette {
    *PALETA.read().unwrap()
}

pub fn rail() -> Color {
    p().rail
}
pub fn sidebar() -> Color {
    p().sidebar
}
pub fn main_bg() -> Color {
    p().main
}
pub fn surface() -> Color {
    p().surface
}
pub fn surface_hi() -> Color {
    mix(p().surface, Color::WHITE, 0.06)
}
pub fn line() -> Color {
    mix(p().main, Color::WHITE, 0.07)
}
pub const FG: Color = rgb(0xe6, 0xe8, 0xeb);
pub const FG_DIM: Color = rgb(0x9a, 0xa2, 0xad);
pub const FG_FAINT: Color = rgb(0x6b, 0x72, 0x7c);
pub const DANGER: Color = rgb(0xe5, 0x5b, 0x5b);
pub const WARN: Color = rgb(0xe8, 0xb3, 0x39);
pub const READ_BLUE: Color = rgb(0x53, 0xbd, 0xeb);

pub fn accent() -> Color {
    p().accent
}
/// Texto em cima da cor de destaque: escuro em cores claras, claro em escuras.
pub fn accent_ink() -> Color {
    if luminance(accent()) > 0.45 {
        mix(accent(), Color::BLACK, 0.88)
    } else {
        Color::WHITE
    }
}
pub fn bubble_me() -> Color {
    mix(p().main, accent(), 0.30)
}
pub fn bubble_them() -> Color {
    mix(p().surface, Color::WHITE, 0.025)
}

/// Seis tons de avatar, discretos (preto, cinza e um toque de cor).
pub fn tone(t: u8) -> Color {
    const TONS: [Color; 6] = [
        rgb(0x3a, 0x4a, 0x42),
        rgb(0x3b, 0x45, 0x58),
        rgb(0x4d, 0x40, 0x58),
        rgb(0x58, 0x46, 0x3c),
        rgb(0x55, 0x3e, 0x4a),
        rgb(0x3c, 0x52, 0x56),
    ];
    TONS[(t as usize) % 6]
}

/* ---------------- recipientes ---------------- */

pub fn fill(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Color(color)), text_color: Some(FG), ..Default::default() }
}

pub fn rounded(color: Color, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(color)),
        text_color: Some(FG),
        border: Border { radius: radius.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn panel(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(surface())),
        text_color: Some(FG),
        border: Border { color: line(), width: 1.0, radius: 14.0.into() },
        shadow: Shadow { color: with_alpha(Color::BLACK, 0.45), offset: Vector::new(0.0, 8.0), blur_radius: 28.0 },
        ..Default::default()
    }
}

pub fn header(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(main_bg())),
        text_color: Some(FG),
        shadow: Shadow { color: with_alpha(Color::BLACK, 0.35), offset: Vector::new(0.0, 2.0), blur_radius: 6.0 },
        ..Default::default()
    }
}

pub fn scrim_a(a: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Color(with_alpha(Color::BLACK, 0.62 * a.clamp(0.0, 1.0)))), ..Default::default() }
}

/// Item da barra da esquerda com o canto animado (círculo → quadrado
/// arredondado, como no Discord).
pub fn rail_item_r(sel: f32, radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let base = match status {
            button::Status::Hovered => surface_hi(),
            _ => surface(),
        };
        let bg = mix(base, accent(), sel.clamp(0.0, 1.0));
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if sel > 0.5 { accent_ink() } else { FG },
            border: Border { radius: radius.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

pub fn scrim(_: &Theme) -> container::Style {
    container::Style { background: Some(Background::Color(with_alpha(Color::BLACK, 0.62))), ..Default::default() }
}

pub fn bubble(mine: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(if mine { bubble_me() } else { bubble_them() })),
        text_color: Some(FG),
        border: Border { radius: 14.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn quote(mine: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(with_alpha(Color::BLACK, if mine { 0.22 } else { 0.18 }))),
        text_color: Some(FG_DIM),
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn system_note(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(with_alpha(surface(), 0.9))),
        text_color: Some(FG_DIM),
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn avatar(t: u8) -> impl Fn(&Theme) -> container::Style {
    let c = tone(t);
    move |_| container::Style {
        background: Some(Background::Color(c)),
        text_color: Some(Color::from_rgb(0.86, 0.87, 0.9)),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn square_avatar(t: u8) -> impl Fn(&Theme) -> container::Style {
    let c = tone(t);
    move |_| container::Style {
        background: Some(Background::Color(c)),
        text_color: Some(Color::from_rgb(0.86, 0.87, 0.9)),
        border: Border { radius: 14.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn badge(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(accent())),
        text_color: Some(accent_ink()),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn chip(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(with_alpha(color, 0.16))),
        text_color: Some(color),
        border: Border { radius: 999.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn toast(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(mix(surface(), Color::WHITE, 0.1))),
        text_color: Some(FG),
        border: Border { radius: 999.0.into(), ..Default::default() },
        shadow: Shadow { color: with_alpha(Color::BLACK, 0.5), offset: Vector::new(0.0, 6.0), blur_radius: 20.0 },
        ..Default::default()
    }
}

/* ---------------- botões ---------------- */

pub fn primary(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => mix(accent(), Color::WHITE, 0.12),
        button::Status::Pressed => mix(accent(), Color::BLACK, 0.15),
        button::Status::Disabled => surface_hi(),
        _ => accent(),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: if matches!(status, button::Status::Disabled) { FG_FAINT } else { accent_ink() },
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn secondary(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => mix(surface_hi(), Color::WHITE, 0.05),
        button::Status::Pressed => surface(),
        _ => surface_hi(),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: if matches!(status, button::Status::Disabled) { FG_FAINT } else { FG },
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn ghost(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => surface_hi(),
        button::Status::Pressed => surface(),
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: if matches!(status, button::Status::Disabled) { FG_FAINT } else { FG },
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// Botão redondo de ícone.
pub fn icon_btn(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if active {
            with_alpha(accent(), 0.18)
        } else {
            match status {
                button::Status::Hovered => surface_hi(),
                button::Status::Pressed => surface(),
                _ => Color::TRANSPARENT,
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if active { accent() } else { FG_DIM },
            border: Border { radius: 999.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

/// Botão redondo grande da chamada.
pub fn call_btn(kind: CallBtn) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let base = match kind {
            CallBtn::Normal => surface_hi(),
            CallBtn::On => FG,
            CallBtn::Danger => DANGER,
            CallBtn::Accept => accent(),
        };
        let bg = match status {
            button::Status::Hovered => mix(base, Color::WHITE, 0.12),
            button::Status::Pressed => mix(base, Color::BLACK, 0.12),
            _ => base,
        };
        let fg = match kind {
            CallBtn::On => rgb(0x11, 0x13, 0x16),
            CallBtn::Accept => accent_ink(),
            _ => Color::WHITE,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: fg,
            border: Border { radius: 999.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallBtn {
    Normal,
    On,
    Danger,
    Accept,
}

pub fn row_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if selected {
            surface_hi()
        } else {
            match status {
                button::Status::Hovered => with_alpha(surface(), 0.85),
                button::Status::Pressed => surface(),
                _ => Color::TRANSPARENT,
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if selected { FG } else { FG_DIM },
            border: Border { radius: 8.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

pub fn rail_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if selected {
            accent()
        } else {
            match status {
                button::Status::Hovered => surface_hi(),
                _ => surface(),
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if selected { accent_ink() } else { FG },
            border: Border { radius: if selected { 16.0 } else { 24.0 }.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

pub fn danger(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => rgb(0xf0, 0x6e, 0x6e),
            _ => DANGER,
        })),
        text_color: Color::WHITE,
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn danger_text(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => with_alpha(DANGER, 0.12),
            _ => Color::TRANSPARENT,
        })),
        text_color: DANGER,
        border: Border { radius: 10.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// Bolinha de cor (seleção de cor de destaque ou de avatar).
pub fn swatch(color: Color, selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(color)),
        text_color: if luminance(color) > 0.45 { Color::BLACK } else { Color::WHITE },
        border: Border {
            color: if selected { FG } else if matches!(status, button::Status::Hovered) { FG_DIM } else { Color::TRANSPARENT },
            width: if selected { 3.0 } else { 2.0 },
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

/// Opção de uma lista de escolha (selecionada fica com borda da cor de destaque).
pub fn option(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => surface_hi(),
            _ => surface(),
        })),
        text_color: FG,
        border: Border { color: if selected { accent() } else { line() }, width: if selected { 2.0 } else { 1.0 }, radius: 12.0.into() },
        ..Default::default()
    }
}

/* ---------------- campos ---------------- */

pub fn input(_: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => accent(),
        _ => line(),
    };
    text_input::Style {
        background: Background::Color(mix(surface(), Color::BLACK, 0.1)),
        border: Border { color: border_color, width: 1.0, radius: 10.0.into() },
        icon: FG_DIM,
        placeholder: FG_FAINT,
        value: FG,
        selection: with_alpha(accent(), 0.35),
    }
}

pub fn editor(_: &Theme, status: text_editor::Status) -> text_editor::Style {
    let border_color = match status {
        text_editor::Status::Focused { .. } => with_alpha(accent(), 0.7),
        _ => Color::TRANSPARENT,
    };
    text_editor::Style {
        background: Background::Color(surface()),
        border: Border { color: border_color, width: 1.0, radius: 12.0.into() },
        placeholder: FG_FAINT,
        value: FG,
        selection: with_alpha(accent(), 0.35),
    }
}

pub fn scroll(_: &Theme, _status: scrollable::Status) -> scrollable::Style {
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(surface_hi()),
            border: Border { radius: 999.0.into(), ..Default::default() },
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(surface()),
            border: Border { radius: 999.0.into(), ..Default::default() },
            shadow: Shadow::default(),
            icon: FG_DIM,
        },
    }
}

/* ---------------- textos ---------------- */

/// Iniciais de um nome, para o avatar.
pub fn initials(name: &str) -> String {
    let partes: Vec<&str> = name.split_whitespace().take(2).collect();
    if partes.is_empty() {
        return "?".into();
    }
    partes.iter().filter_map(|p| p.chars().next()).collect::<String>().to_uppercase()
}

fn local(ts_ms: i64) -> chrono::DateTime<chrono::Local> {
    use chrono::TimeZone;
    chrono::Local.timestamp_millis_opt(ts_ms).single().unwrap_or_else(chrono::Local::now)
}

/// Hora local ao lado da mensagem.
pub fn hhmm(ts_ms: i64) -> String {
    local(ts_ms).format("%H:%M").to_string()
}

/// Quando, na lista de conversas: hora se hoje, "Ontem", ou a data.
pub fn list_time(ts_ms: i64) -> String {
    let d = local(ts_ms).date_naive();
    let hoje = chrono::Local::now().date_naive();
    if d == hoje {
        hhmm(ts_ms)
    } else if hoje.signed_duration_since(d).num_days() == 1 {
        "Ontem".into()
    } else {
        local(ts_ms).format("%d/%m/%y").to_string()
    }
}

/// Separador de dia na conversa.
pub fn day_label(ts_ms: i64) -> String {
    use chrono::Datelike;
    let d = local(ts_ms).date_naive();
    let hoje = chrono::Local::now().date_naive();
    let dias = hoje.signed_duration_since(d).num_days();
    match dias {
        0 => "Hoje".into(),
        1 => "Ontem".into(),
        2..=6 => ["Segunda-feira", "Terça-feira", "Quarta-feira", "Quinta-feira", "Sexta-feira", "Sábado", "Domingo"]
            [d.weekday().num_days_from_monday() as usize]
            .into(),
        _ => local(ts_ms).format("%d/%m/%Y").to_string(),
    }
}

pub fn day_key(ts_ms: i64) -> chrono::NaiveDate {
    local(ts_ms).date_naive()
}

/// Duração no formato 1:23.
pub fn mmss(secs: f32) -> String {
    let s = secs.max(0.0).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Tamanho em bytes, legível.
pub fn bytes_label(n: i64) -> String {
    let n = n.max(0) as f64;
    if n < 1024.0 {
        format!("{n:.0} B")
    } else if n < 1024.0 * 1024.0 {
        format!("{:.0} KB", n / 1024.0)
    } else {
        format!("{:.1} MB", n / 1024.0 / 1024.0).replace('.', ",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iniciais_de_um_nome() {
        assert_eq!(initials("Ana Souza"), "AS");
        assert_eq!(initials("Beto"), "B");
        assert_eq!(initials("ana maria souza"), "AM");
        assert_eq!(initials(""), "?");
    }

    #[test]
    fn hora_e_no_fuso_do_computador() {
        // Meia-noite UTC precisa sair na hora local, não "00:00" fixo.
        use chrono::TimeZone;
        let esperado = chrono::Local.timestamp_millis_opt(0).single().unwrap().format("%H:%M").to_string();
        assert_eq!(hhmm(0), esperado);
    }

    #[test]
    fn cores_hexadecimais_vao_e_voltam() {
        let c = from_hex("#2FD47C").unwrap();
        assert_eq!(hex(c), "#2FD47C");
        assert!(from_hex("zzz").is_none());
    }

    #[test]
    fn texto_sobre_a_cor_de_destaque_e_legivel() {
        apply(&Look { accent: "#EAB308".into(), backdrop: Backdrop::Grafite });
        assert!(luminance(accent_ink()) < 0.2, "amarelo pede texto escuro");
        apply(&Look { accent: "#6366F1".into(), backdrop: Backdrop::Grafite });
        assert_eq!(accent_ink(), Color::WHITE, "anil pede texto claro");
        apply(&Look::default());
    }

    #[test]
    fn tamanhos_legiveis() {
        assert_eq!(bytes_label(500), "500 B");
        assert_eq!(bytes_label(2048), "2 KB");
        assert_eq!(bytes_label(3 * 1024 * 1024 + 300_000), "3,3 MB");
    }

    #[test]
    fn duracao_curta() {
        assert_eq!(mmss(83.0), "1:23");
        assert_eq!(mmss(0.4), "0:00");
    }
}
