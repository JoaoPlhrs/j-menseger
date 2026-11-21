//! Chamadas: tela da chamada, chamada chegando, sala de voz e o painel
//! "voz conectada" da barra lateral.

use crate::app::Msg;
use crate::calls::{duration_label, CallInfo, CallState, RoomInfo, SoundBanner};
use crate::icons::{icon, logo, Icon};
use crate::state::*;
use crate::theme::{self as t, CallBtn};
use crate::ui::{self, El, BOLD, SEMI};
use iced::widget::{button, column, container, image, opaque, row, stack, text, tooltip, Space};
use iced::{Alignment, Color, ContentFit, Length, Padding};

fn round_btn<'a>(i: Icon, kind: CallBtn, label: &str, msg: Option<Msg>) -> El<'a> {
    let cor = match kind {
        CallBtn::On => t::rgb(0x11, 0x13, 0x16),
        CallBtn::Accept => t::accent_ink(),
        _ => t::FG,
    };
    ui::tip(button(container(icon(i, 22.0, cor)).center(22)).padding(14).style(t::call_btn(kind)).on_press_maybe(msg), label.to_string(), tooltip::Position::Top)
}

fn banner<'a>(b: &'a SoundBanner) -> El<'a> {
    let mut r = row![text(b.emoji.clone()).size(22), text(format!("{} tocou {}", b.who, b.name)).size(13.5).font(SEMI).color(t::FG)]
        .spacing(10)
        .align_y(Alignment::Center);
    if b.save.is_some() {
        r = r.push(ui::ghost(Some(Icon::Plus), "Salvar no meu soundboard", Some(Msg::SaveBannerSound)));
    }
    container(r).padding([6, 14]).style(t::toast).into()
}

