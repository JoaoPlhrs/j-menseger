//! Busca de GIFs no Tenor e no Giphy.
//!
//! As chaves de API são do próprio J-Menseger, embutidas ao compilar. A
//! busca vai direto deste computador para o serviço (ele vê o termo buscado
//! e o IP).
//! O GIF escolhido é baixado aqui e enviado cifrado como uma foto: quem
//! recebe não acessa o Tenor nem o Giphy.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Provider {
    #[default]
    Tenor,
    Giphy,
}

impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Provider::Tenor => "Tenor",
            Provider::Giphy => "GIPHY",
        }
    }
}

/// Chaves de API **do próprio J-Menseger** (não da pessoa).
///
/// Ficam embutidas no programa na hora de compilar. Dá para passar pelas
/// variáveis `JM_TENOR_KEY` e `JM_GIPHY_KEY` ao compilar, ou escrever aqui.
/// Vazias, a busca de GIFs fica desligada (GIF do computador continua
/// funcionando).
const TENOR_APP_KEY: &str = "";
const GIPHY_APP_KEY: &str = "HPMOkQnffVZSf1lRx8PmAhJ5j2D0UMgj";

pub fn tenor_key() -> &'static str {
    option_env!("JM_TENOR_KEY").filter(|k| !k.trim().is_empty()).unwrap_or(TENOR_APP_KEY).trim()
}

pub fn giphy_key() -> &'static str {
    option_env!("JM_GIPHY_KEY").filter(|k| !k.trim().is_empty()).unwrap_or(GIPHY_APP_KEY).trim()
}

/// Serviço preferido (gravado só neste computador).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GifKeys {
    #[serde(default)]
    pub provider: Provider,
}

/// Quais serviços esta versão do programa tem chave.
pub fn available() -> Vec<Provider> {
    let mut v = Vec::new();
    if !tenor_key().is_empty() {
        v.push(Provider::Tenor);
    }
    if !giphy_key().is_empty() {
        v.push(Provider::Giphy);
    }
    v
}

impl GifKeys {
    /// Serviço que dá para usar agora: o preferido, ou o outro se só ele
    /// tiver chave.
    pub fn usable(&self) -> Option<(Provider, String)> {
        pick(self.provider, tenor_key(), giphy_key())
    }
}

fn pick(pref: Provider, t: &str, g: &str) -> Option<(Provider, String)> {
    match pref {
        Provider::Tenor if !t.is_empty() => Some((Provider::Tenor, t.to_string())),
        Provider::Giphy if !g.is_empty() => Some((Provider::Giphy, g.to_string())),
        _ if !t.is_empty() => Some((Provider::Tenor, t.to_string())),
        _ if !g.is_empty() => Some((Provider::Giphy, g.to_string())),
        _ => None,
    }
}

/// Um resultado da busca.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: String,
    pub title: String,
    /// GIF pequeno para a grade.
    pub preview: String,
    /// GIF para mandar (até 2 MB, quando o serviço informa o tamanho).
    pub send: String,
    pub w: u32,
    pub h: u32,
}

const LIMITE: u32 = 24;
const MAX_PREVIEW: usize = 1_500_000;

fn agente() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .try_proxy_from_env(true)
        .user_agent("J-Menseger")
        .build()
}

fn erro_http(p: Provider, e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(400 | 401 | 403, _) => format!("O {} recusou a chave do aplicativo. A busca de GIFs volta numa próxima versão.", p.label()),
        ureq::Error::Status(429, _) => format!("Limite de buscas do {} atingido. Tente daqui a pouco.", p.label()),
        ureq::Error::Status(c, _) => format!("O {} respondeu com erro {c}.", p.label()),
        ureq::Error::Transport(t) => {
            tracing::warn!("busca de GIFs: {}", t.kind());
            format!("Sem conexão com o {}. Confira a internet e tente de novo.", p.label())
        }
    }
}

