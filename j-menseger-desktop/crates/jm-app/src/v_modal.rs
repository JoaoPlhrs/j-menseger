//! Janelas por cima da tela principal e a tela de bloqueio do PIN.

use crate::app::{Key, Msg};
use crate::icons::{icon, logo, Icon};
use crate::state::*;
use crate::theme as t;
use crate::ui::{self, El, BOLD, SEMI};
use iced::widget::{button, canvas, column, container, image, row, scrollable, stack, text, toggler, Space};
use iced::{Alignment, Color, ContentFit, Length};
use jm_core::engine::{ContactStatus, NotifPreview};
use jm_core::groups::{ChannelKind, Template};
use jm_media::screen::{Quality, SourceKind};

fn card<'a>(w: f32, body: El<'a>) -> El<'a> {
    container(body).padding(24).width(w).style(t::panel).into()
}

fn head<'a>(title: &str, sub: Option<&str>) -> El<'a> {
    let mut c = column![row![text(title.to_string()).size(19).font(BOLD).color(t::FG).width(Length::Fill), ui::icon_button_sm(Icon::X, "Fechar", Some(Msg::Key(Key::Escape)))]
        .align_y(Alignment::Center)]
    .spacing(4);
    if let Some(s) = sub {
        c = c.push(text(s.to_string()).size(13.5).color(t::FG_DIM));
    }
    c.into()
}

fn limited<'a>(m: &Main, body: El<'a>) -> El<'a> {
    scrollable(body)
        .height(Length::Shrink)
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4).spacing(8),
        ))
        .style(t::scroll)
        .into_max(m.size.height - 140.0)
}

/// `scrollable` com altura máxima (a janela não pode passar da tela).
trait IntoMax<'a> {
    fn into_max(self, h: f32) -> El<'a>;
}

impl<'a> IntoMax<'a> for iced::widget::Scrollable<'a, Msg> {
    fn into_max(self, h: f32) -> El<'a> {
        container(self).max_height(h.max(200.0)).into()
    }
}

