//! Identidade: as 12 palavras, a chave privada e os códigos `npub`/`nsec`.
//!
//! A conta é um par de chaves criado no aparelho. As 12 palavras em português
//! (BIP-39) recriam exatamente a mesma chave em qualquer aparelho, usando a
//! derivação da NIP-06 — é isso que faz o computador e o celular serem o mesmo
//! usuário, e não duas contas parecidas.

use bip39::{Language, Mnemonic};
use nostr::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("as palavras não formam uma frase de recuperação válida")]
    InvalidWords,
    #[error("código de chave inválido")]
    InvalidKey,
    #[error("erro interno: {0}")]
    Other(String),
}

/// Quantas palavras a frase de recuperação tem.
pub const WORD_COUNT: usize = 12;

/// Sorteia uma frase de recuperação nova, em português.
pub fn new_words() -> Result<String, IdentityError> {
    let mut entropy = [0u8; 16]; // 128 bits = 12 palavras
    use rand::RngCore;
    rand::rng().fill_bytes(&mut entropy);
    let m = Mnemonic::from_entropy_in(Language::Portuguese, &entropy)
        .map_err(|e| IdentityError::Other(e.to_string()))?;
    Ok(m.words().collect::<Vec<_>>().join(" "))
}

/// Deixa a frase no formato que a derivação espera.
///
/// Aceita o que a pessoa realmente digita: maiúsculas, espaços sobrando,
/// quebras de linha e acentos que ela tenha colocado por conta própria. A lista
/// oficial em português não tem acento nenhum, então removê-los é seguro e
/// evita que a conta seja recusada por um detalhe de teclado.
pub fn normalize_words(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c if c.is_ascii_alphabetic() => c,
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Devolve a frase normalizada se ela for válida, em português ou em inglês.
///
/// O inglês entra porque uma conta pode ter vindo de outro aplicativo Nostr.
pub fn valid_words(input: &str) -> Option<String> {
    let n = normalize_words(input);
    let count = n.split(' ').filter(|w| !w.is_empty()).count();
    if count != 12 && count != 24 {
        return None;
    }
    for lang in [Language::Portuguese, Language::English] {
        if Mnemonic::parse_in_normalized(lang, &n).is_ok() {
            return Some(n);
        }
    }
    None
}

/// Deriva a chave da conta a partir da frase de recuperação (NIP-06).
pub fn keys_from_words(input: &str) -> Result<Keys, IdentityError> {
    let words = valid_words(input).ok_or(IdentityError::InvalidWords)?;
    Keys::from_mnemonic(&words, None).map_err(|e| IdentityError::Other(e.to_string()))
}

/// Aceita qualquer forma de entrar na conta: 12 palavras, `nsec1…` ou a chave
/// em hexadecimal.
pub fn keys_from_any(input: &str) -> Result<Keys, IdentityError> {
    let s = input.trim();
    if s.starts_with("nsec1") || s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Keys::parse(s).map_err(|_| IdentityError::InvalidKey);
    }
    keys_from_words(s)
}

/// Extrai a chave pública de um código colado pela pessoa: `npub1…`,
/// `nprofile1…`, hexadecimal ou um link `jmenseger://add?k=…`.
pub fn parse_pubkey(input: &str) -> Option<PublicKey> {
    let s = input.trim();
    // Procura o código dentro de um texto maior (link de convite, mensagem).
    let candidate = s
        .split(|c: char| !(c.is_ascii_alphanumeric()))
        .find(|p| p.starts_with("npub1") || p.starts_with("nprofile1"))
        .unwrap_or(s);
    if let Ok(pk) = PublicKey::parse(candidate) {
        return Some(pk);
    }
    PublicKey::parse(s).ok()
}

/// Código público da conta, o que a pessoa compartilha para ser adicionada.
pub fn npub(pk: &PublicKey) -> String {
    pk.to_bech32().unwrap_or_else(|_| pk.to_hex())
}

/// Chave secreta em formato legível. Nunca deve aparecer em registros.
pub fn nsec(keys: &Keys) -> String {
    keys.secret_key()
        .to_bech32()
        .unwrap_or_else(|_| keys.secret_key().to_secret_hex())
}

/// Versão curta a partir da chave em hexadecimal (o que a interface guarda).
pub fn short_hex(pk_hex: &str) -> String {
    match PublicKey::from_hex(pk_hex) {
        Ok(pk) => short_id(&pk),
        Err(_) => pk_hex.chars().take(8).collect(),
    }
}

/// Código público (`npub1…`) a partir da chave em hexadecimal.
pub fn npub_hex(pk_hex: &str) -> String {
    PublicKey::from_hex(pk_hex).map(|pk| npub(&pk)).unwrap_or_else(|_| pk_hex.to_string())
}

/// Versão curta do código público, para caber na tela.
pub fn short_id(pk: &PublicKey) -> String {
    let n = npub(pk);
    if n.len() > 16 {
        format!("{}…{}", &n[..10], &n[n.len() - 4..])
    } else {
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frase_nova_tem_doze_palavras_e_deriva() {
        let w = new_words().unwrap();
        assert_eq!(w.split(' ').count(), WORD_COUNT);
        assert!(valid_words(&w).is_some(), "frase sorteada deve ser válida");
        let k = keys_from_words(&w).unwrap();
        // A mesma frase sempre dá a mesma chave.
        assert_eq!(k.public_key(), keys_from_words(&w).unwrap().public_key());
    }

    #[test]
    fn normaliza_o_que_a_pessoa_digita() {
        let w = new_words().unwrap();
        let bagunçado = format!("  {}  ", w.to_uppercase().replace(' ', "\n  "));
        assert_eq!(valid_words(&bagunçado).as_deref(), Some(w.as_str()));
    }

    #[test]
    fn frase_errada_e_recusada() {
        assert!(valid_words("abacate abaixo abalar").is_none(), "poucas palavras");
        assert!(
            valid_words("abacate abacate abacate abacate abacate abacate abacate abacate abacate abacate abacate abacate")
                .is_none(),
            "soma de verificação inválida"
        );
    }

    #[test]
    fn aceita_nsec_e_hexadecimal() {
        let k = Keys::generate();
        assert_eq!(keys_from_any(&nsec(&k)).unwrap().public_key(), k.public_key());
        assert_eq!(
            keys_from_any(&k.secret_key().to_secret_hex()).unwrap().public_key(),
            k.public_key()
        );
    }

    #[test]
    fn le_o_codigo_publico_de_um_link_de_convite() {
        let k = Keys::generate();
        let n = npub(&k.public_key());
        assert_eq!(parse_pubkey(&n), Some(k.public_key()));
        let link = format!("jmenseger://add?k={}&n=Ana", n);
        assert_eq!(parse_pubkey(&link), Some(k.public_key()));
        assert_eq!(parse_pubkey(&k.public_key().to_hex()), Some(k.public_key()));
        assert_eq!(parse_pubkey("qualquer texto"), None);
    }
}