/// Busca GIFs. Com `query` vazio, traz os populares do momento.
pub fn search(p: Provider, key: &str, query: &str) -> Result<Vec<Hit>, String> {
    let q = query.trim();
    let lim = LIMITE.to_string();
    let corpo = match p {
        Provider::Tenor => {
            let url = if q.is_empty() { "https://tenor.googleapis.com/v2/featured" } else { "https://tenor.googleapis.com/v2/search" };
            let mut r = agente()
                .get(url)
                .query("key", key)
                .query("client_key", "j-menseger")
                .query("limit", &lim)
                .query("media_filter", "gif,mediumgif,tinygif,nanogif")
                .query("contentfilter", "medium")
                .query("locale", "pt_BR");
            if !q.is_empty() {
                r = r.query("q", q);
            }
            r.call().map_err(|e| erro_http(p, e))?.into_string().map_err(|e| e.to_string())?
        }
        Provider::Giphy => {
            let url = if q.is_empty() { "https://api.giphy.com/v1/gifs/trending" } else { "https://api.giphy.com/v1/gifs/search" };
            let mut r = agente().get(url).query("api_key", key).query("limit", &lim).query("rating", "pg-13");
            if !q.is_empty() {
                r = r.query("q", q).query("lang", "pt");
            }
            r.call().map_err(|e| erro_http(p, e))?.into_string().map_err(|e| e.to_string())?
        }
    };
    let v: Value = serde_json::from_str(&corpo).map_err(|_| format!("Resposta estranha do {}.", p.label()))?;
    Ok(match p {
        Provider::Tenor => parse_tenor(&v),
        Provider::Giphy => parse_giphy(&v),
    })
}

