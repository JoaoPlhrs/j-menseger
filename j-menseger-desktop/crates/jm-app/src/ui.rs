//! Peças de interface usadas em várias telas.

use crate::app::Msg;
use crate::icons::{icon, Icon};
use crate::theme as t;
use iced::widget::{button, column, container, image, mouse_area, row, text, toggler, tooltip, Space};
use iced::{Alignment, Color, ContentFit, Element, Font, Length, Padding};

pub type El<'a> = Element<'a, Msg>;

pub const BOLD: Font = Font { weight: iced::font::Weight::Bold, ..Font::DEFAULT };
pub const SEMI: Font = Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT };

/// Avatar redondo: a foto, ou as iniciais sobre a cor escolhida.
pub fn avatar<'a>(name: &str, tone: u8, pic: Option<image::Handle>, size: f32) -> El<'a> {
    match pic {
        Some(h) => image(h).width(size).height(size).content_fit(ContentFit::Cover).border_radius(size / 2.0).into(),
        None => container(text(t::initials(name)).size((size * 0.38).max(10.0)).font(SEMI))
            .center(size)
            .style(t::avatar(tone))
            .into(),
    }
}

/// Avatar com anel (quem está falando numa chamada, ou destaque).
pub fn ringed<'a>(inner: El<'a>, size: f32, ring: Option<Color>) -> El<'a> {
    let cor = ring.unwrap_or(Color::TRANSPARENT);
    container(inner)
        .center(size + 8.0)
        .style(move |_| container::Style {
            border: iced::Border { color: cor, width: 2.5, radius: 999.0.into() },
            ..Default::default()
        })
        .into()
}

/// Avatar de grupo: quadrado de cantos redondos.
pub fn group_avatar<'a>(name: &str, tone: u8, pic: Option<image::Handle>, size: f32) -> El<'a> {
    match pic {
        Some(h) => image(h).width(size).height(size).content_fit(ContentFit::Cover).border_radius(size * 0.3).into(),
        None => container(text(t::initials(name)).size((size * 0.36).max(10.0)).font(SEMI))
            .center(size)
            .style(t::square_avatar(tone))
            .into(),
    }
}

/// Dica que aparece ao passar o mouse.
pub fn tip<'a>(content: impl Into<El<'a>>, label: impl Into<String>, pos: tooltip::Position) -> El<'a> {
    let label: String = label.into();
    tooltip(
        content,
        container(text(label).size(12.5).color(t::FG)).padding([5, 9]).style(|_| container::Style {
            background: Some(iced::Background::Color(t::mix(t::surface(), Color::BLACK, 0.35))),
            text_color: Some(t::FG),
            border: iced::Border { radius: 7.0.into(), color: t::line(), width: 1.0 },
            shadow: iced::Shadow { color: t::with_alpha(Color::BLACK, 0.4), offset: iced::Vector::new(0.0, 3.0), blur_radius: 10.0 },
            ..Default::default()
        }),
        pos,
    )
    .delay(std::time::Duration::from_millis(350))
    .into()
}

/// Botão redondo de ícone, com dica.
pub fn icon_button<'a>(i: Icon, label: &str, msg: Option<Msg>, active: bool) -> El<'a> {
    let cor = if active { t::accent() } else { t::FG_DIM };
    let b = button(container(icon(i, 20.0, cor)).center(20.0)).padding(8).style(t::icon_btn(active)).on_press_maybe(msg);
    tip(b, label.to_string(), tooltip::Position::Bottom)
}

pub fn icon_button_sm<'a>(i: Icon, label: &str, msg: Option<Msg>) -> El<'a> {
    let b = button(container(icon(i, 16.0, t::FG_DIM)).center(16.0)).padding(6).style(t::icon_btn(false)).on_press_maybe(msg);
    tip(b, label.to_string(), tooltip::Position::Top)
}

fn labeled<'a>(i: Option<Icon>, label: &str, color: Color) -> El<'a> {
    let mut r = row![].spacing(8).align_y(Alignment::Center);
    if let Some(i) = i {
        r = r.push(icon(i, 17.0, color));
    }
    r.push(text(label.to_string()).size(14.5).font(SEMI)).into()
}

