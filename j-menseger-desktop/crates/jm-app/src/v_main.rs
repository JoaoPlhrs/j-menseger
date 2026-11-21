//! A janela principal: barra de grupos, lista lateral, conteúdo, menus e
//! avisos por cima.

use crate::app::{Key, Msg};
use crate::calls::CallState;
use crate::icons::{icon, logo, Icon};
use crate::state::*;
use crate::theme as t;
use crate::ui::{self, El, BOLD, SEMI};
use iced::widget::{button, column, container, mouse_area, opaque, pin, row, scrollable, stack, text, tooltip, Space};
use iced::{Alignment, Color, Length, Padding, Point};
use jm_core::engine::{ChatSummary, ContactStatus, MessageBody};
use jm_core::groups::ChannelKind;
use jm_core::messages::Delivery;

pub const RAIL_W: f32 = 72.0;
pub const SIDEBAR_W: f32 = 272.0;

impl Main {
    pub fn view(&self) -> El<'_> {
        if let Some(l) = &self.lock {
            return crate::v_modal::lock_screen(self, l);
        }
        let base: El<'_> = if self.call_full && self.in_voice() {
            crate::v_call::stage(self)
        } else {
            row![self.rail(), self.sidebar(), self.content()].height(Length::Fill).into()
        };
        let mut camadas: Vec<El<'_>> = vec![container(base).style(t::fill(t::main_bg())).width(Length::Fill).height(Length::Fill).into()];

        if self.file_hover {
            camadas.push(
                container(
                    container(
                        column![icon(Icon::Upload, 40.0, t::accent()), text("Solte aqui").size(20).font(BOLD), ui::note("Fotos vão para a conversa aberta. Áudios viram um som novo do soundboard.")]
                            .spacing(10)
                            .align_x(Alignment::Center),
                    )
                    .padding(32)
                    .max_width(420)
                    .style(t::panel),
                )
                .center(Length::Fill)
                .style(t::scrim)
                .into(),
            );
        }

        if let Some(c) = &self.call {
            if let CallState::Ringing { video } = c.state {
                camadas.push(modal_layer(crate::v_call::incoming(self, c, video), false, self.motion.v(&self.motion.ring)));
            }
        }

        if let Some(m) = &self.modal {
            camadas.push(modal_layer(crate::v_modal::view(self, m), true, self.motion.v(&self.motion.modal)));
        }

        if let Some(menu) = &self.menu {
            camadas.push(self.context_menu(menu));
        }

        if let Some((txt, _)) = &self.toast {
            let sobe = (1.0 - self.motion.v(&self.motion.toast)) * 56.0;
            camadas.push(
                container(ui::slide(container(text(txt.clone()).size(14).color(t::FG)).padding([10, 18]).style(t::toast), 0.0, sobe))
                    .align_bottom(Length::Fill)
                    .center_x(Length::Fill)
                    .padding(Padding { bottom: 92.0, ..Padding::ZERO })
                    .into(),
            );
        }
        stack(camadas).width(Length::Fill).height(Length::Fill).into()
    }

    /* ------------------------------------------------------------ barra */

    fn rail(&self) -> El<'_> {
        let mut col = column![].spacing(8).align_x(Alignment::Center).width(RAIL_W);
        col = col.push(ui::vspace(4.0));

        let n_conversas = self.dm_unread() + self.requests().len() as u32;
        let sel = self.section == Section::Chats;
        let anim = self.motion.rail_of("chats", sel);
        col = col.push(rail_entry(
            "chats",
            anim,
            container(icon(Icon::Chat, 22.0, if anim.1 > 0.5 { t::accent_ink() } else { t::FG })).center(48).into(),
            "Conversas",
            Msg::Go(Section::Chats),
            n_conversas,
            false,
        ));
        let sel = self.section == Section::Calls;
        let anim = self.motion.rail_of("calls", sel);
        col = col.push(rail_entry(
            "calls",
            anim,
            container(icon(Icon::Phone, 22.0, if anim.1 > 0.5 { t::accent_ink() } else { t::FG })).center(48).into(),
            "Chamadas",
            Msg::Go(Section::Calls),
            0,
            false,
        ));
        col = col.push(container(Space::new()).width(32).height(2).style(t::rounded(t::surface_hi(), 1.0)));

        let mut grupos = column![].spacing(8).align_x(Alignment::Center);
        for g in &self.groups {
            let sel = self.section == Section::Group(g.lid.clone());
            let anim = self.motion.rail_of(&g.lid, sel);
            let pic = self.pic(g.pic.as_deref());
            let conteudo: El<'_> = match pic {
                Some(h) => iced::widget::image(h)
                    .width(48)
                    .height(48)
                    .content_fit(iced::ContentFit::Cover)
                    .border_radius(rail_radius(anim))
                    .into(),
                None => container(text(t::initials(&g.meta.name)).size(16).font(SEMI)).center(48).into(),
            };
            let em_sala = self.room.as_ref().is_some_and(|r| r.lid == g.lid);
            grupos = grupos.push(rail_entry(&g.lid, anim, conteudo, &g.meta.name, Msg::Go(Section::Group(g.lid.clone())), if g.muted { 0 } else { self.group_unread(&g.lid) }, em_sala));
        }
        grupos = grupos.push(ui::tip(
            button(container(icon(Icon::Plus, 22.0, t::accent())).center(48))
                .padding(0)
                .style(|th, st| {
                    let mut s = t::rail_item(false)(th, st);
                    s.text_color = t::accent();
                    s
                })
                .on_press(Msg::OpenNewGroup),
            "Criar grupo",
            tooltip::Position::Right,
        ));
        col = col.push(scrollable(grupos).height(Length::Fill).style(t::scroll).direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(0).scroller_width(0),
        )));

        let aviso = self.words_pending;
        let anim = self.motion.rail_of("settings", self.section == Section::Settings);
        col = col.push(rail_entry(
            "settings",
            anim,
            container(icon(Icon::Settings, 22.0, if anim.1 > 0.5 { t::accent_ink() } else { t::FG })).center(48).into(),
            if aviso { "Ajustes · anote sua chave de recuperação" } else { "Ajustes" },
            Msg::Go(Section::Settings),
            u32::from(aviso),
            false,
        ));
        col = col.push(ui::vspace(6.0));
        container(col.padding([8, 0])).height(Length::Fill).style(t::fill(t::rail())).into()
    }

    /* ------------------------------------------------------------ lateral */

    fn sidebar(&self) -> El<'_> {
        let corpo: El<'_> = match &self.section {
            Section::Chats => self.chat_list(),
            Section::Calls => self.calls_list(),
            Section::Group(lid) => match self.group(lid) {
                Some(g) => crate::v_group::sidebar(self, g),
                None => self.chat_list(),
            },
            Section::Settings => crate::v_settings::sidebar(self),
        };
        let mut col = column![container(corpo).height(Length::Fill)];
        if let Some(p) = self.update_bar() {
            col = col.push(p);
        }
        if let Some(p) = crate::v_call::voice_panel(self) {
            col = col.push(p);
        }
        col = col.push(self.user_panel());
        container(col).width(SIDEBAR_W).height(Length::Fill).style(t::fill(t::sidebar())).into()
    }

    /// Aviso de versão nova, em cima do painel do usuário.
    fn update_bar(&self) -> Option<El<'_>> {
        let p = self.update.as_ref()?;
        let corpo: El<'_> = match &self.update_prog {
            Some(pr) => {
                let v = pr.load(std::sync::atomic::Ordering::Relaxed) as f32 / 10.0;
                column![
                    text(format!("Baixando a versão {}… {:.0}%", p.versao, v)).size(13).color(t::FG),
                    iced::widget::progress_bar(0.0..=100.0, v).girth(4),
                ]
                .spacing(6)
                .into()
            }
            None => row![
                column![
                    text(format!("Versão {} disponível", p.versao)).size(13.5).font(SEMI).color(t::FG),
                    text(if p.novidades.is_empty() { "Instala por cima, sem perder nada".to_string() } else { ui::ellipsis(&p.novidades, 60) }).size(11.5).color(t::FG_DIM),
                ]
                .spacing(1)
                .width(Length::Fill),
                ui::primary(Some(Icon::Download), "Atualizar", Some(Msg::UpdateStart)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        };
        Some(container(corpo).padding([10, 12]).style(t::fill(t::mix(t::sidebar(), t::accent(), 0.12))).into())
    }

    fn user_panel(&self) -> El<'_> {
        let abertos = self.open_relays();
        let total = self.settings.relays.len();
        let (cor, estado) = if abertos > 0 {
            (t::accent(), format!("{abertos}/{total} servidores"))
        } else {
            (t::WARN, "Conectando…".to_string())
        };
        let meu = self.me.status.as_ref().and_then(|x| x.clean(jm_core::engine::now_ms()));
        let (cor, estado) = match &meu {
            Some(s) if s.dnd => (t::DANGER, if s.t.is_empty() { "Não perturbe".to_string() } else { format!("{} {}", s.e, s.t).trim().to_string() }),
            Some(s) if !s.t.is_empty() || !s.e.is_empty() => (cor, format!("{} {}", s.e, s.t).trim().to_string()),
            _ => (cor, estado),
        };
        let info = column![
            text(ui::ellipsis(&self.me.name, 22)).size(14).font(SEMI).color(t::FG),
            row![container(Space::new()).width(7).height(7).style(t::rounded(cor, 4.0)), text(ui::ellipsis(&estado, 24)).size(11.5).wrapping(iced::widget::text::Wrapping::None).color(t::FG_DIM)]
                .spacing(6)
                .align_y(Alignment::Center),
        ]
        .spacing(2)
        .width(Length::Fill);
        let me = self.me_pk();
        let perfil = button(row![ui::avatar(&self.me.name, self.me.tone, self.pic(self.me.pic.as_deref()), 34.0), info].spacing(10).align_y(Alignment::Center))
            .padding([4, 6])
            .width(Length::Fill)
            .style(t::row_item(false))
            .on_press(Msg::OpenProfile(me));
        let mut r = row![perfil].spacing(2).align_y(Alignment::Center);
        if self.call.is_some() || self.room.is_some() {
            let mudo = self.call.as_ref().map(|c| c.muted).or(self.room.as_ref().map(|r| r.muted)).unwrap_or(false);
            r = r.push(ui::icon_button(if mudo { Icon::MicOff } else { Icon::Mic }, if mudo { "Ligar microfone" } else { "Desligar microfone" }, Some(Msg::ToggleMute), mudo));
        }
        r = r.push(ui::icon_button(Icon::Smile, "Definir status", Some(Msg::OpenSetStatus), meu.is_some()));
        r = r.push(ui::icon_button(Icon::Settings, "Ajustes", Some(Msg::Go(Section::Settings)), false));
        container(r).padding([8, 8]).style(t::fill(t::mix(t::sidebar(), Color::BLACK, 0.25))).into()
    }

    fn sidebar_head<'a>(&self, title: &str, extra: Vec<El<'a>>) -> El<'a> {
        let mut r = row![text(title.to_string()).size(17).font(BOLD).color(t::FG).width(Length::Fill)].spacing(2).align_y(Alignment::Center);
        for e in extra {
            r = r.push(e);
        }
        container(r).padding([12, 14]).height(56).center_y(56).style(t::header).into()
    }

    fn chat_list(&self) -> El<'_> {
        let head = self.sidebar_head(
            "Conversas",
            vec![
                ui::icon_button(Icon::UserPlus, "Adicionar contato", Some(Msg::OpenAddContact(String::new())), false),
                ui::icon_button(Icon::Plus, "Nova conversa", Some(Msg::Menu(CtxKind::New)), false),
            ],
        );
        let busca = container(
            iced::widget::text_input("Buscar conversa", &self.search)
                .on_input(Msg::Search)
                .padding([8, 10])
                .size(13.5)
                .style(t::input),
        )
        .padding([8, 10]);

        let q = self.search.trim().to_lowercase();
        let mut lista = column![].spacing(2).padding([4, 8]);
        let pedidos = self.requests();
        if !pedidos.is_empty() && q.is_empty() {
            let n = pedidos.len();
            lista = lista.push(
                button(
                    row![
                        container(icon(Icon::UserPlus, 18.0, t::accent())).center(40).style(t::rounded(t::with_alpha(t::accent(), 0.14), 999.0)),
                        column![
                            text("Pedidos de conversa").size(14).font(SEMI).color(t::FG),
                            text(format!("{n} {} fora dos seus contatos", if n == 1 { "pessoa" } else { "pessoas" })).size(12.5).color(t::FG_DIM)
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        ui::badge(n as u32),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding([8, 8])
                .width(Length::Fill)
                .style(t::row_item(self.show_requests))
                .on_press(Msg::ShowRequests),
            );
        }
        let mut conversas: Vec<&ChatSummary> = self
            .chats
            .iter()
            .filter(|c| !c.is_group)
            .filter(|c| {
                let pk = c.peer.as_deref().unwrap_or("");
                // Pedidos ficam na tela própria; bloqueados não aparecem.
                !matches!(self.contact_status(pk), Some(ContactStatus::Request) | Some(ContactStatus::Blocked))
            })
            .filter(|c| q.is_empty() || self.chat_title(&c.key).to_lowercase().contains(&q))
            .collect();
        conversas.sort_by_key(|c| (!c.pinned, std::cmp::Reverse(c.last_ts)));
        if conversas.is_empty() && (pedidos.is_empty() || !q.is_empty()) {
            lista = lista.push(
                container(
                    column![
                        text(if q.is_empty() { "Nenhuma conversa ainda" } else { "Nada encontrado" }).size(14).font(SEMI).color(t::FG_DIM),
                        ui::note(if q.is_empty() { "Adicione alguém pelo código para começar." } else { "Tente outro nome." }),
                    ]
                    .spacing(4)
                    .align_x(Alignment::Center),
                )
                .padding(24)
                .center_x(Length::Fill),
            );
        }
        for c in conversas {
            lista = lista.push(self.chat_row(c));
        }
        column![head, busca, scrollable(lista).height(Length::Fill).style(t::scroll)].into()
    }

    fn chat_row<'a>(&'a self, c: &'a ChatSummary) -> El<'a> {
        let pk = c.peer.clone().unwrap_or_default();
        let nome = self.chat_title(&c.key);
        let sel = self.chat.as_ref().is_some_and(|x| x.key == c.key);
        let digitando = self.typing_in(&c.key).is_some();
        let previa: El<'a> = if digitando {
            ui::typing_label(self.motion.clock(), 13.0)
        } else {
            let mut r = row![].spacing(4).align_y(Alignment::Center);
            if c.last_from_me && c.last_kind != "sys" {
                if let Some(st) = c.last_status {
                    r = r.push(status_icon(st, 14.0));
                }
            }
            if c.ttl > 0 {
                r = r.push(icon(Icon::Timer, 13.0, t::FG_FAINT));
            }
            r.push(text(ui::ellipsis(&c.last_text, 30)).size(13).wrapping(iced::widget::text::Wrapping::None).color(if c.unread > 0 { t::FG } else { t::FG_DIM }))
                .into()
        };
        let mut direita = column![text(if c.last_ts > 0 { t::list_time(c.last_ts) } else { String::new() }).size(11.5).color(if c.unread > 0 && !c.muted {
            t::accent()
        } else {
            t::FG_FAINT
        })]
        .spacing(4)
        .align_x(Alignment::End);
        let mut icones = row![].spacing(4).align_y(Alignment::Center);
        if c.muted {
            icones = icones.push(icon(Icon::BellOff, 13.0, t::FG_FAINT));
        }
        if c.pinned {
            icones = icones.push(icon(Icon::Pin, 13.0, t::FG_FAINT));
        }
        if c.unread > 0 {
            icones = icones.push(if c.muted {
                container(text(c.unread.to_string()).size(11.5).font(BOLD).color(t::FG)).padding([1, 6]).style(t::rounded(t::surface_hi(), 999.0)).into()
            } else {
                ui::badge(c.unread)
            });
        }
        direita = direita.push(icones);
        let linha = row![
            ui::avatar(&nome, self.tone_of(&pk), self.pic_of(&pk), 44.0),
            container(
                column![text(ui::ellipsis(&nome, 24)).size(14.5).font(SEMI).wrapping(iced::widget::text::Wrapping::None).color(t::FG), previa].spacing(3),
            )
            .width(Length::Fill)
            .clip(true),
            direita,
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        let b = button(linha).padding([8, 8]).width(Length::Fill).style(t::row_item(sel)).on_press(Msg::OpenChat(c.key.clone()));
        mouse_area(b).on_right_press(Msg::RowMenu(c.key.clone())).into()
    }

    fn calls_list(&self) -> El<'_> {
        let head = self.sidebar_head(
            "Chamadas",
            if self.call_log.is_empty() {
                vec![]
            } else {
                vec![ui::icon_button(Icon::Trash, "Limpar histórico", Some(Msg::Confirm(ConfirmKind::ClearCalls)), false)]
            },
        );
        let mut lista = column![].spacing(2).padding([6, 8]);
        if self.call_log.is_empty() {
            lista = lista.push(
                container(
                    column![
                        icon(Icon::Phone, 30.0, t::FG_FAINT),
                        text("Nenhuma chamada ainda").size(14).font(SEMI).color(t::FG_DIM),
                        ui::note("Ligue pelo topo de uma conversa."),
                    ]
                    .spacing(6)
                    .align_x(Alignment::Center),
                )
                .padding(28)
                .center_x(Length::Fill),
            );
        }
        for c in &self.call_log {
            let perdida = c.status == "missed" && c.dir == "in";
            let (i, cor) = if perdida {
                (Icon::PhoneOff, t::DANGER)
            } else if c.dir == "in" {
                (Icon::Down, t::accent())
            } else {
                (Icon::Up, t::FG_DIM)
            };
            let rotulo = match (c.dir.as_str(), c.status.as_str()) {
                ("in", "missed") => "Perdida",
                ("in", "declined") => "Recusada",
                ("in", _) => "Recebida",
                (_, "missed") | (_, "cancelled") => "Não atendida",
                (_, "declined") => "Recusada",
                (_, "busy") => "Ocupado",
                (_, "failed") => "Falhou",
                _ => "Feita",
            };
            let dur = if c.dur > 0 { format!(" · {}", t::mmss(c.dur as f32)) } else { String::new() };
            let nome = self.name_of(&c.peer);
            let pode_ligar = self.contact_status(&c.peer) == Some(ContactStatus::Ok);
            let linha = row![
                ui::avatar(&nome, self.tone_of(&c.peer), self.pic_of(&c.peer), 40.0),
                column![
                    text(ui::ellipsis(&nome, 22)).size(14).font(SEMI).color(if perdida { t::DANGER } else { t::FG }),
                    row![
                        icon(i, 13.0, cor),
                        icon(if c.video { Icon::Video } else { Icon::Phone }, 13.0, t::FG_FAINT),
                        text(format!("{rotulo} · {}{dur}", t::list_time(c.ts))).size(12).wrapping(iced::widget::text::Wrapping::None).color(t::FG_DIM)
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center),
                ]
                .spacing(3)
                .width(Length::Fill),
                ui::icon_button(Icon::Phone, "Ligar de novo", pode_ligar.then(|| Msg::Call(c.peer.clone(), false)), false),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            lista = lista.push(
                button(linha)
                    .padding([6, 6])
                    .width(Length::Fill)
                    .style(t::row_item(self.chat.as_ref().is_some_and(|x| x.key == format!("dm:{}", c.peer))))
                    .on_press(Msg::OpenChat(format!("dm:{}", c.peer))),
            );
        }
        column![head, scrollable(lista).height(Length::Fill).style(t::scroll)].into()
    }

    /* ------------------------------------------------------------ conteúdo */

    fn content(&self) -> El<'_> {
        let principal: El<'_> = if self.section == Section::Settings {
            ui::slide(crate::v_settings::page(self), (1.0 - self.motion.v(&self.motion.page)) * 28.0, 0.0)
        } else if self.show_requests {
            self.requests_page()
        } else if let Some(c) = &self.chat {
            crate::v_chat::view(self, c)
        } else {
            self.empty_state()
        };
        let mut col = column![];
        if let Some(c) = &self.call {
            if !matches!(c.state, CallState::Ringing { .. }) {
                col = col.push(crate::v_call::call_panel(self, c));
            }
        }
        if let Some(r) = &self.room {
            let ver = matches!(&self.section, Section::Group(l) if *l == r.lid);
            if ver {
                col = col.push(crate::v_call::room_panel(self, r));
            }
        }
        col = col.push(container(principal).height(Length::Fill));
        let mut r = row![container(col).width(Length::Fill).height(Length::Fill)];
        if let (Section::Group(lid), true) = (&self.section, self.show_members) {
            if let Some(g) = self.group(lid) {
                if self.chat.is_some() {
                    r = r.push(crate::v_group::members(self, g));
                }
            }
        }
        let mut camadas: Vec<El<'_>> = vec![container(r).width(Length::Fill).height(Length::Fill).style(t::fill(t::main_bg())).into()];
        let em_voz = self.room.is_some() || self.call.as_ref().is_some_and(|c| matches!(c.state, CallState::Active { .. } | CallState::Reconnecting));
        if self.popup == Some(Popup::CallSounds) && em_voz {
            camadas.push(
                container(ui::slide(opaque(crate::v_chat::sounds_popup(self, true)), 0.0, (1.0 - self.motion.v(&self.motion.popup)) * 14.0))
                    .align_top(Length::Fill)
                    .center_x(Length::Fill)
                    .padding(Padding { top: 236.0, ..Padding::ZERO })
                    .into(),
            );
        }
        stack(camadas).width(Length::Fill).height(Length::Fill).into()
    }

    fn empty_state(&self) -> El<'_> {
        let (titulo, sub): (&str, &str) = match &self.section {
            Section::Calls => ("Chamadas", "Escolha uma chamada para abrir a conversa, ou ligue de novo pelo ícone ao lado."),
            Section::Group(_) => ("Nenhum canal de texto", "Peça a um admin para criar um canal."),
            _ => ("Suas conversas aparecem aqui", "Tudo é cifrado de ponta a ponta. Os servidores não sabem quem fala com quem nem o que é dito."),
        };
        container(
            column![
                logo(88.0),
                text(titulo).size(22).font(BOLD).color(t::FG),
                text(sub).size(14).color(t::FG_DIM).align_x(Alignment::Center),
                ui::vspace(6.0),
                row![
                    ui::primary(Some(Icon::UserPlus), "Adicionar contato", Some(Msg::OpenAddContact(String::new()))),
                    ui::secondary(Some(Icon::Qr), "Meu código", Some(Msg::OpenMyCode)),
                ]
                .spacing(10),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .max_width(440),
        )
        .center(Length::Fill)
        .into()
    }

    fn requests_page(&self) -> El<'_> {
        let mut col = column![
            ui::title("Pedidos de conversa"),
            ui::subtitle("Pessoas fora dos seus contatos que mandaram mensagem. Ninguém sabe que você viu até você aceitar."),
            ui::vspace(6.0)
        ]
        .spacing(6)
        .max_width(680);
        let pedidos = self.requests();
        if pedidos.is_empty() {
            col = col.push(ui::note("Nenhum pedido agora."));
        }
        for c in pedidos {
            let nome = c.display_name();
            let previa = self.summary(&format!("dm:{}", c.pk)).map(|s| ui::ellipsis(&s.last_text, 60)).unwrap_or_default();
            col = col.push(
                container(
                    row![
                        button(row![
                            ui::avatar(&nome, c.tone, self.pic_of(&c.pk), 44.0),
                            column![text(nome.clone()).size(15).font(SEMI).color(t::FG), text(previa).size(13).color(t::FG_DIM)].spacing(3)
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center))
                        .padding(4)
                        .width(Length::Fill)
                        .style(t::row_item(false))
                        .on_press(Msg::OpenChat(format!("dm:{}", c.pk))),
                        ui::secondary(Some(Icon::Ban), "Bloquear", Some(Msg::Block(c.pk.clone()))),
                        ui::primary(Some(Icon::Check), "Aceitar", Some(Msg::Accept(c.pk.clone()))),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding(10)
                .style(t::rounded(t::surface(), 12.0)),
            );
        }
        container(scrollable(container(col).padding(28).center_x(Length::Fill)).style(t::scroll)).height(Length::Fill).into()
    }

    /* ------------------------------------------------------------ menus */

    fn context_menu(&self, menu: &CtxMenu) -> El<'_> {
        let itens: Vec<El<'_>> = match &menu.kind {
            CtxKind::New => vec![
                ui::menu_item(Icon::Chat, "Nova conversa", Msg::OpenNewChat, false),
                ui::menu_item(Icon::UserPlus, "Adicionar contato", Msg::OpenAddContact(String::new()), false),
                ui::menu_item(Icon::Qr, "Meu código", Msg::OpenMyCode, false),
                ui::menu_item(Icon::Users, "Criar grupo", Msg::OpenNewGroup, false),
            ],
            CtxKind::Chat(key) => {
                let c = self.summary(key);
                let fixada = c.is_some_and(|c| c.pinned);
                let muda = c.is_some_and(|c| c.muted);
                let aberta = self.chat.as_ref().is_some_and(|c| &c.key == key);
                let mut v = Vec::new();
                if aberta {
                    v.push(ui::menu_item(Icon::Eye, "Buscar na conversa", Msg::OpenSearch, false));
                    v.push(ui::menu_item(Icon::Pin, "Mensagens fixadas", Msg::OpenPins, false));
                }
                v.extend([
                    ui::menu_item(Icon::Pin, if fixada { "Desafixar" } else { "Fixar no topo" }, Msg::PinChat(key.clone(), !fixada), false),
                    ui::menu_item(
                        if muda { Icon::Bell } else { Icon::BellOff },
                        if muda { "Ativar notificações" } else { "Silenciar" },
                        Msg::MuteChat(key.clone(), !muda),
                        false,
                    ),
                ]);
                if c.is_some_and(|c| c.unread > 0) {
                    v.push(ui::menu_item(Icon::Checks, "Marcar como lida", Msg::MarkRead(key.clone()), false));
                }
                if let Some(pk) = Main::peer_of(key) {
                    v.push(ui::menu_item(Icon::Timer, "Mensagens temporárias", Msg::Choose(ChooseKind::Ttl(key.clone())), false));
                    v.push(ui::menu_item(Icon::User, "Ver perfil", Msg::OpenProfile(pk.clone()), false));
                    if self.contact_status(&pk) != Some(ContactStatus::Blocked) {
                        v.push(ui::menu_item(Icon::Ban, "Bloquear", Msg::Confirm(ConfirmKind::Block(pk)), true));
                    }
                } else {
                    v.push(ui::menu_item(Icon::Timer, "Mensagens temporárias", Msg::Choose(ChooseKind::Ttl(key.clone())), false));
                }
                v.push(ui::menu_item(Icon::Trash, "Apagar conversa", Msg::Confirm(ConfirmKind::DeleteChat(key.clone())), true));
                v
            }
            CtxKind::Group(lid) => {
                let muda = self.group(lid).is_some_and(|g| g.muted);
                vec![
                    ui::menu_item(Icon::Info, "Dados do grupo", Msg::OpenGroupInfo(lid.clone()), false),
                    ui::menu_item(Icon::UserPlus, "Convidar pessoas", Msg::OpenInvite(lid.clone()), false),
                    ui::menu_item(Icon::Sparkle, "Cargos", Msg::OpenRoles(lid.clone()), false),
                    ui::menu_item(
                        if muda { Icon::Bell } else { Icon::BellOff },
                        if muda { "Ativar notificações" } else { "Silenciar grupo" },
                        Msg::MuteGroup(lid.clone(), !muda),
                        false,
                    ),
                    ui::menu_item(Icon::Logout, "Sair do grupo", Msg::Confirm(ConfirmKind::LeaveGroup(lid.clone())), true),
                ]
            }
            CtxKind::Member(lid, pk) => {
                let mut v = Vec::new();
                let me = self.me_pk();
                if *pk == me {
                    v.push(ui::menu_item(Icon::User, "Meu perfil", Msg::OpenProfile(pk.clone()), false));
                } else {
                    match self.contact_status(pk) {
                        Some(ContactStatus::Ok) => {
                            v.push(ui::menu_item(Icon::Chat, "Conversar", Msg::OpenChat(format!("dm:{pk}")), false));
                            v.push(ui::menu_item(Icon::User, "Ver perfil", Msg::OpenProfile(pk.clone()), false));
                        }
                        Some(_) => v.push(ui::menu_item(Icon::User, "Ver perfil", Msg::OpenProfile(pk.clone()), false)),
                        None => v.push(ui::menu_item(
                            Icon::UserPlus,
                            "Adicionar aos contatos",
                            Msg::OpenAddContact(jm_core::identity::npub_hex(pk)),
                            false,
                        )),
                    }
                    if let Some(g) = self.group(lid) {
                        if g.meta.is_admin(&me) {
                            v.push(ui::menu_item(Icon::Sparkle, "Cargos…", Msg::OpenMemberRoles(lid.clone(), pk.clone()), false));
                        }
                        if g.meta.is_admin(&me) && g.meta.owner != *pk {
                            let admin = g.meta.is_admin(pk);
                            v.push(ui::menu_item(
                                Icon::Shield,
                                if admin { "Remover admin" } else { "Tornar admin" },
                                Msg::SetAdmin(lid.clone(), pk.clone(), !admin),
                                false,
                            ));
                            v.push(ui::menu_item(Icon::Ban, "Remover do grupo", Msg::Confirm(ConfirmKind::RemoveMember(lid.clone(), pk.clone())), true));
                        }
                    }
                }
                v
            }
            CtxKind::Message(id) => self.message_menu(id),
        };
        if itens.is_empty() {
            return Space::new().into();
        }
        let largura = if matches!(menu.kind, CtxKind::Message(_)) { 268.0 } else { 232.0 };
        let altura = itens.len() as f32 * 38.0 + 16.0;
        let x = menu.at.x.min(self.size.width - largura - 8.0).max(4.0);
        let y = if menu.at.y + altura > self.size.height - 8.0 { (menu.at.y - altura).max(4.0) } else { menu.at.y };
        let caixa = container(column(itens).spacing(2)).padding(6).width(largura).style(t::panel);
        stack![
            mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill)).on_press(Msg::CloseOverlays).on_right_press(Msg::CloseOverlays),
            pin(ui::slide(opaque(caixa), 0.0, (1.0 - self.motion.v(&self.motion.menu)) * -8.0)).position(Point::new(x, y)),
        ]
        .into()
    }

    fn message_menu(&self, id: &str) -> Vec<El<'_>> {
        let Some(c) = &self.chat else { return vec![] };
        let Some(m) = c.messages.iter().find(|m| m.id == id) else { return vec![] };
        let mut v: Vec<El<'_>> = Vec::new();
        let apagada = matches!(m.body, MessageBody::Deleted);
        let pode_reagir = !apagada && !m.body.is_system() && self.can_write(&c.key);
        if pode_reagir {
            let mut r = row![].spacing(2).align_y(Alignment::Center);
            let minha = m.reacts.get(&self.me_pk()).cloned();
            for e in ui::QUICK_REACTS {
                let sel = minha.as_deref() == Some(e);
                r = r.push(
                    button(text(e).size(20))
                        .padding([4, 5])
                        .style(t::icon_btn(sel))
                        .on_press(Msg::React(m.id.clone(), e.to_string())),
                );
            }
            r = r.push(ui::icon_button_sm(Icon::Plus, "Mais reações", Some(Msg::ReactMore(m.id.clone()))));
            v.push(container(r).padding([2, 2]).into());
            v.push(ui::menu_item(Icon::Reply, "Responder", Msg::Reply(m.id.clone()), false));
        }
        if let MessageBody::Text(txt) = &m.body {
            if m.from_me && self.can_write(&c.key) {
                v.push(ui::menu_item(Icon::Edit, "Editar", Msg::EditStart(m.id.clone()), false));
            }
            v.push(ui::menu_item(Icon::Copy, "Copiar texto", Msg::Copy(txt.clone()), false));
        }
        let pode_encaminhar = match &m.body {
            MessageBody::Text(_) | MessageBody::Sound { .. } => true,
            MessageBody::Media(x) => !x.vo && x.ready,
            _ => false,
        };
        if pode_encaminhar {
            v.push(ui::menu_item(Icon::Share, "Encaminhar", Msg::OpenForward(m.id.clone()), false));
        }
        if let MessageBody::Media(x) = &m.body {
            if x.kind == jm_core::messages::MediaKind::File && x.ready {
                v.push(ui::menu_item(Icon::Download, "Salvar arquivo", Msg::SaveFile(x.mid.clone(), x.name.clone().unwrap_or_else(|| "arquivo".into())), false));
            }
        }
        if matches!(m.body, MessageBody::Poll { .. }) {
            v.push(ui::menu_item(Icon::Users, "Ver quem votou", Msg::OpenVoters(m.id.clone()), false));
        }
        if !apagada && !m.body.is_system() && self.can_pin(&c.key) {
            let fixada = c.pins.contains(&m.id);
            v.push(ui::menu_item(Icon::Pin, if fixada { "Desafixar" } else { "Fixar na conversa" }, Msg::PinMsg(m.id.clone(), !fixada), false));
        }
        if let MessageBody::Media(x) = &m.body {
            if x.ready && x.kind == jm_core::messages::MediaKind::Image {
                v.push(ui::menu_item(Icon::Download, "Salvar foto", Msg::SavePhoto(x.mid.clone()), false));
            }
        }
        if !m.body.is_system() {
            v.push(ui::menu_item(Icon::Trash, "Apagar…", Msg::AskDelete(m.id.clone()), true));
        }
        v
    }

    /// Pode fixar mensagens: em conversa a dois, sempre; em grupo, admins e
    /// cargos com esse poder.
    pub fn can_pin(&self, key: &str) -> bool {
        match key.strip_prefix("g:").and_then(|r| r.split(':').next()) {
            Some(lid) => self.group(lid).is_some_and(|g| g.meta.can(&self.me_pk(), "pin")),
            None => self.can_write(key),
        }
    }

    /// Pode apagar mensagens dos outros (grupo, admin ou cargo com "del").
    pub fn can_moderate(&self, key: &str) -> bool {
        key.strip_prefix("g:").and_then(|r| r.split(':').next()).and_then(|lid| self.group(lid)).is_some_and(|g| g.meta.can(&self.me_pk(), "del"))
    }

    /// A pessoa pode escrever nesta conversa (contato aceito ou canal de grupo).
    pub fn can_write(&self, key: &str) -> bool {
        match Main::peer_of(key) {
            Some(pk) => self.contact_status(&pk) == Some(ContactStatus::Ok),
            None => true,
        }
    }
}

