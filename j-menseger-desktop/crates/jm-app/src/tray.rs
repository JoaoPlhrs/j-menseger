//! Bandeja do Windows e "abrir ao ligar o computador".
//!
//! Fechar a janela não encerra o J-Menseger: ele fica na bandeja (perto do
//! relógio), conectado, e as notificações continuam chegando. Pelo ícone dá
//! para abrir de novo ou sair de verdade. Em outros sistemas não há bandeja:
//! fechar encerra, como antes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Ev {
    Show,
    Quit,
}

/// Existe bandeja neste sistema?
pub const SUPPORTED: bool = cfg!(windows);

#[cfg(windows)]
mod imp {
    use super::Ev;
    use std::cell::RefCell;
    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

    struct Bandeja {
        _icone: TrayIcon,
        abrir: String,
        sair: String,
    }

    thread_local! {
        static BANDEJA: RefCell<Option<Bandeja>> = const { RefCell::new(None) };
    }

    /// Cria o ícone (uma vez). Tem de ser chamado na linha principal.
    pub fn ensure() {
        BANDEJA.with(|b| {
            if b.borrow().is_some() {
                return;
            }
            // O ícone embutido no executável (recurso 1, posto pelo build.rs).
            let Ok(icone) = tray_icon::Icon::from_resource(1, Some((32, 32))) else { return };
            let abrir = MenuItem::new("Abrir J-Menseger", true, None);
            let sair = MenuItem::new("Sair", true, None);
            let menu = Menu::new();
            let _ = menu.append(&abrir);
            let _ = menu.append(&sair);
            let Ok(t) = TrayIconBuilder::new()
                .with_tooltip("J-Menseger")
                .with_icon(icone)
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
            else {
                return;
            };
            *b.borrow_mut() = Some(Bandeja { _icone: t, abrir: abrir.id().0.clone(), sair: sair.id().0.clone() });
        });
    }

    /// O que a pessoa fez no ícone desde a última vez.
    pub fn poll() -> Vec<Ev> {
        let mut v = Vec::new();
        while let Ok(e) = TrayIconEvent::receiver().try_recv() {
            match e {
                TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } | TrayIconEvent::DoubleClick { .. } => v.push(Ev::Show),
                _ => {}
            }
        }
        while let Ok(e) = MenuEvent::receiver().try_recv() {
            BANDEJA.with(|b| {
                if let Some(b) = b.borrow().as_ref() {
                    if e.id.0 == b.abrir {
                        v.push(Ev::Show);
                    } else if e.id.0 == b.sair {
                        v.push(Ev::Quit);
                    }
                }
            });
        }
        v
    }

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const NOME: &str = "J-Menseger";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Abre junto com o Windows?
    pub fn autostart() -> bool {
        use windows_sys::Win32::System::Registry::*;
        unsafe {
            let mut h: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, wide(RUN).as_ptr(), 0, KEY_READ, &mut h) != 0 {
                return false;
            }
            let r = RegQueryValueExW(h, wide(NOME).as_ptr(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            RegCloseKey(h);
            r == 0
        }
    }

    pub fn set_autostart(on: bool) -> Result<(), String> {
        use windows_sys::Win32::System::Registry::*;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        unsafe {
            let mut h: HKEY = std::ptr::null_mut();
            let r = RegCreateKeyExW(HKEY_CURRENT_USER, wide(RUN).as_ptr(), 0, std::ptr::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, std::ptr::null(), &mut h, std::ptr::null_mut());
            if r != 0 {
                return Err(format!("Não deu para mudar a inicialização (erro {r})."));
            }
            let r = if on {
                let v = wide(&format!("\"{}\" --bandeja", exe.display()));
                RegSetValueExW(h, wide(NOME).as_ptr(), 0, REG_SZ, v.as_ptr() as *const u8, (v.len() * 2) as u32)
            } else {
                RegDeleteValueW(h, wide(NOME).as_ptr())
            };
            RegCloseKey(h);
            if r != 0 && on {
                return Err(format!("Não deu para mudar a inicialização (erro {r})."));
            }
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Ev;
    pub fn ensure() {}
    pub fn poll() -> Vec<Ev> {
        Vec::new()
    }
    pub fn autostart() -> bool {
        false
    }
    pub fn set_autostart(_: bool) -> Result<(), String> {
        Err("Só no Windows.".into())
    }
}

pub use imp::*;

/// Foi aberto pelo Windows ao ligar (começa escondido na bandeja)?
pub fn started_hidden() -> bool {
    SUPPORTED && std::env::args().any(|a| a == "--bandeja")
}