/// Chamada de duas pessoas, em cima da conversa.
pub fn call_panel<'a>(m: &'a Main, c: &'a CallInfo) -> El<'a> {
    let nome = m.name_of(&c.peer);
    let estado: String = match &c.state {
        CallState::Calling => "Chamando…".into(),
        CallState::Ringing { .. } => "Tocando…".into(),
        CallState::Connecting => "Conectando…".into(),
        CallState::Active { since } => duration_label(*since),
        CallState::Reconnecting => "Reconectando…".into(),
        CallState::Ended { reason } => reason.label().into(),
    };
    let acabou = matches!(c.state, CallState::Ended { .. });
    let ativo = matches!(c.state, CallState::Active { .. } | CallState::Reconnecting);

    let palco: El<'a> = match (&m.remote_frame, c.receiving_video) {
        (Some((_, h)), true) => container(image(h.clone()).content_fit(ContentFit::Contain).width(Length::Fill).height(Length::Fill))
            .height(Length::Fixed((m.size.height * 0.48).clamp(220.0, 560.0)))
            .width(Length::Fill)
            .style(t::rounded(iced::Color::BLACK, 12.0))
            .into(),
        _ => {
            let anel = (c.remote_speaking && !c.remote_muted).then(t::accent);
            let mut nome_linha = row![text(nome.clone()).size(18).font(BOLD).color(t::FG)].spacing(6).align_y(Alignment::Center);
            if c.remote_muted && ativo {
                nome_linha = nome_linha.push(ui::tip(icon(Icon::MicOff, 15.0, t::DANGER), "Microfone desligado", tooltip::Position::Right));
            }
            column![
                ui::ringed(ui::avatar(&nome, m.tone_of(&c.peer), m.pic_of(&c.peer), 84.0), 84.0, anel),
                nome_linha,
                text(estado.clone()).size(13.5).color(if acabou { t::FG_DIM } else { t::accent() }),
            ]
            .spacing(8)
            .align_x(Alignment::Center)
            .into()
        }
    };

    let mut avisos = column![].spacing(6).align_x(Alignment::Center);
    if c.receiving_video {
        avisos = avisos.push(row![text(nome.clone()).size(14).font(SEMI), text(estado).size(13).color(t::accent())].spacing(10));
    }
    if let Some(s) = &c.sharing {
        avisos = avisos.push(
            container(row![icon(Icon::Screen, 15.0, t::accent()), text(format!("Transmitindo: {}", ui::ellipsis(s, 40))).size(13).color(t::FG)].spacing(8).align_y(Alignment::Center))
                .padding([5, 12])
                .style(t::chip(t::accent())),
        );
    }
    if c.sem_microfone && !acabou {
        avisos = avisos.push(
            container(row![icon(Icon::MicOff, 14.0, t::WARN), text("Sem microfone: você só ouve").size(13).color(t::WARN)].spacing(8).align_y(Alignment::Center))
                .padding([5, 12])
                .style(t::chip(t::WARN)),
        );
    }
    if let Some((_, h)) = &m.local_preview {
        // Sua câmera, pequena, como no canto das chamadas de vídeo.
        avisos = avisos.push(
            container(image(h.clone()).content_fit(ContentFit::Cover).width(176).height(132).border_radius(10.0))
                .style(t::rounded(iced::Color::BLACK, 10.0)),
        );
    }
    if let Some(b) = &c.banner {
        avisos = avisos.push(banner(b));
    }

    let controles: El<'a> = if acabou {
        Space::new().height(0).into()
    } else {
        let mut r = row![].spacing(14).align_y(Alignment::Center);
        r = r.push(round_btn(
            if c.muted { Icon::MicOff } else { Icon::Mic },
            if c.muted { CallBtn::On } else { CallBtn::Normal },
            if c.muted { "Ligar microfone" } else { "Desligar microfone" },
            Some(Msg::ToggleMute),
        ));
        r = r.push(round_btn(
            Icon::Wave,
            if m.popup == Some(Popup::CallSounds) { CallBtn::On } else { CallBtn::Normal },
            "Soundboard",
            ativo.then(|| Msg::SetPopup(Some(Popup::CallSounds))),
        ));
        let camera_on = c.sharing.as_deref().is_some_and(|s| s.starts_with("Câmera"));
        r = r.push(round_btn(
            Icon::Video,
            if camera_on { CallBtn::On } else { CallBtn::Normal },
            if camera_on { "Desligar a câmera" } else { "Ligar a câmera" },
            ativo.then_some(Msg::ToggleCamera),
        ));
        r = r.push(if c.sharing.is_some() && !camera_on {
            round_btn(Icon::Screen, CallBtn::On, "Parar de transmitir", Some(Msg::StopSharing))
        } else {
            round_btn(Icon::Screen, CallBtn::Normal, "Transmitir a tela", ativo.then_some(Msg::OpenShare))
        });
        r = r.push(round_btn(Icon::Volume, CallBtn::Normal, "Volume desta pessoa", Some(Msg::OpenPeerVolume(c.peer.clone()))));
        r = r.push(round_btn(Icon::Maximize, CallBtn::Normal, "Tela cheia (F11)", ativo.then_some(Msg::CallFull(true))));
        r = r.push(round_btn(Icon::PhoneOff, CallBtn::Danger, "Desligar", Some(Msg::HangUp)));
        if !c.muted && !c.sem_microfone {
            r = r.push(mic_meter(c.mic_level));
        }
        r.into()
    };

    let col = column![palco, avisos, controles].spacing(14).align_x(Alignment::Center).width(Length::Fill);
    container(col).padding([18, 20]).width(Length::Fill).style(t::fill(t::mix(t::main_bg(), iced::Color::BLACK, 0.35))).into()
}