/// Uma entrada da barra da esquerda, com marcador de selecionado e selo.
/// Canto do ícone: círculo parado, quadrado arredondado com o mouse em
/// cima ou selecionado.
fn rail_radius((hover, sel): (f32, f32)) -> f32 {
    24.0 - 8.0 * hover.max(sel)
}

/// Item da barra da esquerda. `anim` = (mouse em cima, selecionado), de 0
/// a 1: o marcador cresce (8 → 20 → 36 px) e o canto muda, como no Discord.
fn rail_entry<'a>(key: &str, anim: (f32, f32), conteudo: El<'a>, label: &str, msg: Msg, n: u32, voz: bool) -> El<'a> {
    let (hover, sel) = anim;
    let base = if n > 0 { 8.0 } else { 0.0 };
    let com_mouse = base + (20.0_f32 - base).max(0.0) * hover;
    let altura = com_mouse + (36.0 - com_mouse) * sel;
    let marcador = container(Space::new()).width(4).height(altura.max(0.0)).style(t::rounded(t::FG, 2.0));
    let b = button(conteudo).padding(0).width(48).height(48).style(t::rail_item_r(sel, rail_radius(anim))).on_press(msg);
    let mut camada = vec![b.into()];
    if n > 0 {
        let s = if n > 99 { "99+".to_string() } else { n.to_string() };
        camada.push(
            container(
                container(text(s).size(11).font(BOLD).color(Color::WHITE)).padding([1, 5]).style(|_| iced::widget::container::Style {
                    background: Some(iced::Background::Color(t::DANGER)),
                    border: iced::Border { color: t::rail(), width: 2.5, radius: 999.0.into() },
                    ..Default::default()
                }),
            )
            .align_right(54)
            .align_bottom(54)
            .into(),
        );
    }
    if voz {
        camada.push(
            container(container(icon(Icon::Volume, 12.0, t::accent_ink())).center(20).style(t::rounded(t::accent(), 999.0)))
                .align_right(52)
                .align_top(52)
                .into(),
        );
    }
    let item = mouse_area(stack(camada).width(52).height(52)).on_enter(Msg::RailHover(Some(key.to_string()))).on_exit(Msg::RailHover(None));
    let r = row![container(marcador).center_y(52).width(8), ui::tip(item, label.to_string(), tooltip::Position::Right), ui::hspace(12.0)]
        .align_y(Alignment::Center);
    r.into()
}

