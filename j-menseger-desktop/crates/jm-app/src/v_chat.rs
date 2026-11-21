//! A conversa: cabeçalho, mensagens (texto, respostas, reações, fotos, voz,
//! sons, convites) e o campo de escrever.

use crate::app::Msg;
use crate::icons::{icon, Icon};
use crate::state::*;
use crate::theme as t;
use crate::ui::{self, El, BOLD, SEMI};
use crate::update::{EDITOR_ID, MESSAGES_ID};
use iced::keyboard::key::Named;
use iced::widget::{button, column, container, image, mouse_area, rich_text, row, scrollable, span, stack, text, text_editor, tooltip, Space};
use iced::{Alignment, Color, ContentFit, Length, Padding};
use jm_core::engine::{ContactStatus, Message as CoreMessage, MessageBody};
use jm_core::messages::{ChatKey, MediaKind};
use std::collections::BTreeMap;

pub fn view<'a>(m: &'a Main, c: &'a OpenChat) -> El<'a> {
    let lista = messages(m, c);
    let mut camadas: Vec<El<'a>> = vec![lista];
    if let Some(p) = &m.popup {
        let caixa: Option<El<'a>> = match p {
            Popup::Emoji => Some(emoji_popup()),
            Popup::Sounds => Some(sounds_popup(m, false)),
            Popup::Attach => Some(attach_popup()),
            Popup::Gif => Some(gif_popup(m)),
            Popup::CallSounds => None,
        };
        if let Some(caixa) = caixa {
            camadas.push(
                container(ui::slide(iced::widget::opaque(caixa), 0.0, (1.0 - m.motion.v(&m.motion.popup)) * 14.0))
                    .align_bottom(Length::Fill)
                    .align_left(Length::Fill)
                    .padding(Padding { left: 14.0, bottom: 6.0, ..Padding::ZERO })
                    .into(),
            );
        }
    }
    column![header(m, c), stack(camadas).height(Length::Fill), composer(m, c)].into()
}

/* ------------------------------------------------------------ cabeçalho */

fn header<'a>(m: &'a Main, c: &'a OpenChat) -> El<'a> {
    let conteudo: El<'a> = match ChatKey::parse(&c.key) {
        Some(ChatKey::Direct { peer }) => {
            let nome = m.name_of(&peer);
            let contato = m.contacts.get(&peer);
            let ttl = m.summary(&c.key).map(|s| s.ttl).unwrap_or(0);
            let estado: El<'a> = if m.typing_in(&c.key).is_some() {
                ui::typing_label(m.motion.clock(), 12.5)
            } else {
                match contato.map(|x| x.status) {
                    Some(ContactStatus::Request) => text("Fora dos seus contatos").size(12.5).color(t::WARN).into(),
                    Some(ContactStatus::Blocked) => text("Bloqueado").size(12.5).color(t::DANGER).into(),
                    _ if m.listening(&peer).is_some() => {
                        let l = m.listening(&peer).unwrap_or_default();
                        let quem = if l.a.is_empty() { l.s.clone() } else { format!("{} — {}", l.s, l.a) };
                        row![icon(Icon::Music, 13.0, crate::v_modal::SPOTIFY), text(ui::ellipsis(&format!("Ouvindo {quem}"), 60)).size(12.5).color(t::FG_DIM)]
                            .spacing(5)
                            .align_y(Alignment::Center)
                            .into()
                    }
                    _ if ttl > 0 => row![icon(Icon::Timer, 13.0, t::FG_DIM), text(format!("Temporárias: {}", jm_core::engine::ttl_label(ttl))).size(12.5).color(t::FG_DIM)]
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .into(),
                    _ => {
                        let verificado = contato.is_some_and(|x| x.verified);
                        row![
                            icon(if verificado { Icon::Shield } else { Icon::Lock }, 13.0, if verificado { t::accent() } else { t::FG_DIM }),
                            text(if verificado { "Verificado · conversa cifrada" } else { "Conversa cifrada" }).size(12.5).color(t::FG_DIM)
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .into()
                    }
                }
            };
            let pode_ligar = contato.is_some_and(|x| x.status == ContactStatus::Ok) && m.room.is_none();
            let quem = button(
                row![ui::avatar(&nome, m.tone_of(&peer), m.pic_of(&peer), 38.0), column![text(nome.clone()).size(15.5).font(BOLD).color(t::FG), estado].spacing(2)]
                    .spacing(10)
                    .align_y(Alignment::Center),
            )
            .padding([4, 8])
            .style(t::ghost)
            .on_press(Msg::OpenProfile(peer.clone()));
            row![
                quem,
                ui::fill(),
                ui::icon_button(Icon::Phone, "Chamada de voz", pode_ligar.then(|| Msg::Call(peer.clone(), false)), false),
                ui::icon_button(Icon::Video, "Chamada de vídeo (você recebe a câmera do celular)", pode_ligar.then(|| Msg::Call(peer.clone(), true)), false),
                ui::icon_button(Icon::More, "Mais opções", Some(Msg::Menu(CtxKind::Chat(c.key.clone()))), false),
            ]
            .spacing(2)
            .align_y(Alignment::Center)
            .into()
        }
        Some(ChatKey::Channel { lid, channel }) => {
            let g = m.group(&lid);
            let ch = g.and_then(|g| g.meta.channels.iter().find(|x| x.id == channel));
            let nome = ch.map(|x| x.name.clone()).unwrap_or(channel.clone());
            let n = g.map(|g| g.meta.members.len()).unwrap_or(0);
            let sub = match ch.and_then(|x| x.topic.clone()).filter(|t| !t.is_empty()) {
                Some(t) => t,
                None => format!("{} · {n} {}", g.map(|g| g.meta.name.clone()).unwrap_or_default(), if n == 1 { "membro" } else { "membros" }),
            };
            let ttl = m.summary(&c.key).map(|s| s.ttl).unwrap_or(0);
            let mut info = row![text(nome).size(15.5).font(BOLD).color(t::FG)].spacing(8).align_y(Alignment::Center);
            if ttl > 0 {
                info = info.push(ui::tip(icon(Icon::Timer, 14.0, t::FG_DIM), format!("Temporárias: {}", jm_core::engine::ttl_label(ttl)), tooltip::Position::Bottom));
            }
            row![
                container(icon(Icon::Hash, 20.0, t::FG_DIM)).padding([0, 6]),
                column![info, text(ui::ellipsis(&sub, 90)).size(12.5).color(t::FG_DIM)].spacing(2).width(Length::Fill),
                ui::icon_button(Icon::Users, if m.show_members { "Esconder membros" } else { "Mostrar membros" }, Some(Msg::ToggleMembers), m.show_members),
                ui::icon_button(Icon::More, "Mais opções", Some(Msg::Menu(CtxKind::Chat(c.key.clone()))), false),
            ]
            .spacing(2)
            .align_y(Alignment::Center)
            .into()
        }
        None => Space::new().into(),
    };
    container(conteudo).padding([0, 12]).height(60).center_y(60).width(Length::Fill).style(t::header).into()
}