pub fn primary<'a>(i: Option<Icon>, label: &str, msg: Option<Msg>) -> button::Button<'a, Msg> {
    let cor = if msg.is_some() { t::accent_ink() } else { t::FG_FAINT };
    button(container(labeled(i, label, cor)).center_x(Length::Shrink)).padding([11, 18]).style(t::primary).on_press_maybe(msg)
}

pub fn secondary<'a>(i: Option<Icon>, label: &str, msg: Option<Msg>) -> button::Button<'a, Msg> {
    button(container(labeled(i, label, t::FG)).center_x(Length::Shrink)).padding([11, 18]).style(t::secondary).on_press_maybe(msg)
}

/// Versões que ocupam a largura toda, com o texto no centro.
pub fn primary_wide<'a>(i: Option<Icon>, label: &str, msg: Option<Msg>) -> button::Button<'a, Msg> {
    let cor = if msg.is_some() { t::accent_ink() } else { t::FG_FAINT };
    button(container(labeled(i, label, cor)).center_x(Length::Fill)).width(Length::Fill).padding([11, 18]).style(t::primary).on_press_maybe(msg)
}

pub fn secondary_wide<'a>(i: Option<Icon>, label: &str, msg: Option<Msg>) -> button::Button<'a, Msg> {
    button(container(labeled(i, label, t::FG)).center_x(Length::Fill)).width(Length::Fill).padding([11, 18]).style(t::secondary).on_press_maybe(msg)
}

pub fn danger_wide<'a>(i: Option<Icon>, label: &str, msg: Option<Msg>) -> button::Button<'a, Msg> {
    button(container(labeled(i, label, Color::WHITE)).center_x(Length::Fill)).width(Length::Fill).padding([11, 18]).style(t::danger).on_press_maybe(msg)
}

pub fn ghost<'a>(i: Option<Icon>, label: &str, msg: Option<Msg>) -> button::Button<'a, Msg> {
    button(labeled(i, label, t::FG)).padding([9, 12]).style(t::ghost).on_press_maybe(msg)
}

pub fn danger<'a>(i: Option<Icon>, label: &str, msg: Option<Msg>) -> button::Button<'a, Msg> {
    button(container(labeled(i, label, Color::WHITE)).center_x(Length::Shrink)).padding([11, 18]).style(t::danger).on_press_maybe(msg)
}

pub fn danger_text<'a>(i: Option<Icon>, label: &str, msg: Option<Msg>) -> button::Button<'a, Msg> {
    button(labeled(i, label, t::DANGER)).padding([9, 12]).style(t::danger_text).on_press_maybe(msg)
}

/// Rótulo de seção ("CONTA", "MENSAGENS"…).
pub fn label<'a>(s: &str) -> El<'a> {
    container(text(s.to_uppercase()).size(11.5).font(BOLD).color(t::FG_FAINT)).padding(Padding { top: 18.0, bottom: 6.0, left: 4.0, right: 4.0 }).into()
}

pub fn note<'a>(s: impl Into<String>) -> El<'a> {
    let s: String = s.into();
    text(s).size(13).color(t::FG_FAINT).into()
}

pub fn title<'a>(s: impl Into<String>) -> El<'a> {
    let s: String = s.into();
    text(s).size(22).font(BOLD).color(t::FG).into()
}

pub fn subtitle<'a>(s: impl Into<String>) -> El<'a> {
    let s: String = s.into();
    text(s).size(14).color(t::FG_DIM).into()
}

/// O que fica à direita de uma linha de ajustes.
pub enum Trail<'a> {
    None,
    Chevron,
    Value(String),
    Switch(bool, fn(bool) -> Msg),
    SwitchOff(bool),
    Custom(El<'a>),
}