/// Ícone do estado de entrega (relógio, um tique, dois tiques, azul).
pub fn status_icon<'a>(st: Delivery, size: f32) -> El<'a> {
    match st {
        Delivery::Pending => icon(Icon::Clock, size, t::FG_FAINT),
        Delivery::Sent => icon(Icon::Check, size, t::FG_DIM),
        Delivery::Delivered => icon(Icon::Checks, size, t::FG_DIM),
        Delivery::Read => icon(Icon::Checks, size, t::READ_BLUE),
    }
}

/// Camada de janela: fundo escurecido e a janela no centro. Clicar fora
/// fecha (quando a janela permite).
pub fn modal_layer<'a>(content: El<'a>, closable: bool, t_: f32) -> El<'a> {
    let caixa = ui::slide(opaque(content), 0.0, (1.0 - t_) * 26.0);
    let fundo = container(caixa).center(Length::Fill).padding(24).style(t::scrim_a(t_));
    if closable {
        mouse_area(fundo).on_press(Msg::Key(Key::Escape)).into()
    } else {
        opaque(fundo)
    }
}

pub fn kind_icon(k: ChannelKind) -> Icon {
    match k {
        ChannelKind::Text => Icon::Hash,
        ChannelKind::Voice => Icon::Volume,
    }
}
