//! Ajustes, com as mesmas páginas do celular.

use crate::app::Msg;
use crate::icons::{icon, logo, Icon};
use crate::state::*;
use crate::theme::{self as t, Backdrop};
use crate::ui::{self, El, Trail, BOLD, SEMI};
use iced::widget::{button, column, container, qr_code, row, scrollable, slider, text, Space};
use iced::{Alignment, Length};
use jm_core::engine::{CallsFrom, NotifPreview};

pub fn sidebar(m: &Main) -> El<'_> {
    let item = |p: SettingsPage, i: Icon, label: &'static str, dot: bool| -> El<'_> {
        let sel = m.page == p;
        let mut r = row![icon(i, 18.0, if sel { t::FG } else { t::FG_DIM }), text(label).size(14).font(SEMI).color(if sel { t::FG } else { t::FG_DIM }).width(Length::Fill)]
            .spacing(10)
            .align_y(Alignment::Center);
        if dot {
            r = r.push(container(Space::new()).width(8).height(8).style(t::rounded(t::WARN, 4.0)));
        }
        button(r).padding([8, 10]).width(Length::Fill).style(t::row_item(sel)).on_press(if p == SettingsPage::Recovery { Msg::OpenRecovery } else { Msg::Page(p) }).into()
    };
    let rotulo = |s: &str| -> El<'_> { container(text(s.to_uppercase()).size(11).font(BOLD).color(t::FG_FAINT)).padding(iced::Padding { top: 14.0, bottom: 4.0, left: 10.0, right: 0.0 }).into() };
    let col = column![
        rotulo("Conta"),
        item(SettingsPage::Profile, Icon::User, "Perfil", false),
        item(SettingsPage::MyCode, Icon::Qr, "Meu código", false),
        item(SettingsPage::Recovery, Icon::Key, "Chave de recuperação", m.words_pending),
        item(SettingsPage::Devices, Icon::Screen, "Aparelhos conectados", false),
        rotulo("Privacidade e avisos"),
        item(SettingsPage::Privacy, Icon::Shield, "Privacidade e segurança", false),
        item(SettingsPage::Notifications, Icon::Bell, "Notificações", false),
        rotulo("Aplicativo"),
        item(SettingsPage::Appearance, Icon::Drop, "Aparência", false),
        item(SettingsPage::Soundboard, Icon::Wave, "Soundboard", false),
        item(SettingsPage::Network, Icon::Server, "Rede e servidores", false),
        item(SettingsPage::Audio, Icon::Mic, "Voz e áudio", false),
        item(SettingsPage::Gifs, Icon::Image, "GIFs", false),
        item(SettingsPage::Storage, Icon::Db, "Armazenamento", false),
        rotulo("Sobre"),
        item(SettingsPage::About, Icon::Sparkle, "Sobre o J-Menseger", false),
        button(row![icon(Icon::Share, 18.0, t::FG_DIM), text("Convidar amigos").size(14).font(SEMI).color(t::FG_DIM)].spacing(10).align_y(Alignment::Center))
            .padding([8, 10])
            .width(Length::Fill)
            .style(t::row_item(false))
            .on_press(Msg::CopyInvite),
        button(row![icon(Icon::Logout, 18.0, t::DANGER), text("Sair e apagar").size(14).font(SEMI).color(t::DANGER)].spacing(10).align_y(Alignment::Center))
            .padding([8, 10])
            .width(Length::Fill)
            .style(t::danger_text)
            .on_press(Msg::Confirm(ConfirmKind::Wipe)),
        ui::vspace(12.0),
    ]
    .spacing(2)
    .padding([4, 8]);
    column![
        container(text("Ajustes").size(17).font(BOLD).color(t::FG)).padding([0, 14]).height(56).center_y(56).width(Length::Fill).style(t::header),
        scrollable(col).height(Length::Fill).style(t::scroll)
    ]
    .into()
}

fn frame<'a>(titulo: &str, sub: Option<&str>, corpo: Vec<El<'a>>) -> El<'a> {
    let mut col = column![ui::title(titulo)].spacing(6).max_width(700);
    if let Some(s) = sub {
        col = col.push(ui::subtitle(s));
    }
    for c in corpo {
        col = col.push(c);
    }
    col = col.push(ui::vspace(40.0));
    scrollable(container(col).padding([28, 32]).center_x(Length::Fill)).height(Length::Fill).style(t::scroll).into()
}

pub fn page(m: &Main) -> El<'_> {
    match m.page {
        SettingsPage::Profile => profile(m),
        SettingsPage::MyCode => my_code(m),
        SettingsPage::Recovery => recovery(m),
        SettingsPage::Privacy => privacy(m),
        SettingsPage::Blocked => blocked(m),
        SettingsPage::Notifications => notifications(m),
        SettingsPage::Appearance => appearance(m),
        SettingsPage::Soundboard => soundboard(m),
        SettingsPage::Network => network(m),
        SettingsPage::Storage => storage(m),
        SettingsPage::About => about(m),
        SettingsPage::Audio => audio(m),
        SettingsPage::Gifs => gifs(m),
        SettingsPage::Devices => devices(m),
    }
}

/* ------------------------------------------------------------ aparelhos */

fn devices(m: &Main) -> El<'_> {
    let (nome, sistema) = crate::platform::device_name();
    let mut linhas: Vec<El<'_>> = vec![ui::setting(Some(Icon::Screen), &format!("{nome} (este computador)"), Some(sistema), Trail::None, None, false)];
    let agora = jm_core::engine::now_ms();
    for (id, n, p, last) in m.engine.devices() {
        let dias = (agora - last) / 86_400_000;
        let visto = if agora - last < 7 * 3_600_000 { "Ativo recentemente".to_string() } else if dias < 1 { "Visto hoje".into() } else { format!("Visto há {dias} dias") };
        let icone = if p == "Android" || p == "iOS" { Icon::Phone } else { Icon::Screen };
        linhas.push(ui::setting(
            Some(icone),
            &n,
            Some(format!("{p} · {visto}")),
            Trail::Custom(ui::danger_text(None, "Desconectar", Some(Msg::RemoveDevice(id.clone()))).into()),
            None,
            false,
        ));
    }
    frame(
        "Aparelhos conectados",
        Some("Onde esta conta está aberta (os aparelhos se avisam pela própria conta, cifrado)."),
        vec![
            ui::vspace(8.0),
            ui::group(linhas),
            ui::vspace(6.0),
            ui::note("Desconectar manda o aparelho sair e apagar a conta dele assim que ficar on-line. Se alguém roubou suas 12 palavras, isso não basta: crie uma conta nova e avise seus contatos."),
        ],
    )
}

