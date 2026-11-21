//! Instalador e desinstalador animados.
//!
//! O próprio programa é o instalador, como o do Discord: quando o arquivo se
//! chama `…instalador.exe` (ou com `--instalar`), em vez do aplicativo abre
//! uma janelinha que instala para o usuário atual, sem pedir administrador:
//!
//! - **Windows:** `%LOCALAPPDATA%\Programs\J-Menseger\J-Menseger.exe`,
//!   atalho no Menu Iniciar (e na Área de Trabalho, se escolhido) e registro
//!   em "Aplicativos instalados", com desinstalação (`--desinstalar`).
//! - **Linux:** `~/.local/bin/j-menseger`, ícone e atalho no menu.
//!
//! As conversas ficam onde sempre ficaram (pasta de dados), então instalar
//! por cima de uma versão antiga não perde nada.

use crate::canvas_setup::{Background as Fundo, Check};
use crate::theme as t;
use iced::widget::{button, canvas, checkbox, column, container, mouse_area, row, stack, text, Space};
use iced::{Alignment, Color, Element, Length, Size, Subscription, Task, Theme};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const NOME: &str = "J-Menseger";
const VERSAO: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Install,
    Uninstall,
}

/// Decide se este processo é o instalador, o desinstalador ou o app.
pub fn mode() -> Option<Mode> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--desinstalar") {
        return Some(Mode::Uninstall);
    }
    if args.iter().any(|a| a == "--portatil") {
        return None;
    }
    if args.iter().any(|a| a == "--instalar") {
        return Some(Mode::Install);
    }
    let exe = std::env::current_exe().ok()?;
    let nome = exe.file_name()?.to_string_lossy().to_lowercase();
    let parece_instalador = ["instalador", "setup", "installer", "instalar"].iter().any(|p| nome.contains(p));
    (parece_instalador && !same_file(&exe, &paths().exe)).then_some(Mode::Install)
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Onde as coisas ficam.
#[derive(Debug, Clone)]
pub struct Paths {
    pub dir: PathBuf,
    pub exe: PathBuf,
    pub menu_lnk: Option<PathBuf>,
    pub desktop_lnk: Option<PathBuf>,
    /// Ícone e atalho do menu no Linux.
    pub extras: Vec<PathBuf>,
}

pub fn paths() -> Paths {
    // Para testes: instalar numa pasta qualquer.
    let teste = std::env::var("J_MENSEGER_INSTALAR_EM").ok().map(PathBuf::from);
    if cfg!(windows) {
        let local = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
        let dir = teste.unwrap_or_else(|| local.join("Programs").join(NOME));
        let menu = std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join(r"Microsoft\Windows\Start Menu\Programs").join(format!("{NOME}.lnk")));
        Paths { exe: dir.join(format!("{NOME}.exe")), dir, menu_lnk: menu, desktop_lnk: desktop_dir().map(|d| d.join(format!("{NOME}.lnk"))), extras: vec![] }
    } else {
        let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
        let base = teste.unwrap_or_else(|| home.join(".local"));
        let dir = base.join("bin");
        Paths {
            exe: dir.join("j-menseger"),
            dir,
            menu_lnk: None,
            desktop_lnk: None,
            extras: vec![base.join("share/applications/j-menseger.desktop"), base.join("share/icons/hicolor/256x256/apps/j-menseger.png")],
        }
    }
}

pub fn is_installed() -> bool {
    paths().exe.exists()
}

/// Andamento, escrito pela tarefa de instalar e lido pela tela.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub frac: f32,
    pub stage: String,
    /// Cada etapa e a fração em que começou, para o texto acompanhar a
    /// barra (que anda suave, atrás do progresso real).
    pub stages: Vec<(f32, String)>,
    pub done: bool,
    pub error: Option<String>,
}

type Shared = Arc<Mutex<Progress>>;

