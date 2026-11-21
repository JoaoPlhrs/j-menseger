//! Regras de mídia do protocolo: limites, formatos aceitos e validação.
//!
//! Fotos, áudios e sons viajam fatiados dentro dos próprios envelopes
//! cifrados: um cabeçalho (`mh`) e blocos de 20 000 bytes (`mc`). Não existe
//! servidor de arquivos. Tudo aqui segue o aplicativo de celular, porque é
//! ele quem está do outro lado da conversa.

use crate::messages::{MediaHeader, MediaKind, SoundMeta};
use crate::protocol::CHUNK;
use serde::{Deserialize, Serialize};

/// Maior mídia aceita, em bytes.
pub const MAX_MEDIA: usize = 2_400_000;
/// Foto de perfil ou de grupo: data URL pequena.
pub const PIC_MAX: usize = 24_000;
/// Miniatura embutida no cabeçalho de uma foto.
pub const THUMB_MAX: usize = 3_000;
/// Som próprio do soundboard.
pub const MAX_SOUND_BYTES: usize = 160_000;

/// Quantos blocos uma mídia do tamanho máximo ocupa.
pub fn max_chunks() -> u32 {
    MAX_MEDIA.div_ceil(CHUNK) as u32
}

/// Formatos aceitos. O celular grava áudio em WebM ou Ogg (Opus), às vezes
/// MP4; fotos saem em WebP ou JPEG. GIF animado vem do computador (o
/// celular ainda precisa ser atualizado para aceitar).
pub fn mime_ok(mime: &str) -> bool {
    let m = mime.to_ascii_lowercase();
    let (base, params) = match m.split_once(';') {
        Some((b, p)) => (b, Some(p)),
        None => (m.as_str(), None),
    };
    let base_ok = matches!(
        base,
        "audio/webm" | "audio/ogg" | "audio/mp4" | "audio/mpeg" | "audio/aac" | "image/webp" | "image/jpeg" | "image/png" | "image/gif"
    );
    let params_ok = params.is_none_or(|p| {
        p.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "=,. -".contains(c))
    });
    base_ok && params_ok
}

/// Resultado da validação de uma foto de perfil ou de grupo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pic {
    /// Foto válida.
    Set(String),
    /// Texto vazio: a pessoa removeu a foto.
    Remove,
    /// Qualquer outra coisa: ignorada.
    Invalid,
}

/// Aceita só `data:image/(webp|jpeg|png);base64,...`, pequena.
///
/// Qualquer outra coisa (um endereço da internet, por exemplo) seria um jeito
/// de alguém descobrir quando você abre o perfil dele.
pub fn clean_pic(p: &str) -> Pic {
    clean_image_url(p, PIC_MAX)
}

/// Banner do perfil: imagem larga (3:1), no mesmo limite de tamanho da foto.
/// Vai numa mensagem separada do perfil para o envelope caber nos servidores.
pub const BANNER_MAX: usize = 24_000;

pub fn clean_banner(p: &str) -> Pic {
    clean_image_url(p, BANNER_MAX)
}

/// Jogos favoritos: até 6 nomes de até 30 caracteres, sem repetir.
pub const MAX_GAMES: usize = 6;

pub fn clean_games(v: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for g in v {
        let g: String = g.trim().chars().filter(|c| !c.is_control()).take(30).collect();
        if !g.is_empty() && !out.iter().any(|x| x.to_lowercase() == g.to_lowercase()) {
            out.push(g);
        }
        if out.len() == MAX_GAMES {
            break;
        }
    }
    out
}

fn clean_image_url(p: &str, max: usize) -> Pic {
    if p.is_empty() {
        return Pic::Remove;
    }
    if p.len() > max {
        return Pic::Invalid;
    }
    let Some(rest) = ["data:image/webp;base64,", "data:image/jpeg;base64,", "data:image/png;base64,"]
        .iter()
        .find_map(|pre| p.strip_prefix(pre))
    else {
        return Pic::Invalid;
    };
    let corpo = rest.trim_end_matches('=');
    let iguais = rest.len() - corpo.len();
    if corpo.is_empty()
        || iguais > 2
        || !corpo.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'+' || c == b'/')
    {
        return Pic::Invalid;
    }
    Pic::Set(p.to_string())
}