/// Linha de ajustes, como as do celular: ícone, título, explicação e o que
/// fica à direita (valor, seta ou interruptor).
pub fn setting<'a>(i: Option<Icon>, name: &str, sub: Option<String>, trail: Trail<'a>, msg: Option<Msg>, danger: bool) -> El<'a> {
    let cor_titulo = if danger { t::DANGER } else { t::FG };
    let mut r = row![].spacing(14).align_y(Alignment::Center).padding([10, 12]);
    if let Some(i) = i {
        r = r.push(
            container(icon(i, 19.0, if danger { t::DANGER } else { t::FG_DIM }))
                .center(36.0)
                .style(t::rounded(t::with_alpha(Color::WHITE, 0.04), 10.0)),
        );
    }
    let mut meio = column![text(name.to_string()).size(14.5).font(SEMI).color(cor_titulo)].spacing(2).width(Length::Fill);
    if let Some(s) = sub {
        meio = meio.push(text(s).size(12.5).color(t::FG_DIM));
    }
    r = r.push(meio);
    let mut clique = msg;
    match trail {
        Trail::None => {}
        Trail::Chevron => r = r.push(icon(Icon::ChevRight, 16.0, t::FG_FAINT)),
        Trail::Value(v) => {
            r = r.push(text(v).size(13.5).color(t::FG_DIM));
            r = r.push(icon(Icon::ChevRight, 16.0, t::FG_FAINT));
        }
        Trail::Switch(on, f) => {
            if clique.is_none() {
                clique = Some(f(!on));
            }
            r = r.push(toggler(on).on_toggle(f).size(20));
        }
        Trail::SwitchOff(on) => r = r.push(toggler(on).size(20)),
        Trail::Custom(e) => r = r.push(e),
    }
    let b = button(r).padding(0).width(Length::Fill).style(t::row_item(false)).on_press_maybe(clique);
    b.into()
}

/// Caixa com as linhas de uma seção de ajustes.
pub fn group<'a>(rows: Vec<El<'a>>) -> El<'a> {
    container(column(rows).spacing(2).padding(4)).width(Length::Fill).style(t::rounded(t::surface(), 14.0)).into()
}

/// Selo com número (não lidas).
pub fn badge<'a>(n: u32) -> El<'a> {
    let s = if n > 99 { "99+".to_string() } else { n.to_string() };
    container(text(s).size(11.5).font(BOLD).color(t::accent_ink())).padding([1, 6]).style(t::badge).into()
}

/// Item de menu de contexto.
pub fn menu_item<'a>(i: Icon, label: &str, msg: Msg, danger: bool) -> El<'a> {
    let cor = if danger { t::DANGER } else { t::FG };
    button(row![icon(i, 17.0, if danger { t::DANGER } else { t::FG_DIM }), text(label.to_string()).size(14).color(cor)].spacing(10).align_y(Alignment::Center))
        .width(Length::Fill)
        .padding([8, 10])
        .style(move |th, st| {
            let mut s = t::ghost(th, st);
            s.border.radius = 7.0.into();
            s
        })
        .on_press(msg)
        .into()
}

/// Campo de formulário com rótulo em cima.
pub fn field<'a>(label: &str, input: impl Into<El<'a>>) -> El<'a> {
    column![text(label.to_string()).size(12.5).font(SEMI).color(t::FG_DIM), input.into()].spacing(6).into()
}

pub fn input<'a>(placeholder: &str, value: &str, on: fn(String) -> Msg) -> iced::widget::TextInput<'a, Msg> {
    iced::widget::text_input(placeholder, value).on_input(on).padding([10, 12]).size(14.5).style(t::input)
}

pub fn hspace<'a>(w: f32) -> El<'a> {
    Space::new().width(w).into()
}

pub fn vspace<'a>(h: f32) -> El<'a> {
    Space::new().height(h).into()
}

pub fn fill<'a>() -> El<'a> {
    Space::new().width(Length::Fill).into()
}

/// Forma de onda clicável (posição na mensagem de voz). Cada barra leva
/// para aquele ponto do áudio.
pub fn waveform<'a>(wf: &[u8], progress: f32, played: Color, rest: Color, on_seek: Option<&dyn Fn(f32) -> Msg>) -> El<'a> {
    let n = wf.len().max(1);
    let barras: Vec<El<'a>> = wf
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let frac = i as f32 / n as f32;
            let cor = if frac < progress { played } else { rest };
            let h = 4.0 + (*v as f32).clamp(0.0, 31.0) / 31.0 * 22.0;
            let barra = container(Space::new()).width(3).height(h).style(t::rounded(cor, 2.0));
            let celula = container(barra).center_y(28).width(5);
            match on_seek {
                Some(f) => mouse_area(celula).on_press(f(frac)).interaction(iced::mouse::Interaction::Pointer).into(),
                None => celula.into(),
            }
        })
        .collect();
    row(barras).spacing(0).align_y(Alignment::Center).into()
}