fn set(p: &Shared, frac: f32, stage: &str) {
    let mut g = p.lock().unwrap();
    g.frac = frac;
    g.stage = stage.to_string();
    if g.stages.last().is_none_or(|(_, s)| s != stage) {
        g.stages.push((frac, stage.to_string()));
    }
}

/// Instala. Roda numa thread própria.
pub fn install(desktop: bool, p: Shared) {
    if let Err(e) = install_inner(desktop, &p) {
        p.lock().unwrap().error = Some(e);
    }
}

fn install_inner(desktop: bool, p: &Shared) -> Result<(), String> {
    let ps = paths();
    set(p, 0.02, "Preparando…");
    std::fs::create_dir_all(&ps.dir).map_err(|e| format!("Não deu para criar a pasta {}: {e}", ps.dir.display()))?;
    let origem = std::env::current_exe().map_err(|e| e.to_string())?;
    if same_file(&origem, &ps.exe) {
        return Err("Este já é o programa instalado.".into());
    }
    let novo = ps.exe.with_extension("novo");
    copy_with_progress(&origem, &novo, |f| set(p, 0.05 + 0.70 * f, "Copiando o J-Menseger…"))?;
    set(p, 0.78, "Copiando o J-Menseger…");
    replace_exe(&novo, &ps.exe)?;
    set(p, 0.84, "Criando atalhos…");
    shortcuts(&ps, desktop)?;
    set(p, 0.92, "Registrando no sistema…");
    register(&ps)?;
    set(p, 1.0, "Pronto!");
    p.lock().unwrap().done = true;
    Ok(())
}