/// Como uma mídia fica registrada na mensagem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaInfo {
    pub mid: String,
    #[serde(rename = "type")]
    pub kind: MediaKind,
    pub mime: String,
    pub size: u64,
    pub n: u32,
    #[serde(default)]
    pub sha: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dur: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wf: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub th: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sb: Option<SoundMeta>,
    /// Blocos já recebidos.
    #[serde(default)]
    pub got: u32,
    /// A mídia está inteira e conferida.
    #[serde(default)]
    pub ready: bool,
    /// Chegou corrompida (o hash não bateu).
    #[serde(default)]
    pub err: bool,
}

/// Confere um cabeçalho que veio de outro aparelho, com os mesmos cortes do
/// celular. `None` quando não dá para confiar nele.
pub fn validate_header(h: &MediaHeader) -> Option<MediaInfo> {
    if !is_mid(&h.mid) {
        return None;
    }
    if h.n == 0 || h.n > max_chunks() || h.size == 0 || h.size > MAX_MEDIA as u64 || !mime_ok(&h.mime) {
        return None;
    }
    let mut info = MediaInfo {
        mid: h.mid.clone(),
        kind: h.kind,
        mime: h.mime.clone(),
        size: h.size,
        n: h.n,
        sha: if is_hex64(&h.sha) { h.sha.to_ascii_lowercase() } else { String::new() },
        dur: h.dur.map(|d| d.min(3600)),
        wf: h.wf.as_ref().map(|v| v.iter().take(48).map(|x| (*x).clamp(0, 31) as u8).collect()),
        w: None,
        h: None,
        th: None,
        sb: None,
        got: 0,
        ready: false,
        err: false,
    };
    if let (Some(w), Some(hh)) = (h.w, h.h) {
        info.w = Some(w.min(10_000));
        info.h = Some(hh.min(10_000));
    }
    if let Some(th) = &h.th {
        let prefixo_ok = ["data:image/webp;base64,", "data:image/jpeg;base64,", "data:image/png;base64,"]
            .iter()
            .any(|p| th.starts_with(p));
        if prefixo_ok && th.len() < THUMB_MAX {
            info.th = Some(th.clone());
        }
    }
    if h.kind == MediaKind::Audio {
        if let Some(sb) = &h.sb {
            info.sb = Some(clean_sound_meta(&sb.n, &sb.e));
        }
    }
    Some(info)
}

/// Nome e emoji de um som próprio, aparados como no celular.
pub fn clean_sound_meta(n: &str, e: &str) -> SoundMeta {
    let n: String = n.trim().chars().take(24).collect();
    let e: String = e.trim().chars().take(8).collect();
    SoundMeta {
        n: if n.is_empty() { "Som".into() } else { n },
        e: if e.is_empty() { "🔊".into() } else { e },
    }
}

/// Um bloco recebido é plausível?
pub fn chunk_ok(mid: &str, i: u32, d: &str) -> bool {
    is_mid(mid) && i <= 200 && d.len() <= 30_000
}

/// Identificador de mídia: 16 bytes aleatórios em hexadecimal minúsculo.
pub fn is_mid(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}

/// Identificador de um som próprio: `c` + 15 dígitos do SHA-256 dos bytes.
/// Igual nos dois aparelhos, então o mesmo som não é mandado duas vezes.
pub fn sound_id(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let h = hex::encode(Sha256::digest(bytes));
    format!("c{}", &h[..15])
}