/* ------------------------------------------------------------ perfil */

fn profile(m: &Main) -> El<'_> {
    let f = &m.forms;
    let foto = m.pic(m.me.pic.as_deref());
    let tem_foto = foto.is_some();
    let nome = if f.prof_name.trim().is_empty() { m.me.name.clone() } else { f.prof_name.clone() };
    let avatar = button(ui::avatar(&nome, f.prof_tone, foto, 104.0)).padding(0).style(t::ghost).on_press(Msg::ProfPhoto);
    let mut botoes = row![ui::secondary(Some(Icon::Camera), if tem_foto { "Mudar foto" } else { "Colocar foto" }, Some(Msg::ProfPhoto))].spacing(8);
    if tem_foto {
        botoes = botoes.push(ui::danger_text(Some(Icon::Trash), "Remover foto", Some(Msg::ProfPhotoRemove)));
    }
    let tons = row((0..6u8)
        .map(|i| button(Space::new().width(28).height(28)).padding(0).style(t::swatch(t::tone(i), i == f.prof_tone)).on_press(Msg::ProfTone(i)).into())
        .collect::<Vec<El<'_>>>())
    .spacing(10);
    let mudou = f.prof_name.trim() != m.me.name || f.prof_note.trim() != m.me.note || f.prof_tone != m.me.tone;
    frame(
        "Seu perfil",
        Some("Seus contatos e grupos recebem as mudanças de forma cifrada. Nada fica público."),
        vec![
            ui::vspace(10.0),
            banner_view(m, m.me.banner.as_deref(), m.me.tone, 640.0),
            row![
                ui::secondary(Some(Icon::Image), if m.me.banner.is_some() { "Mudar banner" } else { "Colocar banner" }, Some(Msg::BannerPick)),
                if m.me.banner.is_some() { El::from(ui::danger_text(Some(Icon::Trash), "Remover banner", Some(Msg::BannerRemove))) } else { ui::hspace(0.0) },
            ]
            .spacing(8)
            .into(),
            ui::vspace(10.0),
            row![avatar, column![botoes, ui::note("A foto é recortada em quadrado e reduzida antes de sair do computador.")].spacing(10)]
                .spacing(22)
                .align_y(Alignment::Center)
                .into(),
            ui::label("Cor do avatar"),
            tons.into(),
            ui::vspace(8.0),
            ui::field("Nome", ui::input("Seu nome", &f.prof_name, Msg::ProfName).on_submit(Msg::ProfSave)),
            ui::vspace(4.0),
            ui::field("Recado", ui::input("Disponível para conversar", &f.prof_note, Msg::ProfNote).on_submit(Msg::ProfSave)),
            ui::vspace(10.0),
            row![ui::primary(Some(Icon::Check), "Salvar", mudou.then_some(Msg::ProfSave))].into(),
            ui::label("Jogos favoritos"),
            ui::note("Aparecem no seu perfil para os seus contatos, como no Discord. Até 6."),
            games_chips(&m.me.games, true),
            row![
                ui::input("Nome do jogo", &f.game, Msg::GameInput).on_submit(Msg::GameAdd).width(Length::Fill),
                ui::primary(Some(Icon::Plus), "Adicionar", (!f.game.trim().is_empty() && m.me.games.len() < 6).then_some(Msg::GameAdd)),
            ]
            .spacing(8)
            .into(),
            ui::label("Atividade"),
            ui::group(vec![ui::setting(
                Some(Icon::Music),
                "Mostrar a música do Spotify",
                Some("Seus contatos veem o que está tocando no Spotify deste computador, como no Discord.".into()),
                Trail::Switch(m.listen_share, Msg::ListenShare),
                None,
                false,
            )]),
            match &m.now_playing {
                Some((t, _)) => {
                    let quem = if t.artist.is_empty() { t.title.clone() } else { format!("{} — {}", t.title, t.artist) };
                    ui::note(&if m.listen_share { format!("Mostrando agora: {quem}") } else { format!("Tocando agora (não está sendo mostrado): {quem}") })
                }
                None => ui::note("Nada tocando no Spotify deste computador agora. Pausado, a música some do seu perfil."),
            },
        ],
    )
}

/// Banner do perfil (3:1). Sem imagem, uma faixa na cor do avatar.
pub fn banner_view<'a>(m: &'a Main, url: Option<&str>, tone: u8, largura: f32) -> El<'a> {
    let altura = largura / 3.0;
    match m.pic(url) {
        Some(h) => iced::widget::image(h)
            .width(largura)
            .height(altura)
            .content_fit(iced::ContentFit::Cover)
            .border_radius(12.0)
            .into(),
        None => container(Space::new())
            .width(largura)
            .height(altura)
            .style(t::rounded(t::mix(t::tone(tone), t::accent(), 0.25), 12.0))
            .into(),
    }
}

/// Jogos favoritos como etiquetas. `editavel` põe o X para tirar.
pub fn games_chips<'a>(jogos: &[String], editavel: bool) -> El<'a> {
    if jogos.is_empty() {
        return if editavel { ui::note("Nenhum jogo ainda.") } else { Space::new().into() };
    }
    let itens: Vec<El<'a>> = jogos
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let mut r = row![icon(Icon::Game, 16.0, t::accent()), text(g.clone()).size(13.5).font(SEMI).color(t::FG)].spacing(6).align_y(Alignment::Center);
            if editavel {
                r = r.push(ui::icon_button_sm(Icon::X, "Tirar", Some(Msg::GameRemove(i))));
            }
            container(r).padding(if editavel { [2, 4] } else { [6, 10] }).style(t::rounded(t::surface_hi(), 999.0)).into()
        })
        .collect();
    iced::widget::Row::with_children(itens).spacing(8).wrap().into()
}

/* ------------------------------------------------------------ código */