/// Medidor de nível do microfone.
pub fn level_bars<'a>(levels: &[f32], n: usize, color: Color) -> El<'a> {
    let ini = levels.len().saturating_sub(n);
    let mut v: Vec<f32> = levels[ini..].to_vec();
    while v.len() < n {
        v.insert(0, 0.0);
    }
    row(v.into_iter()
        .map(|l| {
            let h = 3.0 + l.clamp(0.0, 1.0).sqrt() * 26.0;
            container(container(Space::new()).width(3).height(h).style(t::rounded(color, 2.0))).center_y(30).width(5).into()
        })
        .collect::<Vec<El<'a>>>())
    .align_y(Alignment::Center)
    .into()
}

/// Recorta um texto longo com reticências, sem partir caracteres.
pub fn ellipsis(s: &str, max: usize) -> String {
    let limpo: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if limpo.chars().count() <= max {
        return limpo;
    }
    let mut out: String = limpo.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// A mensagem só tem emojis (até 3)? Aparece maior, como no celular.
pub fn only_emoji(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.chars().count() > 24 {
        return false;
    }
    let mut n = 0;
    for c in s.chars() {
        if c.is_whitespace() || c == '\u{200d}' || c == '\u{fe0f}' || ('\u{1f3fb}'..='\u{1f3ff}').contains(&c) {
            continue;
        }
        if c.is_alphanumeric() || c.is_ascii() {
            return false;
        }
        n += 1;
    }
    (1..=3).contains(&n)
}

/// Divide um texto em pedaços comuns e links (http/https), para os links
/// abrirem no navegador.
pub fn split_links(s: &str) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut resto = s;
    while let Some(i) = find_url(resto) {
        if i > 0 {
            out.push((resto[..i].to_string(), false));
        }
        let fim = resto[i..].find(|c: char| c.is_whitespace()).map(|f| i + f).unwrap_or(resto.len());
        let mut url = &resto[i..fim];
        // Pontuação colada no fim não faz parte do link.
        while url.ends_with(['.', ',', ';', ':', '!', '?', ')', ']', '"', '\'']) {
            url = &url[..url.len() - 1];
        }
        let fim = i + url.len();
        out.push((url.to_string(), true));
        resto = &resto[fim..];
    }
    if !resto.is_empty() {
        out.push((resto.to_string(), false));
    }
    out
}

fn find_url(s: &str) -> Option<usize> {
    let a = s.find("https://");
    let b = s.find("http://");
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (x, y) => x.or(y),
    }
}

/// Emojis do painel de emojis da conversa.
pub const EMOJIS: &[&str] = &[
    "😀", "😃", "😄", "😁", "😆", "😅", "🤣", "😂", "🙂", "🙃", "😉", "😊", "😇", "🥰", "😍", "🤩", "😘", "😗", "😚", "😋", "😛",
    "😜", "🤪", "😝", "🤑", "🤗", "🤭", "🤫", "🤔", "🤐", "🤨", "😐", "😑", "😶", "😏", "😒", "🙄", "😬", "😌", "😔", "😪", "🤤",
    "😴", "😷", "🤒", "🤕", "🤢", "🤮", "🥵", "🥶", "🥴", "😵", "🤯", "🤠", "🥳", "😎", "🤓", "🧐", "😕", "😟", "🙁", "😮", "😯",
    "😲", "😳", "🥺", "😦", "😧", "😨", "😰", "😥", "😢", "😭", "😱", "😖", "😣", "😞", "😓", "😩", "😫", "🥱", "😤", "😡", "😠",
    "🤬", "😈", "💀", "💩", "🤡", "👻", "👽", "🤖", "😺", "😸", "😹", "😻", "🙈", "🙉", "🙊", "💋", "💯", "💥", "💫", "💦", "💨",
    "👋", "🤚", "✋", "🖖", "👌", "🤌", "✌️", "🤞", "🤟", "🤘", "🤙", "👈", "👉", "👆", "👇", "👍", "👎", "✊", "👊", "👏", "🙌",
    "👐", "🤝", "🙏", "💪", "🧠", "👀", "❤️", "🧡", "💛", "💚", "💙", "💜", "🖤", "🤍", "💔", "❣️", "💕", "💞", "💓", "💗", "💖",
    "🔥", "✨", "⭐", "🌟", "🎉", "🎊", "🎁", "🎂", "🍕", "🍔", "🍟", "🌮", "🍿", "🍩", "🍪", "☕", "🍺", "🍻", "🥂", "⚽", "🏀",
    "🎮", "🎧", "🎵", "🎶", "📷", "📱", "💻", "⏰", "📌", "✅", "❌", "⚠️", "❓", "❗", "💤", "🚀", "🌈", "☀️", "🌙", "⚡", "🐶",
    "🐱", "🦊", "🐼", "🐸", "🦆", "🦄", "🐝",
];