pub fn view<'a>(m: &'a Main, modal: &'a Modal) -> El<'a> {
    match modal {
        Modal::AddContact { code, nick, error, busy } => add_contact(code, nick, error.as_deref(), *busy),
        Modal::MyCode => card(
            420.0,
            column![
                head("Meu código", Some("Mostre o QR para alguém ler com o celular, ou copie o código.")),
                container(crate::v_settings::qr_box(m, 5.0)).center_x(Length::Fill).padding(4),
                crate::v_settings::short_code_box(m),
                container(text(m.engine.npub()).size(12.5).wrapping(iced::widget::text::Wrapping::Glyph).color(t::FG_DIM))
                    .padding(10)
                    .width(Length::Fill)
                    .style(t::rounded(t::surface_hi(), 10.0)),
                row![
                    ui::primary_wide(Some(Icon::Copy), "Copiar código", Some(Msg::CopyMyCode)),
                    ui::secondary_wide(Some(Icon::Share), "Copiar convite", Some(Msg::CopyInvite))
                ]
                .spacing(8),
            ]
            .spacing(12)
            .into(),
        ),
        Modal::NewChat => new_chat(m),
        Modal::NewGroup { name, template, busy } => new_group(name, *template, *busy),
        Modal::Invite { lid } => invite(m, lid),
        Modal::Profile { pk, nick } => profile(m, pk, nick),
        Modal::Choose(k) => choose(m, k),
        Modal::Confirm(k) => confirm(m, k),
        Modal::DeleteMsg { id, mine } => card(
            380.0,
            column![
                head("Apagar mensagem?", Some(if *mine { "Apagar para todos tira a mensagem dos aparelhos de todos da conversa." } else { "A mensagem some só deste computador." })),
                ui::vspace(4.0),
                if *mine { El::from(ui::danger_wide(Some(Icon::Trash), "Apagar para todos", Some(Msg::DeleteForAll(id.clone())))) } else { Space::new().into() },
                ui::secondary_wide(None, "Apagar só para mim", Some(Msg::DeleteForMe(id.clone()))),
                ui::ghost(None, "Cancelar", Some(Msg::Key(Key::Escape))),
            ]
            .spacing(8)
            .align_x(Alignment::Center)
            .into(),
        ),
        Modal::ReactPicker { id } => {
            let linhas: Vec<El<'a>> = ui::EMOJIS
                .chunks(10)
                .map(|c| {
                    row(c.iter()
                        .map(|e| button(container(text(*e).size(22)).center(30)).padding(2).style(t::ghost).on_press(Msg::React(id.clone(), e.to_string())).into())
                        .collect::<Vec<El<'a>>>())
                    .spacing(2)
                    .into()
                })
                .collect();
            card(410.0, column![head("Reagir", None), scrollable(column(linhas).spacing(2)).height(300).style(t::scroll)].spacing(10).into())
        }
        Modal::Pin(p) => pin_pad(p),
        Modal::Forgot { input, error } => forgot(input, error.as_deref(), false),
        Modal::Viewer { mid } => viewer(m, mid),
        Modal::PhotoPreview(d) => card(
            520.0,
            column![
                if is_gif_mime(&d.mime) {
                    head("Mandar GIF", Some("O GIF vai do jeito que está, animado e cifrado como uma foto."))
                } else {
                    head("Mandar foto", Some("A foto foi reduzida e perdeu os metadados (como localização) antes de sair."))
                },
                container(image(m.anim_frame(PREVIA_GIF).filter(|_| is_gif_mime(&d.mime)).unwrap_or_else(|| d.handle.clone())).content_fit(ContentFit::Contain).width(Length::Fill).height(Length::Fixed((m.size.height - 320.0).clamp(160.0, 420.0))))
                    .style(t::rounded(Color::BLACK, 10.0)),
                text(format!("{}×{} · {}", d.w, d.h, t::bytes_label(d.bytes.len() as i64))).size(12.5).color(t::FG_DIM),
                row![
                    if is_gif_mime(&d.mime) { El::from(Space::new()) } else { ui::secondary(Some(Icon::Eye), "Visualização única", Some(Msg::SendPhotoOnce)).into() },
                    ui::fill(),
                    ui::ghost(None, "Cancelar", Some(Msg::Key(Key::Escape))),
                    ui::primary(Some(Icon::Send), "Enviar", Some(Msg::SendPhoto))
                ]
                .spacing(8),
            ]
            .spacing(12)
            .into(),
        ),
        Modal::GroupInfo(f) => group_info(m, f),
        Modal::EditChannel { name, topic, .. } => card(
            420.0,
            column![
                head("Editar canal", None),
                ui::field("Nome", ui::input("nome", name, Msg::ChannelEditName).on_submit(Msg::ChannelEditSave)),
                ui::field("Assunto (opcional)", ui::input("Sobre o que é este canal", topic, Msg::ChannelEditTopic).on_submit(Msg::ChannelEditSave)),
                row![ui::fill(), ui::ghost(None, "Cancelar", Some(Msg::Key(Key::Escape))), ui::primary(Some(Icon::Check), "Salvar", Some(Msg::ChannelEditSave))].spacing(8),
            ]
            .spacing(12)
            .into(),
        ),
        Modal::AddSound => card(
            440.0,
            column![
                head("Adicionar som", Some("Até 8 segundos. Você escolhe o trecho antes de salvar.")),
                row![
                    tile_button(Icon::Mic, "Gravar agora", Msg::SbRecord),
                    tile_button(Icon::Music, "Arquivo de áudio", Msg::SbImport)
                ]
                .spacing(10),
                ui::note("Funciona com MP3, M4A, OGG, WAV, FLAC e outros formatos comuns. Também dá para soltar o arquivo na janela."),
            ]
            .spacing(14)
            .into(),
        ),
        Modal::RecordSound { rec, levels } => record_sound(rec.as_ref(), levels),
        Modal::Trim(st) => trim(m, st),
        Modal::EditSound { name, emoji, .. } => card(
            440.0,
            column![
                head("Editar som", None),
                ui::field("Nome", ui::input("Nome do som", name, Msg::SbEditName).on_submit(Msg::SbEditSave)),
                text("Emoji").size(12.5).font(SEMI).color(t::FG_DIM),
                emoji_grid(emoji, Msg::SbEditEmoji),
                row![ui::fill(), ui::ghost(None, "Cancelar", Some(Msg::Key(Key::Escape))), ui::primary(Some(Icon::Check), "Salvar", Some(Msg::SbEditSave))].spacing(8),
            ]
            .spacing(10)
            .into(),
        ),
        Modal::Share { sources } => share(m, sources),
        Modal::Crop(c) => crop(m, c),
        Modal::Search { all, query } => search(m, all, query),
        Modal::Forward { query, .. } => forward(m, query),
        Modal::SetStatus { e, t: txt, dnd, dur } => {
            let duracoes = row([(0u8, "Não limpar"), (1, "1 hora"), (2, "4 horas"), (3, "Hoje")]
                .into_iter()
                .map(|(k, n)| button(text(n).size(13)).padding([6, 10]).style(t::option(*dur == k)).on_press(Msg::StatusDur(k)).into())
                .collect::<Vec<El<'a>>>())
            .spacing(6);
            let prontos = row([("🎮", "Jogando"), ("📚", "Estudando"), ("💼", "Trabalhando"), ("🌙", "Dormindo")]
                .into_iter()
                .map(|(em, tx)| button(text(format!("{em} {tx}")).size(13)).padding([6, 10]).style(t::option(false)).on_press(Msg::StatusField(2, format!("{em}|{tx}"))).into())
                .collect::<Vec<El<'a>>>())
            .spacing(6);
            card(
                460.0,
                column![
                    head("Definir status", Some("Aparece para os seus contatos, junto com a música do Spotify.")),
                    row![
                        iced::widget::text_input("🎮", e).on_input(|v| Msg::StatusField(0, v)).padding([10, 10]).size(16).width(56).style(t::input),
                        iced::widget::text_input("Ex.: Jogando Valorant", txt).on_input(|v| Msg::StatusField(1, v)).on_submit(Msg::StatusSave(false)).padding([10, 12]).size(14.5).style(t::input),
                    ]
                    .spacing(8),
                    prontos,
                    row![
                        column![text("Não perturbe").size(14).color(t::FG), text("Sem som nem aviso de mensagem. Os outros veem que você não quer ser incomodado.").size(12).color(t::FG_DIM)]
                            .spacing(2)
                            .width(Length::Fill),
                        toggler(*dnd).on_toggle(Msg::StatusDnd).size(20)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    ui::field("Limpar depois de", duracoes),
                    row![ui::ghost(None, "Tirar status", Some(Msg::StatusSave(true))), ui::fill(), ui::primary(Some(Icon::Check), "Salvar", Some(Msg::StatusSave(false)))].spacing(8),
                ]
                .spacing(12)
                .into(),
            )
        }
        Modal::PeerVolume { pk, v } => card(
            380.0,
            column![
                head(&format!("Volume de {}", m.name_of(pk)), Some("Só muda o que você ouve. Fica guardado para as próximas chamadas.")),
                row![
                    icon(Icon::Volume, 18.0, t::FG_DIM),
                    iced::widget::slider(0.0..=2.0, *v, Msg::PeerVolume).step(0.05_f32).width(Length::Fill),
                    text(format!("{:.0}%", v * 100.0)).size(14).font(SEMI).color(t::FG).width(48)
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![ui::ghost(None, "Voltar ao normal", Some(Msg::PeerVolume(1.0))), ui::fill(), ui::primary(None, "Pronto", Some(Msg::Key(Key::Escape)))].spacing(8),
            ]
            .spacing(14)
            .into(),
        ),
        Modal::NewEvent { title, date, time, desc } => card(
            440.0,
            column![
                head("Novo evento", Some("Todos da conversa podem responder Vou, Talvez ou Não vou. Quem vai recebe um lembrete 15 minutos antes.")),
                ui::field("Nome", iced::widget::text_input("Ex.: Jogatina de sábado", title).on_input(|v| Msg::EventField(0, v)).padding([10, 12]).size(14.5).style(t::input)),
                row![
                    ui::field("Data", iced::widget::text_input("23/11/2026", date).on_input(|v| Msg::EventField(1, v)).padding([10, 12]).size(14.5).style(t::input)),
                    ui::field("Hora", iced::widget::text_input("20:00", time).on_input(|v| Msg::EventField(2, v)).padding([10, 12]).size(14.5).style(t::input)),
                ]
                .spacing(10),
                ui::field("Descrição (opcional)", iced::widget::text_input("Onde, o que levar…", desc).on_input(|v| Msg::EventField(3, v)).padding([10, 12]).size(14.5).style(t::input)),
                row![ui::fill(), ui::ghost(None, "Cancelar", Some(Msg::Key(Key::Escape))), ui::primary(Some(Icon::Send), "Criar evento", Some(Msg::EventSend))].spacing(8),
            ]
            .spacing(12)
            .into(),
        ),
        Modal::ViewOnce { handle } => card(
            640.0,
            column![
                head("Visualização única", Some("Ao fechar, a foto some deste aparelho.")),
                container(image(handle.clone()).content_fit(ContentFit::Contain).width(Length::Fill).height(Length::Fixed((m.size.height - 240.0).clamp(200.0, 560.0))))
                    .style(t::rounded(Color::BLACK, 10.0)),
            ]
            .spacing(12)
            .into(),
        ),
        Modal::Pins { list } => pins(m, list),
        Modal::NewPoll { q, opts, multi } => new_poll(q, opts, *multi),
        Modal::Voters { id } => voters(m, id),
        Modal::Roles { lid, form } => roles(m, lid, form.as_ref()),
        Modal::MemberRoles { lid, pk } => member_roles(m, lid, pk),
    }
}

pub const SEARCH_ID: &str = "busca-conversa";

fn forward<'a>(m: &'a Main, query: &'a str) -> El<'a> {
    let q = sem_acento(query.trim());
    let mut col = column![].spacing(2);
    let mut n = 0;
    for c in &m.chats {
        let titulo = match c.key.strip_prefix("g:").and_then(|r| r.split(':').next()).and_then(|lid| m.group(lid)) {
            Some(g) => format!("{} · {}", m.chat_title(&c.key), g.meta.name),
            None => m.chat_title(&c.key),
        };
        if !q.is_empty() && !sem_acento(&titulo).contains(&q) {
            continue;
        }
        if let Some(pk) = &c.peer {
            if m.contact_status(pk) != Some(ContactStatus::Ok) {
                continue;
            }
        }
        n += 1;
        if n > 60 {
            break;
        }
        let ic = if c.is_group { Icon::Hash } else { Icon::User };
        col = col.push(
            button(row![icon(ic, 16.0, t::FG_DIM), text(titulo).size(14.5).color(t::FG)].spacing(10).align_y(Alignment::Center))
                .padding([9, 10])
                .width(Length::Fill)
                .style(t::row_item(false))
                .on_press(Msg::ForwardTo(c.key.clone())),
        );
    }
    if n == 0 {
        col = col.push(ui::note("Nenhuma conversa encontrada."));
    }
    card(
        440.0,
        column![
            head("Encaminhar para…", None),
            iced::widget::text_input("Buscar conversa", query).on_input(Msg::ForwardQuery).padding([10, 12]).size(14.5).style(t::input),
            limited(m, col.into()),
        ]
        .spacing(12)
        .into(),
    )
}
pub const ROLE_NAME_ID: &str = "nome-cargo";

fn sem_acento(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            x => x,
        })
        .collect()
}

fn quem<'a>(m: &Main, msg: &jm_core::engine::Message) -> String {
    if msg.from_me {
        "Você".into()
    } else {
        m.name_of(&msg.author)
    }
}

fn search<'a>(m: &'a Main, all: &'a [jm_core::engine::Message], query: &'a str) -> El<'a> {
    let q = sem_acento(query.trim());
    let mut lista = column![].spacing(2);
    if q.chars().count() < 2 {
        lista = lista.push(ui::note(format!("{} mensagens de texto neste computador. Digite pelo menos 2 letras.", all.len())));
    } else {
        let achadas: Vec<&jm_core::engine::Message> = all
            .iter()
            .rev()
            .filter(|x| match &x.body {
                jm_core::engine::MessageBody::Text(t) => sem_acento(t).contains(&q),
                jm_core::engine::MessageBody::Poll { q: p, .. } => sem_acento(p).contains(&q),
                _ => false,
            })
            .take(60)
            .collect();
        if achadas.is_empty() {
            lista = lista.push(ui::note("Nada encontrado."));
        }
        for x in achadas {
            let texto = match &x.body {
                jm_core::engine::MessageBody::Text(t) => t.clone(),
                jm_core::engine::MessageBody::Poll { q, .. } => format!("Enquete: {q}"),
                _ => String::new(),
            };
            // Mostra um trecho em volta do que foi achado.
            let pos = sem_acento(&texto).find(&q).unwrap_or(0);
            let ini = texto.char_indices().map(|(i, _)| i).filter(|i| *i <= pos).rev().nth(30).unwrap_or(0);
            let trecho: String = texto[ini..].chars().take(140).collect();
            lista = lista.push(
                button(
                    column![
                        row![
                            text(quem(m, x)).size(13).font(SEMI).color(t::FG),
                            text(format!("{} {}", t::day_label(x.ts), t::hhmm(x.ts))).size(12).color(t::FG_FAINT)
                        ]
                        .spacing(8),
                        text(format!("{}{}", if ini > 0 { "…" } else { "" }, trecho)).size(13.5).color(t::FG_DIM)
                    ]
                    .spacing(2),
                )
                .padding([8, 10])
                .width(Length::Fill)
                .style(t::row_item(false))
                .on_press(Msg::JumpTo(x.id.clone())),
            );
        }
    }
    card(
        520.0,
        column![
            head("Buscar na conversa", None),
            iced::widget::text_input("Digite uma palavra", query).id(SEARCH_ID).on_input(Msg::SearchInput).padding([10, 12]).size(14.5).style(t::input),
            limited(m, lista.into()),
        ]
        .spacing(12)
        .into(),
    )
}