/// `c` seguido de 15 dígitos hexadecimais.
pub fn is_custom_sound_id(s: &str) -> bool {
    s.len() == 16
        && s.starts_with('c')
        && s[1..].bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cab() -> MediaHeader {
        MediaHeader {
            mid: "0123456789abcdef0123456789abcdef".into(),
            kind: MediaKind::Image,
            mime: "image/webp".into(),
            size: 50_000,
            n: 3,
            sha: "ab".repeat(32),
            dur: None,
            wf: None,
            w: Some(800),
            h: Some(600),
            th: Some("data:image/webp;base64,AAAA".into()),
            sb: None,
        }
    }

    #[test]
    fn aceita_os_formatos_do_celular() {
        for m in ["audio/webm", "audio/webm;codecs=opus", "audio/ogg", "image/webp", "image/jpeg", "IMAGE/PNG", "image/gif"] {
            assert!(mime_ok(m), "{m}");
        }
        for m in ["video/mp4", "text/html", "image/svg+xml", "audio/webm;<script>"] {
            assert!(!mime_ok(m), "{m}");
        }
    }

    #[test]
    fn foto_de_perfil_so_em_data_url_pequena() {
        assert_eq!(clean_pic(""), Pic::Remove);
        assert!(matches!(clean_pic("data:image/jpeg;base64,/9j/4AAQ=="), Pic::Set(_)));
        assert_eq!(clean_pic("https://rastreador.exemplo/foto.png"), Pic::Invalid);
        assert_eq!(clean_pic("data:image/svg+xml;base64,AAAA"), Pic::Invalid);
        assert_eq!(clean_pic("data:image/png;base64,AA AA"), Pic::Invalid);
        assert_eq!(clean_pic("data:image/png;base64,AAA==="), Pic::Invalid);
        let grande = format!("data:image/webp;base64,{}", "A".repeat(PIC_MAX));
        assert_eq!(clean_pic(&grande), Pic::Invalid);
    }

    #[test]
    fn cabecalho_valido_passa_inteiro() {
        let info = validate_header(&cab()).unwrap();
        assert_eq!(info.n, 3);
        assert_eq!(info.w, Some(800));
        assert!(info.th.is_some());
        assert!(!info.ready);
    }

    #[test]
    fn cabecalho_perigoso_e_recusado() {
        let mut h = cab();
        h.mid = "../../etc/passwd".into();
        assert!(validate_header(&h).is_none(), "id de mídia estranho");

        let mut h = cab();
        h.size = (MAX_MEDIA + 1) as u64;
        assert!(validate_header(&h).is_none(), "grande demais");

        let mut h = cab();
        h.n = max_chunks() + 1;
        assert!(validate_header(&h).is_none(), "blocos demais");

        let mut h = cab();
        h.mime = "application/x-msdownload".into();
        assert!(validate_header(&h).is_none(), "formato não aceito");
    }

    #[test]
    fn forma_de_onda_e_aparada() {
        let mut h = cab();
        h.kind = MediaKind::Audio;
        h.mime = "audio/webm".into();
        h.wf = Some((0..100).map(|i| i * 7 - 50).collect());
        let info = validate_header(&h).unwrap();
        let wf = info.wf.unwrap();
        assert_eq!(wf.len(), 48);
        assert!(wf.iter().all(|v| *v <= 31));
    }

    #[test]
    fn som_proprio_so_em_audio_e_com_nome_aparado() {
        let mut h = cab();
        h.sb = Some(SoundMeta { n: "x".repeat(40), e: "🔊".into() });
        assert!(validate_header(&h).unwrap().sb.is_none(), "foto não carrega som");

        h.kind = MediaKind::Audio;
        h.mime = "audio/ogg".into();
        let sb = validate_header(&h).unwrap().sb.unwrap();
        assert_eq!(sb.n.chars().count(), 24);
    }

    #[test]
    fn id_de_som_proprio() {
        let id = sound_id(b"abc");
        assert!(is_custom_sound_id(&id));
        assert_eq!(id, sound_id(b"abc"), "mesmo som, mesmo id");
        assert_ne!(id, sound_id(b"abd"));
        assert!(!is_custom_sound_id("aplausos"));
    }
}