/* ------------------------------------------------------------ mensagens */

fn messages<'a>(m: &'a Main, c: &'a OpenChat) -> El<'a> {
    let grupo = c.key.starts_with("g:");
    let mut col = column![].spacing(0).padding(Padding { top: 12.0, bottom: 10.0, left: 18.0, right: 18.0 });
    if c.more {
        col = col.push(container(ui::secondary(Some(Icon::Up), "Mensagens anteriores", Some(Msg::LoadOlder))).center_x(Length::Fill).padding(8));
    } else {
        col = col.push(
            container(
                container(
                    row![
                        icon(Icon::Lock, 14.0, t::WARN),
                        text("Mensagens e chamadas são cifradas de ponta a ponta. Ninguém fora desta conversa, nem os servidores, consegue ler ou ouvir.")
                            .size(12.5)
                            .color(t::with_alpha(t::WARN, 0.92))
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .max_width(520)
                .style(t::rounded(t::with_alpha(t::WARN, 0.10), 10.0)),
            )
            .center_x(Length::Fill)
            .padding([10, 0]),
        );
    }
    let mut anterior: Option<&CoreMessage> = None;
    for msg in &c.messages {
        let novo_dia = anterior.is_none_or(|p| t::day_key(p.ts) != t::day_key(msg.ts));
        if novo_dia {
            col = col.push(
                container(container(text(t::day_label(msg.ts)).size(12).font(SEMI).color(t::FG_DIM)).padding([4, 12]).style(t::system_note))
                    .center_x(Length::Fill)
                    .padding([10, 0]),
            );
        }
        let emenda = !novo_dia
            && anterior.is_some_and(|p| p.author == msg.author && !p.body.is_system() && msg.ts - p.ts < 5 * 60_000);
        col = col.push(message_row(m, c, msg, emenda, grupo));
        anterior = Some(msg);
    }
    if c.messages.is_empty() {
        col = col.push(container(ui::note("Nenhuma mensagem ainda. Diga oi!")).center_x(Length::Fill).padding(30));
    }
    scrollable(col).id(MESSAGES_ID).anchor_bottom().height(Length::Fill).width(Length::Fill).style(t::scroll).into()
}

fn message_row<'a>(m: &'a Main, c: &'a OpenChat, msg: &'a CoreMessage, emenda: bool, grupo: bool) -> El<'a> {
    if msg.body.is_system() {
        return container(container(text(m.engine.system_text(msg)).size(12.5).color(t::FG_DIM)).padding([5, 12]).style(t::system_note))
            .center_x(Length::Fill)
            .padding([6, 0])
            .into();
    }
    let mine = msg.from_me;
    let mostrar_nome = grupo && !mine && !emenda;
    let bolha = bubble(m, c, msg, mostrar_nome);
    let reacoes = reactions(m, msg);
    let mut coluna = column![bolha].spacing(3).align_x(if mine { Alignment::End } else { Alignment::Start });
    if let Some(r) = reacoes {
        coluna = coluna.push(r);
    }
    let pairando = m.hover.as_deref() == Some(msg.id.as_str());
    let ferramentas: El<'a> = if pairando && !matches!(msg.body, MessageBody::Deleted) {
        let mut r = row![].spacing(0).align_y(Alignment::Center);
        if m.can_write(&c.key) {
            r = r.push(ui::icon_button_sm(Icon::Smile, "Reagir", Some(Msg::ReactMore(msg.id.clone()))));
            r = r.push(ui::icon_button_sm(Icon::Reply, "Responder", Some(Msg::Reply(msg.id.clone()))));
        }
        r = r.push(ui::icon_button_sm(Icon::More, "Mais", Some(Msg::MsgMenu(msg.id.clone()))));
        container(r).padding(2).style(t::rounded(t::surface(), 999.0)).into()
    } else {
        Space::new().width(0).into()
    };
    let linha: El<'a> = if mine {
        row![ui::fill(), ferramentas, coluna].spacing(6).align_y(Alignment::Center).into()
    } else if grupo {
        let lado: El<'a> = if emenda {
            Space::new().width(34).into()
        } else {
            let nome = m.name_of(&msg.author);
            button(ui::avatar(&nome, m.tone_of(&msg.author), m.pic_of(&msg.author), 34.0))
                .padding(0)
                .style(t::ghost)
                .on_press(Msg::MemberMenu(group_of(&c.key), msg.author.clone()))
                .into()
        };
        row![container(lado).align_top(Length::Shrink), coluna, ferramentas, ui::fill()].spacing(8).align_y(Alignment::Start).into()
    } else {
        row![coluna, ferramentas, ui::fill()].spacing(6).align_y(Alignment::Center).into()
    };
    let topo = if emenda { 2.0 } else { 8.0 };
    mouse_area(container(linha).padding(Padding { top: topo, ..Padding::ZERO }).width(Length::Fill))
        .on_enter(Msg::Hover(Some(msg.id.clone())))
        .on_exit(Msg::Unhover(msg.id.clone()))
        .on_right_press(Msg::MsgMenu(msg.id.clone()))
        .into()
}

