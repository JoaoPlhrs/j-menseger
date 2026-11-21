//! Ícones próprios, desenhados em traço (24 × 24), no estilo dos do celular.
//! Os que existiam no celular (mais, onda, lixeira, play, editar, câmera,
//! setas, parar) usam os mesmos traços de lá.

use iced::widget::svg;
use iced::{Color, Element, Length};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Chat,
    Users,
    Phone,
    PhoneOff,
    Video,
    Mic,
    MicOff,
    Volume,
    Send,
    Smile,
    Image,
    Plus,
    Wave,
    ChevDown,
    ChevRight,
    Back,
    X,
    Check,
    Checks,
    Clock,
    Hash,
    Settings,
    User,
    UserPlus,
    Lock,
    Key,
    Shield,
    Bell,
    BellOff,
    Pin,
    Reply,
    Copy,
    Trash,
    Play,
    Pause,
    Stop,
    Edit,
    Camera,
    Up,
    Down,
    Qr,
    Timer,
    Ban,
    Server,
    Share,
    Download,
    Logout,
    Sparkle,
    Eye,
    EyeOff,
    Screen,
    Sliders,
    Drop,
    Db,
    Info,
    More,
    Alert,
    Refresh,
    Music,
    Upload,
    Game,
    Maximize,
    Minimize,
}

