//! Entrada: criar conta (nome → cor → palavras → avisos) ou recuperar uma
//! conta com as 12 palavras, como no celular.

use crate::app::{App, Msg, Screen, Screenish};
use crate::icons::{icon, logo, Icon};
use crate::theme as t;
use crate::ui::{self, El, BOLD, SEMI};
use iced::widget::{button, column, container, row, text, toggler, Space};
use iced::{Alignment, Length};

pub fn view(app: &App) -> El<'_> {
    let corpo: El<'_> = match &app.screen {
        Screen::Loading => column![logo(84.0), text("Abrindo…").size(15).color(t::FG_DIM)].spacing(18).align_x(Alignment::Center).into(),
        Screen::Welcome => welcome(app),
        Screen::Create(n) => create(app, *n),
        Screen::Restore(n) => restore(app, *n),
        Screen::Main(_) => Space::new().into(),
    };
    let mut col = column![corpo].spacing(16).align_x(Alignment::Center).max_width(460);
    if let Some(e) = &app.error {
        col = col.push(
            container(row![icon(Icon::Alert, 16.0, t::DANGER), text(e.clone()).size(13.5).color(t::DANGER)].spacing(8).align_y(Alignment::Center))
                .padding([10, 14])
                .style(t::chip(t::DANGER)),
        );
    }
    container(col).center(Length::Fill).padding(24).style(t::fill(t::main_bg())).into()
}

fn card<'a>(content: El<'a>) -> El<'a> {
    container(content).padding(28).width(Length::Fill).style(t::panel).into()
}

fn steps<'a>(at: u8, total: u8) -> El<'a> {
    row((0..total)
        .map(|i| {
            let on = i <= at;
            container(Space::new())
                .width(if i == at { 22 } else { 8 })
                .height(8)
                .style(t::rounded(if on { t::accent() } else { t::surface_hi() }, 4.0))
                .into()
        })
        .collect::<Vec<El<'a>>>())
    .spacing(6)
    .into()
}

fn welcome(app: &App) -> El<'_> {
    let _ = app;
    column![
        logo(96.0),
        row![text("J-").size(34).font(BOLD).color(t::accent()), text("Menseger").size(34).font(BOLD).color(t::FG)],
        text("Mensagens e chamadas cifradas de ponta a ponta. Sem telefone, sem e-mail, sem servidor próprio.")
            .size(15)
            .color(t::FG_DIM)
            .align_x(Alignment::Center),
        ui::vspace(10.0),
        ui::primary_wide(Some(Icon::Sparkle), "Criar conta", Some(Msg::OnbGo(Screenish::Create(0)))),
        ui::secondary_wide(Some(Icon::Key), "Já tenho uma conta", Some(Msg::OnbGo(Screenish::Restore(0)))),
        text("Sua conta é um par de chaves criado neste computador. Ninguém mais tem acesso a ela.")
            .size(12.5)
            .color(t::FG_FAINT)
            .align_x(Alignment::Center),
    ]
    .spacing(14)
    .align_x(Alignment::Center)
    .into()
}

fn back<'a>(to: Screenish) -> El<'a> {
    button(row![icon(Icon::Back, 16.0, t::FG_DIM), text("Voltar").size(13.5).color(t::FG_DIM)].spacing(4).align_y(Alignment::Center))
        .padding([6, 8])
        .style(t::ghost)
        .on_press(Msg::OnbGo(to))
        .into()
}