fn group_of(key: &str) -> String {
    key.strip_prefix("g:").and_then(|r| r.split(':').next()).unwrap_or("").to_string()
}

fn name_color(tone: u8) -> Color {
    t::mix(t::tone(tone), Color::WHITE, 0.5)
}

fn bubble<'a>(m: &'a Main, c: &'a OpenChat, msg: &'a CoreMessage, mostrar_nome: bool) -> El<'a> {
    let mine = msg.from_me;
    let mut dentro = column![].spacing(5);
    if mostrar_nome {
        dentro = dentro.push(text(m.name_of(&msg.author)).size(13).font(SEMI).color(name_color(m.tone_of(&msg.author))));
    }
    if let Some(r) = &msg.reply {
        dentro = dentro.push(quote(m, c, r, mine));
    }
    // Mensagem só de emojis: grande e sem balão, como no celular.
    if let MessageBody::Text(s) = &msg.body {
        if ui::only_emoji(s) && msg.reply.is_none() && !mostrar_nome {
            return column![text(s.trim().to_string()).size(40), footer(msg, true)].spacing(0).align_x(if mine { Alignment::End } else { Alignment::Start }).into();
        }
    }
    let corpo: El<'a> = match &msg.body {
        MessageBody::Text(s) => rich(s),
        MessageBody::Deleted => row![icon(Icon::Ban, 14.0, t::FG_FAINT), text(if mine { "Você apagou esta mensagem" } else { "Mensagem apagada" }).size(14).color(t::FG_FAINT)]
            .spacing(6)
            .align_y(Alignment::Center)
            .into(),
        MessageBody::Sound { id } => {
            let info = m.sounds.info(id);
            let (e, n) = info.map(|s| (s.emoji, s.name)).unwrap_or(("🔊".into(), "Som".into()));
            sound_tile(e, n, Msg::PlaySound(id.clone()))
        }
        MessageBody::Media(x) => match (x.kind, &x.sb) {
            (MediaKind::Image, _) => photo(m, c, msg, x),
            (MediaKind::Audio, Some(sb)) => {
                if x.ready {
                    sound_tile(sb.e.clone(), sb.n.clone(), Msg::PlayMediaSound(x.mid.clone()))
                } else {
                    column![sound_tile(sb.e.clone(), sb.n.clone(), Msg::Noop), progress_line(m, c, msg, x)].spacing(4).into()
                }
            }
            (MediaKind::Audio, None) => voice(m, c, msg, x),
        },
        MessageBody::Invite { lid, name, joined, data } => invite(m, msg, lid, name, *joined, data.as_deref()),
        MessageBody::System { .. } => Space::new().into(),
    };
    dentro = dentro.push(corpo);
    // O balão acompanha o conteúdo; a hora fica no canto direito de baixo.
    let balao = column![dentro, footer(msg, false)].spacing(5).align_x(Alignment::End);
    container(balao).padding([8, 11]).max_width(540).style(t::bubble(mine)).into()
}

fn rich<'a>(s: &str) -> El<'a> {
    let partes = ui::split_links(s);
    if !partes.iter().any(|(_, l)| *l) {
        return text(s.to_string()).size(14.5).color(t::FG).into();
    }
    let spans: Vec<iced::widget::text::Span<'a, String>> = partes
        .into_iter()
        .map(|(txt, link)| if link { span(txt.clone()).link(txt).underline(true).color(t::READ_BLUE) } else { span(txt) })
        .collect();
    rich_text(spans).size(14.5).color(t::FG).on_link_click(Msg::OpenUrl).into()
}

fn footer<'a>(msg: &'a CoreMessage, solto: bool) -> El<'a> {
    let mut r = row![].spacing(4).align_y(Alignment::Center);
    if msg.exp.is_some() {
        r = r.push(icon(Icon::Timer, 12.0, t::FG_FAINT));
    }
    r = r.push(text(t::hhmm(msg.ts)).size(11).color(if solto { t::FG_FAINT } else { t::with_alpha(t::FG, 0.55) }));
    if msg.from_me {
        if let Some(st) = msg.status {
            r = r.push(crate::v_main::status_icon(st, 14.0));
        }
    }
    // Sem largura "preencher": isso esticava o balão até o máximo mesmo com
    // mensagem curta. O alinhamento à direita vem da coluna do balão.
    r.into()
}