fn copy_with_progress(de: &Path, para: &Path, mut cb: impl FnMut(f32)) -> Result<(), String> {
    use std::io::{Read, Write};
    let mut a = std::fs::File::open(de).map_err(|e| format!("Não deu para ler o instalador: {e}"))?;
    let total = a.metadata().map(|m| m.len()).unwrap_or(1).max(1);
    let mut b = std::fs::File::create(para).map_err(|e| format!("Não deu para gravar em {}: {e}", para.display()))?;
    let mut buf = vec![0u8; 512 * 1024];
    let mut feito = 0u64;
    loop {
        let n = a.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        b.write_all(&buf[..n]).map_err(|e| format!("Falha ao gravar: {e}"))?;
        feito += n as u64;
        cb(feito as f32 / total as f32);
    }
    b.sync_all().ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(para, std::fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

/// Troca o executável. No Windows um programa aberto não pode ser
/// apagado, mas pode ser renomeado: o antigo vira `.old` e some depois.
fn replace_exe(novo: &Path, exe: &Path) -> Result<(), String> {
    let velho = exe.with_extension("old");
    let _ = std::fs::remove_file(&velho);
    if exe.exists() {
        if cfg!(windows) {
            std::fs::rename(exe, &velho).map_err(|e| format!("Feche o J-Menseger e tente de novo ({e})."))?;
        } else {
            let _ = std::fs::remove_file(exe);
        }
    }
    std::fs::rename(novo, exe).map_err(|e| format!("Não deu para colocar o programa no lugar: {e}"))
}

fn shortcuts(ps: &Paths, desktop: bool) -> Result<(), String> {
    #[cfg(windows)]
    let lnk = |destino: &Path| -> Result<(), String> {
        if let Some(d) = destino.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        let mut l = mslnk::ShellLink::new(&ps.exe).map_err(|e| e.to_string())?;
        l.set_name(Some("Mensagens e chamadas cifradas".into()));
        l.set_working_dir(Some(ps.dir.to_string_lossy().into_owned()));
        l.set_icon_location(Some(ps.exe.to_string_lossy().into_owned()));
        l.create_lnk(destino).map_err(|e| format!("Não deu para criar o atalho: {e}"))
    };
    #[cfg(windows)]
    {
        if let Some(m) = &ps.menu_lnk {
            lnk(m)?;
        }
        match &ps.desktop_lnk {
            Some(d) if desktop => lnk(d)?,
            Some(d) => {
                let _ = std::fs::remove_file(d);
            }
            None => {}
        }
    }
    #[cfg(not(windows))]
    let _ = desktop;
    if !cfg!(windows) {
        if let [atalho, icone] = ps.extras.as_slice() {
            for f in [atalho, icone] {
                if let Some(d) = f.parent() {
                    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
                }
            }
            std::fs::write(icone, include_bytes!("../recursos/icone.png")).map_err(|e| e.to_string())?;
            let txt = format!(
                "[Desktop Entry]\nType=Application\nName=J-Menseger\nComment=Mensagens e chamadas cifradas de ponta a ponta\nExec=\"{}\" --portatil\nIcon=j-menseger\nCategories=Network;InstantMessaging;\nTerminal=false\n",
                ps.exe.display()
            );
            std::fs::write(atalho, txt).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(windows)]
const CHAVE: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\J-Menseger";

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn register(ps: &Paths) -> Result<(), String> {
    use windows_sys::Win32::System::Registry::*;
    unsafe {
        let mut h: HKEY = std::ptr::null_mut();
        let r = RegCreateKeyExW(HKEY_CURRENT_USER, wide(CHAVE).as_ptr(), 0, std::ptr::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, std::ptr::null(), &mut h, std::ptr::null_mut());
        if r != 0 {
            return Err(format!("Não deu para registrar o programa (erro {r})."));
        }
        let exe = ps.exe.to_string_lossy().into_owned();
        let texto = |nome: &str, valor: &str| {
            let v = wide(valor);
            RegSetValueExW(h, wide(nome).as_ptr(), 0, REG_SZ, v.as_ptr() as *const u8, (v.len() * 2) as u32);
        };
        let numero = |nome: &str, valor: u32| {
            RegSetValueExW(h, wide(nome).as_ptr(), 0, REG_DWORD, &valor as *const u32 as *const u8, 4);
        };
        texto("DisplayName", NOME);
        texto("DisplayVersion", VERSAO);
        texto("Publisher", NOME);
        texto("DisplayIcon", &format!("{exe},0"));
        texto("InstallLocation", &ps.dir.to_string_lossy());
        texto("UninstallString", &format!("\"{exe}\" --desinstalar"));
        numero("NoModify", 1);
        numero("NoRepair", 1);
        let kb = std::fs::metadata(&ps.exe).map(|m| (m.len() / 1024) as u32).unwrap_or(0);
        numero("EstimatedSize", kb);
        RegCloseKey(h);
    }
    Ok(())
}

#[cfg(not(windows))]
fn register(_: &Paths) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn unregister() {
    use windows_sys::Win32::System::Registry::*;
    unsafe {
        RegDeleteTreeW(HKEY_CURRENT_USER, wide(CHAVE).as_ptr());
    }
}

#[cfg(not(windows))]
fn unregister() {}

/// Área de Trabalho de verdade (pode estar no OneDrive).
#[cfg(windows)]
fn desktop_dir() -> Option<PathBuf> {
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_Desktop, SHGetKnownFolderPath};
    unsafe {
        let mut p: windows_sys::core::PWSTR = std::ptr::null_mut();
        if SHGetKnownFolderPath(&FOLDERID_Desktop, 0, std::ptr::null_mut(), &mut p) != 0 || p.is_null() {
            return std::env::var("USERPROFILE").ok().map(|u| PathBuf::from(u).join("Desktop"));
        }
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
        CoTaskMemFree(p as *const _);
        Some(PathBuf::from(s))
    }
}

#[cfg(not(windows))]
fn desktop_dir() -> Option<PathBuf> {
    None
}

/// Desinstala. `apagar_dados` também apaga conta e conversas deste
/// computador.
pub fn uninstall(apagar_dados: bool, dados: PathBuf, p: Shared) {
    let ps = paths();
    set(&p, 0.1, "Tirando os atalhos…");
    for l in [&ps.menu_lnk, &ps.desktop_lnk].into_iter().flatten() {
        let _ = std::fs::remove_file(l);
    }
    for e in &ps.extras {
        let _ = std::fs::remove_file(e);
    }
    set(&p, 0.35, "Tirando do registro…");
    unregister();
    if apagar_dados {
        set(&p, 0.6, "Apagando as conversas deste computador…");
        let _ = std::fs::remove_dir_all(&dados);
    }
    set(&p, 0.85, "Removendo o programa…");
    remove_program(&ps);
    set(&p, 1.0, "Pronto");
    p.lock().unwrap().done = true;
}

#[cfg(windows)]
fn remove_program(ps: &Paths) {
    use std::os::windows::process::CommandExt;
    // O próprio programa está aberto: a pasta é apagada logo depois que ele
    // fechar, por um comando que espera alguns segundos.
    let _ = std::process::Command::new("cmd")
        .args(["/C", &format!("ping 127.0.0.1 -n 4 > nul & rmdir /S /Q \"{}\"", ps.dir.display())])
        .creation_flags(0x0800_0000)
        .spawn();
}

#[cfg(not(windows))]
fn remove_program(ps: &Paths) {
    let _ = std::fs::remove_file(&ps.exe);
}

/// Abre o programa instalado (ou este mesmo, sem instalar).
fn launch(exe: &Path, portatil: bool) {
    let mut c = std::process::Command::new(exe);
    if portatil {
        c.arg("--portatil");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0000_0008); // DETACHED_PROCESS
    }
    let _ = c.spawn();
}

/* ================================================================ tela */

#[derive(Debug, Clone)]
pub enum Msg {
    Frame(Instant),
    Start,
    Desktop(bool),
    Erase(bool),
    OpenNow,
    Portable,
    Close,
    Drag,
}

#[derive(Debug, Clone, PartialEq)]
enum Stage {
    Welcome,
    Working,
    Done,
    Error(String),
}

pub struct Setup {
    mode: Mode,
    stage: Stage,
    desktop: bool,
    erase: bool,
    progress: Shared,
    shown: f32,
    t0: Instant,
    now: Instant,
    done_at: Option<Instant>,
    instalado: bool,
    dados: PathBuf,
}

impl Setup {
    fn new(mode: Mode, dados: PathBuf) -> Self {
        Setup {
            mode,
            stage: Stage::Welcome,
            desktop: true,
            erase: false,
            progress: Arc::new(Mutex::new(Progress::default())),
            shown: 0.0,
            t0: Instant::now(),
            now: Instant::now(),
            done_at: None,
            instalado: is_installed(),
            dados,
        }
    }

    fn clock(&self) -> f32 {
        self.now.saturating_duration_since(self.t0).as_secs_f32()
    }

    fn update(&mut self, m: Msg) -> Task<Msg> {
        match m {
            Msg::Frame(at) => {
                self.now = at.max(self.now);
                if self.stage == Stage::Working {
                    let p = self.progress.lock().unwrap().clone();
                    if let Some(e) = p.error {
                        self.stage = Stage::Error(e);
                        return Task::none();
                    }
                    // A barra anda suave atrás do progresso de verdade.
                    self.shown += (p.frac - self.shown) * 0.09 + 0.002;
                    self.shown = self.shown.min(p.frac);
                    if p.done && self.shown > 0.995 {
                        self.stage = Stage::Done;
                        self.done_at = Some(self.now);
                    }
                }
                if let (Stage::Done, Some(quando)) = (&self.stage, self.done_at) {
                    let espera = if self.mode == Mode::Install { 2.2 } else { 3.0 };
                    if self.now.saturating_duration_since(quando).as_secs_f32() > espera {
                        return self.finish();
                    }
                }
                Task::none()
            }
            Msg::Start => {
                *self.progress.lock().unwrap() = Progress::default();
                self.shown = 0.0;
                self.stage = Stage::Working;
                let p = Arc::clone(&self.progress);
                match self.mode {
                    Mode::Install => {
                        let d = self.desktop;
                        std::thread::spawn(move || install(d, p));
                    }
                    Mode::Uninstall => {
                        let (e, dados) = (self.erase, self.dados.clone());
                        std::thread::spawn(move || uninstall(e, dados, p));
                    }
                }
                Task::none()
            }
            Msg::Desktop(v) => {
                self.desktop = v;
                Task::none()
            }
            Msg::Erase(v) => {
                self.erase = v;
                Task::none()
            }
            Msg::OpenNow => self.finish(),
            Msg::Portable => {
                if let Ok(exe) = std::env::current_exe() {
                    launch(&exe, true);
                }
                iced::exit()
            }
            Msg::Close => iced::exit(),
            Msg::Drag => iced::window::latest().and_then(iced::window::drag),
        }
    }

    fn finish(&mut self) -> Task<Msg> {
        if self.mode == Mode::Install && self.stage == Stage::Done {
            launch(&paths().exe, false);
        } else if self.mode == Mode::Install && self.instalado {
            launch(&paths().exe, false);
        }
        iced::exit()
    }

    fn view(&self) -> Element<'_, Msg> {
        let c = self.clock();
        let fundo = canvas(Fundo { t: c, accent: t::accent() }).width(Length::Fill).height(Length::Fill);
        let fechar = button(container(text("✕").size(15).color(t::FG_DIM)).center(30)).padding(0).style(t::ghost).on_press(Msg::Close);
        let barra = row![Space::new().width(Length::Fill), fechar].padding(8);

        // Logo flutuando; trabalhando, ela "respira".
        let flutua = (c * 1.7).sin() * 6.0;
        let lado = if self.stage == Stage::Working { 92.0 + (c * 4.0).sin() * 4.0 } else { 92.0 };
        let logo = column![Space::new().height(10.0 + flutua), container(crate::icons::logo(lado)).center(100), Space::new().height(10.0 - flutua)];

        let titulo = text(NOME).size(30).font(crate::ui::BOLD).color(t::FG);
        let corpo: Element<'_, Msg> = match (&self.stage, self.mode) {
            (Stage::Welcome, Mode::Install) => {
                let rotulo = if self.instalado { "Atualizar" } else { "Instalar" };
                let mut col = column![
                    text("Mensagens e chamadas cifradas de ponta a ponta.").size(14).color(t::FG_DIM).align_x(Alignment::Center),
                    Space::new().height(14),
                    grande(rotulo, t::primary, Msg::Start),
                    checkbox(self.desktop).label("Criar atalho na Área de Trabalho").on_toggle(Msg::Desktop).size(16).text_size(13.5).style(caixa),
                ]
                .spacing(12)
                .align_x(Alignment::Center);
                if self.instalado {
                    col = col.push(button(text("Abrir o J-Menseger instalado").size(13).color(t::FG_DIM)).style(t::ghost).on_press(Msg::OpenNow));
                } else {
                    col = col.push(button(text("Só abrir, sem instalar").size(13).color(t::FG_DIM)).style(t::ghost).on_press(Msg::Portable));
                }
                col = col.push(
                    text(format!("Versão {VERSAO}. Instala só para você, sem pedir administrador. Suas conversas continuam onde estão."))
                        .size(11.5)
                        .color(t::FG_FAINT)
                        .align_x(Alignment::Center),
                );
                col.into()
            }
            (Stage::Welcome, Mode::Uninstall) => column![
                text("Desinstalar o J-Menseger deste computador?").size(15).color(t::FG).align_x(Alignment::Center),
                Space::new().height(8),
                checkbox(self.erase).label("Apagar também a conta e as conversas deste computador").on_toggle(Msg::Erase).size(16).text_size(13).style(caixa),
                text(if self.erase {
                    "Sem as 12 palavras anotadas, não dá para recuperar a conta depois."
                } else {
                    "A conta e as conversas ficam guardadas: instalando de novo, tudo volta."
                })
                .size(12)
                .color(if self.erase { t::DANGER } else { t::FG_FAINT })
                .align_x(Alignment::Center),
                Space::new().height(8),
                row![grande_l("Cancelar", t::secondary, Msg::Close, 150.0), grande_l("Desinstalar", t::danger, Msg::Start, 150.0)].spacing(10),
            ]
            .spacing(10)
            .align_x(Alignment::Center)
            .into(),
            (Stage::Working, _) => {
                let p = self.progress.lock().unwrap().clone();
                let etapa = p.stages.iter().rev().find(|(f, _)| *f <= self.shown + 0.001).map(|(_, s)| s.clone()).unwrap_or_else(|| "Preparando…".into());
                column![
                    text(etapa).size(15).color(t::FG),
                    barra_de_progresso(self.shown, c),
                    text(format!("{:.0}%", self.shown * 100.0)).size(12.5).color(t::FG_DIM),
                ]
                .spacing(12)
                .align_x(Alignment::Center)
                .into()
            }
            (Stage::Done, modo) => {
                let p = self.done_at.map(|d| self.now.saturating_duration_since(d).as_secs_f32()).unwrap_or(0.0);
                let mut col = column![
                    canvas(Check { p: (p / 0.7).min(1.0), color: t::accent() }).width(64).height(64),
                    text(if modo == Mode::Install { "Tudo pronto!" } else { "Desinstalado" }).size(18).font(crate::ui::BOLD).color(t::FG),
                ]
                .spacing(10)
                .align_x(Alignment::Center);
                if modo == Mode::Install {
                    col = col.push(text("Abrindo o J-Menseger…").size(13).color(t::FG_DIM));
                    col = col.push(grande("Abrir agora", t::primary, Msg::OpenNow));
                } else {
                    col = col.push(text("Obrigado por ter usado o J-Menseger.").size(13).color(t::FG_DIM));
                    col = col.push(grande("Fechar", t::secondary, Msg::Close));
                }
                col.into()
            }
            (Stage::Error(e), _) => column![
                text("Algo deu errado").size(17).font(crate::ui::BOLD).color(t::DANGER),
                text(e.clone()).size(13).color(t::FG_DIM).align_x(Alignment::Center),
                Space::new().height(6),
                row![grande_l("Fechar", t::secondary, Msg::Close, 140.0), grande_l("Tentar de novo", t::primary, Msg::Start, 140.0)].spacing(10),
            ]
            .spacing(10)
            .align_x(Alignment::Center)
            .into(),
        };
        let conteudo = column![barra, container(column![logo, titulo, Space::new().height(14), corpo].align_x(Alignment::Center)).padding([0, 34]).center_x(Length::Fill)]
            .width(Length::Fill)
            .height(Length::Fill);
        // Arrastar a janela pelo fundo (ela não tem barra de título).
        stack![mouse_area(fundo).on_press(Msg::Drag), conteudo].into()
    }
}