fn pins<'a>(m: &'a Main, list: &'a [jm_core::engine::Message]) -> El<'a> {
    let pode = m.chat.as_ref().is_some_and(|c| m.can_pin(&c.key));
    let mut col = column![].spacing(4);
    if list.is_empty() {
        col = col.push(ui::note("Nenhuma mensagem fixada. Clique com o botão direito numa mensagem e escolha “Fixar na conversa”."));
    }
    for x in list {
        let mut r = row![button(
            column![text(quem(m, x)).size(13).font(SEMI).color(t::FG), text(ui::ellipsis(&m.engine.preview(x), 160)).size(13.5).color(t::FG_DIM)].spacing(2)
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(t::row_item(false))
        .on_press(Msg::JumpTo(x.id.clone()))]
        .spacing(4)
        .align_y(Alignment::Center);
        if pode {
            r = r.push(ui::icon_button_sm(Icon::X, "Desafixar", Some(Msg::PinMsg(x.id.clone(), false))));
        }
        col = col.push(r);
    }
    card(480.0, column![head("Mensagens fixadas", None), limited(m, col.into())].spacing(12).into())
}

fn new_poll<'a>(q: &'a str, opts: &'a [String], multi: bool) -> El<'a> {
    let mut lista = column![].spacing(6);
    for (i, o) in opts.iter().enumerate() {
        let mut r = row![iced::widget::text_input(&format!("Opção {}", i + 1), o)
            .on_input(move |v| Msg::PollOpt(i, v))
            .padding([10, 12])
            .size(14.5)
            .style(t::input)]
        .spacing(4)
        .align_y(Alignment::Center);
        if opts.len() > 2 {
            r = r.push(ui::icon_button_sm(Icon::X, "Tirar opção", Some(Msg::PollDelOpt(i))));
        }
        lista = lista.push(r);
    }
    if opts.len() < 10 {
        lista = lista.push(ui::ghost(Some(Icon::Plus), "Adicionar opção", Some(Msg::PollAddOpt)));
    }
    card(
        460.0,
        column![
            head("Nova enquete", None),
            ui::field("Pergunta", ui::input("Ex.: Que dia fica melhor?", q, Msg::PollQ)),
            ui::field("Opções", lista),
            row![text("Permitir mais de uma resposta").size(14).color(t::FG).width(Length::Fill), toggler(multi).on_toggle(Msg::PollMulti).size(20)]
                .align_y(Alignment::Center),
            row![ui::fill(), ui::ghost(None, "Cancelar", Some(Msg::Key(Key::Escape))), ui::primary(Some(Icon::Send), "Enviar enquete", Some(Msg::PollSend))].spacing(8),
        ]
        .spacing(12)
        .into(),
    )
}

fn voters<'a>(m: &'a Main, id: &'a str) -> El<'a> {
    let msg = m.chat.as_ref().and_then(|c| c.messages.iter().find(|x| x.id == id).cloned()).or_else(|| m.engine.message(id));
    let Some(jm_core::engine::Message { body: jm_core::engine::MessageBody::Poll { q, o, votes, .. }, .. }) = msg else {
        return card(400.0, column![head("Votos", None), ui::note("Enquete indisponível.")].spacing(12).into());
    };
    let me = m.me_pk();
    let mut col = column![].spacing(6);
    for (i, opcao) in o.iter().enumerate() {
        let quem: Vec<&String> = votes.iter().filter(|(_, v)| v.contains(&(i as u8))).map(|(pk, _)| pk).collect();
        col = col.push(text(format!("{opcao} · {}", quem.len())).size(13).font(BOLD).color(t::FG_DIM));
        if quem.is_empty() {
            col = col.push(text("Ninguém ainda").size(13).color(t::FG_FAINT));
        }
        for pk in quem {
            let nome = if *pk == me { "Você".to_string() } else { m.name_of(pk) };
            col = col.push(row![ui::avatar(&nome, m.tone_of(pk), m.pic_of(pk), 24.0), text(nome).size(14).color(t::FG)].spacing(8).align_y(Alignment::Center));
        }
        col = col.push(ui::vspace(6.0));
    }
    card(420.0, column![head(&q, None), limited(m, col.into())].spacing(12).into())
}

fn cor_cargo(c: u8) -> Color {
    let (r, g, b) = jm_core::groups::ROLE_CORES[(c % 8) as usize];
    Color::from_rgb8(r, g, b)
}

fn roles<'a>(m: &'a Main, lid: &'a str, form: Option<&'a RoleForm>) -> El<'a> {
    let Some(g) = m.group(lid) else { return Space::new().into() };
    let admin = g.meta.is_admin(&m.me_pk());
    if let Some(f) = form {
        let mut cores = row![].spacing(8);
        for c in 0..8u8 {
            let sel = f.c == c;
            cores = cores.push(
                button(Space::new().width(26).height(26))
                    .padding(0)
                    .style(move |_, _| iced::widget::button::Style {
                        background: Some(iced::Background::Color(cor_cargo(c))),
                        border: iced::Border { color: if sel { t::FG } else { Color::TRANSPARENT }, width: 3.0, radius: 999.0.into() },
                        ..Default::default()
                    })
                    .on_press(Msg::RoleColor(c)),
            );
        }
        let mut botoes = row![].spacing(8);
        if let Some(id) = &f.id {
            botoes = botoes.push(ui::danger_text(Some(Icon::Trash), "Apagar cargo", Some(Msg::RoleDelete(id.clone()))));
        }
        botoes = botoes.push(ui::fill()).push(ui::ghost(None, "Voltar", Some(Msg::OpenRoles(lid.to_string())))).push(ui::primary(Some(Icon::Check), "Salvar", Some(Msg::RoleSave)));
        return card(
            420.0,
            column![
                head(if f.id.is_some() { "Editar cargo" } else { "Novo cargo" }, None),
                ui::field("Nome", ui::input("Ex.: Moderador", &f.name, Msg::RoleName).id(ROLE_NAME_ID).on_submit(Msg::RoleSave)),
                ui::field("Cor", cores),
                ui::field("Poderes", {
                    let mut c = column![].spacing(6);
                    for (k, rotulo) in jm_core::groups::PERMS {
                        let on = f.p.iter().any(|x| x == k);
                        c = c.push(row![text(rotulo).size(14).color(t::FG).width(Length::Fill), toggler(on).on_toggle(move |v| Msg::RolePerm(k.to_string(), v)).size(20)].align_y(Alignment::Center));
                    }
                    c.push(text("Mudar canais, membros e cargos continua só com admins.").size(12).color(t::FG_DIM))
                }),
                botoes,
            ]
            .spacing(14)
            .into(),
        );
    }
    let mut col = column![].spacing(2);
    if g.meta.roles.is_empty() {
        col = col.push(ui::note("Nenhum cargo ainda."));
    }
    for r in &g.meta.roles {
        let n = g.meta.mroles.values().filter(|v| v.contains(&r.id)).count();
        let linha = row![
            container(Space::new()).width(14).height(14).style(t::rounded(cor_cargo(r.c), 7.0)),
            text(r.n.clone()).size(14.5).font(SEMI).color(cor_cargo(r.c)).width(Length::Fill),
            text(format!("{n} {}", if n == 1 { "pessoa" } else { "pessoas" })).size(12.5).color(t::FG_DIM)
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        let b = button(linha).padding([9, 10]).width(Length::Fill).style(t::row_item(false));
        col = col.push(if admin { b.on_press(Msg::RoleForm(Some(r.id.clone()))) } else { b });
    }
    let mut corpo = column![
        head("Cargos", Some(if admin { "Cargos dão nome e cor às pessoas do grupo. Para dar um cargo, clique com o botão direito na pessoa." } else { "Só admins criam e dão cargos." })),
        limited(m, col.into())
    ]
    .spacing(12);
    if admin && g.meta.roles.len() < jm_core::groups::MAX_ROLES {
        corpo = corpo.push(row![ui::fill(), ui::primary(Some(Icon::Plus), "Novo cargo", Some(Msg::RoleForm(None)))]);
    }
    card(440.0, corpo.into())
}