fn snippet(m: &Main, q: &CoreMessage) -> String {
    match &q.body {
        MessageBody::Text(s) => ui::ellipsis(s, 80),
        _ => m.engine.preview(q),
    }
}

fn quote<'a>(m: &'a Main, c: &'a OpenChat, id: &str, mine: bool) -> El<'a> {
    let (quem, oque, tom) = match c.messages.iter().find(|x| x.id == id).or(None) {
        Some(q) => {
            let quem = if q.from_me { "Você".to_string() } else { m.name_of(&q.author) };
            (quem, snippet(m, q), m.tone_of(&q.author))
        }
        None => match m.engine.message(id) {
            Some(q) => (if q.from_me { "Você".to_string() } else { m.name_of(&q.author) }, snippet(m, &q), m.tone_of(&q.author)),
            None => (String::new(), "Mensagem indisponível".into(), 0),
        },
    };
    let cor = if mine { t::FG } else { name_color(tom) };
    container(
        row![
            container(Space::new()).width(3).height(Length::Fill).style(t::rounded(cor, 2.0)),
            column![text(quem).size(12.5).font(SEMI).color(cor), text(oque).size(13).color(t::FG_DIM)].spacing(1)
        ]
        .spacing(8)
        .height(Length::Shrink),
    )
    .padding([6, 8])
    .max_width(420)
    .style(t::quote(mine))
    .into()
}

fn sound_tile<'a>(emoji: String, nome: String, msg: Msg) -> El<'a> {
    row![
        container(text(emoji).size(26)).center(44).style(t::rounded(t::with_alpha(Color::BLACK, 0.18), 12.0)),
        column![text(nome).size(14.5).font(SEMI).color(t::FG), text("Som do soundboard").size(12).color(t::FG_DIM)].spacing(2),
        ui::hspace(8.0),
        button(container(icon(Icon::Play, 16.0, t::accent_ink())).center(16)).padding(9).style(t::call_btn(t::CallBtn::Accept)).on_press(msg),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn progress_line<'a>(m: &'a Main, c: &'a OpenChat, msg: &'a CoreMessage, x: &'a jm_core::media::MediaInfo) -> El<'a> {
    let _ = m;
    if x.err {
        return row![icon(Icon::Alert, 13.0, t::DANGER), text("Chegou incompleta").size(12).color(t::DANGER)].spacing(4).into();
    }
    let (feito, total, verbo) = match c.progress.get(&msg.id) {
        Some((d, n)) => (*d, *n, "Enviando"),
        None => (x.got, x.n.max(1), if msg.from_me { "Enviando" } else { "Recebendo" }),
    };
    let pct = (feito as f32 / total.max(1) as f32 * 100.0).round() as u32;
    text(format!("{verbo}… {pct}%")).size(12).color(t::FG_DIM).into()
}

fn fit(w: Option<u32>, h: Option<u32>, max_w: f32, max_h: f32) -> (f32, f32) {
    let (w, h) = (w.unwrap_or(4).max(1) as f32, h.unwrap_or(3).max(1) as f32);
    let k = (max_w / w).min(max_h / h).min(1.0).max(0.05);
    ((w * k).max(120.0).min(max_w), (h * k).max(90.0).min(max_h))
}

fn photo<'a>(m: &'a Main, c: &'a OpenChat, msg: &'a CoreMessage, x: &'a jm_core::media::MediaInfo) -> El<'a> {
    let (w, h) = fit(x.w, x.h, 320.0, 340.0);
    if x.ready && is_gif_mime(&x.mime) {
        if let Some(hd) = m.anim_frame(&x.mid).or_else(|| m.media_image(&x.mid)) {
            let selo = container(container(text("GIF").size(10.5).font(BOLD).color(Color::WHITE)).padding([2, 6]).style(t::rounded(t::with_alpha(Color::BLACK, 0.6), 5.0)))
                .padding(7);
            return mouse_area(stack![image(hd).width(w).height(h).content_fit(ContentFit::Cover).border_radius(10.0), selo].width(w).height(h))
                .on_press(Msg::OpenViewer(x.mid.clone()))
                .interaction(iced::mouse::Interaction::Pointer)
                .into();
        }
    }
    if x.ready {
        if let Some(hd) = m.media_image(&x.mid) {
            return mouse_area(image(hd).width(w).height(h).content_fit(ContentFit::Cover).border_radius(10.0))
                .on_press(Msg::OpenViewer(x.mid.clone()))
                .interaction(iced::mouse::Interaction::Pointer)
                .into();
        }
    }
    let fundo: El<'a> = match x.th.as_deref().and_then(|th| m.pic(Some(th))) {
        Some(hd) => image(hd).width(w).height(h).content_fit(ContentFit::Cover).border_radius(10.0).opacity(0.55_f32).into(),
        None => container(Space::new()).width(w).height(h).style(t::rounded(t::with_alpha(Color::BLACK, 0.25), 10.0)).into(),
    };
    let rotulo: El<'a> = if x.ready {
        text("Foto apagada deste aparelho").size(12.5).color(t::FG).into()
    } else {
        container(progress_line(m, c, msg, x)).padding([5, 10]).style(t::rounded(t::with_alpha(Color::BLACK, 0.55), 999.0)).into()
    };
    stack![fundo, container(rotulo).center(Length::Fill)].width(w).height(h).into()
}