/// Reações rápidas (as do celular).
pub const QUICK_REACTS: [&str; 6] = ["❤️", "😂", "😮", "😢", "🙏", "👍"];

/// Emojis para sons do soundboard.
pub const SOUND_EMOJIS: [&str; 36] = [
    "🔊", "😂", "🤣", "😱", "😮", "🤯", "😭", "😎", "👏", "🎉", "🥁", "🎺", "🎸", "🎵", "📯", "🦆", "🐶", "🐱", "🐮", "🦗", "💥", "🔥",
    "💀", "👻", "🤖", "👽", "🚨", "⏰", "📞", "🍕", "💤", "❤️", "👍", "👎", "🙈", "🤡",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corta_texto_longo_com_reticencias() {
        assert_eq!(ellipsis("curto", 10), "curto");
        assert_eq!(ellipsis("1234567890", 10), "1234567890");
        assert_eq!(ellipsis("12345678901", 10), "123456789…");
        assert_eq!(ellipsis("linha\nquebrada", 20), "linha quebrada");
        let acentos = "áéíóúãõçàâ e mais";
        assert!(ellipsis(acentos, 5).chars().count() <= 5);
    }

    #[test]
    fn mensagem_so_de_emoji() {
        assert!(only_emoji("😂"));
        assert!(only_emoji("❤️ 🔥"));
        assert!(only_emoji("👍🏽"));
        assert!(!only_emoji("oi 😂"));
        assert!(!only_emoji("😂😂😂😂"));
        assert!(!only_emoji(""));
        assert!(!only_emoji("123"));
    }

    #[test]
    fn links_viram_pedacos_clicaveis() {
        let p = split_links("veja https://exemplo.com/a?b=1, e http://x.org.");
        assert_eq!(
            p,
            vec![
                ("veja ".to_string(), false),
                ("https://exemplo.com/a?b=1".to_string(), true),
                (", e ".to_string(), false),
                ("http://x.org".to_string(), true),
                (".".to_string(), false),
            ]
        );
        assert_eq!(split_links("sem link"), vec![("sem link".to_string(), false)]);
    }
}

/// Desloca o conteúdo (entradas animadas). Parado, não muda nada: o
/// `float` repassa o estado do conteúdo, então ligar e desligar o
/// deslocamento não perde foco nem rolagem.
pub fn slide<'a>(content: impl Into<El<'a>>, dx: f32, dy: f32) -> El<'a> {
    if dx.abs() < 0.3 && dy.abs() < 0.3 {
        return content.into();
    }
    iced::widget::float(content).translate(move |_, _| iced::Vector::new(dx, dy)).into()
}

/// Três pontinhos pulando, como o "digitando" do Discord.
pub fn typing_dots<'a>(clock: f32, cor: Color) -> El<'a> {
    let pontos: Vec<El<'a>> = (0..3)
        .map(|i| {
            let fase = (clock * 7.0 - i as f32 * 0.9).sin().max(0.0);
            let sobe = fase * 3.5;
            column![
                Space::new().height(3.5 - sobe),
                container(Space::new()).width(5).height(5).style(t::rounded(t::with_alpha(cor, 0.55 + 0.45 * fase), 3.0)),
                Space::new().height(sobe),
            ]
            .into()
        })
        .collect();
    row(pontos).spacing(3).align_y(Alignment::End).into()
}

/// "digitando" com os pontinhos animados.
pub fn typing_label<'a>(clock: f32, size: f32) -> El<'a> {
    row![typing_dots(clock, t::accent()), text("digitando").size(size).color(t::accent())].spacing(6).align_y(Alignment::Center).into()
}
