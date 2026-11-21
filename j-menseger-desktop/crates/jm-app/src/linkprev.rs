//! Prévia de links, feita por quem manda.
//!
//! Só o seu computador acessa o site (o mesmo que aconteceria se você mesmo
//! abrisse o link); título, descrição e uma miniatura vão cifrados junto com
//! a mensagem. Quem recebe não acessa nada — o site não fica sabendo quem
//! recebeu o link. Dá para desligar em Privacidade.

use jm_core::messages::LinkPreview;
use std::io::Read;
use std::time::Duration;

const MAX_HTML: u64 = 600_000;
const MAX_IMG: u64 = 1_500_000;

fn agente() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(4))
        .timeout(Duration::from_secs(6))
        .redirects(4)
        .try_proxy_from_env(!(cfg!(debug_assertions) && std::env::var("J_MENSEGER_TESTE_LINKS").is_ok()))
        .user_agent("Mozilla/5.0 (compatible; J-Menseger link preview)")
        .build()
}

/// O primeiro link do texto, se houver.
pub fn first_link(texto: &str) -> Option<String> {
    texto
        .split_whitespace()
        .find(|p| p.starts_with("https://") || p.starts_with("http://"))
        .map(|p| p.trim_end_matches(['.', ',', ')', ']', '!', '?', ';', ':', '"', '\'']).to_string())
}

/// Endereço local/privado (roteador, rede de casa): nunca acessar.
fn privado(url: &str) -> bool {
    // Só nos testes automáticos (versão de desenvolvimento): site local de teste.
    if cfg!(debug_assertions) && std::env::var("J_MENSEGER_TESTE_LINKS").is_ok() {
        return false;
    }
    let host = url.split("://").nth(1).unwrap_or("").split(['/', '?', '#']).next().unwrap_or("").split('@').last().unwrap_or("");
    let host = host.rsplit_once(':').map(|(h, p)| if p.chars().all(|c| c.is_ascii_digit()) { h } else { host }).unwrap_or(host).to_ascii_lowercase();
    host.is_empty()
        || host == "localhost"
        || host.ends_with(".local")
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
        || (host.starts_with("172.") && host.split('.').nth(1).and_then(|x| x.parse::<u8>().ok()).is_some_and(|x| (16..=31).contains(&x)))
        || host.starts_with('[')
        || !host.contains('.')
}

fn desescapa(s: &str) -> String {
    s.replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'").replace("&#x27;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&nbsp;", " ")
}

/// Valor de uma etiqueta <meta property|name="..." content="...">.
fn meta(html: &str, chave: &str) -> Option<String> {
    let baixo = html.to_ascii_lowercase();
    let mut pos = 0;
    while let Some(i) = baixo[pos..].find("<meta") {
        let ini = pos + i;
        let fim = baixo[ini..].find('>').map(|f| ini + f)?;
        let tag = &html[ini..fim];
        let tag_b = &baixo[ini..fim];
        let tem = [format!("property=\"{chave}\""), format!("name=\"{chave}\""), format!("property='{chave}'"), format!("name='{chave}'")]
            .iter()
            .any(|p| tag_b.contains(p.as_str()));
        if tem {
            for aspas in ['"', '\''] {
                let k = format!("content={aspas}");
                if let Some(c) = tag_b.find(&k) {
                    let r = &tag[c + k.len()..];
                    if let Some(e) = r.find(aspas) {
                        let v = desescapa(r[..e].trim());
                        if !v.is_empty() {
                            return Some(v);
                        }
                    }
                }
            }
        }
        pos = fim;
    }
    None
}

fn titulo(html: &str) -> Option<String> {
    let b = html.to_ascii_lowercase();
    let i = b.find("<title")?;
    let a = b[i..].find('>')? + i + 1;
    let f = b[a..].find("</title>")? + a;
    let t = desescapa(html[a..f].trim());
    (!t.is_empty()).then_some(t)
}

fn absoluto(base: &str, u: &str) -> String {
    if u.starts_with("https://") || u.starts_with("http://") {
        return u.to_string();
    }
    if let Some(r) = u.strip_prefix("//") {
        return format!("https://{r}");
    }
    let raiz: String = base.splitn(4, '/').take(3).collect::<Vec<_>>().join("/");
    if u.starts_with('/') {
        format!("{raiz}{u}")
    } else {
        format!("{}/{u}", base.rsplit_once('/').map(|(a, _)| a).unwrap_or(&raiz))
    }
}

/// Busca título, descrição e miniatura. `None` se der errado (a mensagem
/// sai sem prévia, nunca deixa de sair).
pub fn fetch(url: &str) -> Option<LinkPreview> {
    if privado(url) {
        return None;
    }
    let ag = agente();
    let r = ag.get(url).call().ok()?;
    let tipo = r.content_type().to_ascii_lowercase();
    if !tipo.contains("html") {
        return None;
    }
    let final_url = r.get_url().to_string();
    let mut buf = Vec::new();
    r.into_reader().take(MAX_HTML).read_to_end(&mut buf).ok()?;
    let html = String::from_utf8_lossy(&buf);
    let t = meta(&html, "og:title").or_else(|| meta(&html, "twitter:title")).or_else(|| titulo(&html))?;
    let d = meta(&html, "og:description").or_else(|| meta(&html, "description")).unwrap_or_default();
    let img = meta(&html, "og:image").or_else(|| meta(&html, "twitter:image")).map(|i| absoluto(&final_url, &i));
    let i = img.filter(|u| !privado(u)).and_then(|u| {
        let r = ag.get(&u).call().ok()?;
        let mut b = Vec::new();
        r.into_reader().take(MAX_IMG).read_to_end(&mut b).ok()?;
        jm_media::photo::link_thumb(&b).ok()
    });
    LinkPreview { u: url.to_string(), t, d, i }.clean()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_as_etiquetas_og() {
        let h = r#"<html><head><title>Fallback</title><meta property="og:title" content="Título &amp; tal"><meta name='description' content='Algo legal'></head></html>"#;
        assert_eq!(meta(h, "og:title").as_deref(), Some("Título & tal"));
        assert_eq!(meta(h, "description").as_deref(), Some("Algo legal"));
        assert_eq!(titulo(h).as_deref(), Some("Fallback"));
    }

    #[test]
    fn nao_acessa_rede_local() {
        assert!(privado("http://192.168.0.1/admin"));
        assert!(privado("http://localhost:8080"));
        assert!(privado("http://172.20.1.1/"));
        assert!(!privado("https://www.youtube.com/watch?v=x"));
        assert_eq!(first_link("olha isso https://exemplo.com/a.").as_deref(), Some("https://exemplo.com/a"));
    }
}