fn voice<'a>(m: &'a Main, c: &'a OpenChat, msg: &'a CoreMessage, x: &'a jm_core::media::MediaInfo) -> El<'a> {
    let tocando = m.track.as_ref().filter(|t| t.0 == x.mid);
    let (pos, dur, pausada) = match tocando {
        Some((_, p, d, pa)) => (*p, *d, *pa),
        None => (0.0, x.dur.unwrap_or(0) as f32, true),
    };
    let em_curso = tocando.is_some_and(|t| !t.3 && t.1 + 0.05 < t.2);
    let progresso = if tocando.is_some() && dur > 0.0 { (pos / dur).clamp(0.0, 1.0) } else { 0.0 };
    let wf: Vec<u8> = x.wf.clone().filter(|v| !v.is_empty()).unwrap_or_else(|| vec![6; 48]);
    let botao = button(container(icon(if em_curso { Icon::Pause } else { Icon::Play }, 16.0, t::accent_ink())).center(16))
        .padding(10)
        .style(t::call_btn(t::CallBtn::Accept))
        .on_press_maybe(x.ready.then(|| Msg::PlayVoice(x.mid.clone())));
    let mid = x.mid.clone();
    let buscar = move |p: f32| Msg::SeekVoice(mid.clone(), p);
    let onda = ui::waveform(
        &wf,
        progresso,
        if msg.from_me { t::FG } else { t::accent() },
        t::with_alpha(t::FG, 0.35),
        if tocando.is_some() { Some(&buscar) } else { None },
    );
    let tempo = if tocando.is_some() && !pausada || progresso > 0.0 { t::mmss(pos) } else { t::mmss(dur) };
    let mut info = row![text(tempo).size(11.5).color(t::FG_DIM)].spacing(8);
    if !x.ready {
        info = info.push(progress_line(m, c, msg, x));
    }
    row![botao, column![onda, info].spacing(2)].spacing(10).align_y(Alignment::Center).into()
}

fn invite<'a>(m: &'a Main, msg: &'a CoreMessage, lid: &'a str, nome: &'a str, joined: bool, data: Option<&'a jm_core::engine::InviteData>) -> El<'a> {
    let pic = data.and_then(|d| m.pic(Some(d.pic.as_str()).filter(|p| !p.is_empty())));
    let tom = data.map(|d| d.meta.t).unwrap_or(0);
    let canais = data.map(|d| d.meta.channels.len()).unwrap_or(0);
    let membros = data.map(|d| d.meta.members.len()).unwrap_or(0);
    let ja_esta = m.group(lid).is_some();
    let acao: El<'a> = if ja_esta || joined {
        ui::secondary_wide(Some(Icon::Users), "Abrir grupo", m.group(lid).map(|_| Msg::Go(Section::Group(lid.to_string())))).into()
    } else if msg.from_me {
        ui::note("Convite enviado")
    } else if data.is_some() {
        ui::primary_wide(Some(Icon::UserPlus), "Entrar no grupo", Some(Msg::JoinInvite(msg.id.clone()))).into()
    } else {
        ui::note("Convite indisponível")
    };
    column![
        row![
            ui::group_avatar(nome, tom, pic, 48.0),
            column![
                text("Convite para o grupo").size(12).color(t::FG_DIM),
                text(nome.to_string()).size(15.5).font(BOLD).color(t::FG),
                text(format!("{membros} {} · {canais} {}", if membros == 1 { "membro" } else { "membros" }, if canais == 1 { "canal" } else { "canais" }))
                    .size(12)
                    .color(t::FG_DIM),
            ]
            .spacing(2)
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        acao,
    ]
    .spacing(10)
    .width(280)
    .into()
}

fn reactions<'a>(m: &'a Main, msg: &'a CoreMessage) -> Option<El<'a>> {
    if msg.reacts.is_empty() || matches!(msg.body, MessageBody::Deleted) {
        return None;
    }
    let me = m.me_pk();
    let mut grupos: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (pk, e) in &msg.reacts {
        grupos.entry(e.as_str()).or_default().push(pk.as_str());
    }
    let mut v: Vec<(&str, Vec<&str>)> = grupos.into_iter().collect();
    v.sort_by_key(|(_, q)| std::cmp::Reverse(q.len()));
    let mut r = row![].spacing(4);
    for (e, quem) in v {
        let minha = quem.contains(&me.as_str());
        let nomes: Vec<String> = quem.iter().map(|pk| if *pk == me { "Você".to_string() } else { m.first_name(pk) }).collect();
        let chip = button(row![text(e.to_string()).size(14), text(quem.len().to_string()).size(12).font(SEMI).color(if minha { t::accent() } else { t::FG_DIM })].spacing(4).align_y(Alignment::Center))
            .padding([2, 8])
            .style(move |_, st| iced::widget::button::Style {
                background: Some(iced::Background::Color(if minha {
                    t::with_alpha(t::accent(), 0.16)
                } else if matches!(st, iced::widget::button::Status::Hovered) {
                    t::surface_hi()
                } else {
                    t::surface()
                })),
                text_color: t::FG,
                border: iced::Border { color: if minha { t::accent() } else { t::line() }, width: 1.0, radius: 999.0.into() },
                ..Default::default()
            })
            .on_press_maybe(m.chat.as_ref().filter(|c| m.can_write(&c.key)).map(|_| Msg::React(msg.id.clone(), e.to_string())));
        r = r.push(ui::tip(chip, nomes.join(", "), tooltip::Position::Top));
    }
    Some(r.into())
}

