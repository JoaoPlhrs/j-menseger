//! Notificações com botões: responder (Windows), marcar como lida e abrir.
//!
//! O que a pessoa faz na notificação chega à interface por uma fila que é
//! lida a cada ~250 ms (`poll`). No Windows o aviso é montado à mão (XML do
//! "toast") porque é o único jeito de ter o campo de resposta; no Linux usa
//! os botões do próprio sistema (sem campo de texto, que nem todo Linux tem).

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Action {
    /// Abrir a conversa.
    Open(String),
    /// Responder com este texto.
    Reply(String, String),
    /// Marcar como lida.
    Read(String),
}

fn fila() -> &'static (Mutex<Sender<Action>>, Mutex<Receiver<Action>>) {
    static F: OnceLock<(Mutex<Sender<Action>>, Mutex<Receiver<Action>>)> = OnceLock::new();
    F.get_or_init(|| {
        let (tx, rx) = channel();
        (Mutex::new(tx), Mutex::new(rx))
    })
}

#[cfg_attr(not(any(windows, target_os = "linux")), allow(dead_code))]
fn push(a: Action) {
    let _ = fila().0.lock().unwrap().send(a);
}

/// O que chegou das notificações desde a última vez.
pub fn poll() -> Vec<Action> {
    let rx = fila().1.lock().unwrap();
    rx.try_iter().collect()
}

/// Mostra o aviso de uma mensagem. `chat` liga os botões à conversa; `reply`
/// pede o campo de resposta (só quando o texto da mensagem aparece).
pub fn message(title: String, body: String, chat: Option<String>, reply: bool) {
    std::thread::spawn(move || {
        if let Err(e) = imp::show(&title, &body, chat.as_deref(), reply) {
            tracing::debug!("notificação com botões falhou ({e}); mostrando simples");
            crate::platform::notify(title, body);
        }
    });
}

#[cfg(windows)]
mod imp {
    use super::{push, Action};
    use windows::core::{Interface, HSTRING};
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::Foundation::{IReference, TypedEventHandler};
    use windows::UI::Notifications::{ToastActivatedEventArgs, ToastNotification, ToastNotificationManager};

    pub const AUMID: &str = "JMenseger.App";

    fn esc(s: &str) -> String {
        s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
    }

    /// Registra o nome e o ícone do app para as notificações (sem isso o
    /// Windows mostra "PowerShell" como origem). Barato: roda a cada aviso.
    fn registrar() {
        use windows_sys::Win32::System::Registry::*;
        let chave = format!("Software\\Classes\\AppUserModelId\\{AUMID}");
        let icone = std::env::temp_dir().join("j-menseger-icone.png");
        if !icone.exists() {
            let _ = std::fs::write(&icone, include_bytes!("../recursos/icone.png"));
        }
        let w = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        unsafe {
            let mut h: HKEY = std::ptr::null_mut();
            if RegCreateKeyExW(HKEY_CURRENT_USER, w(&chave).as_ptr(), 0, std::ptr::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, std::ptr::null(), &mut h, std::ptr::null_mut()) != 0 {
                return;
            }
            for (n, v) in [("DisplayName", "J-Menseger".to_string()), ("IconUri", icone.to_string_lossy().into_owned())] {
                let v = w(&v);
                RegSetValueExW(h, w(n).as_ptr(), 0, REG_SZ, v.as_ptr() as *const u8, (v.len() * 2) as u32);
            }
            RegCloseKey(h);
        }
    }

    pub fn show(title: &str, body: &str, chat: Option<&str>, reply: bool) -> windows::core::Result<()> {
        registrar();
        let c = chat.unwrap_or("");
        let mut acoes = String::new();
        if let Some(c) = chat {
            if reply {
                acoes.push_str(r#"<input id="r" type="text" placeHolderContent="Responder…"/>"#);
                acoes.push_str(&format!(r#"<action content="Enviar" arguments="reply|{}" hint-inputId="r" activationType="foreground"/>"#, esc(c)));
            }
            acoes.push_str(&format!(r#"<action content="Marcar como lida" arguments="read|{}" activationType="foreground"/>"#, esc(c)));
        }
        let xml = format!(
            r#"<toast launch="open|{}"><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual>{}</toast>"#,
            esc(c),
            esc(title),
            esc(body),
            if acoes.is_empty() { String::new() } else { format!("<actions>{acoes}</actions>") }
        );
        let doc = XmlDocument::new()?;
        doc.LoadXml(&HSTRING::from(xml))?;
        let toast = ToastNotification::CreateToastNotification(&doc)?;
        toast.Activated(&TypedEventHandler::new(move |_, insp: windows::core::Ref<windows::core::IInspectable>| {
            let Some(insp) = insp.as_ref() else { return Ok(()) };
            let Ok(args) = insp.cast::<ToastActivatedEventArgs>() else { return Ok(()) };
            let a = args.Arguments().map(|s| s.to_string()).unwrap_or_default();
            let (tipo, chat) = a.split_once('|').unwrap_or((a.as_str(), ""));
            if chat.is_empty() {
                return Ok(());
            }
            match tipo {
                "reply" => {
                    let texto = args
                        .UserInput()
                        .ok()
                        .and_then(|m| m.Lookup(&HSTRING::from("r")).ok())
                        .and_then(|v| v.cast::<IReference<HSTRING>>().ok())
                        .and_then(|r| r.Value().ok())
                        .map(|h| h.to_string())
                        .unwrap_or_default();
                    if texto.trim().is_empty() {
                        push(Action::Open(chat.into()));
                    } else {
                        push(Action::Reply(chat.into(), texto));
                    }
                }
                "read" => push(Action::Read(chat.into())),
                _ => push(Action::Open(chat.into())),
            }
            Ok(())
        }))?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))?.Show(&toast)?;
        // O aviso precisa continuar vivo para os botões funcionarem.
        std::thread::sleep(std::time::Duration::from_secs(25));
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{push, Action};

    pub fn show(title: &str, body: &str, chat: Option<&str>, _reply: bool) -> Result<(), String> {
        let mut n = notify_rust::Notification::new();
        n.appname("J-Menseger").summary(title).body(body);
        if chat.is_some() {
            n.action("default", "Abrir").action("read", "Marcar como lida");
        }
        let h = n.show().map_err(|e| e.to_string())?;
        if let Some(c) = chat {
            let c = c.to_string();
            h.wait_for_action(|a| match a {
                "read" => push(Action::Read(c.clone())),
                "default" => push(Action::Open(c.clone())),
                _ => {}
            });
        }
        Ok(())
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn show(title: &str, body: &str, _chat: Option<&str>, _reply: bool) -> Result<(), String> {
        notify_rust::Notification::new().appname("J-Menseger").summary(title).body(body).show().map(|_| ()).map_err(|e| e.to_string())
    }
}
