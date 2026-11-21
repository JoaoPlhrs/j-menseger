//! J-Menseger para computador.
//!
//! Aplicativo nativo: a interface é desenhada pela GPU (ou por software, quando
//! não há GPU), sem navegador embutido em nenhuma camada.

// No Windows, um programa comum abre uma janela de console preta junto com a
// janela do aplicativo. Esta linha diz ao sistema que aqui só existe a
// interface gráfica. Fica valendo apenas na versão final: durante o
// desenvolvimento o console é útil para ler os registros.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod atualizar;
mod bridge;
mod calls;
mod canvas;
mod canvas_setup;
mod gifs;
mod linkprev;
mod icons;
mod motion;
mod notif;
mod nowplaying;
mod platform;
mod setup;
mod sounds;
mod state;
mod theme;
mod tray;
mod ui;
mod update;
mod v_call;
mod v_chat;
mod v_group;
mod v_main;
mod v_modal;
mod v_onboard;
mod v_settings;

use app::{App, Msg};
use iced::{Size, Task};

fn main() -> iced::Result {
    let db = db_path();
    // Chamado como instalador (…instalador.exe, --instalar) ou desinstalador.
    if let Some(modo) = setup::mode() {
        return setup::run(modo, db.parent().map(|p| p.to_path_buf()).unwrap_or_default());
    }
    if let Some(dir) = db.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    iniciar_registros(&db);
    tracing::info!("dados em {}", db.display());

    // Para testes locais: J_MENSEGER_RELAYS=ws://127.0.0.1:7781
    let relays = std::env::var("J_MENSEGER_RELAYS").ok().map(|v| {
        v.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    });

    iced::application(
        move || {
            let (app, task) = App::new(db.clone(), relays.clone());
            let ponte = Task::run(bridge_stream(), |ev| Msg::Engine(Box::new(ev)));
            (app, Task::batch([task, ponte]))
        },
        App::update,
        App::view,
    )
    .title(App::title)
    .theme(App::theme)
    .subscription(App::subscription)
    .window(iced::window::Settings {
        size: Size::new(1180.0, 760.0),
        min_size: Some(Size::new(880.0, 560.0)),
        icon: icone(),
        // No Windows, fechar manda para a bandeja (o app decide).
        exit_on_close_request: !tray::SUPPORTED,
        visible: !tray::started_hidden(),
        ..Default::default()
    })
    .run()
}

/// Ícone da janela e da barra de tarefas.
///
/// Vai dentro do executável: um arquivo solto ao lado dele sumiria assim que
/// alguém movesse o programa de lugar.
fn icone() -> Option<iced::window::Icon> {
    iced::window::icon::from_file_data(include_bytes!("../recursos/icone.png"), None).ok()
}

/// Liga os registros.
///
/// Na versão final do Windows não existe console para onde escrever, então os
/// registros vão para um arquivo ao lado do banco. Sem isso, um problema
/// relatado por quem usa o aplicativo não deixaria rastro nenhum.
fn iniciar_registros(db: &std::path::Path) {
    let filtro = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "jm_app=info,jm_core=info".into());
    let arquivo = if cfg!(windows) && !cfg!(debug_assertions) {
        db.parent()
            .map(|d| d.join("jmenseger.log"))
            .and_then(|c| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(c)
                    .ok()
            })
    } else {
        None
    };
    match arquivo {
        Some(f) => tracing_subscriber::fmt()
            .with_env_filter(filtro)
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(f))
            .init(),
        None => tracing_subscriber::fmt().with_env_filter(filtro).init(),
    }
}

/// Fluxo com os avisos do motor, consumido pela interface.
fn bridge_stream() -> impl futures_util::Stream<Item = jm_core::engine::EngineEvent> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    bridge::subscribe(tx);
    futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|ev| (ev, rx))
    })
}

/// Onde os dados ficam, seguindo a convenção de cada sistema.
fn db_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("J_MENSEGER_DATA") {
        return std::path::PathBuf::from(p);
    }
    let base = if cfg!(windows) {
        std::env::var("APPDATA").ok().map(std::path::PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .ok()
            .map(|h| std::path::PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| std::path::PathBuf::from(h).join(".local/share"))
            })
    };
    base.unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("J-Menseger")
        .join("jmenseger.db")
}