/// Chamada chegando.
pub fn incoming<'a>(m: &'a Main, c: &'a CallInfo, video: bool) -> El<'a> {
    let nome = m.name_of(&c.peer);
    container(
        column![
            stack![
                iced::widget::canvas(crate::canvas::Pulse { t: m.motion.clock(), color: t::accent(), inner: 52.0 }).width(170).height(170),
                container(ui::ringed(ui::avatar(&nome, m.tone_of(&c.peer), m.pic_of(&c.peer), 96.0), 96.0, Some(t::accent()))).center(170),
            ],
            text(nome).size(22).font(BOLD).color(t::FG),
            text(if video { "Chamada de vídeo chegando" } else { "Chamada de voz chegando" }).size(14).color(t::accent()),
            ui::vspace(6.0),
            row![
                column![round_btn(Icon::PhoneOff, CallBtn::Danger, "Recusar", Some(Msg::DeclineCall)), text("Recusar").size(12.5).color(t::FG_DIM)]
                    .spacing(6)
                    .align_x(Alignment::Center),
                ui::hspace(36.0),
                column![round_btn(Icon::Phone, CallBtn::Accept, "Atender", Some(Msg::AcceptCall)), text("Atender").size(12.5).color(t::FG_DIM)]
                    .spacing(6)
                    .align_x(Alignment::Center),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(10)
        .width(Length::Fill)
        .align_x(Alignment::Center),
    )
    .padding(32)
    .width(340)
    .style(t::panel)
    .into()
}

/// Sala de voz do grupo aberto.
pub fn room_panel<'a>(m: &'a Main, r: &'a RoomInfo) -> El<'a> {
    let me = m.me_pk();
    let canal = m
        .group(&r.lid)
        .and_then(|g| g.meta.channels.iter().find(|c| c.id == r.ch))
        .map(|c| c.name.clone())
        .unwrap_or(r.ch.clone());
    let mut cartoes: Vec<El<'a>> = Vec::new();
    cartoes.push(tile(m, &me, r.me_speaking && !r.muted, r.muted, true, true));
    for p in &r.peers {
        cartoes.push(tile(m, &p.pk, p.speaking, !p.mic, p.connected, false));
    }
    let grade = row(cartoes).spacing(12).wrap();
    // Quem está mandando imagem (câmera ou tela): miniaturas; clicar abre a tela cheia.
    let camera = r.sharing.as_deref().is_some_and(|s| s.starts_with("Câmera"));
    let mut videos: Vec<El<'a>> = Vec::new();
    if let (true, Some((_, h))) = (camera, &m.local_preview) {
        videos.push(mini_video(h.clone(), format!("{} (você)", m.first_name(&me))));
    }
    for p in r.peers.iter().filter(|p| p.video) {
        if let Some((_, h)) = m.room_frames.get(&p.pk) {
            videos.push(mini_video(h.clone(), m.name_of(&p.pk)));
        }
    }
    let mut col = column![
        row![
            icon(Icon::Volume, 18.0, t::accent()),
            text(canal.clone()).size(15).font(BOLD).color(t::FG),
            text(duration_label(r.since)).size(13).color(t::FG_DIM),
            ui::fill(),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(12);
    if !videos.is_empty() {
        col = col.push(row(videos).spacing(10).wrap());
    }
    col = col.push(grade);
    if r.sem_microfone {
        col = col.push(ui::note("Sem microfone: você só ouve."));
    }
    if let Some(b) = &r.banner {
        col = col.push(container(banner(b)).center_x(Length::Fill));
    }
    col = col.push(
        container(
            row![
                round_btn(
                    if r.muted { Icon::MicOff } else { Icon::Mic },
                    if r.muted { CallBtn::On } else { CallBtn::Normal },
                    if r.muted { "Ligar microfone" } else { "Desligar microfone" },
                    Some(Msg::ToggleMute)
                ),
                round_btn(
                    Icon::Wave,
                    if m.popup == Some(Popup::CallSounds) { CallBtn::On } else { CallBtn::Normal },
                    "Soundboard",
                    Some(Msg::SetPopup(Some(Popup::CallSounds)))
                ),
                round_btn(Icon::Video, if camera { CallBtn::On } else { CallBtn::Normal }, if camera { "Desligar a câmera" } else { "Ligar a câmera" }, Some(Msg::ToggleCamera)),
                if r.sharing.is_some() && !camera {
                    round_btn(Icon::Screen, CallBtn::On, "Parar de transmitir", Some(Msg::StopSharing))
                } else {
                    round_btn(Icon::Screen, CallBtn::Normal, "Transmitir a tela para a sala", Some(Msg::OpenShare))
                },
                round_btn(Icon::Maximize, CallBtn::Normal, "Tela cheia (F11)", Some(Msg::CallFull(true))),
                round_btn(Icon::PhoneOff, CallBtn::Danger, "Sair da sala", Some(Msg::LeaveRoom)),
                if !r.muted && !r.sem_microfone { mic_meter(r.mic_level) } else { Space::new().into() },
            ]
            .spacing(14)
            .align_y(Alignment::Center),
        )
        .center_x(Length::Fill),
    );
    container(col).padding([16, 20]).width(Length::Fill).style(t::fill(t::mix(t::main_bg(), iced::Color::BLACK, 0.35))).into()
}

/// Miniatura de quem está mandando imagem na sala.
fn mini_video<'a>(h: iced::widget::image::Handle, nome: String) -> El<'a> {
    let etiqueta = container(container(text(ui::ellipsis(&nome, 22)).size(12).font(SEMI).color(Color::WHITE)).padding([3, 8]).style(t::rounded(Color { a: 0.62, ..Color::BLACK }, 6.0)))
        .align_bottom(Length::Fill)
        .align_left(Length::Fill)
        .padding(6);
    button(
        container(stack![image(h).content_fit(ContentFit::Contain).width(Length::Fill).height(Length::Fill), etiqueta])
            .width(240)
            .height(135)
            .clip(true)
            .style(t::rounded(Color::BLACK, 10.0)),
    )
    .padding(0)
    .style(t::ghost)
    .on_press(Msg::CallFull(true))
    .into()
}

/// Nível do seu microfone (para conferir se ele está pegando a voz).
fn mic_meter<'a>(level: f32) -> El<'a> {
    let n = 8;
    let acesos = ((level.clamp(0.0, 1.0).sqrt()) * n as f32).round() as usize;
    ui::tip(
        row((0..n)
            .map(|i| {
                let cor = if i < acesos { t::accent() } else { t::surface_hi() };
                container(Space::new()).width(4).height(6.0 + i as f32 * 2.2).style(t::rounded(cor, 2.0)).into()
            })
            .collect::<Vec<El<'a>>>())
        .spacing(2)
        .align_y(Alignment::End),
        "Seu microfone",
        tooltip::Position::Top,
    )
}

fn tile<'a>(m: &'a Main, pk: &str, falando: bool, mudo: bool, conectado: bool, eu: bool) -> El<'a> {
    let nome = if eu { format!("{} (você)", m.first_name(pk)) } else { m.name_of(pk) };
    let mut base = column![
        ui::ringed(ui::avatar(&m.name_of(pk), m.tone_of(pk), m.pic_of(pk), 56.0), 56.0, falando.then(t::accent)),
        text(ui::ellipsis(&nome, 16)).size(13).font(SEMI).color(t::FG),
    ]
    .spacing(6)
    .align_x(Alignment::Center);
    if !conectado {
        base = base.push(text("conectando…").size(11).color(t::FG_FAINT));
    }
    let mut camadas: Vec<El<'a>> = vec![container(base).padding([12, 10]).width(132).center_x(132).into()];
    if mudo {
        camadas.push(
            container(container(icon(Icon::MicOff, 13.0, t::FG)).center(24).style(t::rounded(t::DANGER, 999.0)))
                .align_right(132)
                .padding(Padding { top: 8.0, right: 8.0, ..Padding::ZERO })
                .into(),
        );
    }
    let caixa = container(stack(camadas)).style(move |_| iced::widget::container::Style {
        background: Some(iced::Background::Color(t::surface())),
        border: iced::Border { color: if falando { t::accent() } else { t::line() }, width: if falando { 2.0 } else { 1.0 }, radius: 14.0.into() },
        ..Default::default()
    });
    if eu {
        return caixa.into();
    }
    // Clicar em alguém: volume só dessa pessoa.
    ui::tip(button(caixa).padding(0).style(t::ghost).on_press(Msg::OpenPeerVolume(pk.to_string())), "Volume desta pessoa", tooltip::Position::Bottom)
}

/// "Voz conectada": fica na barra lateral enquanto há chamada ou sala.
pub fn voice_panel(m: &Main) -> Option<El<'_>> {
    let (titulo, sub, acao, cor): (String, String, Msg, iced::Color) = if let Some(r) = &m.room {
        let g = m.group(&r.lid);
        let canal = g.and_then(|g| g.meta.channels.iter().find(|c| c.id == r.ch)).map(|c| c.name.clone()).unwrap_or_default();
        ("Voz conectada".into(), format!("{canal} / {}", g.map(|g| g.meta.name.clone()).unwrap_or_default()), Msg::LeaveRoom, t::accent())
    } else if m.joining_room {
        ("Entrando na sala…".into(), String::new(), Msg::LeaveRoom, t::WARN)
    } else if let Some(c) = &m.call {
        match &c.state {
            CallState::Ended { .. } | CallState::Ringing { .. } => return None,
            CallState::Active { since } => (format!("Em chamada · {}", duration_label(*since)), m.name_of(&c.peer), Msg::HangUp, t::accent()),
            _ => ("Chamando…".into(), m.name_of(&c.peer), Msg::HangUp, t::WARN),
        }
    } else {
        return None;
    };
    let ir: Option<Msg> = m.room.as_ref().map(|r| Msg::Go(Section::Group(r.lid.clone())));
    let info = button(
        column![
            row![icon(Icon::Volume, 14.0, cor), text(titulo).size(13).font(BOLD).color(cor)].spacing(6).align_y(Alignment::Center),
            text(ui::ellipsis(&sub, 30)).size(12).color(t::FG_DIM)
        ]
        .spacing(2),
    )
    .padding([4, 6])
    .width(Length::Fill)
    .style(t::row_item(false))
    .on_press_maybe(ir);
    Some(
        container(row![info, ui::icon_button(Icon::PhoneOff, "Desconectar", Some(acao), false)].spacing(4).align_y(Alignment::Center))
            .padding([6, 8])
            .style(t::fill(t::mix(t::sidebar(), iced::Color::BLACK, 0.15)))
            .into(),
    )
}

/* ------------------------------------------------------------ tela cheia */

/// Uma pessoa no palco da tela cheia.
struct Quadro {
    pk: String,
    nome: String,
    falando: bool,
    mudo: bool,
    conectado: bool,
    /// Imagem que está chegando dessa pessoa (câmera ou tela).
    video: Option<image::Handle>,
    /// Você está transmitindo a tela.
    ao_vivo: bool,
}

const TOPO: f32 = 60.0;
const BASE: f32 = 104.0;
const GAP: f32 = 12.0;

fn palco_bg() -> Color {
    t::mix(t::main_bg(), Color::BLACK, 0.62)
}

/// Chamada ou sala em tela cheia, no jeito do Discord: todo mundo em
/// quadros grandes, quem fala ganha a borda colorida, controles embaixo.
pub fn stage(m: &Main) -> El<'_> {
    let me = m.me_pk();
    let mut quadros: Vec<Quadro> = Vec::new();
    let (titulo, sub, since, muted, sem_mic, nivel, banner_som, sair): (String, String, Option<std::time::Instant>, bool, bool, f32, Option<&SoundBanner>, Msg);
    if let Some(r) = &m.room {
        let g = m.group(&r.lid);
        titulo = g.and_then(|g| g.meta.channels.iter().find(|c| c.id == r.ch)).map(|c| c.name.clone()).unwrap_or_else(|| r.ch.clone());
        sub = g.map(|g| g.meta.name.clone()).unwrap_or_default();
        since = Some(r.since);
        (muted, sem_mic, nivel, banner_som, sair) = (r.muted, r.sem_microfone, r.mic_level, r.banner.as_ref(), Msg::LeaveRoom);
        let camera = r.sharing.as_deref().is_some_and(|s| s.starts_with("Câmera"));
        let minha = if camera { m.local_preview.as_ref().map(|(_, h)| h.clone()) } else { None };
        // Quem manda imagem vem primeiro (vai para o palco).
        for p in r.peers.iter().filter(|p| p.video) {
            let video = m.room_frames.get(&p.pk).map(|(_, h)| h.clone());
            quadros.push(Quadro { pk: p.pk.clone(), nome: m.name_of(&p.pk), falando: p.speaking, mudo: !p.mic, conectado: p.connected, video, ao_vivo: false });
        }
        quadros.push(Quadro {
            pk: me.clone(),
            nome: format!("{} (você)", m.first_name(&me)),
            falando: r.me_speaking && !r.muted,
            mudo: r.muted,
            conectado: true,
            video: minha,
            ao_vivo: r.sharing.is_some() && !camera,
        });
        for p in r.peers.iter().filter(|p| !p.video) {
            quadros.push(Quadro { pk: p.pk.clone(), nome: m.name_of(&p.pk), falando: p.speaking, mudo: !p.mic, conectado: p.connected, video: None, ao_vivo: false });
        }
    } else if let Some(c) = &m.call {
        titulo = m.name_of(&c.peer);
        sub = if c.video { "Chamada de vídeo".into() } else { "Chamada de voz".into() };
        since = match c.state {
            CallState::Active { since } => Some(since),
            _ => None,
        };
        (muted, sem_mic, nivel, banner_som, sair) = (c.muted, c.sem_microfone, c.mic_level, c.banner.as_ref(), Msg::HangUp);
        let video = if c.receiving_video { m.remote_frame.as_ref().map(|(_, h)| h.clone()) } else { None };
        quadros.push(Quadro {
            pk: c.peer.clone(),
            nome: m.name_of(&c.peer),
            falando: c.remote_speaking && !c.remote_muted,
            mudo: c.remote_muted,
            conectado: matches!(c.state, CallState::Active { .. }),
            video,
            ao_vivo: false,
        });
        quadros.push(Quadro {
            pk: me.clone(),
            nome: format!("{} (você)", m.first_name(&me)),
            falando: !c.muted && c.mic_level > 0.06,
            mudo: c.muted || c.sem_microfone,
            conectado: true,
            video: None,
            ao_vivo: c.sharing.is_some(),
        });
    } else {
        return Space::new().into();
    }

    /* ---- topo: marca, onde você está, tempo e sair da tela cheia */
    let mut onde = row![logo(26.0), text(titulo).size(16).font(BOLD).color(t::FG)].spacing(10).align_y(Alignment::Center);
    if !sub.is_empty() {
        onde = onde.push(text("·").size(16).color(t::FG_FAINT)).push(text(sub).size(14).color(t::FG_DIM));
    }
    let tempo: El<'_> = match since {
        Some(s) => container(row![container(Space::new()).width(8).height(8).style(t::rounded(t::accent(), 99.0)), text(duration_label(s)).size(13).font(SEMI).color(t::FG)].spacing(7).align_y(Alignment::Center))
            .padding([5, 11])
            .style(t::rounded(Color { a: 0.55, ..Color::BLACK }, 999.0))
            .into(),
        None => text("Reconectando…").size(13).color(t::WARN).into(),
    };
    let topo = row![
        onde,
        ui::fill(),
        tempo,
        ui::hspace(10.0),
        text("Esc para sair").size(12).color(t::FG_FAINT),
        ui::icon_button(Icon::Minimize, "Sair da tela cheia (Esc)", Some(Msg::CallFull(false)), false),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .height(TOPO)
    .padding([0, 22]);

    /* ---- palco */
    let largura = (m.size.width - 48.0).max(200.0);
    let altura = (m.size.height - TOPO - BASE).max(160.0);
    let foco = quadros.iter().position(|q| q.video.is_some());
    let palco: El<'_> = match foco {
        Some(i) => {
            // Alguém mandando imagem: ela ocupa o palco, o resto vai para a faixa de baixo.
            let outros: Vec<&Quadro> = quadros.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, q)| q).collect();
            let faixa_h = if outros.is_empty() { 0.0 } else { (altura * 0.2).clamp(96.0, 150.0) };
            let principal = quadro(m, &quadros[i], largura, altura - faixa_h - if outros.is_empty() { 0.0 } else { GAP });
            let mut col = column![principal].spacing(GAP).align_x(Alignment::Center);
            if !outros.is_empty() {
                let w = faixa_h * 16.0 / 9.0;
                col = col.push(row(outros.into_iter().map(|q| quadro(m, q, w, faixa_h))).spacing(GAP));
            }
            col.into()
        }
        None => {
            let (cols, w, h) = grade(quadros.len(), largura, altura);
            let mut linhas = column![].spacing(GAP).align_x(Alignment::Center);
            for fila in quadros.chunks(cols) {
                linhas = linhas.push(row(fila.iter().map(|q| quadro(m, q, w, h))).spacing(GAP));
            }
            linhas.into()
        }
    };

    /* ---- controles */
    let em_chamada = m.call.is_some() && m.room.is_none();
    let sons_aberto = m.popup == Some(Popup::CallSounds);
    let mut barra = row![
        round_btn(
            if muted { Icon::MicOff } else { Icon::Mic },
            if muted { CallBtn::On } else { CallBtn::Normal },
            if muted { "Ligar microfone" } else { "Desligar microfone" },
            Some(Msg::ToggleMute)
        ),
        round_btn(Icon::Wave, if sons_aberto { CallBtn::On } else { CallBtn::Normal }, "Soundboard", Some(Msg::SetPopup(if sons_aberto { None } else { Some(Popup::CallSounds) }))),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    {
        let compartilhando = m.room.as_ref().map(|r| r.sharing.clone()).unwrap_or_else(|| m.call.as_ref().and_then(|c| c.sharing.clone()));
        let camera_on = compartilhando.as_deref().is_some_and(|s| s.starts_with("Câmera"));
        let transmitindo = compartilhando.is_some() && !camera_on;
        barra = barra.push(round_btn(
            Icon::Video,
            if camera_on { CallBtn::On } else { CallBtn::Normal },
            if camera_on { "Desligar a câmera" } else { "Ligar a câmera" },
            Some(Msg::ToggleCamera),
        ));
        barra = barra.push(if transmitindo {
            round_btn(Icon::Screen, CallBtn::On, "Parar de transmitir", Some(Msg::StopSharing))
        } else {
            round_btn(Icon::Screen, CallBtn::Normal, "Transmitir a tela", Some(Msg::OpenShare))
        });
    }
    barra = barra.push(round_btn(Icon::Minimize, CallBtn::Normal, "Sair da tela cheia (Esc)", Some(Msg::CallFull(false))));
    barra = barra.push(ui::hspace(4.0));
    barra = barra.push(
        ui::tip(
            button(container(icon(Icon::PhoneOff, 22.0, Color::WHITE)).center_y(22).center_x(44))
                .padding([14, 18])
                .style(t::call_btn(CallBtn::Danger))
                .on_press(sair),
            if em_chamada { "Desligar" } else { "Sair da sala" },
            tooltip::Position::Top,
        ),
    );
    if !muted && !sem_mic {
        barra = barra.push(ui::hspace(4.0)).push(mic_meter(nivel));
    }
    let base = container(container(barra).padding([10, 16]).style(t::rounded(t::mix(t::surface(), Color::BLACK, 0.25), 18.0)))
        .center_x(Length::Fill)
        .height(BASE)
        .center_y(BASE);

    let mut camadas: Vec<El<'_>> = vec![column![topo, container(palco).center(Length::Fill), base].into()];
    if let Some(b) = banner_som {
        camadas.push(container(banner(b)).center_x(Length::Fill).padding(Padding { top: TOPO + 10.0, ..Padding::ZERO }).into());
    }
    if sons_aberto {
        camadas.push(
            container(ui::slide(opaque(crate::v_chat::sounds_popup(m, true)), 0.0, (1.0 - m.motion.v(&m.motion.popup)) * 14.0))
                .align_bottom(Length::Fill)
                .center_x(Length::Fill)
                .padding(Padding { bottom: BASE - 6.0, ..Padding::ZERO })
                .into(),
        );
    }
    container(stack(camadas).width(Length::Fill).height(Length::Fill)).width(Length::Fill).height(Length::Fill).style(t::fill(palco_bg())).into()
}