fn num(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

pub fn parse_tenor(v: &Value) -> Vec<Hit> {
    let Some(lista) = v["results"].as_array() else { return Vec::new() };
    lista
        .iter()
        .filter_map(|r| {
            let f = &r["media_formats"];
            let fmt = |n: &str| -> Option<(String, u32, u32, u64)> {
                let m = &f[n];
                let url = m["url"].as_str()?.to_string();
                let d = m["dims"].as_array();
                let w = d.and_then(|d| d.first()).and_then(num).unwrap_or(0) as u32;
                let h = d.and_then(|d| d.get(1)).and_then(num).unwrap_or(0) as u32;
                Some((url, w, h, num(&m["size"]).unwrap_or(0)))
            };
            let preview = fmt("tinygif").or_else(|| fmt("nanogif")).or_else(|| fmt("mediumgif"))?;
            let cabe = |x: &(String, u32, u32, u64)| x.3 > 0 && x.3 <= jm_media::gif::GIF_MAX as u64;
            let envio = fmt("gif").filter(cabe).or_else(|| fmt("mediumgif").filter(cabe)).unwrap_or_else(|| preview.clone());
            Some(Hit {
                id: r["id"].as_str().unwrap_or_default().to_string(),
                title: r["content_description"].as_str().unwrap_or_default().to_string(),
                preview: preview.0,
                send: envio.0,
                w: envio.1,
                h: envio.2,
            })
        })
        .collect()
}

pub fn parse_giphy(v: &Value) -> Vec<Hit> {
    let Some(lista) = v["data"].as_array() else { return Vec::new() };
    lista
        .iter()
        .filter_map(|r| {
            let im = &r["images"];
            let fmt = |n: &str| -> Option<(String, u32, u32, u64)> {
                let m = &im[n];
                let url = m["url"].as_str()?.to_string();
                Some((url, num(&m["width"]).unwrap_or(0) as u32, num(&m["height"]).unwrap_or(0) as u32, num(&m["size"]).unwrap_or(0)))
            };
            let preview = fmt("fixed_width_downsampled").or_else(|| fmt("fixed_width_small")).or_else(|| fmt("fixed_width"))?;
            let cabe = |x: &(String, u32, u32, u64)| x.3 > 0 && x.3 <= jm_media::gif::GIF_MAX as u64;
            let envio = fmt("downsized").filter(cabe).or_else(|| fmt("fixed_width").filter(cabe)).unwrap_or_else(|| preview.clone());
            Some(Hit {
                id: r["id"].as_str().unwrap_or_default().to_string(),
                title: r["title"].as_str().unwrap_or_default().to_string(),
                preview: preview.0,
                send: envio.0,
                w: envio.1,
                h: envio.2,
            })
        })
        .collect()
}

/// Só baixa de endereços de mídia do Tenor e do Giphy, por HTTPS: a
/// resposta da busca não consegue mandar o programa buscar outra coisa.
pub fn allowed(url: &str) -> bool {
    let Some(resto) = url.strip_prefix("https://") else { return false };
    let host = resto.split(['/', '?', '#']).next().unwrap_or("").to_ascii_lowercase();
    if host.contains(':') || host.contains('@') {
        return false;
    }
    host == "media.tenor.com" || host.ends_with(".tenor.com") || host == "giphy.com" || host.ends_with(".giphy.com")
}

/// Baixa um GIF (com limite de tamanho).
pub fn download(url: &str, max: usize) -> Result<Vec<u8>, String> {
    if !allowed(url) {
        return Err("Endereço de GIF não permitido.".into());
    }
    let r = agente().get(url).call().map_err(|e| match e {
        ureq::Error::Status(c, _) => format!("Não deu para baixar o GIF (erro {c})."),
        ureq::Error::Transport(_) => "Não deu para baixar o GIF. Confira a internet.".to_string(),
    })?;
    let mut buf = Vec::new();
    r.into_reader().take(max as u64 + 1).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    if buf.len() > max {
        return Err("GIF grande demais.".into());
    }
    Ok(buf)
}

pub fn download_preview(url: &str) -> Result<Vec<u8>, String> {
    download(url, MAX_PREVIEW)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_a_resposta_do_tenor() {
        // Formato da API v2 (search/featured), resumido.
        let v: Value = serde_json::from_str(
            r#"{"results":[{"id":"123","content_description":"gato dançando",
              "media_formats":{
                "gif":{"url":"https://media.tenor.com/a/grande.gif","dims":[498,280],"size":3500000},
                "mediumgif":{"url":"https://media.tenor.com/a/medio.gif","dims":[320,180],"size":900000},
                "tinygif":{"url":"https://media.tenor.com/a/pequeno.gif","dims":[220,124],"size":150000}}}],
              "next":"x"}"#,
        )
        .unwrap();
        let r = parse_tenor(&v);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].preview, "https://media.tenor.com/a/pequeno.gif");
        // O "gif" passa de 2 MB: manda o médio.
        assert_eq!(r[0].send, "https://media.tenor.com/a/medio.gif");
        assert_eq!((r[0].w, r[0].h), (320, 180));
        assert_eq!(r[0].title, "gato dançando");
    }

    #[test]
    fn le_a_resposta_do_giphy() {
        let v: Value = serde_json::from_str(
            r#"{"data":[{"id":"abc","title":"oi GIF",
              "images":{
                "downsized":{"url":"https://media2.giphy.com/media/abc/giphy-downsized.gif","width":"480","height":"270","size":"1200000"},
                "fixed_width":{"url":"https://media2.giphy.com/media/abc/200w.gif","width":"200","height":"113","size":"400000"},
                "fixed_width_downsampled":{"url":"https://media2.giphy.com/media/abc/200w_d.gif","width":"200","height":"113","size":"90000"}}}],
              "pagination":{},"meta":{"status":200}}"#,
        )
        .unwrap();
        let r = parse_giphy(&v);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].preview, "https://media2.giphy.com/media/abc/200w_d.gif");
        assert_eq!(r[0].send, "https://media2.giphy.com/media/abc/giphy-downsized.gif");
        assert_eq!((r[0].w, r[0].h), (480, 270));
    }

    #[test]
    fn so_baixa_do_tenor_e_do_giphy() {
        assert!(allowed("https://media.tenor.com/x/y.gif"));
        assert!(allowed("https://media2.giphy.com/media/abc/200w.gif"));
        assert!(!allowed("http://media.tenor.com/x.gif"));
        assert!(!allowed("https://evil.com/media.tenor.com/x.gif"));
        assert!(!allowed("https://media.tenor.com.evil.com/x.gif"));
        assert!(!allowed("https://user@media.tenor.com/x.gif"));
        assert!(!allowed("https://localhost:8080/x.gif"));
    }

    #[test]
    fn usa_o_servico_que_tem_chave() {
        assert!(pick(Provider::Tenor, "", "").is_none());
        assert_eq!(pick(Provider::Tenor, "", "g"), Some((Provider::Giphy, "g".into())));
        assert_eq!(pick(Provider::Tenor, "t", "g").unwrap().0, Provider::Tenor);
        assert_eq!(pick(Provider::Giphy, "t", "g").unwrap().0, Provider::Giphy);
    }
}