fn create(app: &App, n: u8) -> El<'_> {
    let o = &app.onb;
    let topo = row![back(if n == 0 { Screenish::Welcome } else { Screenish::Create(n.min(3) - 1) }), ui::fill(), steps(n.min(3), 4)]
        .align_y(Alignment::Center);
    let corpo: El<'_> = match n {
        0 => column![
            ui::title("Como quer ser chamado?"),
            ui::subtitle("Seus contatos veem este nome. Dá para mudar depois."),
            ui::vspace(4.0),
            ui::field("Nome", ui::input("Seu nome", &o.name, Msg::OnbName).on_submit(Msg::OnbGo(Screenish::Create(1)))),
            ui::field("Recado (opcional)", ui::input("Disponível para conversar", &o.note, Msg::OnbNote)),
            ui::vspace(6.0),
            ui::primary_wide(None, "Continuar", Some(Msg::OnbGo(Screenish::Create(1)))),
        ]
        .spacing(12)
        .into(),
        1 => {
            let nome = if o.name.trim().is_empty() { "Você" } else { o.name.trim() };
            let tons = row((0..6u8)
                .map(|i| {
                    button(Space::new().width(30).height(30))
                        .padding(0)
                        .style(t::swatch(t::tone(i), i == o.tone))
                        .on_press(Msg::OnbTone(i))
                        .into()
                })
                .collect::<Vec<El<'_>>>())
            .spacing(12);
            column![
                ui::title("Escolha uma cor"),
                ui::subtitle("É a cor do seu avatar enquanto você não coloca uma foto."),
                container(ui::avatar(nome, o.tone, None, 96.0)).center_x(Length::Fill).padding(10),
                container(tons).center_x(Length::Fill),
                ui::vspace(6.0),
                ui::primary_wide(None, "Continuar", Some(Msg::OnbGo(Screenish::Create(2)))),
            ]
            .spacing(12)
            .into()
        }
        2 => {
            let palavras: Vec<&str> = o.words.split_whitespace().collect();
            let grade = column(palavras
                .chunks(3)
                .enumerate()
                .map(|(l, c)| {
                    row(c.iter()
                        .enumerate()
                        .map(|(i, w)| {
                            let num = l * 3 + i + 1;
                            container(row![
                                text(format!("{num}")).size(11.5).color(t::FG_FAINT).width(18),
                                text(w.to_string()).size(15).font(SEMI)
                            ]
                            .align_y(Alignment::Center))
                            .padding([9, 10])
                            .width(Length::Fill)
                            .style(t::rounded(t::surface_hi(), 9.0))
                            .into()
                        })
                        .collect::<Vec<El<'_>>>())
                    .spacing(8)
                    .into()
                })
                .collect::<Vec<El<'_>>>())
            .spacing(8);
            column![
                ui::title("Sua chave de recuperação"),
                ui::subtitle("Estas 12 palavras recriam sua conta em outro aparelho. Anote no papel, na ordem, e guarde longe do computador. Nunca mande para ninguém."),
                grade,
                container(row![icon(Icon::Shield, 16.0, t::WARN), text("Quem tiver estas palavras controla sua conta.").size(13).color(t::WARN)].spacing(8).align_y(Alignment::Center))
                    .padding([8, 12])
                    .style(t::chip(t::WARN)),
                ui::primary_wide(Some(Icon::Check), "Anotei, continuar", Some(Msg::OnbGo(Screenish::Create(3)))),
                ui::ghost(None, "Anotar depois", Some(Msg::OnbGo(Screenish::Create(4)))),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .into()
        }
        _ => {
            let depois = n == 4;
            column![
                ui::title("Avisos"),
                ui::subtitle("O J-Menseger pode mostrar uma notificação quando chega mensagem ou chamada e a janela não está em foco."),
                ui::vspace(4.0),
                container(row![
                    icon(Icon::Bell, 20.0, t::FG_DIM),
                    column![text("Notificações").size(14.5).font(SEMI), text("Mensagens e chamadas").size(12.5).color(t::FG_DIM)].spacing(2).width(Length::Fill),
                    toggler(o.notify).on_toggle(Msg::OnbNotify).size(20),
                ]
                .spacing(12)
                .align_y(Alignment::Center))
                .padding(14)
                .style(t::rounded(t::surface_hi(), 12.0)),
                if depois {
                    ui::note("Você escolheu anotar as palavras depois. Vamos lembrar em Ajustes até você anotar.")
                } else {
                    Space::new().into()
                },
                ui::vspace(6.0),
                ui::primary_wide(Some(Icon::Sparkle), if app.busy { "Criando…" } else { "Começar" }, (!app.busy).then_some(Msg::OnbCreate { words_later: depois })),
            ]
            .spacing(12)
            .into()
        }
    };
    card(column![topo, corpo].spacing(16).into())
}

fn restore(app: &App, n: u8) -> El<'_> {
    let o = &app.onb;
    let topo = row![back(if n == 0 { Screenish::Welcome } else { Screenish::Restore(0) }), ui::fill(), steps(n, 2)].align_y(Alignment::Center);
    let corpo: El<'_> = if n == 0 {
        column![
            ui::title("Entrar na sua conta"),
            ui::subtitle("Digite as 12 palavras de recuperação, separadas por espaço. Também dá para colar uma chave nsec1… de outro app Nostr."),
            ui::input("palavra1 palavra2 palavra3 …", &o.input, Msg::OnbInput).on_submit(Msg::OnbGo(Screenish::Restore(1))).secure(false),
            ui::note("As conversas guardadas nos servidores voltam (cerca de 30 dias). O histórico mais antigo fica só no aparelho antigo."),
            ui::vspace(6.0),
            ui::primary_wide(None, "Continuar", Some(Msg::OnbGo(Screenish::Restore(1)))),
        ]
        .spacing(12)
        .into()
    } else {
        column![
            ui::title("Como quer ser chamado?"),
            ui::subtitle("Seus contatos veem este nome."),
            ui::field("Nome", ui::input("Seu nome", &o.name, Msg::OnbName).on_submit(Msg::OnbRestore)),
            container(row![
                icon(Icon::Bell, 20.0, t::FG_DIM),
                column![text("Notificações").size(14.5).font(SEMI), text("Mensagens e chamadas").size(12.5).color(t::FG_DIM)].spacing(2).width(Length::Fill),
                toggler(o.notify).on_toggle(Msg::OnbNotify).size(20),
            ]
            .spacing(12)
            .align_y(Alignment::Center))
            .padding(14)
            .style(t::rounded(t::surface_hi(), 12.0)),
            ui::vspace(6.0),
            ui::primary_wide(Some(Icon::Key), if app.busy { "Entrando…" } else { "Entrar" }, (!app.busy).then_some(Msg::OnbRestore)),
        ]
        .spacing(12)
        .into()
    };
    card(column![topo, corpo].spacing(16).into())
}