fn path(i: Icon) -> &'static str {
    match i {
        Icon::Chat => r#"<path d="M20 11.5a7.5 7.5 0 0 1-11.1 6.6L4 19.5l1.4-4.6A7.5 7.5 0 1 1 20 11.5Z"/>"#,
        Icon::Users => r#"<circle cx="9" cy="8.5" r="3.5"/><path d="M2.5 19.5c.8-3.2 3.4-5 6.5-5s5.7 1.8 6.5 5"/><path d="M15.5 5.2a3.4 3.4 0 0 1 0 6.6M18.2 14.6c1.6.7 2.8 2.4 3.3 4.9"/>"#,
        Icon::Phone => r#"<path d="M6.6 3.5h2.6l1.4 4-2 1.3a12 12 0 0 0 6.6 6.6l1.3-2 4 1.4v2.6a2 2 0 0 1-2.2 2A16.6 16.6 0 0 1 4.6 5.7a2 2 0 0 1 2-2.2Z"/>"#,
        Icon::PhoneOff => r#"<path d="M3.5 13.5c4.6-4 12.4-4 17 0l-1.6 2.6-3.4-1.1v-2.6a10 10 0 0 0-7 0V15l-3.4 1.1Z"/>"#,
        Icon::Video => r#"<rect x="3" y="6.5" width="12.5" height="11" rx="2.2"/><path d="M15.5 10.5 21 7.5v9l-5.5-3"/>"#,
        Icon::Mic => r#"<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21"/>"#,
        Icon::MicOff => r#"<path d="M15 9.5V6a3 3 0 0 0-5.8-1M9 9v2a3 3 0 0 0 4.6 2.5M5.5 11a6.5 6.5 0 0 0 10.6 5M18.5 11a6.4 6.4 0 0 1-.5 2.4M12 17.5V21M3.5 3.5l17 17"/>"#,
        Icon::Volume => r#"<path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4Z"/><path d="M15.5 9a4.2 4.2 0 0 1 0 6M18 6.5a7.8 7.8 0 0 1 0 11"/>"#,
        Icon::Send => r#"<path d="M4 12 20 4.5 13.5 20l-2.3-6.2Z"/><path d="M11.2 13.8 20 4.5"/>"#,
        Icon::Smile => r#"<circle cx="12" cy="12" r="8.5"/><path d="M8.5 14.2c1.8 2 5.2 2 7 0"/><circle cx="9.2" cy="10" r=".6" fill="currentColor"/><circle cx="14.8" cy="10" r=".6" fill="currentColor"/>"#,
        Icon::Image => r#"<rect x="3.5" y="4.5" width="17" height="15" rx="2.5"/><circle cx="9" cy="10" r="1.6"/><path d="m4 17 4.6-4.2 3.4 3 3.2-3.2L20 17"/>"#,
        Icon::Plus => r#"<path d="M12 5V19M5 12H19"/>"#,
        Icon::Wave => r#"<path d="M4 10V14M8 6V18M12 3.5V20.5M16 7.5V16.5M20 10.5V13.5"/>"#,
        Icon::ChevDown => r#"<path d="M6 9.5 12 15.5 18 9.5"/>"#,
        Icon::ChevRight => r#"<path d="M9.5 6 15.5 12 9.5 18"/>"#,
        Icon::Back => r#"<path d="M15 5.5 8.5 12l6.5 6.5"/>"#,
        Icon::X => r#"<path d="M6 6l12 12M18 6 6 18"/>"#,
        Icon::Check => r#"<path d="m5 12.5 4.2 4.2L19 7"/>"#,
        Icon::Checks => r#"<path d="m2.5 12.5 4.2 4.2L16.5 7M12 15.8l.9.9L22.5 7"/>"#,
        Icon::Clock => r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>"#,
        Icon::Hash => r#"<path d="M9.5 4 8 20M16 4l-1.5 16M4.5 9h16M3.5 15h16"/>"#,
        Icon::Settings => r#"<circle cx="12" cy="12" r="3"/><path d="M19.4 13.5a7.7 7.7 0 0 0 0-3l2-1.5-2-3.4-2.3.9a7.6 7.6 0 0 0-2.6-1.5L14 2.5h-4l-.5 2.5A7.6 7.6 0 0 0 6.9 6.5l-2.3-.9-2 3.4 2 1.5a7.7 7.7 0 0 0 0 3l-2 1.5 2 3.4 2.3-.9a7.6 7.6 0 0 0 2.6 1.5l.5 2.5h4l.5-2.5a7.6 7.6 0 0 0 2.6-1.5l2.3.9 2-3.4Z"/>"#,
        Icon::User => r#"<circle cx="12" cy="8" r="4"/><path d="M4 20.5c1-4 4.2-6 8-6s7 2 8 6"/>"#,
        Icon::UserPlus => r#"<circle cx="10" cy="8" r="4"/><path d="M2.5 20.5c1-4 4-6 7.5-6 1.6 0 3 .4 4.2 1.1M18.5 13v6M15.5 16h6"/>"#,
        Icon::Lock => r#"<rect x="4.5" y="10.5" width="15" height="10" rx="2.5"/><path d="M8 10.5V8a4 4 0 0 1 8 0v2.5"/>"#,
        Icon::Key => r#"<circle cx="8" cy="15" r="4.5"/><path d="m11.2 11.8 8.3-8.3M16.5 6.5l2.5 2.5M14 9l2 2"/>"#,
        Icon::Shield => r#"<path d="M12 3 4.5 6v5.5c0 4.5 3.2 8.2 7.5 9.5 4.3-1.3 7.5-5 7.5-9.5V6Z"/><path d="m8.8 12.2 2.3 2.3 4.3-4.6"/>"#,
        Icon::Bell => r#"<path d="M6 16.5V11a6 6 0 0 1 12 0v5.5l1.5 1.5h-15ZM10 20.5a2.2 2.2 0 0 0 4 0"/>"#,
        Icon::BellOff => r#"<path d="M9 5.6A6 6 0 0 1 18 11v4M6 11v5.5L4.5 18H17M10 20.5a2.2 2.2 0 0 0 4 0M3.5 3.5l17 17"/>"#,
        Icon::Pin => r#"<path d="M9 3.5h6l-1 5 3.5 3.5h-11L10 8.5ZM12 12v8.5"/>"#,
        Icon::Reply => r#"<path d="M9.5 7 4 12.5 9.5 18M4 12.5h10a6 6 0 0 1 6 6v1"/>"#,
        Icon::Copy => r#"<rect x="8.5" y="8.5" width="11.5" height="11.5" rx="2.2"/><path d="M15.5 8.5V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7.5a2 2 0 0 0 2 2h2.5"/>"#,
        Icon::Trash => r#"<path d="M4.5 7H19.5M9.5 7V4.8C9.5 4.1 10 3.5 10.7 3.5H13.3C14 3.5 14.5 4.1 14.5 4.8V7M6.5 7L7.3 18.7A2 2 0 0 0 9.3 20.5H14.7A2 2 0 0 0 16.7 18.7L17.5 7M10 11V16.5M14 11V16.5"/>"#,
        Icon::Play => r#"<path d="M8 5.2V18.8A.8.8 0 0 0 9.2 19.5L20.2 12.7A.8.8 0 0 0 20.2 11.3L9.2 4.5A.8.8 0 0 0 8 5.2Z" fill="currentColor"/>"#,
        Icon::Pause => r#"<rect x="6.5" y="5" width="3.6" height="14" rx="1" fill="currentColor"/><rect x="13.9" y="5" width="3.6" height="14" rx="1" fill="currentColor"/>"#,
        Icon::Stop => r#"<rect x="6.5" y="6.5" width="11" height="11" rx="2" fill="currentColor"/>"#,
        Icon::Edit => r#"<path d="M4 20 4.9 15.9 16.4 4.4A2.3 2.3 0 0 1 19.6 7.6L8.1 19.1 4 20ZM14.6 6.2 17.8 9.4"/>"#,
        Icon::Camera => r#"<path d="M8.5 6.5 9.8 4.6A1.3 1.3 0 0 1 10.9 4H13.1A1.3 1.3 0 0 1 14.2 4.6L15.5 6.5H18A2.5 2.5 0 0 1 20.5 9V17.5A2.5 2.5 0 0 1 18 20H5.2A1.7 1.7 0 0 1 3.5 18.3V9A2.5 2.5 0 0 1 6 6.5Z"/><circle cx="12" cy="13" r="3.5"/>"#,
        Icon::Up => r#"<path d="M12 19V5M6 11 12 5 18 11"/>"#,
        Icon::Down => r#"<path d="M12 5V19M6 13 12 19 18 13"/>"#,
        Icon::Qr => r#"<rect x="3.5" y="3.5" width="7" height="7" rx="1.2"/><rect x="13.5" y="3.5" width="7" height="7" rx="1.2"/><rect x="3.5" y="13.5" width="7" height="7" rx="1.2"/><path d="M13.5 13.5h3v3h-3zM17.5 17.5h3v3h-3zM20.5 13.5v1.5M13.5 20.5H15"/>"#,
        Icon::Timer => r#"<circle cx="12" cy="13.5" r="7.5"/><path d="M12 9.5v4l2.5 1.5M9.5 2.5h5"/>"#,
        Icon::Ban => r#"<circle cx="12" cy="12" r="8.5"/><path d="m6 6 12 12"/>"#,
        Icon::Server => r#"<rect x="3.5" y="4" width="17" height="7" rx="2"/><rect x="3.5" y="13" width="17" height="7" rx="2"/><path d="M7.5 7.5h.01M7.5 16.5h.01"/>"#,
        Icon::Share => r#"<circle cx="17.5" cy="5.5" r="2.5"/><circle cx="6.5" cy="12" r="2.5"/><circle cx="17.5" cy="18.5" r="2.5"/><path d="m8.7 10.8 6.6-4M8.7 13.2l6.6 4"/>"#,
        Icon::Download => r#"<path d="M12 4v11M7 10.5 12 15.5 17 10.5M4.5 19.5h15"/>"#,
        Icon::Upload => r#"<path d="M12 15.5v-11M7 9 12 4 17 9M4.5 19.5h15"/>"#,
        Icon::Logout => r#"<path d="M14.5 4.5h3a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2h-3M9.5 16.5 5 12l4.5-4.5M5 12h10"/>"#,
        Icon::Sparkle => r#"<path d="M12 3.5 13.8 10.2 20.5 12 13.8 13.8 12 20.5 10.2 13.8 3.5 12 10.2 10.2Z"/>"#,
        Icon::Eye => r#"<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z"/><circle cx="12" cy="12" r="3"/>"#,
        Icon::EyeOff => r#"<path d="M9.9 5.7A9.6 9.6 0 0 1 12 5.5C18 5.5 21.5 12 21.5 12a17 17 0 0 1-3 3.7M6.3 6.8A16.8 16.8 0 0 0 2.5 12S6 18.5 12 18.5a9.4 9.4 0 0 0 4.4-1.1M3.5 3.5l17 17M10 10a3 3 0 0 0 4.1 4.1"/>"#,
        Icon::Screen => r#"<rect x="3" y="4.5" width="18" height="12" rx="2"/><path d="M8.5 20h7M12 16.5V20"/>"#,
        Icon::Sliders => r#"<path d="M4.5 6.5h9M17.5 6.5h2M4.5 12h3M11.5 12h8M4.5 17.5h11M19 17.5h.5"/><circle cx="15.5" cy="6.5" r="2"/><circle cx="9.5" cy="12" r="2"/><circle cx="17" cy="17.5" r="2"/>"#,
        Icon::Drop => r#"<path d="M12 3.5s6.5 7 6.5 11a6.5 6.5 0 0 1-13 0c0-4 6.5-11 6.5-11Z"/>"#,
        Icon::Db => r#"<ellipse cx="12" cy="6" rx="7.5" ry="2.8"/><path d="M4.5 6v12c0 1.5 3.4 2.8 7.5 2.8s7.5-1.3 7.5-2.8V6M4.5 12c0 1.5 3.4 2.8 7.5 2.8s7.5-1.3 7.5-2.8"/>"#,
        Icon::Info => r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.8h.01"/>"#,
        Icon::More => r#"<circle cx="5.5" cy="12" r="1.3" fill="currentColor"/><circle cx="12" cy="12" r="1.3" fill="currentColor"/><circle cx="18.5" cy="12" r="1.3" fill="currentColor"/>"#,
        Icon::Alert => r#"<path d="M12 4 21 19.5H3Z"/><path d="M12 10v4.5M12 17h.01"/>"#,
        Icon::Refresh => r#"<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3M19.5 4.5v4h-4"/>"#,
        Icon::Maximize => r#"<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>"#,
        Icon::Minimize => r#"<path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5"/>"#,
        Icon::Game => r#"<path d="M7.5 7.5h9a4.5 4.5 0 0 1 4.4 5.4l-.9 4.3a2.4 2.4 0 0 1-4.1 1.1L14 16.5h-4l-1.9 1.8A2.4 2.4 0 0 1 4 17.2l-.9-4.3a4.5 4.5 0 0 1 4.4-5.4Z"/><path d="M8 10.5v3M6.5 12h3M15.5 11.2h.01M17.3 13h.01"/>"#,
        Icon::Music => r#"<path d="M9 18V5.5l11-2v12.5"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="17.5" cy="16" r="2.5"/>"#,
    }
}

