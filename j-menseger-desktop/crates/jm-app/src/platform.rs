//! O que depende do sistema: abrir links, notificações, escolher arquivos e
//! esconder a janela das capturas de tela.

use crate::state::{CropDraft, CropTarget, PhotoDraft};
use iced::widget::image;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Abre um endereço no navegador. Só http(s): nada de arquivos locais nem
/// esquemas que o sistema rodaria como programa.
pub fn open_url(url: &str) {
    let u = url.trim();
    if !(u.starts_with("https://") || u.starts_with("http://")) || u.chars().any(|c| c.is_control() || c == '"') {
        return;
    }
    let r = if cfg!(windows) {
        std::process::Command::new("rundll32.exe").args(["url.dll,FileProtocolHandler", u]).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(u).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(u).spawn()
    };
    if let Err(e) = r {
        tracing::warn!("não deu para abrir o link: {e}");
    }
}

/// Notificação do sistema. Roda fora da thread da interface (no Linux ela
/// passa pelo D-Bus e pode demorar).
pub fn notify(title: String, body: String) {
    std::thread::spawn(move || {
        let r = notify_rust::Notification::new().appname("J-Menseger").summary(&title).body(&body).show();
        if let Err(e) = r {
            tracing::debug!("notificação não mostrada: {e}");
        }
    });
}

/// Esconde (ou mostra de novo) o conteúdo da janela em capturas de tela e
/// gravações. Só existe no Windows 10 2004 ou mais novo; nos outros sistemas
/// devolve `false` e a interface avisa.
pub fn protect_window(w: &dyn iced::window::Window, on: bool) -> bool {
    #[cfg(windows)]
    {
        use iced::window::raw_window_handle::RawWindowHandle;
        use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE};
        let Ok(h) = w.window_handle() else { return false };
        if let RawWindowHandle::Win32(h) = h.as_raw() {
            let hwnd = h.hwnd.get() as windows_sys::Win32::Foundation::HWND;
            let modo = if on { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE };
            return unsafe { SetWindowDisplayAffinity(hwnd, modo) } != 0;
        }
        false
    }
    #[cfg(not(windows))]
    {
        let _ = (w, on);
        false
    }
}

pub const fn can_protect_window() -> bool {
    cfg!(windows)
}