/// Quantas colunas e o tamanho de cada quadro para caber todo mundo no
/// palco, mantendo 16:9 e o maior tamanho possível.
fn grade(n: usize, largura: f32, altura: f32) -> (usize, f32, f32) {
    let n = n.max(1);
    let mut melhor = (1, 0.0f32, 0.0f32);
    for cols in 1..=n {
        let linhas = n.div_ceil(cols);
        let w_max = (largura - GAP * (cols as f32 - 1.0)) / cols as f32;
        let h_max = (altura - GAP * (linhas as f32 - 1.0)) / linhas as f32;
        let w = w_max.min(h_max * 16.0 / 9.0).min(960.0);
        if w > melhor.1 {
            melhor = (cols, w, w * 9.0 / 16.0);
        }
    }
    melhor
}

fn quadro<'a>(m: &'a Main, q: &Quadro, w: f32, h: f32) -> El<'a> {
    let tom = m.tone_of(&q.pk);
    let fundo: El<'a> = match &q.video {
        Some(v) => container(image(v.clone()).content_fit(ContentFit::Contain).width(Length::Fill).height(Length::Fill)).style(t::rounded(Color::BLACK, 10.0)).into(),
        None => {
            let tam = (h * 0.36).clamp(40.0, 132.0);
            let mut c = column![ui::ringed(ui::avatar(&q.nome, tom, m.pic_of(&q.pk), tam), tam, q.falando.then(t::accent))].align_x(Alignment::Center).spacing(8);
            if !q.conectado {
                c = c.push(text("conectando…").size(12.5).color(t::FG_DIM));
            }
            if q.ao_vivo {
                c = c.push(text("Você está transmitindo a sua tela").size(12.5).color(t::FG_DIM));
            }
            container(c).center(Length::Fill).style(t::rounded(t::mix(t::tone(tom), Color::BLACK, 0.55), 10.0)).into()
        }
    };
    let pilula = Color { a: 0.62, ..Color::BLACK };
    let mut nome = row![].spacing(6).align_y(Alignment::Center);
    if q.mudo {
        nome = nome.push(icon(Icon::MicOff, 14.0, t::DANGER));
    }
    nome = nome.push(text(ui::ellipsis(&q.nome, 28)).size(13).font(SEMI).color(Color::WHITE));
    let etiqueta = container(container(nome).padding([4, 9]).style(t::rounded(pilula, 6.0)))
        .align_bottom(Length::Fill)
        .align_left(Length::Fill)
        .padding(10);
    let mut camadas: Vec<El<'a>> = vec![fundo, etiqueta.into()];
    if q.ao_vivo {
        camadas.push(
            container(container(text("AO VIVO").size(11).font(BOLD).color(Color::WHITE)).padding([3, 8]).style(t::rounded(t::DANGER, 5.0)))
                .align_top(Length::Fill)
                .align_right(Length::Fill)
                .padding(10)
                .into(),
        );
    }
    let falando = q.falando;
    container(stack(camadas))
        .width(w)
        .height(h)
        .clip(true)
        .style(move |_| iced::widget::container::Style {
            border: iced::Border { color: if falando { t::accent() } else { Color::TRANSPARENT }, width: if falando { 3.0 } else { 0.0 }, radius: 10.0.into() },
            ..Default::default()
        })
        .into()
}
