//! Grupos no estilo servidor do Discord: canais de texto, salas de voz e a
//! lista de membros.

use crate::app::Msg;
use crate::icons::{icon, Icon};
use crate::state::*;
use crate::theme as t;
use crate::ui::{self, El, BOLD, SEMI};
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{Alignment, Length};
use jm_core::groups::Group;

pub fn sidebar<'a>(m: &'a Main, g: &'a Group) -> El<'a> {
    let lid = g.lid.clone();
    let admin = m.is_admin(&lid);
    let head = button(
        container(
            row![
                text(ui::ellipsis(&g.meta.name, 22)).size(16).font(BOLD).color(t::FG).width(Length::Fill),
                if g.muted { icon(Icon::BellOff, 15.0, t::FG_FAINT) } else { Space::new().into() },
                icon(Icon::ChevDown, 16.0, t::FG_DIM)
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .center_y(Length::Fill),
    )
    .padding([0, 14])
    .height(56)
    .width(Length::Fill)
    .style(|th, st| {
        let mut s = t::ghost(th, st);
        s.border.radius = 0.0.into();
        s
    })
    .on_press(Msg::GroupMenu(lid.clone()));

    let n = g.meta.members.len();
    let mut card = column![row![
        ui::group_avatar(&g.meta.name, g.meta.t, m.pic(g.pic.as_deref()), 42.0),
        column![
            text(format!("{n} {}", if n == 1 { "membro" } else { "membros" })).size(12.5).color(t::FG_DIM),
            row![
                ui::ghost(Some(Icon::UserPlus), "Convidar", Some(Msg::OpenInvite(lid.clone()))),
                ui::icon_button_sm(Icon::Info, "Dados do grupo", Some(Msg::OpenGroupInfo(lid.clone())))
            ]
            .spacing(2)
            .align_y(Alignment::Center),
        ]
        .spacing(2)
    ]
    .spacing(10)
    .align_y(Alignment::Center)]
    .spacing(6);
    if !g.meta.desc.is_empty() {
        card = card.push(text(g.meta.desc.clone()).size(12.5).color(t::FG_DIM));
    }

    let mut lista = column![container(card).padding([10, 12])].spacing(1);
    lista = lista.push(category("Canais de texto", admin.then(|| Msg::OpenGroupInfo(lid.clone()))));
    for ch in g.meta.text_channels() {
        let key = format!("g:{}:{}", lid, ch.id);
        let sel = m.chat.as_ref().is_some_and(|c| c.key == key);
        let s = m.summary(&key);
        let nao_lidas = s.map(|s| s.unread).unwrap_or(0);
        let mut r = row![icon(Icon::Hash, 17.0, if sel || nao_lidas > 0 { t::FG } else { t::FG_FAINT }), text(ui::ellipsis(&ch.name, 22)).size(14.5).font(if nao_lidas > 0 { BOLD } else { SEMI }).color(if sel || nao_lidas > 0 {
            t::FG
        } else {
            t::FG_DIM
        }).width(Length::Fill)]
        .spacing(8)
        .align_y(Alignment::Center);
        if s.is_some_and(|s| s.ttl > 0) {
            r = r.push(icon(Icon::Timer, 13.0, t::FG_FAINT));
        }
        if nao_lidas > 0 && !g.muted {
            r = r.push(ui::badge(nao_lidas));
        }
        let b = button(r).padding([7, 10]).width(Length::Fill).style(t::row_item(sel)).on_press(Msg::OpenChat(key.clone()));
        let item: El<'a> = if admin { mouse_area(b).on_right_press(Msg::ChannelEdit(ch.id.clone())).into() } else { b.into() };
        lista = lista.push(container(item).padding([0, 8]));
    }

    let voz: Vec<_> = g.meta.voice_channels().collect();
    if !voz.is_empty() || admin {
        lista = lista.push(category("Salas de voz", admin.then(|| Msg::OpenGroupInfo(lid.clone()))));
    }
    let me = m.me_pk();
    for ch in voz {
        let estou = m.room.as_ref().is_some_and(|r| r.lid == lid && r.ch == ch.id);
        let mut presentes: Vec<String> = m.rooms.get(&(lid.clone(), ch.id.clone())).cloned().unwrap_or_default();
        presentes.retain(|p| p != &me);
        if estou {
            presentes.insert(0, me.clone());
        }
        let cheia = presentes.len() >= crate::calls::MAX_ROOM && !estou;
        let mut r = row![
            icon(Icon::Volume, 17.0, if estou { t::accent() } else { t::FG_FAINT }),
            text(ui::ellipsis(&ch.name, 22)).size(14.5).font(SEMI).color(if estou { t::FG } else { t::FG_DIM }).width(Length::Fill)
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if !presentes.is_empty() {
            r = r.push(text(format!("{}/{}", presentes.len(), crate::calls::MAX_ROOM)).size(11.5).color(t::FG_FAINT));
        }
        let msg = if estou || cheia { None } else { Some(Msg::JoinRoom(lid.clone(), ch.id.clone())) };
        let b = button(r).padding([7, 10]).width(Length::Fill).style(t::row_item(estou)).on_press_maybe(msg);
        let item: El<'a> = if admin { mouse_area(b).on_right_press(Msg::ChannelEdit(ch.id.clone())).into() } else { b.into() };
        let mut bloco = column![item].spacing(2);
        for pk in presentes {
            let falando = if pk == me {
                m.room.as_ref().is_some_and(|r| r.me_speaking && !r.muted)
            } else {
                m.room.as_ref().is_some_and(|r| r.peers.iter().any(|p| p.pk == pk && p.speaking))
            };
            let mudo = if pk == me {
                m.room.as_ref().is_some_and(|r| r.muted)
            } else {
                m.room.as_ref().is_some_and(|r| r.peers.iter().any(|p| p.pk == pk && !p.mic))
            };
            let nome = m.name_of(&pk);
            let mut linha = row![
                ui::ringed(ui::avatar(&nome, m.tone_of(&pk), m.pic_of(&pk), 22.0), 22.0, falando.then(t::accent)),
                text(ui::ellipsis(&nome, 20)).size(13).color(if falando { t::FG } else { t::FG_DIM }).width(Length::Fill)
            ]
            .spacing(6)
            .align_y(Alignment::Center);
            if mudo {
                linha = linha.push(icon(Icon::MicOff, 13.0, t::DANGER));
            }
            bloco = bloco.push(container(linha).padding(iced::Padding { left: 28.0, right: 10.0, top: 0.0, bottom: 0.0 }));
        }
        if cheia {
            bloco = bloco.push(container(text("Sala cheia").size(11.5).color(t::FG_FAINT)).padding(iced::Padding { left: 36.0, ..iced::Padding::ZERO }));
        }
        lista = lista.push(container(bloco).padding([0, 8]));
    }
    lista = lista.push(ui::vspace(12.0));

    column![container(head).style(t::header), scrollable(lista).height(Length::Fill).style(t::scroll)].into()
}

fn category<'a>(label: &str, add: Option<Msg>) -> El<'a> {
    let mut r = row![text(label.to_uppercase()).size(11.5).font(BOLD).color(t::FG_FAINT).width(Length::Fill)].align_y(Alignment::Center);
    if let Some(m) = add {
        r = r.push(ui::icon_button_sm(Icon::Plus, "Criar canal", Some(m)));
    }
    container(r).padding(iced::Padding { top: 14.0, bottom: 2.0, left: 18.0, right: 8.0 }).into()
}

/// Lista de membros à direita.
pub fn members<'a>(m: &'a Main, g: &'a Group) -> El<'a> {
    let me = m.me_pk();
    let mut admins: Vec<&String> = g.meta.members.iter().filter(|p| g.meta.is_admin(p)).collect();
    let mut outros: Vec<&String> = g.meta.members.iter().filter(|p| !g.meta.is_admin(p)).collect();
    let ordem = |a: &&String, b: &&String| m.name_of(a).to_lowercase().cmp(&m.name_of(b).to_lowercase());
    admins.sort_by(ordem);
    outros.sort_by(ordem);
    let mut col = column![].spacing(2).padding([12, 8]);
    for (titulo, lista) in [("Admins", admins), ("Membros", outros)] {
        if lista.is_empty() {
            continue;
        }
        col = col.push(container(text(format!("{} — {}", titulo.to_uppercase(), lista.len())).size(11.5).font(BOLD).color(t::FG_FAINT)).padding([8, 8]));
        for pk in lista {
            let nome = m.name_of(pk);
            let falando = m.room.as_ref().filter(|r| r.lid == g.lid).is_some_and(|r| {
                if *pk == me {
                    r.me_speaking && !r.muted
                } else {
                    r.peers.iter().any(|p| &p.pk == pk && p.speaking)
                }
            });
            let mut linha = row![
                ui::ringed(ui::avatar(&nome, m.tone_of(pk), m.pic_of(pk), 30.0), 30.0, falando.then(t::accent)),
                column![
                    text(ui::ellipsis(&nome, 18)).size(13.5).font(SEMI).color(t::FG),
                    match m.listening(pk) {
                        Some(l) => El::from(
                            row![icon(Icon::Music, 11.0, crate::v_modal::SPOTIFY), text(ui::ellipsis(&format!("Ouvindo {}", l.s), 24)).size(11.5).color(t::FG_DIM)]
                                .spacing(4)
                                .align_y(Alignment::Center)
                        ),
                        None => text(if *pk == me {
                            "Você".to_string()
                        } else if g.meta.owner == **pk {
                            "Criou o grupo".to_string()
                        } else {
                            String::new()
                        })
                        .size(11.5)
                        .color(t::FG_FAINT)
                        .into(),
                    }
                ]
                .spacing(0)
                .width(Length::Fill)
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            if g.meta.is_admin(pk) {
                linha = linha.push(ui::tip(icon(Icon::Shield, 14.0, t::accent()), "Admin", iced::widget::tooltip::Position::Left));
            }
            col = col.push(
                button(linha)
                    .padding([5, 8])
                    .width(Length::Fill)
                    .style(t::row_item(false))
                    .on_press(Msg::MemberMenu(g.lid.clone(), (*pk).clone())),
            );
        }
    }
    container(scrollable(col).height(Length::Fill).style(t::scroll)).width(236).height(Length::Fill).style(t::fill(t::sidebar())).into()
}