fn member_roles<'a>(m: &'a Main, lid: &'a str, pk: &'a str) -> El<'a> {
    let Some(g) = m.group(lid) else { return Space::new().into() };
    let tem: Vec<String> = g.meta.mroles.get(pk).cloned().unwrap_or_default();
    let mut col = column![].spacing(4);
    if g.meta.roles.is_empty() {
        col = col.push(ui::note("O grupo ainda não tem cargos."));
        col = col.push(row![ui::fill(), ui::primary(Some(Icon::Plus), "Criar cargo", Some(Msg::OpenRoles(lid.to_string())))]);
    }
    for r in &g.meta.roles {
        let on = tem.contains(&r.id);
        col = col.push(
            row![
                container(Space::new()).width(12).height(12).style(t::rounded(cor_cargo(r.c), 6.0)),
                text(r.n.clone()).size(14.5).color(t::FG).width(Length::Fill),
                toggler(on).on_toggle(move |v| Msg::RoleToggle(lid.to_string(), pk.to_string(), r.id.clone(), v)).size(20)
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .padding([6, 4]),
        );
    }
    card(400.0, column![head(&format!("Cargos de {}", m.name_of(pk)), Some("Até 4 cargos por pessoa.")), limited(m, col.into())].spacing(12).into())
}

fn tile_button<'a>(i: Icon, label: &str, msg: Msg) -> El<'a> {
    button(
        column![container(icon(i, 26.0, t::accent())).center(52).style(t::rounded(t::with_alpha(t::accent(), 0.14), 999.0)), text(label.to_string()).size(14).font(SEMI).color(t::FG)]
            .spacing(10)
            .align_x(Alignment::Center),
    )
    .padding(16)
    .width(Length::Fill)
    .style(t::option(false))
    .on_press(msg)
    .into()
}

fn emoji_grid<'a>(atual: &str, on: fn(String) -> Msg) -> El<'a> {
    column(ui::SOUND_EMOJIS
        .chunks(12)
        .map(|c| {
            row(c.iter()
                .map(|e| {
                    let sel = *e == atual;
                    button(container(text(*e).size(19)).center(26)).padding(2).style(t::icon_btn(sel)).on_press(on(e.to_string())).into()
                })
                .collect::<Vec<El<'a>>>())
            .spacing(2)
            .into()
        })
        .collect::<Vec<El<'a>>>())
    .spacing(2)
    .into()
}

/* ------------------------------------------------------------ contatos */

fn add_contact<'a>(code: &'a str, nick: &'a str, error: Option<&'a str>, busy: bool) -> El<'a> {
    let mut c = column![
        head("Adicionar contato", Some("Digite o código curto (ex.: K7QF-9X2M) ou cole o código completo (npub1…) ou o link de convite.")),
        row![ui::input("K7QF-9X2M ou npub1…", code, Msg::AddCode).on_submit(Msg::AddConfirm).width(Length::Fill), ui::secondary(Some(Icon::Copy), "Colar", Some(Msg::AddPaste))].spacing(8),
        ui::field("Apelido (opcional)", ui::input("Como você quer ver o nome", nick, Msg::AddNick).on_submit(Msg::AddConfirm)),
    ]
    .spacing(12);
    if let Some(e) = error {
        c = c.push(row![icon(Icon::Alert, 15.0, t::DANGER), text(e.to_string()).size(13).color(t::DANGER)].spacing(6).align_y(Alignment::Center));
    }
    c = c.push(ui::note("No computador não há leitor de QR: peça o código ou o convite, ou mostre o seu QR para a pessoa ler com o celular."));
    c = c.push(
        row![
            ui::secondary(Some(Icon::Qr), "Meu código", Some(Msg::OpenMyCode)),
            ui::fill(),
            ui::primary(Some(Icon::UserPlus), if busy { "Adicionando…" } else { "Adicionar" }, (!busy && !code.trim().is_empty()).then_some(Msg::AddConfirm)),
        ]
        .spacing(8),
    );
    card(460.0, c.into())
}

