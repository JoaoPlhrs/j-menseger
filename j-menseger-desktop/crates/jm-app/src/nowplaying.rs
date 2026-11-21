//! Qual música está tocando no Spotify deste computador.
//!
//! - **Windows:** o Spotify põe "Artista - Música" no título da janela
//!   enquanto toca (pausado, o título vira só "Spotify…"). Lemos o título da
//!   janela do processo `Spotify.exe`. Não dá a duração nem o link da faixa.
//! - **Linux:** o Spotify publica a faixa pelo MPRIS (D-Bus da sessão), com
//!   nome, artista, álbum, duração e o identificador da faixa.
//!
//! Nada sai do computador por aqui: quem decide mandar aos contatos é o app,
//! e só com a opção ligada nos ajustes.

/// Faixa vista agora.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub id: Option<String>,
    pub secs: Option<u32>,
}

/// Separa o título da janela do Spotify no Windows ("Artista - Música").
/// Pausado, anúncio ou janela sem música: `None`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn parse_window_title(t: &str) -> Option<Track> {
    let t = t.trim();
    if t.is_empty() || t.starts_with("Spotify") || t.eq_ignore_ascii_case("Advertisement") {
        return None;
    }
    let (artist, title) = t.split_once(" - ")?;
    let (artist, title) = (artist.trim(), title.trim());
    if artist.is_empty() || title.is_empty() {
        return None;
    }
    Some(Track { title: title.into(), artist: artist.into(), ..Default::default() })
}

/// Identificador da faixa a partir do `mpris:trackid` do Spotify
/// (`/com/spotify/track/ID` ou `spotify:track:ID`).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn track_id(trackid: &str) -> Option<String> {
    let id = trackid.rsplit(['/', ':']).next()?;
    (id.len() == 22 && id.chars().all(|c| c.is_ascii_alphanumeric()) && trackid.contains("track")).then(|| id.to_string())
}

/// Lê a música atual. Roda bloqueando: chamar em `spawn_blocking`.
pub fn current() -> Option<Track> {
    imp::current()
}

#[cfg(windows)]
mod imp {
    use super::{parse_window_title, Track};
    use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId};

    fn nome_do_processo(pid: u32) -> Option<String> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut buf = [0u16; 520];
            let mut n = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut n);
            CloseHandle(h);
            (ok != 0).then(|| String::from_utf16_lossy(&buf[..n as usize]))
        }
    }

    unsafe extern "system" fn cada_janela(hwnd: HWND, lp: LPARAM) -> BOOL {
        let achou = &mut *(lp as *mut Option<Track>);
        let n = GetWindowTextLengthW(hwnd);
        if n <= 0 || n > 600 {
            return 1;
        }
        let mut buf = vec![0u16; n as usize + 1];
        let lido = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if lido <= 0 {
            return 1;
        }
        let titulo = String::from_utf16_lossy(&buf[..lido as usize]);
        if !titulo.contains(" - ") {
            return 1;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let e_spotify = nome_do_processo(pid).is_some_and(|p| p.to_ascii_lowercase().ends_with("\\spotify.exe"));
        if e_spotify {
            if let Some(t) = parse_window_title(&titulo) {
                *achou = Some(t);
                return 0;
            }
        }
        1
    }

    pub fn current() -> Option<Track> {
        let mut achou: Option<Track> = None;
        unsafe {
            EnumWindows(Some(cada_janela), &mut achou as *mut Option<Track> as LPARAM);
        }
        achou
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{track_id, Track};
    use std::collections::HashMap;
    use zbus::zvariant::OwnedValue;

    fn texto(v: &OwnedValue) -> Option<String> {
        String::try_from(v.try_clone().ok()?).ok()
    }

    fn lista(v: &OwnedValue) -> Option<String> {
        let l: Vec<String> = Vec::<String>::try_from(v.try_clone().ok()?).ok()?;
        Some(l.join(", "))
    }

    pub fn current() -> Option<Track> {
        let con = zbus::blocking::Connection::session().ok()?;
        let p = zbus::blocking::Proxy::new(&con, "org.mpris.MediaPlayer2.spotify", "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player").ok()?;
        let estado: String = p.get_property("PlaybackStatus").ok()?;
        if estado != "Playing" {
            return None;
        }
        let meta: HashMap<String, OwnedValue> = p.get_property("Metadata").ok()?;
        let title = meta.get("xesam:title").and_then(texto).unwrap_or_default();
        if title.trim().is_empty() {
            return None;
        }
        let artist = meta.get("xesam:artist").and_then(lista).or_else(|| meta.get("xesam:artist").and_then(texto)).unwrap_or_default();
        let album = meta.get("xesam:album").and_then(texto).unwrap_or_default();
        let id = meta
            .get("mpris:trackid")
            .and_then(|v| texto(v).or_else(|| v.try_clone().ok().and_then(|v| zbus::zvariant::OwnedObjectPath::try_from(v).ok()).map(|o| o.to_string())))
            .and_then(|t| track_id(&t));
        let secs = meta
            .get("mpris:length")
            .and_then(|v| u64::try_from(v.try_clone().ok()?).ok().or_else(|| i64::try_from(v.try_clone().ok()?).ok().map(|x| x.max(0) as u64)))
            .map(|us| (us / 1_000_000) as u32)
            .filter(|s| *s > 0);
        Some(Track { title, artist, album, id, secs })
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn current() -> Option<super::Track> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_o_titulo_da_janela() {
        let t = parse_window_title("Legião Urbana - Tempo Perdido").unwrap();
        assert_eq!((t.artist.as_str(), t.title.as_str()), ("Legião Urbana", "Tempo Perdido"));
        // Música com hífen no nome: o primeiro " - " separa o artista.
        assert_eq!(parse_window_title("Queen - Bohemian Rhapsody - Remastered 2011").unwrap().title, "Bohemian Rhapsody - Remastered 2011");
        for pausado in ["Spotify", "Spotify Premium", "Spotify Free", "Advertisement", "", "GDI+ Window (Spotify.exe)"] {
            assert!(parse_window_title(pausado).is_none(), "{pausado}");
        }
    }

    #[test]
    fn pega_o_id_da_faixa() {
        assert_eq!(track_id("/com/spotify/track/4uLU6hMCjMI75M1A2tKUQC").as_deref(), Some("4uLU6hMCjMI75M1A2tKUQC"));
        assert_eq!(track_id("spotify:track:4uLU6hMCjMI75M1A2tKUQC").as_deref(), Some("4uLU6hMCjMI75M1A2tKUQC"));
        assert!(track_id("spotify:ad:4uLU6hMCjMI75M1A2tKUQC").is_none());
        assert!(track_id("/org/mpris/MediaPlayer2/TrackList/NoTrack").is_none());
    }
}
