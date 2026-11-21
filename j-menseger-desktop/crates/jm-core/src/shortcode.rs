//! Código de convite curto: 8 caracteres (ex.: `K7QF-9X2M`) no lugar do
//! `npub1…` de 63.
//!
//! Como funciona: quem gera publica nos servidores um registro assinado com a
//! própria chave (NIP-78, tipo 30078) cujo identificador é o código, com prazo
//! de 7 dias (NIP-40). Quem recebe o código procura esse registro e lê a chave
//! pública de quem assinou. A assinatura garante que o código foi publicado
//! por aquela chave; se duas chaves diferentes publicarem o mesmo código, a
//! busca recusa (ambíguo) em vez de escolher uma. O número de segurança
//! continua sendo a forma de conferir com quem se está falando.

/// Alfabeto de Crockford: sem I, L, O e U, que confundem na leitura.
const ALFABETO: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub const KIND: u16 = 30078;
pub const VALIDADE_DIAS: i64 = 7;

/// Identificador do registro nos servidores.
pub fn tag(code: &str) -> String {
    format!("jmenseger:convite:{code}")
}

/// Código novo, já no formato de mostrar (`XXXX-XXXX`).
pub fn generate() -> String {
    let mut b = [0u8; 8];
    rand::Rng::fill(&mut rand::rng(), &mut b[..]);
    let s: String = b.iter().map(|x| ALFABETO[(*x & 31) as usize] as char).collect();
    format!("{}-{}", &s[..4], &s[4..])
}

/// Lê o que a pessoa digitou. Aceita minúsculas, espaço, hífen e as
/// confusões comuns (O → 0, I/L → 1). Devolve no formato `XXXX-XXXX`.
pub fn parse(input: &str) -> Option<String> {
    let mut s = String::new();
    for c in input.trim().chars() {
        let c = c.to_ascii_uppercase();
        let c = match c {
            'O' => '0',
            'I' | 'L' => '1',
            '-' | ' ' => continue,
            c => c,
        };
        if !ALFABETO.contains(&(c as u8)) || !c.is_ascii() {
            return None;
        }
        s.push(c);
    }
    let s = s.strip_prefix("JM").filter(|r| r.len() == 8).map(str::to_string).unwrap_or(s);
    (s.len() == 8).then(|| format!("{}-{}", &s[..4], &s[4..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codigo_tem_formato_e_volta_igual() {
        for _ in 0..50 {
            let c = generate();
            assert_eq!(c.len(), 9);
            assert_eq!(parse(&c).as_deref(), Some(c.as_str()));
        }
    }

    #[test]
    fn leitura_tolera_digitacao() {
        assert_eq!(parse("k7qf 9x2m").as_deref(), Some("K7QF-9X2M"));
        assert_eq!(parse("KOQF-IX2L").as_deref(), Some("K0QF-1X21"));
        assert_eq!(parse("JM-K7QF-9X2M").as_deref(), Some("K7QF-9X2M"));
        assert!(parse("npub1abc").is_none());
        assert!(parse("K7QF-9X2").is_none());
        assert!(parse("K7QF-9X2U").is_none());
    }
}