/* ------------------------------------------------------------ campo */

fn composer<'a>(m: &'a Main, c: &'a OpenChat) -> El<'a> {
    if let Some(pk) = Main::peer_of(&c.key) {
        match m.contact_status(&pk) {
            Some(ContactStatus::Request) => {
                return bar(column![
                    text(format!("{} não está nos seus contatos. Ninguém sabe que você viu esta conversa até você aceitar.", m.name_of(&pk)))
                        .size(13.5)
                        .color(t::FG_DIM),
                    row![
                        ui::secondary_wide(Some(Icon::Ban), "Bloquear", Some(Msg::Block(pk.clone()))),
                        ui::primary_wide(Some(Icon::Check), "Aceitar", Some(Msg::Accept(pk.clone())))
                    ]
                    .spacing(10)
                ]
                .spacing(10)
                .into());
            }
            Some(ContactStatus::Blocked) => {
                return bar(row![
                    text("Você bloqueou este contato. Nada do que ele mandar chega.").size(13.5).color(t::FG_DIM).width(Length::Fill),
                    ui::secondary(None, "Desbloquear", Some(Msg::Unblock(pk.clone())))
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into());
            }
            None => {
                return bar(row![
                    text("Adicione esta pessoa aos contatos para conversar.").size(13.5).color(t::FG_DIM).width(Length::Fill),
                    ui::primary(Some(Icon::UserPlus), "Adicionar", Some(Msg::OpenAddContact(jm_core::identity::npub_hex(&pk))))
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into());
            }
            Some(ContactStatus::Ok) => {}
        }
    }

    if let Some(r) = m.rec.as_ref().filter(|r| r.chat == c.key) {
        let s = r.rec.elapsed();
        return container(
            row![
                container(Space::new()).width(10).height(10).style(t::rounded(t::DANGER, 5.0)),
                text(t::mmss(s as f32)).size(14).font(SEMI).color(t::FG).width(46),
                ui::level_bars(&r.levels, 48, t::accent()),
                ui::fill(),
                ui::icon_button(Icon::Trash, "Cancelar", Some(Msg::RecStop { send: false }), false),
                button(container(icon(Icon::Send, 18.0, t::accent_ink())).center(18)).padding(11).style(t::call_btn(t::CallBtn::Accept)).on_press(Msg::RecStop { send: true }),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding([12, 16])
        .style(t::fill(t::main_bg()))
        .into();
    }

    let mut col = column![].spacing(6);
    if let Some(rid) = &c.reply {
        let alvo = c.messages.iter().find(|x| &x.id == rid);
        let (quem, oque) = match alvo {
            Some(q) => (if q.from_me { "Você".to_string() } else { m.name_of(&q.author) }, snippet(m, q)),
            None => (String::new(), String::new()),
        };
        col = col.push(
            container(
                row![
                    icon(Icon::Reply, 16.0, t::accent()),
                    column![text(format!("Respondendo a {quem}")).size(12.5).font(SEMI).color(t::accent()), text(oque).size(13).color(t::FG_DIM)]
                        .spacing(1)
                        .width(Length::Fill),
                    ui::icon_button_sm(Icon::X, "Cancelar resposta", Some(Msg::CancelReply)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .style(t::rounded(t::surface(), 10.0)),
        );
    }
    let vazio = c.editor.text().trim().is_empty();
    let editor = text_editor(&c.editor)
        .id(EDITOR_ID)
        .placeholder("Mensagem")
        .on_action(Msg::Edit)
        .key_binding(|kp| {
            let focado = matches!(kp.status, text_editor::Status::Focused { .. });
            if focado {
                if let iced::keyboard::Key::Named(Named::Enter) = kp.key.as_ref() {
                    return Some(if kp.modifiers.shift() { text_editor::Binding::Enter } else { text_editor::Binding::Custom(Msg::Send) });
                }
            }
            text_editor::Binding::from_key_press(kp)
        })
        .padding([11, 12])
        .size(14.5)
        .max_height(180)
        .style(t::editor);
    let enviar: El<'a> = if vazio {
        ui::icon_button(Icon::Mic, "Gravar mensagem de voz", Some(Msg::RecStart), false)
    } else {
        ui::tip(
            button(container(icon(Icon::Send, 18.0, t::accent_ink())).center(18)).padding(10).style(t::call_btn(t::CallBtn::Accept)).on_press(Msg::Send),
            "Enviar (Enter) · Shift+Enter quebra a linha",
            tooltip::Position::Top,
        )
    };
    col = col.push(
        row![
            ui::icon_button(Icon::Plus, "Anexar", Some(Msg::SetPopup(Some(Popup::Attach))), m.popup == Some(Popup::Attach)),
            ui::icon_button(Icon::Smile, "Emojis", Some(Msg::SetPopup(Some(Popup::Emoji))), m.popup == Some(Popup::Emoji)),
            gif_button(m.popup == Some(Popup::Gif)),
            editor,
            ui::icon_button(Icon::Wave, "Soundboard", Some(Msg::SetPopup(Some(Popup::Sounds))), m.popup == Some(Popup::Sounds)),
            enviar,
        ]
        .spacing(4)
        .align_y(Alignment::End),
    );
    container(col).padding(Padding { top: 6.0, bottom: 14.0, left: 12.0, right: 12.0 }).into()
}

fn bar<'a>(content: El<'a>) -> El<'a> {
    container(container(content).padding(14).width(Length::Fill).style(t::rounded(t::surface(), 12.0)))
        .padding(Padding { top: 6.0, bottom: 14.0, left: 14.0, right: 14.0 })
        .into()
}

/* ------------------------------------------------------------ janelinhas */

fn emoji_popup<'a>() -> El<'a> {
    let linhas: Vec<El<'a>> = ui::EMOJIS
        .chunks(8)
        .map(|c| {
            row(c.iter()
                .map(|e| button(container(text(*e).size(22)).center(30)).padding(2).style(t::ghost).on_press(Msg::InsertEmoji(e.to_string())).into())
                .collect::<Vec<El<'a>>>())
            .spacing(2)
            .into()
        })
        .collect();
    container(column![
        row![text("Emojis").size(13).font(BOLD).color(t::FG_DIM), ui::fill(), ui::icon_button_sm(Icon::X, "Fechar", Some(Msg::SetPopup(None)))]
            .align_y(Alignment::Center),
        scrollable(column(linhas).spacing(2)).height(250).style(t::scroll)
    ]
    .spacing(6))
    .padding(10)
    .width(340)
    .style(t::panel)
    .into()
}

fn gif_button<'a>(aberto: bool) -> El<'a> {
    let cor = if aberto { t::accent() } else { t::FG_DIM };
    ui::tip(
        button(
            container(text("GIF").size(10.5).font(BOLD).color(cor))
                .padding([2, 4])
                .style(move |_| iced::widget::container::Style {
                    border: iced::Border { color: cor, width: 1.6, radius: 4.0.into() },
                    ..Default::default()
                }),
        )
        .padding([10, 6])
        .style(t::ghost)
        .on_press(Msg::SetPopup(Some(Popup::Gif))),
        "GIFs",
        tooltip::Position::Top,
    )
}

/// Busca de GIFs, no jeito do Discord: campo em cima, grade animada.
fn gif_popup<'a>(m: &'a Main) -> El<'a> {
    let g = &m.gif;
    let servico = m.gif_keys.usable().map(|x| x.0);
    let fechar = ui::icon_button_sm(Icon::X, "Fechar", Some(Msg::SetPopup(None)));
    let largura = 452.0;
    let do_pc = ui::ghost(Some(Icon::Upload), "GIF do computador", Some(Msg::GifFromFile));
    let Some(p) = servico else {
        return container(
            column![
                row![text("GIFs").size(13).font(BOLD).color(t::FG_DIM), ui::fill(), fechar].align_y(Alignment::Center),
                container(
                    column![
                        icon(Icon::Image, 34.0, t::accent()),
                        text("Busca de GIFs em breve").size(15).font(BOLD).color(t::FG),
                        text("A busca no Tenor e no GIPHY ainda não está disponível nesta versão. Dá para mandar um GIF do computador.").size(13).color(t::FG_DIM).align_x(Alignment::Center),
                        ui::primary(Some(Icon::Upload), "GIF do computador", Some(Msg::GifFromFile)),
                    ]
                    .spacing(10)
                    .align_x(Alignment::Center),
                )
                .center_x(Length::Fill)
                .padding([26, 10]),
            ]
            .spacing(6),
        )
        .padding(10)
        .width(largura)
        .style(t::panel)
        .into();
    };
    let busca = iced::widget::text_input(&format!("Buscar no {}", p.label()), &g.query)
        .id(crate::update::GIF_INPUT_ID)
        .on_input(Msg::GifQuery)
        .padding([9, 12])
        .size(14)
        .style(t::input);
    let corpo: El<'a> = if let Some(e) = &g.error {
        container(text(e.clone()).size(13).color(t::DANGER)).center_x(Length::Fill).padding(30).into()
    } else if g.hits.is_empty() && g.loading {
        container(
            row![
                iced::widget::canvas(crate::canvas::Spinner { t: m.motion.clock(), color: t::accent() }).width(22).height(22),
                text("Buscando…").size(13).color(t::FG_DIM)
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .center_x(Length::Fill)
        .padding(30)
        .into()
    } else if g.hits.is_empty() {
        let msg = if g.query.trim().is_empty() { "" } else { "Nenhum GIF encontrado." };
        container(text(msg).size(13).color(t::FG_DIM)).center_x(Length::Fill).padding(30).into()
    } else {
        let lado = (largura - 20.0 - 8.0 - 12.0) / 2.0;
        let cols: Vec<El<'a>> = (0..2)
            .map(|c| {
                column(g.hits.iter().enumerate().filter(|(i, _)| i % 2 == c).map(|(i, h)| {
                    let alt = if h.w > 0 && h.h > 0 { (lado * h.h as f32 / h.w as f32).clamp(70.0, 220.0) } else { 120.0 };
                    let fundo: El<'a> = match m.anim_frame(&h.preview) {
                        Some(f) => image(f).width(lado).height(alt).content_fit(ContentFit::Cover).border_radius(8.0).into(),
                        None => container(Space::new()).width(lado).height(alt).style(t::rounded(t::surface_hi(), 8.0)).into(),
                    };
                    let mut camadas = vec![fundo];
                    if g.sending.as_deref() == Some(h.id.as_str()) {
                        camadas.push(
                            container(container(text("Enviando…").size(12).color(Color::WHITE)).padding([4, 10]).style(t::rounded(t::with_alpha(Color::BLACK, 0.65), 999.0)))
                                .center(Length::Fill)
                                .into(),
                        );
                    }
                    ui::tip(
                        button(stack(camadas).width(lado).height(alt)).padding(0).style(t::ghost).on_press_maybe(g.sending.is_none().then_some(Msg::GifPick(i))),
                        if h.title.is_empty() { "Mandar".to_string() } else { ui::ellipsis(&h.title, 40) },
                        tooltip::Position::Top,
                    )
                }))
                .spacing(8)
                .into()
            })
            .collect();
        scrollable(container(row(cols).spacing(8)).padding(Padding { right: 12.0, ..Padding::ZERO })).height(Length::Fill).style(t::scroll).into()
    };
    let titulo = if g.query.trim().is_empty() { "Em alta" } else { "Resultados" };
    let creditos = match g.from.unwrap_or(p) {
        crate::gifs::Provider::Tenor => "Via Tenor",
        crate::gifs::Provider::Giphy => "Powered by GIPHY",
    };
    container(
        column![
            row![busca, fechar].spacing(6).align_y(Alignment::Center),
            text(titulo).size(12).font(SEMI).color(t::FG_DIM),
            container(corpo).height(340),
            row![do_pc, ui::fill(), text(creditos).size(11.5).color(t::FG_FAINT)].align_y(Alignment::Center),
        ]
        .spacing(8),
    )
    .padding(10)
    .width(largura)
    .style(t::panel)
    .into()
}

fn attach_popup<'a>() -> El<'a> {
    container(
        column![
            ui::menu_item(Icon::Image, "Foto do computador", Msg::PickPhoto, false),
            ui::menu_item(Icon::Image, "GIF", Msg::SetPopup(Some(Popup::Gif)), false),
            ui::menu_item(Icon::Wave, "Som do soundboard", Msg::SetPopup(Some(Popup::Sounds)), false),
        ]
        .spacing(2),
    )
    .padding(6)
    .width(230)
    .style(t::panel)
    .into()
}

/// Grade do soundboard. Na conversa manda o som; na chamada toca para todos.
pub fn sounds_popup<'a>(m: &'a Main, chamada: bool) -> El<'a> {
    let lib = m.sounds.library();
    let mut itens: Vec<El<'a>> = lib
        .into_iter()
        .map(|s| {
            let msg = if chamada { Msg::CallSound(s.id.clone()) } else { Msg::SendSound(s.id.clone()) };
            let tile = button(
                container(
                    column![
                        text(s.emoji.clone()).size(26),
                        text(ui::ellipsis(&s.name, 11)).size(12).wrapping(iced::widget::text::Wrapping::None).color(t::FG)
                    ]
                    .spacing(4)
                    .align_x(Alignment::Center),
                )
                .center(Length::Fill),
            )
            .padding([6, 2])
            .width(84)
            .height(76)
            .style(move |th, st| {
                let mut b = t::secondary(th, st);
                b.border.radius = 12.0.into();
                if !s.builtin {
                    b.border.color = t::with_alpha(t::accent(), 0.5);
                    b.border.width = 1.0;
                }
                b
            })
            .on_press(msg);
            ui::tip(tile, format!("{} · {}", s.name, fmt_dur(s.dur)), tooltip::Position::Top)
        })
        .collect();
    itens.push(
        button(container(column![icon(Icon::Plus, 24.0, t::accent()), text("Adicionar").size(12).color(t::accent())].spacing(5).align_x(Alignment::Center)).center(Length::Fill))
            .padding([6, 2])
            .width(84)
            .height(76)
            .style(|th, st| {
                let mut b = t::ghost(th, st);
                b.border = iced::Border { color: t::with_alpha(t::accent(), 0.5), width: 1.0, radius: 12.0.into() };
                b
            })
            .on_press(Msg::SbAdd)
            .into(),
    );
    let mut linhas = column![].spacing(6);
    let n_linhas = itens.len().div_ceil(4);
    let altura = (n_linhas as f32 * 82.0).min(250.0);
    let mut it = itens.into_iter().peekable();
    while it.peek().is_some() {
        let mut r = row![].spacing(6);
        for _ in 0..4 {
            if let Some(e) = it.next() {
                r = r.push(e);
            }
        }
        linhas = linhas.push(r);
    }
    let sub = if chamada { "Todos na chamada ouvem" } else { "Envie um som para a conversa" };
    container(
        column![
            row![
                column![text("Soundboard").size(14).font(BOLD).color(t::FG), text(sub).size(12).color(t::FG_DIM)].spacing(1),
                ui::fill(),
                ui::icon_button_sm(Icon::X, "Fechar", Some(Msg::SetPopup(None)))
            ]
            .align_y(Alignment::Center),
            scrollable(linhas)
                .height(Length::Fixed(altura))
                .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4).spacing(4)))
                .style(t::scroll),
            row![
                icon(Icon::Volume, 16.0, t::FG_DIM),
                iced::widget::slider(0.0..=1.0, m.sounds.volume(), Msg::Volume).step(0.01_f32).width(Length::Fill),
                ui::icon_button_sm(Icon::Sliders, "Gerenciar sons", Some(Msg::Page(SettingsPage::Soundboard))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(10),
    )
    .padding(12)
    .width(398)
    .style(t::panel)
    .into()
}

pub fn fmt_dur(d: f32) -> String {
    format!("{:.1} s", d).replace('.', ",")
}