fn handle(i: Icon) -> svg::Handle {
    static CACHE: OnceLock<Mutex<HashMap<Icon, svg::Handle>>> = OnceLock::new();
    let mut c = CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    c.entry(i)
        .or_insert_with(|| {
            let s = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#ffffff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">{}</svg>"##,
                path(i).replace("currentColor", "#ffffff")
            );
            svg::Handle::from_memory(s.into_bytes())
        })
        .clone()
}

/// Um ícone na cor pedida.
pub fn icon<'a, M: 'a>(i: Icon, size: f32, color: Color) -> Element<'a, M> {
    svg(handle(i))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_, _| svg::Style { color: Some(color) })
        .into()
}

/// O logo: balão na cor de destaque com o J escuro, como o ícone do app.
pub fn logo<'a, M: 'a>(size: f32) -> Element<'a, M> {
    static CACHE: OnceLock<Mutex<HashMap<String, svg::Handle>>> = OnceLock::new();
    let cor = crate::theme::hex(crate::theme::accent());
    let h = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
        .entry(cor.clone())
        .or_insert_with(|| {
            let s = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><path d="M14 9h36a9 9 0 0 1 9 9v22a9 9 0 0 1-9 9H27l-11 9v-9h-2a9 9 0 0 1-9-9V18a9 9 0 0 1 9-9Z" fill="{cor}"/><path d="M29 19h11M36.5 19v14.5a6.5 6.5 0 0 1-13 0" fill="none" stroke="#0b0c0e" stroke-width="5.6" stroke-linecap="round" stroke-linejoin="round"/></svg>"##
            );
            svg::Handle::from_memory(s.into_bytes())
        })
        .clone();
    svg(h).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_os_icones_sao_svg_valido() {
        // Um caminho com aspas desequilibradas quebraria o desenho em silêncio.
        for i in [Icon::Chat, Icon::Settings, Icon::Play, Icon::Qr, Icon::Music, Icon::EyeOff, Icon::Smile] {
            let p = path(i);
            assert_eq!(p.matches('"').count() % 2, 0, "{i:?}");
            assert!(p.contains("/>"), "{i:?}");
        }
    }
}