fn new_chat(m: &Main) -> El<'_> {
    let mut lista = column![
        ui::setting(Some(Icon::UserPlus), "Adicionar contato", Some("Por código ou link".into()), Trail::Chevron, Some(Msg::OpenAddContact(String::new())), false),
        ui::setting(Some(Icon::Qr), "Meu código", Some("Mostre para alguém te adicionar".into()), Trail::Chevron, Some(Msg::OpenMyCode), false),
        ui::setting(Some(Icon::Users), "Criar grupo", None, Trail::Chevron, Some(Msg::OpenNewGroup), false),
    ]
    .spacing(2);
    let amigos = m.friends();
    if !amigos.is_empty() {
        lista = lista.push(ui::label("Contatos"));
    }
    for c in amigos {
        let nome = c.display_name();
        let sub = if c.note.is_empty() { None } else { Some(c.note.clone()) };
        lista = lista.push(
            button(
                row![
                    ui::avatar(&nome, c.tone, m.pic_of(&c.pk), 38.0),
                    column![text(nome).size(14.5).font(SEMI).color(t::FG), text(sub.unwrap_or_default()).size(12.5).color(t::FG_DIM)].spacing(1)
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .width(Length::Fill)
            .style(t::row_item(false))
            .on_press(Msg::OpenChat(format!("dm:{}", c.pk))),
        );
    }
    card(440.0, column![head("Nova conversa", None), limited(m, lista.into())].spacing(10).into())
}

use crate::ui::Trail;

/// Verde do Spotify, para o cartão "Ouvindo".
pub const SPOTIFY: Color = Color::from_rgb(0x1D as f32 / 255.0, 0xB9 as f32 / 255.0, 0x54 as f32 / 255.0);

/// Cartão "Ouvindo Spotify", como no perfil do Discord.
pub fn listening_card<'a>(l: &jm_core::messages::Listening) -> El<'a> {
    let agora = jm_core::engine::now_ms();
    let passou = ((agora - l.st).max(0) / 1000) as f32;
    let mut info = column![
        text(l.s.clone()).size(15).font(BOLD).color(t::FG),
    ]
    .spacing(2)
    .width(Length::Fill);
    if !l.a.is_empty() {
        info = info.push(text(format!("de {}", l.a)).size(13).color(t::FG_DIM));
    }
    if !l.al.is_empty() {
        info = info.push(text(format!("em {}", l.al)).size(12.5).color(t::FG_FAINT));
    }
    let capa = container(icon(Icon::Music, 30.0, Color::WHITE)).center(64).style(t::rounded(SPOTIFY, 10.0));
    let mut col = column![
        row![icon(Icon::Music, 13.0, SPOTIFY), text("OUVINDO SPOTIFY").size(11.5).font(BOLD).color(SPOTIFY)].spacing(6).align_y(Alignment::Center),
        row![capa, info].spacing(12).align_y(Alignment::Center),
    ]
    .spacing(10);
    match l.d {
        Some(d) if l.st > 0 => {
            let pos = passou.min(d as f32);
            col = col.push(column![
                iced::widget::progress_bar(0.0..=d as f32, pos).girth(4).style(|_| iced::widget::progress_bar::Style {
                    background: iced::Background::Color(t::with_alpha(Color::WHITE, 0.12)),
                    bar: iced::Background::Color(t::FG),
                    border: iced::Border { radius: 2.0.into(), ..Default::default() },
                }),
                row![text(t::mmss(pos)).size(11).color(t::FG_DIM), ui::fill(), text(t::mmss(d as f32)).size(11).color(t::FG_DIM)],
            ]
            .spacing(4));
        }
        _ if l.st > 0 => col = col.push(text(format!("Tocando há {}", t::mmss(passou))).size(11.5).color(t::FG_DIM)),
        _ => {}
    }
    col = col.push(
        button(container(row![icon(Icon::Music, 15.0, Color::WHITE), text("Ouvir no Spotify").size(13.5).font(SEMI).color(Color::WHITE)].spacing(8).align_y(Alignment::Center)).center_x(Length::Fill))
            .padding([8, 12])
            .width(Length::Fill)
            .style(|_, st| iced::widget::button::Style {
                background: Some(iced::Background::Color(if st == iced::widget::button::Status::Hovered { t::mix(SPOTIFY, Color::WHITE, 0.12) } else { SPOTIFY })),
                text_color: Color::WHITE,
                border: iced::Border { radius: 8.0.into(), ..Default::default() },
                ..Default::default()
            })
            .on_press(Msg::OpenUrl(l.url())),
    );
    container(col).padding(14).width(Length::Fill).style(t::rounded(t::with_alpha(Color::BLACK, 0.22), 12.0)).into()
}

fn profile<'a>(m: &'a Main, pk: &'a str, nick: &'a str) -> El<'a> {
    let c = m.contacts.get(pk);
    let nome = m.name_of(pk);
    let original = c.map(|c| c.name.clone()).unwrap_or_default();
    let status = c.map(|c| c.status);
    let mut col = column![
        row![ui::fill(), ui::icon_button_sm(Icon::X, "Fechar", Some(Msg::Key(Key::Escape)))],
        // Foto por cima da borda de baixo do banner, como no Discord.
        stack![
            column![crate::v_settings::banner_view(m, c.and_then(|c| c.banner.as_deref()), m.tone_of(pk), 400.0), ui::vspace(52.0)],
            container(
                container(ui::avatar(&nome, m.tone_of(pk), m.pic_of(pk), 96.0))
                    .padding(5)
                    .style(t::rounded(t::surface(), 999.0))
            )
            .align_bottom(Length::Fill)
            .center_x(Length::Fill),
        ]
        .width(Length::Fill),
        container(text(nome.clone()).size(21).font(BOLD).color(t::FG)).center_x(Length::Fill),
    ]
    .spacing(8);
    if !original.is_empty() && original != nome {
        col = col.push(container(text(format!("Nome escolhido: {original}")).size(12.5).color(t::FG_DIM)).center_x(Length::Fill));
    }
    if let Some(n) = c.map(|c| c.note.clone()).filter(|n| !n.is_empty()) {
        col = col.push(container(text(n).size(13.5).color(t::FG_DIM)).center_x(Length::Fill));
    }
    if let Some(l) = m.listening(pk) {
        col = col.push(listening_card(&l));
    }
    if let Some(jogos) = c.map(|c| &c.games).filter(|g| !g.is_empty()) {
        col = col.push(ui::label("Jogos favoritos"));
        col = col.push(crate::v_settings::games_chips(jogos, false));
    }
    let pode = status == Some(ContactStatus::Ok);
    col = col.push(
        container(
            row![
                ui::secondary(Some(Icon::Chat), "Conversar", Some(Msg::OpenChat(format!("dm:{pk}")))),
                ui::secondary(Some(Icon::Phone), "Ligar", pode.then(|| Msg::Call(pk.to_string(), false))),
            ]
            .spacing(8),
        )
        .center_x(Length::Fill),
    );
    col = col.push(ui::label("Apelido"));
    col = col.push(row![ui::input("Só você vê o apelido", nick, Msg::ProfileNick).on_submit(Msg::ProfileSaveNick).width(Length::Fill), ui::secondary(None, "Salvar", Some(Msg::ProfileSaveNick))].spacing(8));

    if let Some(num) = m.engine.safety_number(pk) {
        let verificado = c.is_some_and(|c| c.verified);
        let grade = column(num
            .chunks(4)
            .map(|l| row(l.iter().map(|g| text(g.clone()).size(17).font(iced::Font::MONOSPACE).color(t::FG).width(Length::Fill).into()).collect::<Vec<El<'a>>>()).spacing(8).into())
            .collect::<Vec<El<'a>>>())
        .spacing(6);
        col = col.push(ui::label("Número de segurança"));
        col = col.push(container(grade).padding(14).style(t::rounded(t::surface_hi(), 12.0)));
        col = col.push(ui::note("Compare estes números com os do celular da outra pessoa, pessoalmente ou por outro meio. Se forem iguais, ninguém está no meio da conversa."));
        col = col.push(
            container(
                row![
                    icon(Icon::Shield, 18.0, if verificado { t::accent() } else { t::FG_DIM }),
                    text("Marcar como verificado").size(14).font(SEMI).color(t::FG).width(Length::Fill),
                    toggler(verificado).on_toggle(Msg::ProfileVerify).size(20)
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .style(t::rounded(t::surface_hi(), 12.0)),
        );
    }
    col = col.push(ui::label("Código"));
    col = col.push(text(jm_core::identity::npub_hex(pk)).size(12).wrapping(iced::widget::text::Wrapping::Glyph).color(t::FG_DIM));
    let mut acoes = row![].spacing(8);
    acoes = acoes.push(match status {
        Some(ContactStatus::Blocked) => ui::secondary(None, "Desbloquear", Some(Msg::Unblock(pk.to_string()))),
        _ => ui::danger_text(Some(Icon::Ban), "Bloquear", Some(Msg::Confirm(ConfirmKind::Block(pk.to_string())))),
    });
    if m.summary(&format!("dm:{pk}")).is_some() {
        acoes = acoes.push(ui::danger_text(Some(Icon::Trash), "Apagar conversa", Some(Msg::Confirm(ConfirmKind::DeleteChat(format!("dm:{pk}"))))));
    }
    col = col.push(ui::vspace(4.0));
    col = col.push(acoes);
    card(460.0, limited(m, col.into()))
}

/* ------------------------------------------------------------ grupos */

fn new_group<'a>(name: &'a str, template: Template, busy: bool) -> El<'a> {
    let modelos = column(Template::all()
        .into_iter()
        .map(|tpl| {
            let sel = tpl == template;
            let canais: Vec<String> = tpl.channels().iter().map(|(n, k)| if *k == ChannelKind::Text { format!("#{n}") } else { format!("🔊 {n}") }).collect();
            button(
                column![text(tpl.label()).size(14.5).font(SEMI).color(t::FG), text(canais.join(" · ")).size(12).color(t::FG_DIM)]
                    .spacing(3)
                    .width(Length::Fill),
            )
            .padding([10, 12])
            .width(Length::Fill)
            .style(t::option(sel))
            .on_press(Msg::NewGroupTemplate(tpl))
            .into()
        })
        .collect::<Vec<El<'a>>>())
    .spacing(8);
    let pode = name.trim().chars().count() >= 2 && !busy;
    card(
        460.0,
        column![
            head("Criar grupo", Some("Grupos têm canais de texto e salas de voz, como um servidor. Só quem você convidar entra.")),
            ui::field("Nome do grupo", ui::input("Nome", name, Msg::NewGroupName).on_submit(Msg::NewGroupCreate)),
            text("Modelo").size(12.5).font(SEMI).color(t::FG_DIM),
            modelos,
            row![ui::fill(), ui::ghost(None, "Cancelar", Some(Msg::Key(Key::Escape))), ui::primary(Some(Icon::Users), if busy { "Criando…" } else { "Criar" }, pode.then_some(Msg::NewGroupCreate))].spacing(8),
        ]
        .spacing(12)
        .into(),
    )
}

fn invite<'a>(m: &'a Main, lid: &'a str) -> El<'a> {
    let g = m.group(lid);
    let membros: Vec<String> = g.map(|g| g.meta.members.clone()).unwrap_or_default();
    let mut lista = column![].spacing(2);
    let amigos = m.friends();
    if amigos.is_empty() {
        lista = lista.push(ui::note("Você ainda não tem contatos. Adicione alguém pelo código primeiro."));
    }
    for c in amigos {
        let nome = c.display_name();
        let ja = membros.iter().any(|p| p == &c.pk);
        lista = lista.push(
            container(
                row![
                    ui::avatar(&nome, c.tone, m.pic_of(&c.pk), 36.0),
                    text(nome).size(14.5).font(SEMI).color(t::FG).width(Length::Fill),
                    if ja {
                        text("No grupo").size(12.5).color(t::FG_FAINT).into()
                    } else {
                        El::from(ui::secondary(Some(Icon::UserPlus), "Convidar", Some(Msg::InviteSend(lid.to_string(), c.pk.clone()))))
                    }
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding([6, 10]),
        );
    }
    card(
        440.0,
        column![
            head(&format!("Convidar para {}", g.map(|g| g.meta.name.clone()).unwrap_or_default()), Some("O convite vai cifrado, numa conversa com a pessoa.")),
            limited(m, lista.into())
        ]
        .spacing(10)
        .into(),
    )
}

fn group_info<'a>(m: &'a Main, f: &'a GroupForm) -> El<'a> {
    let Some(g) = m.group(&f.lid) else { return card(300.0, ui::note("Grupo não encontrado")) };
    let admin = m.is_admin(&f.lid);
    let pic = m.pic(g.pic.as_deref());
    let tem_foto = pic.is_some();
    let mut col = column![head("Dados do grupo", None)].spacing(10);
    let mut foto = row![ui::group_avatar(&g.meta.name, g.meta.t, pic, 72.0)].spacing(14).align_y(Alignment::Center);
    if admin {
        let mut b = column![ui::secondary(Some(Icon::Camera), if tem_foto { "Mudar foto" } else { "Colocar foto" }, Some(Msg::GroupPhoto(f.lid.clone())))].spacing(6);
        if tem_foto {
            b = b.push(ui::danger_text(Some(Icon::Trash), "Remover foto", Some(Msg::GroupPhotoRemove(f.lid.clone()))));
        }
        foto = foto.push(b);
    } else {
        foto = foto.push(column![text(g.meta.name.clone()).size(17).font(BOLD).color(t::FG), text(g.meta.desc.clone()).size(13).color(t::FG_DIM)].spacing(4));
    }
    col = col.push(foto);
    if admin {
        let mudou = f.name.trim() != g.meta.name || f.desc.trim() != g.meta.desc;
        col = col.push(ui::field("Nome", ui::input("Nome do grupo", &f.name, Msg::GroupName)));
        col = col.push(ui::field("Descrição", ui::input("Sobre o grupo", &f.desc, Msg::GroupDesc)));
        col = col.push(row![ui::fill(), ui::primary(Some(Icon::Check), "Salvar", mudou.then_some(Msg::GroupSave))]);

        col = col.push(ui::label("Canais"));
        let mut canais = column![].spacing(2);
        for ch in &g.meta.channels {
            let mut r = row![
                icon(crate::v_main::kind_icon(ch.kind), 16.0, t::FG_DIM),
                {
                    let mut c = column![text(ch.name.clone()).size(14).font(SEMI).color(t::FG)].spacing(1).width(Length::Fill);
                    if let Some(topico) = ch.topic.clone().filter(|x| !x.is_empty()) {
                        c = c.push(text(topico).size(12).color(t::FG_DIM));
                    }
                    c
                },
                ui::icon_button_sm(Icon::Up, "Subir", Some(Msg::ChannelMove(ch.id.clone(), -1))),
                ui::icon_button_sm(Icon::Down, "Descer", Some(Msg::ChannelMove(ch.id.clone(), 1))),
                ui::icon_button_sm(Icon::Edit, "Editar", Some(Msg::ChannelEdit(ch.id.clone()))),
            ]
            .spacing(4)
            .align_y(Alignment::Center);
            let ultimo_texto = ch.kind == ChannelKind::Text && g.meta.text_channels().count() <= 1;
            if !ultimo_texto {
                r = r.push(ui::icon_button_sm(Icon::Trash, "Apagar", Some(Msg::Confirm(ConfirmKind::RemoveChannel(f.lid.clone(), ch.id.clone())))));
            }
            canais = canais.push(container(r).padding([4, 8]).style(t::rounded(t::surface_hi(), 10.0)));
        }
        col = col.push(canais);
        col = col.push(
            row![
                ui::input("novo-canal", &f.ch_name, Msg::ChannelDraftName).on_submit(Msg::ChannelAdd).width(Length::Fill),
                button(row![icon(Icon::Hash, 15.0, t::FG), text("Texto").size(13)].spacing(4).align_y(Alignment::Center))
                    .padding([9, 10])
                    .style(t::option(f.ch_kind == ChannelKind::Text))
                    .on_press(Msg::ChannelDraftKind(ChannelKind::Text)),
                button(row![icon(Icon::Volume, 15.0, t::FG), text("Voz").size(13)].spacing(4).align_y(Alignment::Center))
                    .padding([9, 10])
                    .style(t::option(f.ch_kind == ChannelKind::Voice))
                    .on_press(Msg::ChannelDraftKind(ChannelKind::Voice)),
                ui::primary(Some(Icon::Plus), "Criar", (!f.ch_name.trim().is_empty()).then_some(Msg::ChannelAdd)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
        col = col.push(ui::note("As mudanças valem para todos do grupo. Apagar um canal de texto apaga as mensagens dele em todos os aparelhos. O grupo precisa de pelo menos um canal de texto."));
    }
    col = col.push(ui::label(&format!("Membros · {}", g.meta.members.len())));
    col = col.push(row![
        ui::secondary(Some(Icon::UserPlus), "Convidar pessoas", Some(Msg::OpenInvite(f.lid.clone()))),
        ui::fill(),
        row![text("Silenciar").size(13.5).color(t::FG_DIM), toggler(g.muted).on_toggle({
            let lid = f.lid.clone();
            move |v| Msg::MuteGroup(lid.clone(), v)
        })
        .size(20)]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .align_y(Alignment::Center));
    col = col.push(ui::vspace(6.0));
    col = col.push(row![ui::danger_text(Some(Icon::Logout), "Sair do grupo", Some(Msg::Confirm(ConfirmKind::LeaveGroup(f.lid.clone()))))]);
    card(520.0, limited(m, col.into()))
}

/* ------------------------------------------------------------ escolhas */

fn choose<'a>(m: &'a Main, k: &'a ChooseKind) -> El<'a> {
    let atual: u32 = match k {
        ChooseKind::TtlDefault => m.settings.ttl_default,
        ChooseKind::Ttl(key) => m.summary(key).map(|s| s.ttl).unwrap_or_else(|| m.engine.chat_ttl(key)),
        ChooseKind::NotifPreview => match m.settings.notif_preview {
            NotifPreview::Full => 0,
            NotifPreview::Name => 1,
            NotifPreview::None => 2,
        },
        ChooseKind::CallsFrom => u32::from(m.settings.calls_from == jm_core::engine::CallsFrom::Nobody),
        ChooseKind::PinDelay => m.pin.as_ref().map(|p| p.delay).unwrap_or(60),
    };
    let sub = k.sub();
    let opcoes = column(k
        .options()
        .into_iter()
        .map(|o| {
            let sel = o.value == atual;
            let mut c = column![text(o.label).size(14.5).font(SEMI).color(t::FG)].spacing(2).width(Length::Fill);
            if let Some(s) = o.sub {
                c = c.push(text(s).size(12.5).color(t::FG_DIM));
            }
            button(
                row![c, container(if sel { icon(Icon::Check, 15.0, t::accent_ink()) } else { Space::new().into() }).center(22).style(t::rounded(if sel { t::accent() } else { t::surface_hi() }, 999.0))]
                    .spacing(10)
                    .align_y(Alignment::Center),
            )
            .padding([10, 12])
            .width(Length::Fill)
            .style(t::option(sel))
            .on_press(Msg::Chosen(k.clone(), o.value))
            .into()
        })
        .collect::<Vec<El<'a>>>())
    .spacing(8);
    card(400.0, column![head(k.title(), (!sub.is_empty()).then_some(sub)), opcoes].spacing(12).into())
}

fn confirm<'a>(m: &'a Main, k: &'a ConfirmKind) -> El<'a> {
    let (titulo, texto, ok): (String, String, &str) = match k {
        ConfirmKind::DeleteChat(key) => (format!("Apagar conversa com {}?", m.chat_title(key)), "As mensagens somem deste computador.".into(), "Apagar"),
        ConfirmKind::ClearMedia => ("Apagar fotos e áudios?".into(), "Os textos continuam. A mídia não volta.".into(), "Apagar"),
        ConfirmKind::ClearCalls => ("Limpar histórico de chamadas?".into(), "A lista de chamadas some deste computador.".into(), "Limpar"),
        ConfirmKind::Wipe => (
            "Sair e apagar tudo?".into(),
            "Remove a conta e as conversas deste computador. Para entrar de novo, você vai precisar das 12 palavras.".into(),
            "Apagar tudo",
        ),
        ConfirmKind::DeleteSound(id) => (
            format!("Excluir \"{}\"?", m.sounds.info(id).map(|s| s.name).unwrap_or_default()),
            "Este som foi criado por você e não pode ser recuperado.".into(),
            "Excluir",
        ),
        ConfirmKind::LeaveGroup(lid) => (
            format!("Sair de {}?", m.group(lid).map(|g| g.meta.name.clone()).unwrap_or_default()),
            "Você deixa de receber as mensagens do grupo. Para voltar, alguém precisa te convidar de novo.".into(),
            "Sair",
        ),
        ConfirmKind::RemoveMember(_, pk) => (
            format!("Remover {}?", m.name_of(pk)),
            "A chave do grupo é trocada: a pessoa não lê mais nada do que for dito depois.".into(),
            "Remover",
        ),
        ConfirmKind::RemoveChannel(lid, id) => {
            let ch = m.group(lid).and_then(|g| g.meta.channels.iter().find(|c| &c.id == id));
            let voz = ch.is_some_and(|c| c.kind == ChannelKind::Voice);
            (
                format!("Apagar {}{}?", if voz { "" } else { "#" }, ch.map(|c| c.name.clone()).unwrap_or_default()),
                if voz { "Quem estiver na sala de voz sai dela.".into() } else { "As mensagens deste canal somem para todos do grupo.".into() },
                "Apagar",
            )
        }
        ConfirmKind::Block(pk) => (
            format!("Bloquear {}?", m.name_of(pk)),
            "Nada do que essa pessoa mandar vai chegar, nem chamadas. Ela não fica sabendo.".into(),
            "Bloquear",
        ),
    };
    let mut col = column![head(&titulo, Some(&texto))].spacing(12);
    if *k == ConfirmKind::Wipe && m.words_pending {
        col = col.push(
            container(
                column![
                    row![icon(Icon::Alert, 16.0, t::WARN), text("Você ainda não anotou sua chave de recuperação!").size(13.5).font(SEMI).color(t::WARN)]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    ui::secondary(Some(Icon::Key), "Ver as 12 palavras", Some(Msg::OpenRecovery)),
                ]
                .spacing(8),
            )
            .padding(12)
            .style(t::chip(t::WARN)),
        );
    }
    col = col.push(row![ui::fill(), ui::ghost(None, "Cancelar", Some(Msg::Key(Key::Escape))), ui::danger(None, ok, Some(Msg::ConfirmYes))].spacing(8));
    card(420.0, col.into())
}

/* ------------------------------------------------------------ PIN */

fn dots<'a>(n: usize, erro: bool) -> El<'a> {
    row((0..4)
        .map(|i| {
            let on = i < n;
            let cor = if erro {
                t::DANGER
            } else if on {
                t::accent()
            } else {
                t::surface_hi()
            };
            container(Space::new()).width(14).height(14).style(t::rounded(cor, 7.0)).into()
        })
        .collect::<Vec<El<'a>>>())
    .spacing(14)
    .into()
}

fn keypad<'a>(on_key: fn(char) -> Msg, back: Msg) -> El<'a> {
    let tecla = |c: char| -> El<'a> {
        button(container(text(c.to_string()).size(22).font(SEMI).color(t::FG)).center(56))
            .padding(0)
            .style(|th, st| {
                let mut s = t::secondary(th, st);
                s.border.radius = 999.0.into();
                s
            })
            .on_press(on_key(c))
            .into()
    };
    let linha = |a: char, b: char, c: char| -> El<'a> { row![tecla(a), tecla(b), tecla(c)].spacing(16).into() };
    column![
        linha('1', '2', '3'),
        linha('4', '5', '6'),
        linha('7', '8', '9'),
        row![
            Space::new().width(56).height(56),
            tecla('0'),
            button(container(icon(Icon::Back, 22.0, t::FG_DIM)).center(56)).padding(0).style(t::ghost).on_press(back)
        ]
        .spacing(16),
    ]
    .spacing(14)
    .align_x(Alignment::Center)
    .into()
}

fn pin_pad(p: &PinPad) -> El<'_> {
    let mut col = column![
        row![ui::fill(), ui::icon_button_sm(Icon::X, "Fechar", Some(Msg::Key(Key::Escape)))],
        logo(52.0),
        text(p.title()).size(20).font(BOLD).color(t::FG),
        text(p.msg.clone()).size(13.5).color(if p.error { t::DANGER } else { t::FG_DIM }).align_x(Alignment::Center),
        ui::vspace(6.0),
        dots(p.buf.len(), p.error),
        ui::vspace(10.0),
        keypad(Msg::PinKey, Msg::PinBack),
    ]
    .spacing(8)
    .align_x(Alignment::Center);
    if p.purpose != PinPurpose::Create {
        col = col.push(ui::ghost(None, "Esqueci o PIN", Some(Msg::PinForgot)));
    }
    col = col.push(ui::note("Você também pode digitar os números no teclado."));
    card(360.0, col.into())
}

fn forgot<'a>(input: &'a str, error: Option<&'a str>, tela_cheia: bool) -> El<'a> {
    let mut col = column![
        text("Esqueci o PIN").size(20).font(BOLD).color(t::FG),
        text("Digite as 12 palavras de recuperação desta conta. Se estiverem certas, o PIN é removido.").size(13.5).color(t::FG_DIM),
        ui::input("palavra1 palavra2 …", input, Msg::ForgotInput).on_submit(Msg::ForgotConfirm),
    ]
    .spacing(12);
    if let Some(e) = error {
        col = col.push(text(e.to_string()).size(13).color(t::DANGER));
    }
    col = col.push(
        row![
            ui::fill(),
            ui::ghost(None, "Voltar", Some(if tela_cheia { Msg::LockForgot } else { Msg::Key(Key::Escape) })),
            ui::primary(Some(Icon::Key), "Tirar o PIN", Some(Msg::ForgotConfirm))
        ]
        .spacing(8),
    );
    card(440.0, col.into())
}

/// Tela de bloqueio (ao abrir o app ou ao voltar depois do tempo).
pub fn lock_screen<'a>(_m: &'a Main, l: &'a LockState) -> El<'a> {
    let corpo: El<'a> = if let Some((input, erro)) = &l.forgot {
        forgot(input, erro.as_deref(), true)
    } else {
        column![
            logo(64.0),
            text("J-Menseger bloqueado").size(22).font(BOLD).color(t::FG),
            text(l.msg.clone()).size(14).color(if l.error || l.wait_until.is_some() { t::DANGER } else { t::FG_DIM }),
            ui::vspace(8.0),
            dots(l.buf.len(), l.error),
            ui::vspace(12.0),
            keypad(Msg::LockKey, Msg::LockBack),
            ui::vspace(6.0),
            ui::ghost(None, "Esqueci o PIN", Some(Msg::LockForgot)),
        ]
        .spacing(8)
        .align_x(Alignment::Center)
        .into()
    };
    container(corpo).center(Length::Fill).style(t::fill(t::rail())).into()
}

/* ------------------------------------------------------------ fotos */

fn viewer<'a>(m: &'a Main, mid: &'a str) -> El<'a> {
    let img: El<'a> = match m.anim_frame(mid).or_else(|| m.media_image(mid)) {
        Some(h) => image(h).content_fit(ContentFit::Contain).width(Length::Fill).height(Length::Fill).into(),
        None => ui::note("A foto não está mais neste computador."),
    };
    container(
        column![
            row![ui::fill(), ui::secondary(Some(Icon::Download), "Salvar", Some(Msg::SavePhoto(mid.to_string()))), ui::icon_button(Icon::X, "Fechar", Some(Msg::Key(Key::Escape)), false)]
                .spacing(8)
                .align_y(Alignment::Center),
            container(img).width(Length::Fill).height(Length::Fill),
        ]
        .spacing(10),
    )
    .width(Length::Fixed((m.size.width - 120.0).max(320.0)))
    .height(Length::Fixed((m.size.height - 100.0).max(260.0)))
    .padding(12)
    .style(t::rounded(t::with_alpha(Color::BLACK, 0.85), 14.0))
    .into()
}

fn crop<'a>(m: &'a Main, c: &'a CropState) -> El<'a> {
    let lado = (m.size.height - 300.0).clamp(240.0, 460.0);
    let desenho = canvas(crate::canvas::Crop { w: c.w as f32, h: c.h as f32, x: c.x, y: c.y, side: c.side, ratio: c.target.ratio() })
        .width(lado)
        .height(lado);
    let foto = image(c.handle.clone()).content_fit(ContentFit::Contain).width(lado).height(lado);
    let titulo = match c.target {
        CropTarget::Me => "Foto do perfil",
        CropTarget::Group(_) => "Foto do grupo",
        CropTarget::Banner => "Banner do perfil",
    };
    card(
        lado + 48.0,
        column![
            head(
                titulo,
                Some(if c.target == CropTarget::Banner {
                    "Arraste o retângulo para escolher a faixa. A roda do mouse (ou o canto) muda o tamanho."
                } else {
                    "Arraste o quadrado para escolher. A roda do mouse (ou o canto) muda o tamanho."
                }),
            ),
            container(stack![foto, desenho]).style(t::rounded(Color::BLACK, 10.0)),
            row![
                ui::fill(),
                ui::ghost(None, "Cancelar", (!c.busy).then_some(Msg::Key(Key::Escape))),
                ui::primary(Some(Icon::Check), if c.busy { "Salvando…" } else { "Usar foto" }, (!c.busy).then_some(Msg::CropSave))
            ]
            .spacing(8),
        ]
        .spacing(12)
        .into(),
    )
}

/* ------------------------------------------------------------ sons */

fn record_sound<'a>(rec: Option<&'a jm_media::recorder::Recorder>, levels: &'a [f32]) -> El<'a> {
    let gravando = rec.is_some();
    let tempo = rec.map(|r| r.elapsed()).unwrap_or(0.0);
    card(
        420.0,
        column![
            head("Gravar som", Some(if gravando { "Gravando… clique para parar." } else { "Clique no botão para começar. Grava até 10 segundos." })),
            container(ui::level_bars(levels, 56, if gravando { t::DANGER } else { t::FG_FAINT })).center_x(Length::Fill),
            container(text(format!("{:.1} s", tempo).replace('.', ",")).size(15).font(SEMI).color(t::FG)).center_x(Length::Fill),
            container(
                button(container(icon(if gravando { Icon::Stop } else { Icon::Mic }, 28.0, t::FG)).center(28))
                    .padding(20)
                    .style(t::call_btn(if gravando { t::CallBtn::Danger } else { t::CallBtn::Normal }))
                    .on_press(if gravando { Msg::SbRecStop } else { Msg::SbRecStart })
            )
            .center_x(Length::Fill),
        ]
        .spacing(14)
        .into(),
    )
}

fn trim<'a>(m: &'a Main, st: &'a TrimState) -> El<'a> {
    let tocando = m.track.as_ref().filter(|t| t.0 == "trim" && !t.3 && t.1 + 0.02 < t.2);
    let playhead = tocando.map(|t| st.a + t.1 as f64);
    let largura = 500.0;
    let mut col = column![head("Cortar som", Some("Arraste as bordas para escolher o trecho (até 8 s)."))].spacing(10);
    if !st.overview.is_empty() {
        col = col.push(canvas(crate::canvas::Overview { peaks: st.overview.clone(), dur: st.dur, v0: st.v0, vlen: st.vlen() }).width(Length::Fill).height(34));
    }
    col = col.push(
        canvas(crate::canvas::Trim { peaks: st.peaks.clone(), a: st.a, b: st.b, v0: st.v0, vlen: st.vlen(), playhead })
            .width(Length::Fill)
            .height(96),
    );
    let fmt = |x: f64| format!("{:.1} s", x).replace('.', ",");
    col = col.push(
        row![
            text(format!("{} a {} · {}", fmt(st.a), fmt(st.b), fmt(st.b - st.a))).size(13).color(t::FG_DIM).width(Length::Fill),
            button(container(icon(if tocando.is_some() { Icon::Pause } else { Icon::Play }, 16.0, t::accent_ink())).center(16))
                .padding(10)
                .style(t::call_btn(t::CallBtn::Accept))
                .on_press(Msg::TrimPlay),
        ]
        .align_y(Alignment::Center),
    );
    col = col.push(ui::field("Nome", ui::input("Meu som", &st.name, Msg::TrimName)));
    col = col.push(text("Emoji").size(12.5).font(SEMI).color(t::FG_DIM));
    col = col.push(emoji_grid(&st.emoji, Msg::TrimEmoji));
    col = col.push(
        row![
            ui::fill(),
            ui::ghost(None, "Cancelar", (!st.busy).then_some(Msg::Key(Key::Escape))),
            ui::primary(Some(Icon::Check), if st.busy { "Salvando…" } else { "Salvar som" }, (!st.busy).then_some(Msg::TrimSave))
        ]
        .spacing(8),
    );
    card(largura + 48.0, limited(m, col.into()))
}

/* ------------------------------------------------------------ tela */

fn share<'a>(m: &'a Main, sources: &'a [jm_media::screen::Source]) -> El<'a> {
    let qualidades = row([Quality::P720, Quality::P1080, Quality::P1440]
        .into_iter()
        .map(|q| button(text(q.label()).size(13.5)).padding([7, 12]).style(t::option(m.share_quality == q)).on_press(Msg::ShareQuality(q)).into())
        .collect::<Vec<El<'a>>>())
    .spacing(8);
    let mut lista = column![].spacing(6);
    if sources.is_empty() {
        lista = lista.push(ui::note("Nenhuma tela ou janela encontrada para transmitir."));
    }
    for s in sources.iter().take(16) {
        let i = if s.kind == SourceKind::Monitor { Icon::Screen } else { Icon::Image };
        lista = lista.push(
            button(row![icon(i, 18.0, t::FG_DIM), text(ui::ellipsis(&s.label(), 60)).size(14).color(t::FG)].spacing(10).align_y(Alignment::Center))
                .padding([10, 12])
                .width(Length::Fill)
                .style(t::option(false))
                .on_press(Msg::ShareSource(s.id)),
        );
    }
    card(
        480.0,
        column![
            head("Transmitir a tela", Some("Quem está na chamada vê o que você escolher. Funciona com o celular e com outro computador.")),
            row![text("Qualidade").size(13).color(t::FG_DIM), qualidades].spacing(12).align_y(Alignment::Center),
            if jm_media::sysaudio::supported() {
                El::from(
                    row![
                        column![
                            text("Mandar o áudio do PC").size(14).color(t::FG),
                            text("Som do jogo ou do vídeo. O som do J-Menseger não vai junto (ninguém ouve a própria voz de volta).").size(12).color(t::FG_DIM)
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        toggler(m.share_audio).on_toggle(Msg::ShareAudio).size(20)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
            } else {
                text("Áudio do PC na transmissão: por enquanto só no Windows 10 (2004) ou mais novo.").size(12).color(t::FG_DIM).into()
            },
            limited(m, lista.into()),
        ]
        .spacing(12)
        .into(),
    )
}