pub fn is_image(p: &Path) -> bool {
    matches!(ext(p).as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp")
}

pub fn is_audio(p: &Path) -> bool {
    matches!(ext(p).as_str(), "mp3" | "m4a" | "aac" | "ogg" | "oga" | "opus" | "wav" | "flac" | "webm" | "mp4")
}

fn ext(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

const MAX_INPUT: u64 = 30 * 1024 * 1024;

async fn read_limited(p: &Path, max: u64, msg: &str) -> Result<Vec<u8>, String> {
    let meta = tokio::fs::metadata(p).await.map_err(|e| e.to_string())?;
    if meta.len() > max {
        return Err(msg.to_string());
    }
    tokio::fs::read(p).await.map_err(|e| e.to_string())
}

/// Prepara uma foto para mandar: redimensiona, comprime e tira os metadados.
pub async fn photo_from_path(p: PathBuf) -> Result<PhotoDraft, String> {
    let bytes = read_limited(&p, MAX_INPUT, "Foto grande demais (até 30 MB).").await?;
    tokio::task::spawn_blocking(move || {
        // GIF animado vai como GIF (até 2 MB); maior que isso vira foto parada.
        if jm_media::gif::is_gif(&bytes) && bytes.len() <= jm_media::gif::GIF_MAX {
            return gif_draft(bytes);
        }
        let f = jm_media::photo::prepare(&bytes).map_err(|_| "Não consegui abrir essa imagem.".to_string())?;
        let handle = image::Handle::from_bytes(f.bytes.clone());
        Ok(PhotoDraft { bytes: Arc::new(f.bytes), mime: f.mime.to_string(), w: f.w, h: f.h, th: f.th, handle })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// GIF pronto para mandar, sem mexer nos bytes.
pub fn gif_draft(bytes: Vec<u8>) -> Result<PhotoDraft, String> {
    let g = jm_media::gif::prepare(bytes).map_err(|e| e.to_string())?;
    let handle = image::Handle::from_bytes(g.first_png.clone());
    Ok(PhotoDraft { bytes: Arc::new(g.bytes), mime: "image/gif".into(), w: g.w, h: g.h, th: g.th, handle })
}

/// Pede um GIF do computador.
pub async fn pick_gif() -> Result<Option<PhotoDraft>, String> {
    let Some(f) = rfd::AsyncFileDialog::new().set_title("Escolher GIF").add_filter("GIF", &["gif"]).pick_file().await else {
        return Ok(None);
    };
    photo_from_path(f.path().to_path_buf()).await.map(Some)
}

/// Pede uma foto ao sistema e prepara para mandar.
pub async fn pick_photo() -> Result<Option<PhotoDraft>, String> {
    let Some(f) = rfd::AsyncFileDialog::new()
        .set_title("Escolher foto")
        .add_filter("Imagens", &["jpg", "jpeg", "png", "webp", "gif", "bmp"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    photo_from_path(f.path().to_path_buf()).await.map(Some)
}

/// Pede uma foto para o perfil ou para um grupo, para depois recortar.
pub async fn pick_for_crop(target: CropTarget) -> Result<Option<CropDraft>, String> {
    let Some(f) = rfd::AsyncFileDialog::new()
        .set_title("Escolher foto")
        .add_filter("Imagens", &["jpg", "jpeg", "png", "webp", "gif", "bmp"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    crop_from_path(target, f.path().to_path_buf()).await
}

/// Abre uma foto do disco para recortar.
pub async fn crop_from_path(target: CropTarget, p: PathBuf) -> Result<Option<CropDraft>, String> {
    let bytes = read_limited(&p, MAX_INPUT, "Foto grande demais (até 30 MB).").await?;
    tokio::task::spawn_blocking(move || {
        let img = jm_media::photo::open(&bytes).map_err(|_| "Não consegui abrir essa imagem.".to_string())?;
        let (w, h) = (img.width().max(1), img.height().max(1));
        let (dw, dh, rgba) = jm_media::photo::to_rgba(&bytes, 900).map_err(|_| "Não consegui abrir essa imagem.".to_string())?;
        Ok(Some(CropDraft { target, bytes: Arc::new(bytes), w, h, handle: image::Handle::from_rgba(dw, dh, rgba) }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Lê um arquivo de áudio (para um som novo do soundboard).
pub async fn audio_from_path(p: PathBuf) -> Result<(String, Arc<Vec<f32>>), String> {
    let bytes = read_limited(&p, 20 * 1024 * 1024, "Arquivo grande demais (até 20 MB).").await?;
    let nome: String = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Meu som")
        .replace(['_', '-'], " ")
        .trim()
        .chars()
        .take(24)
        .collect();
    let pcm = tokio::task::spawn_blocking(move || jm_media::audiofile::decode(&bytes))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|_| "Não consegui ler esse áudio. Tente outro arquivo.".to_string())?;
    Ok((if nome.is_empty() { "Meu som".into() } else { nome }, Arc::new(pcm.samples)))
}

pub async fn pick_audio() -> Result<Option<(String, Arc<Vec<f32>>)>, String> {
    let Some(f) = rfd::AsyncFileDialog::new()
        .set_title("Escolher áudio")
        .add_filter("Áudio", &["mp3", "m4a", "aac", "ogg", "oga", "opus", "wav", "flac", "webm"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    audio_from_path(f.path().to_path_buf()).await.map(Some)
}

/// Salva uma foto recebida onde a pessoa escolher.
/// Escolhe um documento qualquer para mandar (até 5 MB).
pub async fn pick_file() -> Result<Option<(String, String, Vec<u8>)>, String> {
    let Some(f) = rfd::AsyncFileDialog::new().set_title("Escolher arquivo").pick_file().await else {
        return Ok(None);
    };
    let meta = tokio::fs::metadata(f.path()).await.map_err(|e| e.to_string())?;
    if meta.len() > jm_core::media::MAX_FILE as u64 {
        return Err("Arquivo grande demais: o limite é 5 MB.".into());
    }
    let bytes = tokio::fs::read(f.path()).await.map_err(|e| e.to_string())?;
    let nome = f.file_name();
    Ok(Some((nome.clone(), mime_of(&nome), bytes)))
}

/// Tipo do arquivo pela extensão (o bastante para o outro lado mostrar o ícone certo).
pub fn mime_of(nome: &str) -> String {
    let ext = nome.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "application/octet-stream",
    }
    .into()
}

/// Salva um arquivo recebido onde a pessoa escolher.
pub async fn save_file(bytes: Vec<u8>, nome: String) -> Result<String, String> {
    let Some(f) = rfd::AsyncFileDialog::new().set_title("Salvar arquivo").set_file_name(&nome).save_file().await else {
        return Err(String::new());
    };
    tokio::fs::write(f.path(), bytes).await.map_err(|e| e.to_string())?;
    Ok("Arquivo salvo".into())
}

/// Nome deste computador e o sistema, para a lista de aparelhos da conta.
pub fn device_name() -> (String, String) {
    let nome = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Computador".into());
    let sistema = if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    };
    (nome, sistema.into())
}

/// Grava o arquivo de backup onde a pessoa escolher.
pub async fn save_backup(bytes: Vec<u8>) -> Result<String, String> {
    let nome = format!("J-Menseger backup {}.jmbackup", chrono::Local::now().format("%Y-%m-%d"));
    let Some(f) = rfd::AsyncFileDialog::new()
        .set_title("Salvar backup")
        .set_file_name(&nome)
        .add_filter("Backup do J-Menseger", &["jmbackup"])
        .save_file()
        .await
    else {
        return Err(String::new());
    };
    tokio::fs::write(f.path(), bytes).await.map_err(|e| e.to_string())?;
    Ok("Backup salvo. Guarde junto com suas palavras de recuperação.".into())
}

pub async fn save_photo(bytes: Vec<u8>, mime: String) -> Result<String, String> {
    let ext = if mime.contains("gif") {
        "gif"
    } else if mime.contains("png") {
        "png"
    } else if mime.contains("webp") {
        "webp"
    } else {
        "jpg"
    };
    let nome = format!("J-Menseger {}.{ext}", chrono::Local::now().format("%Y-%m-%d %H-%M-%S"));
    let Some(f) = rfd::AsyncFileDialog::new().set_title("Salvar foto").set_file_name(&nome).save_file().await else {
        return Err(String::new());
    };
    tokio::fs::write(f.path(), bytes).await.map_err(|e| e.to_string())?;
    Ok("Foto salva".into())
}