fn grande<'a>(rotulo: &'a str, estilo: fn(&Theme, button::Status) -> button::Style, m: Msg) -> Element<'a, Msg> {
    grande_l(rotulo, estilo, m, 240.0)
}

fn grande_l<'a>(rotulo: &'a str, estilo: fn(&Theme, button::Status) -> button::Style, m: Msg, largura: f32) -> Element<'a, Msg> {
    button(container(text(rotulo).size(15.5).font(crate::ui::BOLD)).center_x(Length::Fill))
        .padding([12, 16])
        .width(largura)
        .style(move |th, st| {
            let mut s = estilo(th, st);
            s.border.radius = 10.0.into();
            s
        })
        .on_press(m)
        .into()
}

fn caixa(_: &Theme, st: iced::widget::checkbox::Status) -> iced::widget::checkbox::Style {
    let marcado = matches!(st, iced::widget::checkbox::Status::Active { is_checked: true } | iced::widget::checkbox::Status::Hovered { is_checked: true });
    iced::widget::checkbox::Style {
        background: iced::Background::Color(if marcado { t::accent() } else { t::with_alpha(Color::WHITE, 0.06) }),
        icon_color: t::accent_ink(),
        border: iced::Border { color: if marcado { t::accent() } else { t::with_alpha(Color::WHITE, 0.25) }, width: 1.5, radius: 5.0.into() },
        text_color: Some(t::FG_DIM),
    }
}