fn my_code(m: &Main) -> El<'_> {
    let npub = m.engine.npub();
    let qr = qr_box(m, 6.0);
    frame(
        "Meu código",
        Some("Quem tiver este código pode te adicionar. Ele não revela nada além da sua chave pública."),
        vec![
            ui::vspace(10.0),
            row![
                qr,
                column![
                    short_code_box(m),
                    text("Código completo").size(13).font(SEMI).color(t::FG_DIM),
                    container(text(npub.clone()).size(13).wrapping(iced::widget::text::Wrapping::Glyph).color(t::FG)).padding(12).width(Length::Fill).style(t::rounded(t::surface(), 10.0)),
                    row![ui::primary(Some(Icon::Copy), "Copiar código", Some(Msg::CopyMyCode)), ui::secondary(Some(Icon::Share), "Copiar convite", Some(Msg::CopyInvite))].spacing(8),
                    ui::note("Pessoalmente: a pessoa lê o QR com o celular (Adicionar contato → Ler QR). À distância: mande o convite pelo app que preferir."),
                ]
                .spacing(10)
                .width(Length::Fill)
            ]
            .spacing(24)
            .align_y(Alignment::Center)
            .into(),
        ],
    )
}

/// O código curto (8 caracteres, vale 7 dias), ou o botão de gerar.
pub fn short_code_box(m: &Main) -> El<'_> {
    match &m.short_code {
        Some((c, vence)) => {
            let data = chrono::DateTime::from_timestamp(*vence, 0)
                .map(|d| d.with_timezone(&chrono::Local).format("%d/%m").to_string())
                .unwrap_or_default();
            container(
                row![
                    column![
                        text("Código curto").size(12).font(SEMI).color(t::FG_DIM),
                        text(c.clone()).size(26).font(iced::Font::MONOSPACE).color(t::FG),
                        text(format!("Vale até {data}. Quem tiver o app do PC digita em Adicionar contato.")).size(12).color(t::FG_FAINT),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    ui::primary(Some(Icon::Copy), "Copiar", Some(Msg::CopyShortCode)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding(12)
            .width(Length::Fill)
            .style(t::rounded(t::surface_hi(), 12.0))
            .into()
        }
        None => container(
            row![
                column![
                    text("Código curto").size(13).font(SEMI).color(t::FG),
                    text("8 letras e números no lugar do código longo. Vale 7 dias.").size(12).color(t::FG_DIM)
                ]
                .spacing(2)
                .width(Length::Fill),
                ui::secondary(Some(Icon::Sparkle), if m.short_code_busy { "Gerando…" } else { "Gerar" }, (!m.short_code_busy).then_some(Msg::ShortCodeCreate)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(12)
        .width(Length::Fill)
        .style(t::rounded(t::surface_hi(), 12.0))
        .into(),
    }
}

/// O QR do convite, em fundo branco (os leitores precisam do contraste).
pub fn qr_box(m: &Main, cell: f32) -> El<'_> {
    match &m.qr {
        Some(d) => container(qr_code(d).cell_size(cell).style(|_| qr_code::Style { cell: iced::Color::BLACK, background: iced::Color::WHITE }))
            .padding(14)
            .style(t::rounded(iced::Color::WHITE, 16.0))
            .into(),
        None => ui::note("Não deu para gerar o QR."),
    }
}

/* ------------------------------------------------------------ chave */

fn recovery(m: &Main) -> El<'_> {
    let Some(k) = &m.forms.keys else {
        return frame("Chave de recuperação", None, vec![ui::secondary(Some(Icon::Key), "Mostrar", Some(Msg::OpenRecovery)).into()]);
    };
    let mut corpo: Vec<El<'_>> = vec![ui::vspace(8.0)];
    match &k.words {
        Some(w) => {
            let ver = m.forms.reveal_words;
            let palavras: Vec<&str> = w.split_whitespace().collect();
            let grade = column(palavras
                .chunks(4)
                .enumerate()
                .map(|(l, c)| {
                    row(c.iter()
                        .enumerate()
                        .map(|(i, p)| {
                            let n = l * 4 + i + 1;
                            container(row![
                                text(format!("{n}")).size(11.5).color(t::FG_FAINT).width(20),
                                text(if ver { p.to_string() } else { "••••••".to_string() }).size(15).font(SEMI).color(t::FG)
                            ]
                            .align_y(Alignment::Center))
                            .padding([10, 12])
                            .width(Length::Fill)
                            .style(t::rounded(t::surface(), 10.0))
                            .into()
                        })
                        .collect::<Vec<El<'_>>>())
                    .spacing(8)
                    .into()
                })
                .collect::<Vec<El<'_>>>())
            .spacing(8);
            corpo.push(grade.into());
            corpo.push(ui::secondary(Some(if ver { Icon::EyeOff } else { Icon::Eye }), if ver { "Esconder palavras" } else { "Mostrar palavras" }, Some(Msg::RevealWords)).into());
        }
        None => corpo.push(ui::note("Esta conta foi restaurada pela chave secreta, sem palavras. Guarde a chave abaixo.")),
    }
    corpo.push(ui::label("Usar em outro aparelho"));
    corpo.push(ui::note(
        "No celular ou em outro PC, escolha “Já tenho uma conta” e digite estas palavras. Com este aparelho aberto, o outro recebe seus contatos e grupos sozinho. As mensagens novas chegam nos dois; para levar o histórico antigo, use Armazenamento → Fazer backup.",
    ));
    corpo.push(ui::label("Chave secreta (avançado)"));
    corpo.push(
        container(
            row![
                text(if m.forms.reveal_nsec { k.nsec.clone() } else { "nsec1••••••••••••••••••••••••".to_string() })
                    .size(13)
                    .wrapping(iced::widget::text::Wrapping::Glyph)
                    .color(t::FG)
                    .width(Length::Fill),
                ui::icon_button_sm(if m.forms.reveal_nsec { Icon::EyeOff } else { Icon::Eye }, "Mostrar chave", Some(Msg::RevealNsec)),
                ui::icon_button_sm(Icon::Copy, "Copiar chave", Some(Msg::CopyNsec)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([8, 12])
        .style(t::rounded(t::surface(), 10.0))
        .into(),
    );
    corpo.push(ui::vspace(10.0));
    corpo.push(row![ui::primary(Some(Icon::Check), "Já anotei", Some(Msg::WordsDone))].into());
    frame(
        "Chave de recuperação",
        Some("Quem tiver estas palavras controla sua conta. Anote no papel, na ordem, e guarde longe do computador. Nunca mande para ninguém, nem para o suporte."),
        corpo,
    )
}

/* ------------------------------------------------------------ privacidade */

fn privacy(m: &Main) -> El<'_> {
    let s = &m.settings;
    let pin = m.pin.as_ref();
    let delays = |d: u32| match d {
        0 => "Imediatamente",
        60 => "Após 1 minuto",
        300 => "Após 5 minutos",
        900 => "Após 15 minutos",
        _ => "Imediatamente",
    };
    let prev = match s.notif_preview {
        NotifPreview::Full => "Nome e mensagem",
        NotifPreview::Name => "Só o nome",
        NotifPreview::None => "Nada",
    };
    let turn = !s.turn.url.trim().is_empty();
    let n_bloq = m.blocked().len();

    let mut bloqueio = vec![ui::setting(
        Some(Icon::Lock),
        "PIN de bloqueio",
        Some(if pin.is_some() { "Ligado · pede o PIN ao abrir o app".into() } else { "Desligado · o app abre sem senha".into() }),
        Trail::Switch(pin.is_some(), |_| Msg::PinToggle),
        None,
        false,
    )];
    if let Some(p) = pin {
        bloqueio.push(ui::setting(None, "Trocar PIN", None, Trail::Chevron, Some(Msg::PinChange), false));
        bloqueio.push(ui::setting(None, "Pedir o PIN", Some("Depois de quanto tempo fora do app".into()), Trail::Value(delays(p.delay).into()), Some(Msg::Choose(ChooseKind::PinDelay)), false));
        bloqueio.push(ui::setting(None, "Tirar o PIN", Some("O app volta a abrir sem senha".into()), Trail::None, Some(Msg::PinToggle), true));
    }
    let capturas_sub = if crate::platform::can_protect_window() {
        "Esconde o conteúdo da janela nas capturas e gravações de tela".to_string()
    } else {
        "Disponível no Windows 10 (2004) ou mais novo".to_string()
    };
    bloqueio.push(if crate::platform::can_protect_window() {
        ui::setting(Some(Icon::EyeOff), "Bloquear capturas de tela", Some(capturas_sub), Trail::Switch(s.lock_screenshots, |v| Msg::Toggle(Toggle::LockScreenshots, v)), None, false)
    } else {
        ui::setting(Some(Icon::EyeOff), "Bloquear capturas de tela", Some(capturas_sub), Trail::SwitchOff(false), None, false)
    });

    frame(
        "Privacidade e segurança",
        None,
        vec![
            ui::label("Bloqueio do app"),
            ui::group(bloqueio),
            ui::label("Mensagens"),
            ui::group(vec![
                ui::setting(
                    Some(Icon::Checks),
                    "Confirmação de leitura",
                    Some("Se desligar, você também não vê quando leram".into()),
                    Trail::Switch(s.read_receipts, |v| Msg::Toggle(Toggle::ReadReceipts, v)),
                    None,
                    false,
                ),
                ui::setting(Some(Icon::Chat), "Mostrar quando estou digitando", None, Trail::Switch(s.typing, |v| Msg::Toggle(Toggle::Typing, v)), None, false),
                ui::setting(
                    Some(Icon::Share),
                    "Prévia dos links que eu mando",
                    Some("Seu computador abre o site para pegar título e imagem; quem recebe não acessa nada".into()),
                    Trail::Switch(s.link_previews, |v| Msg::Toggle(Toggle::LinkPreviews, v)),
                    None,
                    false,
                ),
                ui::setting(
                    Some(Icon::Eye),
                    "Mostrar quando estou online",
                    Some("Se desligar, você também não vê quem está online".into()),
                    Trail::Switch(s.presence, |v| Msg::Toggle(Toggle::Presence, v)),
                    None,
                    false,
                ),
                ui::setting(
                    Some(Icon::Timer),
                    "Mensagens temporárias para novas conversas",
                    None,
                    Trail::Value(jm_core::engine::ttl_label(s.ttl_default)),
                    Some(Msg::Choose(ChooseKind::TtlDefault)),
                    false,
                ),
                ui::setting(Some(Icon::Bell), "Prévia nas notificações", None, Trail::Value(prev.into()), Some(Msg::Choose(ChooseKind::NotifPreview)), false),
            ]),
            ui::label("Chamadas"),
            ui::group(vec![
                ui::setting(
                    Some(Icon::Phone),
                    "Quem pode me ligar",
                    None,
                    Trail::Value(if s.calls_from == CallsFrom::Nobody { "Ninguém".into() } else { "Meus contatos".into() }),
                    Some(Msg::Choose(ChooseKind::CallsFrom)),
                    false,
                ),
                if turn {
                    ui::setting(
                        Some(Icon::Shield),
                        "Ocultar meu IP nas chamadas",
                        Some("Toda chamada passa pelo servidor TURN configurado".into()),
                        Trail::Switch(s.force_relay, |v| Msg::Toggle(Toggle::ForceRelay, v)),
                        None,
                        false,
                    )
                } else {
                    ui::setting(
                        Some(Icon::Shield),
                        "Ocultar meu IP nas chamadas",
                        Some("Precisa de um servidor TURN em Rede e servidores".into()),
                        Trail::SwitchOff(false),
                        Some(Msg::Page(SettingsPage::Network)),
                        false,
                    )
                },
            ]),
            ui::label("Contatos"),
            ui::group(vec![ui::setting(Some(Icon::Ban), "Bloqueados", None, Trail::Value(n_bloq.to_string()), Some(Msg::Page(SettingsPage::Blocked)), false)]),
            ui::vspace(10.0),
            ui::note("Tudo o que você manda é cifrado neste computador antes de sair. Os servidores só guardam envelopes lacrados com destinatário, sem remetente, conteúdo nem horário real. Não há telefone, e-mail, agenda de contatos nem anúncios."),
        ],
    )
}

fn blocked(m: &Main) -> El<'_> {
    let lista = m.blocked();
    let mut corpo: Vec<El<'_>> = vec![row![ui::ghost(Some(Icon::Back), "Privacidade e segurança", Some(Msg::Page(SettingsPage::Privacy)))].into()];
    if lista.is_empty() {
        corpo.push(ui::note("Ninguém bloqueado."));
    } else {
        corpo.push(ui::group(
            lista
                .into_iter()
                .map(|c| {
                    let nome = c.display_name();
                    container(
                        row![
                            ui::avatar(&nome, c.tone, m.pic_of(&c.pk), 36.0),
                            text(nome).size(14.5).font(SEMI).color(t::FG).width(Length::Fill),
                            ui::secondary(None, "Desbloquear", Some(Msg::Unblock(c.pk.clone())))
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center),
                    )
                    .padding([8, 12])
                    .into()
                })
                .collect(),
        ));
    }
    corpo.push(ui::note("Quem está bloqueado não consegue te mandar mensagens nem ligar, e não fica sabendo que foi bloqueado."));
    frame("Bloqueados", None, corpo)
}

/* ------------------------------------------------------------ avisos */

fn notifications(m: &Main) -> El<'_> {
    let s = &m.settings;
    frame(
        "Notificações",
        None,
        vec![
            ui::vspace(8.0),
            ui::group(vec![
                ui::setting(
                    Some(Icon::Bell),
                    "Notificações",
                    Some("Avisos de mensagens e chamadas quando a janela não está em foco".into()),
                    Trail::Switch(s.notifications, |v| Msg::Toggle(Toggle::Notifications, v)),
                    None,
                    false,
                ),
                ui::setting(Some(Icon::Volume), "Sons no app", Some("Toque curto quando chega mensagem".into()), Trail::Switch(s.sounds, |v| Msg::Toggle(Toggle::Sounds, v)), None, false),
            ]),
            if crate::tray::SUPPORTED {
                ui::group(vec![
                    ui::setting(
                        Some(Icon::Minimize),
                        "Continuar ligado ao fechar a janela",
                        Some("Fica na bandeja, perto do relógio, recebendo mensagens e chamadas".into()),
                        Trail::Switch(m.tray_close, Msg::TrayClose),
                        None,
                        false,
                    ),
                    ui::setting(
                        Some(Icon::Refresh),
                        "Abrir ao ligar o computador",
                        Some("Começa escondido na bandeja".into()),
                        Trail::Switch(m.autostart, Msg::TrayAutostart),
                        None,
                        false,
                    ),
                ])
            } else {
                ui::vspace(0.0)
            },
            ui::vspace(6.0),
            ui::note(if crate::tray::SUPPORTED {
                "O que aparece no texto da notificação se ajusta em Privacidade e segurança → Prévia nas notificações. Com a janela fechada, o J-Menseger continua na bandeja e avisa do que chegar; para sair de vez, use o ícone da bandeja."
            } else {
                "O que aparece no texto da notificação se ajusta em Privacidade e segurança → Prévia nas notificações. No computador, o app precisa estar aberto (pode ficar minimizado) para receber."
            }),
        ],
    )
}

/* ------------------------------------------------------------ aparência */

fn appearance(m: &Main) -> El<'_> {
    let atual = m.look.accent.to_uppercase();
    let nome_atual = t::PRESETS.iter().find(|(_, h)| h.eq_ignore_ascii_case(&atual)).map(|(n, _)| *n).unwrap_or("Personalizada");
    let cores = row(t::PRESETS
        .iter()
        .map(|(nome, h)| {
            let c = t::from_hex(h).unwrap_or(t::accent());
            let sel = h.eq_ignore_ascii_case(&atual);
            column![
                button(if sel { container(icon(Icon::Check, 18.0, if t::hex(c) == "#EAB308" { iced::Color::BLACK } else { iced::Color::WHITE })).center(40) } else { container(Space::new()).center(40) })
                    .padding(0)
                    .style(t::swatch(c, sel))
                    .on_press(Msg::Accent(h.to_string())),
                text(*nome).size(11.5).color(if sel { t::FG } else { t::FG_DIM })
            ]
            .spacing(5)
            .align_x(Alignment::Center)
            .width(58)
            .into()
        })
        .collect::<Vec<El<'_>>>())
    .spacing(6)
    .wrap();
    let fundos = row(Backdrop::all()
        .into_iter()
        .map(|b| {
            let sel = m.look.backdrop == b;
            button(
                column![
                    container(
                        row![
                            container(Space::new()).width(12).height(Length::Fill).style(t::fill(backdrop_preview(b).0)),
                            container(Space::new()).width(30).height(Length::Fill).style(t::fill(backdrop_preview(b).1)),
                            container(container(Space::new()).width(28).height(10).style(t::rounded(t::accent(), 5.0))).center(Length::Fill).style(t::fill(backdrop_preview(b).2)),
                        ]
                        .height(60)
                    )
                    .clip(true)
                    .style(t::rounded(backdrop_preview(b).2, 10.0)),
                    text(b.label()).size(13).font(SEMI).color(t::FG)
                ]
                .spacing(8)
                .width(150),
            )
            .padding(8)
            .style(t::option(sel))
            .on_press(Msg::SetBackdrop(b))
            .into()
        })
        .collect::<Vec<El<'_>>>())
    .spacing(10)
    .wrap();
    frame(
        "Aparência",
        Some("As cores valem só para este computador."),
        vec![
            ui::label(&format!("Cor de destaque · {nome_atual}")),
            cores.into(),
            ui::vspace(6.0),
            row![
                text("Outra cor").size(13).color(t::FG_DIM).width(80),
                slider(0.0..=359.0, m.forms.hue, Msg::AccentHue).step(1.0_f32).width(Length::Fill),
                container(Space::new()).width(26).height(26).style(t::rounded(t::accent(), 13.0)),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
            ui::label("Fundo"),
            fundos.into(),
            ui::label("Prévia"),
            preview(m),
            ui::label("Movimento"),
            ui::group(vec![ui::setting(
                Some(Icon::Wave),
                "Animações",
                Some("Janelas e menus deslizando, barra da esquerda, aviso de chamada. Desligue se preferir tudo parado.".into()),
                Trail::Switch(m.motion.enabled, Msg::SetAnimations),
                None,
                false,
            )]),
            ui::vspace(10.0),
            row![ui::secondary(Some(Icon::Refresh), "Restaurar cores de fábrica", Some(Msg::LookReset))].into(),
        ],
    )
}

fn backdrop_preview(b: Backdrop) -> (iced::Color, iced::Color, iced::Color) {
    match b {
        Backdrop::Grafite => (t::rgb(0x0a, 0x0b, 0x0d), t::rgb(0x11, 0x13, 0x16), t::rgb(0x16, 0x18, 0x1c)),
        Backdrop::Preto => (t::rgb(0, 0, 0), t::rgb(0x07, 0x07, 0x08), t::rgb(0x0b, 0x0b, 0x0c)),
        Backdrop::Noite => (t::rgb(0x08, 0x0b, 0x14), t::rgb(0x0e, 0x13, 0x20), t::rgb(0x12, 0x18, 0x28)),
    }
}

fn preview(m: &Main) -> El<'_> {
    let _ = m;
    container(
        column![
            row![container(column![text("Oi! Tudo bem?").size(14).color(t::FG), container(text("10:41").size(11).color(t::with_alpha(t::FG, 0.55))).align_right(Length::Fill)].spacing(2)).padding([8, 11]).max_width(260).style(t::bubble(false)), ui::fill()],
            row![
                ui::fill(),
                container(column![
                    text("Tudo ótimo 🎉").size(14).color(t::FG),
                    container(row![text("10:42").size(11).color(t::with_alpha(t::FG, 0.55)), icon(Icon::Checks, 14.0, t::READ_BLUE)].spacing(4)).align_right(Length::Fill)
                ]
                .spacing(2))
                .padding([8, 11])
                .max_width(260)
                .style(t::bubble(true))
            ],
            row![ui::primary(None, "Botão", Some(Msg::Noop)), ui::badge(3)].spacing(10).align_y(Alignment::Center),
        ]
        .spacing(8),
    )
    .padding(16)
    .style(t::rounded(t::main_bg(), 14.0))
    .into()
}

/* ------------------------------------------------------------ soundboard */

fn soundboard(m: &Main) -> El<'_> {
    let lib = m.sounds.library();
    let escondidos = m.sounds.hidden();
    let mut corpo: Vec<El<'_>> = vec![
        ui::vspace(6.0),
        row![ui::primary(Some(Icon::Plus), "Adicionar som", Some(Msg::SbAdd))].into(),
        ui::vspace(4.0),
        row![
            icon(Icon::Volume, 18.0, t::FG_DIM),
            slider(0.0..=1.0, m.sounds.volume(), Msg::Volume).step(0.01_f32).width(Length::Fill),
            text(format!("{}%", (m.sounds.volume() * 100.0).round())).size(12.5).color(t::FG_DIM).width(40)
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into(),
        ui::label(&format!("Na sua lista · {}", lib.len())),
    ];
    if lib.is_empty() {
        corpo.push(ui::note("Nenhum som na lista. Adicione um ou restaure os de fábrica."));
    } else {
        corpo.push(ui::group(
            lib.into_iter()
                .map(|s| {
                    let mut r = row![
                        button(container(icon(Icon::Play, 14.0, t::accent_ink())).center(14)).padding(9).style(t::call_btn(t::CallBtn::Accept)).on_press(Msg::SbPlay(s.id.clone())),
                        container(text(s.emoji.clone()).size(24)).center_x(34),
                        column![
                            text(s.name.clone()).size(14.5).font(SEMI).color(t::FG),
                            text(format!("{} · {}", if s.builtin { "Gravação de fábrica" } else { "Criado por você" }, crate::v_chat::fmt_dur(s.dur))).size(12).color(t::FG_DIM)
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center);
                    if !s.builtin {
                        r = r.push(ui::icon_button_sm(Icon::Edit, "Editar", Some(Msg::SbEdit(s.id.clone()))));
                        r = r.push(ui::icon_button_sm(Icon::Trash, "Excluir", Some(Msg::Confirm(ConfirmKind::DeleteSound(s.id.clone())))));
                    } else {
                        r = r.push(ui::icon_button_sm(Icon::Trash, "Tirar da lista", Some(Msg::SbHide(s.id.clone()))));
                    }
                    container(r).padding([6, 12]).into()
                })
                .collect(),
        ));
    }
    if !escondidos.is_empty() {
        corpo.push(ui::label(&format!("Removidos · {}", escondidos.len())));
        corpo.push(ui::note(escondidos.iter().map(|s| format!("{} {}", s.emoji, s.name)).collect::<Vec<_>>().join(" · ")));
        corpo.push(ui::vspace(6.0));
        corpo.push(row![ui::secondary(Some(Icon::Refresh), "Restaurar sons de fábrica", Some(Msg::SbRestore))].into());
    }
    corpo.push(ui::vspace(10.0));
    corpo.push(ui::note("Os sons de fábrica são gravações reais de instrumentos e plateia, com licença livre. Os sons que você cria ficam neste computador. Numa chamada, quem está com você recebe o som cifrado para poder ouvir."));
    frame("Soundboard", Some("Toque sons nas chamadas ou mande nas conversas. Dá para soltar um arquivo de áudio na janela para criar um som."), corpo)
}

/* ------------------------------------------------------------ rede */

fn network(m: &Main) -> El<'_> {
    let s = &m.settings;
    let abertos = m.open_relays();
    let linhas: Vec<El<'_>> = s
        .relays
        .iter()
        .map(|u| {
            let ok = m.relays.iter().any(|r| r.url.trim_end_matches('/') == u.trim_end_matches('/') && r.connected);
            container(
                row![
                    container(Space::new()).width(9).height(9).style(t::rounded(if ok { t::accent() } else { t::FG_FAINT }, 5.0)),
                    column![text(u.clone()).size(14).color(t::FG), text(if ok { "Conectado" } else { "Desconectado" }).size(12).color(t::FG_DIM)].spacing(1).width(Length::Fill),
                    ui::icon_button_sm(Icon::X, "Remover", Some(Msg::RelayRemove(u.clone()))),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .into()
        })
        .collect();
    let f = &m.forms;
    frame(
        "Rede e servidores",
        Some("As mensagens vão para vários servidores ao mesmo tempo: um servidor fora do ar não derruba o app. Eles só veem envelopes cifrados."),
        vec![
            ui::label(&format!("Servidores · conectado a {abertos} de {}", s.relays.len())),
            ui::group(linhas),
            ui::vspace(6.0),
            row![ui::input("wss://servidor.com", &f.relay, Msg::RelayInput).on_submit(Msg::RelayAdd).width(Length::Fill), ui::primary(Some(Icon::Plus), "Adicionar", Some(Msg::RelayAdd))]
                .spacing(8)
                .into(),
            row![ui::ghost(Some(Icon::Refresh), "Restaurar servidores de fábrica", Some(Msg::RelaysReset))].into(),
            ui::label("Servidor TURN (chamadas)"),
            ui::note("Em algumas redes a conexão direta falha. Um servidor TURN seu repassa a chamada, ainda cifrada, e permite ocultar seu IP do outro lado."),
            ui::vspace(4.0),
            ui::field("Endereço", ui::input("turn:servidor.com:3478", &f.turn_url, Msg::TurnUrl)),
            row![
                container(ui::field("Usuário", ui::input("usuário", &f.turn_user, Msg::TurnUser))).width(Length::Fill),
                container(ui::field("Senha", ui::input("senha", &f.turn_pass, Msg::TurnPass).secure(true))).width(Length::Fill)
            ]
            .spacing(10)
            .into(),
            ui::vspace(4.0),
            row![ui::primary(Some(Icon::Check), "Salvar TURN", Some(Msg::TurnSave))].into(),
        ],
    )
}

/* ------------------------------------------------------------ voz e áudio */

fn audio(m: &Main) -> El<'_> {
    use crate::update::PADRAO;
    let a = &m.audio;
    let entrada = a.input.clone().unwrap_or_else(|| PADRAO.to_string());
    let saida = a.output.clone().unwrap_or_else(|| PADRAO.to_string());
    let lista = |v: &Vec<String>, sel: String, f: fn(String) -> Msg| -> El<'_> {
        iced::widget::pick_list(v.clone(), Some(sel), f).width(Length::Fill).padding([9, 12]).text_size(14).into()
    };
    let testando = m.mic_test.is_some();
    let eco = m.mic_test.as_ref().is_some_and(|t| t.eco.is_some());
    let mut teste = column![
        row![
            ui::secondary(Some(if testando { Icon::Stop } else { Icon::Mic }), if testando { "Parar teste" } else { "Testar microfone" }, Some(Msg::MicTestToggle)),
            row![text("Ouvir minha voz (use fone)").size(13).color(t::FG_DIM), iced::widget::toggler(eco).on_toggle(Msg::MicEcho).size(18)].spacing(8).align_y(Alignment::Center),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    ]
    .spacing(10);
    if let Some(tt) = &m.mic_test {
        let pico = tt.levels.iter().rev().take(8).copied().fold(0.0f32, f32::max);
        teste = teste.push(container(ui::level_bars(&tt.levels, 80, t::accent())).padding([4, 0]));
        teste = teste.push(ui::note(if pico > 0.02 {
            "O microfone está captando. Fale normalmente: as barras devem subir."
        } else {
            "Nada chegando ainda. Fale perto do microfone; se continuar parado, veja as dicas abaixo."
        }));
    }
    if let Some(e) = &m.mic_error {
        teste = teste.push(row![icon(Icon::Alert, 15.0, t::DANGER), text(e.clone()).size(13).color(t::DANGER)].spacing(6).align_y(Alignment::Center));
    }
    let n_mic = m.audio_inputs.len().saturating_sub(1);
    frame(
        "Voz e áudio",
        Some("Valem para chamadas, salas de voz, mensagens de voz e sons."),
        vec![
            ui::label("Microfone"),
            lista(&m.audio_inputs, entrada, Msg::AudioInput),
            ui::note(format!("{n_mic} microfone(s) encontrado(s) neste computador.")),
            ui::vspace(6.0),
            row![
                text("Volume do microfone").size(13.5).color(t::FG_DIM).width(170),
                slider(0.0..=3.0, a.input_gain, Msg::AudioGain).step(0.05_f32).width(Length::Fill),
                text(format!("{:.0}%", a.input_gain * 100.0)).size(12.5).color(t::FG_DIM).width(48),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
            ui::vspace(6.0),
            teste.into(),
            ui::label("Saída de som"),
            lista(&m.audio_outputs, saida, Msg::AudioOutput),
            ui::vspace(6.0),
            row![
                text("Volume das chamadas").size(13.5).color(t::FG_DIM).width(170),
                slider(0.0..=2.0, a.output_volume, Msg::AudioVolume).step(0.05_f32).width(Length::Fill),
                text(format!("{:.0}%", a.output_volume * 100.0)).size(12.5).color(t::FG_DIM).width(48),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
            row![ui::secondary(Some(Icon::Volume), "Tocar som de teste", Some(Msg::SpeakerTest)), ui::ghost(Some(Icon::Refresh), "Atualizar lista", Some(Msg::AudioRefresh))]
                .spacing(8)
                .into(),
            ui::note("Mudanças de microfone e volume valem a partir da próxima chamada ou sala."),
            ui::label("Microfone não funciona?"),
            column![
                bullet("No Windows: Configurações → Privacidade e segurança → Microfone. Ligue \"Acesso ao microfone\" e \"Permitir que aplicativos da área de trabalho acessem o microfone\"."),
                bullet("Escolha o microfone pelo nome aqui em cima em vez de \"Padrão do sistema\" (o padrão às vezes é a webcam ou um dispositivo desligado)."),
                bullet("Se outro programa (Discord, OBS, jogo) estiver com o microfone em modo exclusivo, feche-o e teste de novo."),
                bullet("Se as barras sobem mas o outro lado não ouve, aumente o volume do microfone."),
            ]
            .spacing(6)
            .into(),
        ],
    )
}

/* ------------------------------------------------------------ GIFs */

fn gifs(m: &Main) -> El<'_> {
    use crate::gifs::Provider;
    let disponiveis = crate::gifs::available();
    let mut corpo: Vec<El<'_>> = Vec::new();
    match m.gif_keys.usable() {
        Some((p, _)) => {
            corpo.push(row![icon(Icon::Check, 16.0, t::accent()), text(format!("Busca de GIFs ligada, pelo {}.", p.label())).size(14).color(t::FG)].spacing(8).align_y(Alignment::Center).into());
            if disponiveis.len() > 1 {
                corpo.push(ui::label("Serviço de busca"));
                let escolha = |p: Provider| -> El<'_> {
                    if m.gif_keys.provider == p {
                        ui::primary(Some(Icon::Check), p.label(), Some(Msg::GifProviderSet(p))).into()
                    } else {
                        ui::secondary(None, p.label(), Some(Msg::GifProviderSet(p))).into()
                    }
                };
                corpo.push(row![escolha(Provider::Tenor), escolha(Provider::Giphy)].spacing(8).into());
            }
        }
        None => {
            corpo.push(row![icon(Icon::Alert, 16.0, t::WARN), text("A busca de GIFs ainda não está disponível nesta versão.").size(14).color(t::FG)].spacing(8).align_y(Alignment::Center).into());
            corpo.push(ui::note("Mandar um GIF do computador funciona normalmente, pelo botão GIF da conversa."));
        }
    }
    corpo.push(ui::label("Privacidade"));
    corpo.push(
        column![
            bullet("O que você busca vai direto daqui para o Tenor ou o GIPHY, que veem o termo buscado e o seu endereço de internet."),
            bullet("O GIF escolhido é baixado aqui e enviado cifrado, como uma foto: quem recebe não acessa o Tenor nem o GIPHY."),
        ]
        .spacing(6)
        .into(),
    );
    frame("GIFs", Some("Busque GIFs direto na conversa, pelo botão GIF ao lado dos emojis."), corpo)
}

/* ------------------------------------------------------------ armazenamento */

fn storage(m: &Main) -> El<'_> {
    let (banco, midia) = m.storage;
    frame(
        "Armazenamento",
        None,
        vec![
            ui::label("Backup"),
            ui::group(vec![
                ui::setting(
                    Some(Icon::Download),
                    "Fazer backup das conversas",
                    Some("Arquivo cifrado com a chave da sua conta: só abre com as suas palavras de recuperação. Fotos e áudios não entram.".into()),
                    Trail::None,
                    Some(Msg::BackupExport),
                    false,
                ),
                ui::setting(
                    Some(Icon::Upload),
                    "Restaurar backup",
                    Some("Junta as mensagens, contatos e grupos do arquivo com os daqui. Nada é apagado.".into()),
                    Trail::None,
                    Some(Msg::BackupImport),
                    false,
                ),
            ]),
            ui::vspace(8.0),
            ui::group(vec![
                ui::setting(Some(Icon::Db), "Espaço usado pelo app", None, Trail::Custom(text(t::bytes_label(banco)).size(13.5).color(t::FG_DIM).into()), None, false),
                ui::setting(Some(Icon::Image), "Fotos e áudios", None, Trail::Custom(text(t::bytes_label(midia)).size(13.5).color(t::FG_DIM).into()), None, false),
            ]),
            ui::vspace(8.0),
            ui::group(vec![
                ui::setting(
                    Some(Icon::Trash),
                    "Apagar fotos e áudios",
                    Some("Os textos continuam. A mídia não volta.".into()),
                    Trail::None,
                    Some(Msg::Confirm(ConfirmKind::ClearMedia)),
                    true,
                ),
                ui::setting(
                    Some(Icon::Phone),
                    "Limpar histórico de chamadas",
                    Some(format!("{} chamadas registradas", m.call_log.len())),
                    Trail::None,
                    (!m.call_log.is_empty()).then(|| Msg::Confirm(ConfirmKind::ClearCalls)),
                    true,
                ),
            ]),
        ],
    )
}

/* ------------------------------------------------------------ sobre */

fn about(m: &Main) -> El<'_> {
    frame(
        "Sobre o J-Menseger",
        None,
        vec![
            ui::vspace(8.0),
            container(
                column![
                    logo(96.0),
                    row![text("J-").size(28).font(BOLD).color(t::accent()), text("Menseger").size(28).font(BOLD).color(t::FG)],
                    text(format!("Versão {} · para computador", env!("CARGO_PKG_VERSION"))).size(13.5).color(t::FG_DIM),
                    row![logo(56.0), logo(38.0), logo(26.0)].spacing(14).align_y(Alignment::End),
                ]
                .spacing(10)
                .align_x(Alignment::Center),
            )
            .center_x(Length::Fill)
            .padding(10)
            .into(),
            row![
                ui::secondary(Some(Icon::Refresh), "Procurar atualizações", Some(Msg::UpdateCheck)),
                text(m.update.as_ref().map(|p| format!("Versão {} disponível: use o botão Atualizar na barra lateral.", p.versao)).or_else(|| m.update_msg.clone()).unwrap_or_default()).size(13).color(t::FG_DIM),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
            ui::label("Como sua privacidade é protegida"),
            column![
                bullet("Sua conta é um par de chaves criado neste computador. Não pedimos telefone, e-mail nem agenda."),
                bullet("Cada mensagem é cifrada (NIP-44) e lacrada em um envelope com chave descartável (NIP-59). Os servidores não sabem quem enviou nem o que diz."),
                bullet("Chamadas vão direto entre os aparelhos, cifradas com DTLS-SRTP."),
                bullet("Fotos perdem os metadados (como localização) antes do envio."),
                bullet("A chave da conta fica no banco de dados deste computador, sem cifra extra: quem acessar sua conta do sistema consegue lê-la. Use o PIN e proteja o acesso ao computador."),
                bullet("Sem anúncios e sem rastreamento."),
            ]
            .spacing(6)
            .into(),
            ui::label("Código aberto usado"),
            ui::note("nostr-sdk (MIT) · iced (MIT) · webrtc-rs (MIT/Apache 2.0) · libopus (BSD) · libvpx (BSD) · symphonia (MPL 2.0) · image (MIT/Apache 2.0) · rusqlite e SQLite (MIT, domínio público) · cpal (Apache 2.0) · tokio (MIT) · rfd (MIT) · notify-rust (MIT/Apache 2.0). Sons de fábrica com licença livre (ver LICENCA.txt)."),
        ],
    )
}

fn bullet<'a>(s: &str) -> El<'a> {
    row![text("•").size(14).color(t::accent()), text(s.to_string()).size(13.5).color(t::FG_DIM)].spacing(8).into()
}
