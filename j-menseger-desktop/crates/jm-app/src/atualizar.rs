//! Atualização automática do programa do computador.
//!
//! O site publica `versao.json` com a versão mais nova, o endereço do
//! instalador e o sha256 dele. O app confere ao abrir e a cada 6 horas; se
//! houver versão nova, baixa o instalador para uma pasta temporária, confere
//! o sha256 e roda o instalador em modo automático (`--instalar --auto`),
//! que troca o programa e o abre de novo. As conversas não são tocadas.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Onde fica o manifesto. Para testes: `J_MENSEGER_ATUALIZACAO=<url>`.
pub const MANIFESTO: &str = "https://j-menseger-site.vercel.app/versao.json";
pub const VERSAO: &str = env!("CARGO_PKG_VERSION");
/// Instalador maior que isso não é baixado (proteção contra resposta errada).
const MAX_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Pacote {
    pub versao: String,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub novidades: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Manifesto {
    windows: Option<Pacote>,
}

fn url_manifesto() -> String {
    std::env::var("J_MENSEGER_ATUALIZACAO").unwrap_or_else(|_| MANIFESTO.to_string())
}

/// "0.4.10" > "0.4.9"; partes que não são número contam como 0.
pub fn mais_nova(a: &str, b: &str) -> bool {
    let p = |s: &str| -> Vec<u64> { s.split(['.', '-']).take(3).map(|x| x.parse().unwrap_or(0)).collect() };
    let (x, y) = (p(a), p(b));
    for i in 0..3 {
        let (u, v) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
        if u != v {
            return u > v;
        }
    }
    false
}

fn agente_para(url: &str) -> ureq::Agent {
    let local = url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost");
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .try_proxy_from_env(!local)
        .user_agent(&format!("J-Menseger/{VERSAO}"))
        .build()
}

/// Procura versão nova. `Ok(None)`: já está na mais nova (ou não há
/// atualização para este sistema).
pub fn procurar() -> Result<Option<Pacote>, String> {
    if !cfg!(windows) && std::env::var("J_MENSEGER_ATUALIZACAO").is_err() {
        return Ok(None);
    }
    let txt = agente_para(&url_manifesto()).get(&url_manifesto()).call().map_err(|e| format!("Sem conexão com o site de atualizações ({e}).")).and_then(|r| r.into_string().map_err(|e| e.to_string()))?;
    let m: Manifesto = serde_json::from_str(&txt).map_err(|_| "Resposta estranha do site de atualizações.".to_string())?;
    Ok(m.windows.filter(|p| mais_nova(&p.versao, VERSAO) && p.url.starts_with("https://") || p_teste(p)))
}

/// Nos testes o manifesto pode apontar para http://127.0.0.1.
fn p_teste(p: &Pacote) -> bool {
    std::env::var("J_MENSEGER_ATUALIZACAO").is_ok() && p.url.starts_with("http://127.0.0.1") && mais_nova(&p.versao, VERSAO)
}

/// Baixa o instalador, conferindo o sha256. `feito` recebe o progresso em
/// milésimos.
pub fn baixar(p: &Pacote, feito: Arc<AtomicU64>) -> Result<PathBuf, String> {
    let r = agente_para(&p.url).get(&p.url).call().map_err(|e| format!("Não deu para baixar a atualização ({e})."))?;
    let total: u64 = r.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    if total > MAX_BYTES {
        return Err("Atualização grande demais.".into());
    }
    let mut leitor = r.into_reader().take(MAX_BYTES + 1);
    let mut dados: Vec<u8> = Vec::with_capacity(total as usize);
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = leitor.read(&mut buf).map_err(|e| format!("A conexão caiu durante o download ({e})."))?;
        if n == 0 {
            break;
        }
        dados.extend_from_slice(&buf[..n]);
        if total > 0 {
            feito.store(dados.len() as u64 * 1000 / total, Ordering::Relaxed);
        }
    }
    let soma = hex::encode(Sha256::digest(&dados));
    if !soma.eq_ignore_ascii_case(p.sha256.trim()) {
        return Err("O arquivo baixado não confere com o do site. A atualização foi cancelada.".into());
    }
    let pasta = std::env::temp_dir().join("J-Menseger-atualizacao");
    std::fs::create_dir_all(&pasta).map_err(|e| e.to_string())?;
    let nome = if cfg!(windows) { format!("J-Menseger-{}-instalador.exe", p.versao) } else { format!("J-Menseger-{}-instalador", p.versao) };
    let destino = pasta.join(nome);
    std::fs::write(&destino, &dados).map_err(|e| format!("Não deu para gravar a atualização ({e})."))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&destino, std::fs::Permissions::from_mode(0o755));
    }
    feito.store(1000, Ordering::Relaxed);
    Ok(destino)
}

/// Roda o instalador baixado em modo automático. O app deve fechar logo
/// depois (o instalador reabre o programa novo).
pub fn instalar(instalador: &std::path::Path) -> Result<(), String> {
    let mut c = std::process::Command::new(instalador);
    c.args(["--instalar", "--auto"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0000_0008);
    }
    c.spawn().map(|_| ()).map_err(|e| format!("Não deu para abrir o instalador ({e})."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compara_versoes() {
        assert!(mais_nova("0.4.10", "0.4.9"));
        assert!(mais_nova("0.5.0", "0.4.9"));
        assert!(mais_nova("1.0.0", "0.9.99"));
        assert!(!mais_nova("0.4.0", "0.4.0"));
        assert!(!mais_nova("0.3.9", "0.4.0"));
        assert!(mais_nova("0.6.0-beta", "0.5.0"));
    }
}