/// Barra com um brilho passando por cima.
fn barra_de_progresso<'a>(p: f32, c: f32) -> Element<'a, Msg> {
    canvas(crate::canvas_setup::Bar { p, t: c, color: t::accent() }).width(300).height(10).into()
}

/// Abre a janela do instalador (ou do desinstalador).
pub fn run(mode: Mode, dados: PathBuf) -> iced::Result {
    iced::application(
        move || {
            // `--auto`: atualização pedida pelo próprio app, começa sozinha.
            let auto = mode == Mode::Install && std::env::args().any(|a| a == "--auto");
            (Setup::new(mode, dados.clone()), if auto { Task::done(Msg::Start) } else { Task::none() })
        },
        Setup::update,
        Setup::view,
    )
        .title(|s: &Setup| if s.mode == Mode::Install { format!("Instalar o {NOME}") } else { format!("Desinstalar o {NOME}") })
        .theme(|_: &Setup| Theme::Dark)
        .subscription(|_: &Setup| -> Subscription<Msg> { iced::window::frames().map(Msg::Frame) })
        .window(iced::window::Settings {
            size: Size::new(440.0, 540.0),
            resizable: false,
            decorations: false,
            position: iced::window::Position::Centered,
            icon: iced::window::icon::from_file_data(include_bytes!("../recursos/icone.png"), None).ok(),
            ..Default::default()
        })
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instala_e_desinstala_numa_pasta_de_teste() {
        let dir = std::env::temp_dir().join(format!("jm-instalar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("J_MENSEGER_INSTALAR_EM", &dir);
        let p: Shared = Arc::new(Mutex::new(Progress::default()));
        install(false, Arc::clone(&p));
        let g = p.lock().unwrap().clone();
        assert!(g.error.is_none(), "{:?}", g.error);
        assert!(g.done && g.frac >= 1.0);
        let ps = paths();
        let original = std::fs::metadata(std::env::current_exe().unwrap()).unwrap().len();
        assert_eq!(std::fs::metadata(&ps.exe).unwrap().len(), original);
        for e in &ps.extras {
            assert!(e.exists(), "faltou {}", e.display());
        }
        // Instalar de novo por cima funciona (atualizar).
        install(false, Arc::clone(&p));
        assert!(p.lock().unwrap().error.is_none());
        let dados = dir.join("dados");
        std::fs::create_dir_all(&dados).unwrap();
        uninstall(false, dados.clone(), Arc::clone(&p));
        assert!(!ps.exe.exists());
        assert!(dados.exists(), "sem marcar, os dados ficam");
        uninstall(true, dados.clone(), Arc::clone(&p));
        assert!(!dados.exists());
        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var("J_MENSEGER_INSTALAR_EM");
    }

    #[cfg(windows)]
    #[test]
    fn atalho_lnk_e_gerado() {
        let dir = std::env::temp_dir().join(format!("jm-lnk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let alvo = dir.join("J-Menseger.exe");
        std::fs::write(&alvo, b"x").unwrap();
        let ps = Paths { dir: dir.clone(), exe: alvo, menu_lnk: Some(dir.join("menu/J-Menseger.lnk")), desktop_lnk: Some(dir.join("J.lnk")), extras: vec![] };
        shortcuts(&ps, true).unwrap();
        assert!(dir.join("menu/J-Menseger.lnk").exists() && dir.join("J.lnk").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
