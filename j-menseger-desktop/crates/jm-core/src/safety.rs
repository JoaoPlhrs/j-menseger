//! Número de segurança: o mesmo nos dois aparelhos de uma conversa.
//!
//! Duas pessoas comparam estes 60 dígitos (pessoalmente ou por outro meio).
//! Se baterem, ninguém está no meio fingindo ser a outra pessoa.
//!
//! O cálculo é o do aplicativo de celular, bit a bit: as duas chaves públicas
//! em hexadecimal, em ordem alfabética, viram bytes; dois SHA-256 encadeados
//! dão 64 bytes; de cada 5 bytes, os 4 primeiros formam um número de 32 bits,
//! e o resto da divisão por 100 000 é um grupo de 5 dígitos. São 12 grupos.
//! Mudar qualquer detalhe faria o PC mostrar um número diferente do celular,
//! e a verificação deixaria de servir para alguma coisa.

use sha2::{Digest, Sha256};

/// Doze grupos de cinco dígitos, ou `None` se alguma chave for inválida.
pub fn safety_number(pk_a: &str, pk_b: &str) -> Option<[String; 12]> {
    let a = pk_a.to_ascii_lowercase();
    let b = pk_b.to_ascii_lowercase();
    if !is_hex64(&a) || !is_hex64(&b) {
        return None;
    }
    let (x, y) = if a <= b { (a, b) } else { (b, a) };
    let bytes = hex::decode(format!("{x}{y}")).ok()?;

    let h: [u8; 32] = Sha256::digest(&bytes).into();
    let h2: [u8; 32] = Sha256::digest(h).into();
    let mut all = [0u8; 64];
    all[..32].copy_from_slice(&h);
    all[32..].copy_from_slice(&h2);

    Some(std::array::from_fn(|i| {
        let v = (u64::from(all[i * 5]) << 24)
            + (u64::from(all[i * 5 + 1]) << 16)
            + (u64::from(all[i * 5 + 2]) << 8)
            + u64::from(all[i * 5 + 3]);
        format!("{:05}", v % 100_000)
    }))
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e_igual_para_os_dois_lados() {
        let a = "a".repeat(64);
        let b = "0123456789abcdef".repeat(4);
        assert_eq!(safety_number(&a, &b), safety_number(&b, &a));
    }

    #[test]
    fn tem_doze_grupos_de_cinco_digitos() {
        let n = safety_number(&"1".repeat(64), &"2".repeat(64)).unwrap();
        assert_eq!(n.len(), 12);
        for g in &n {
            assert_eq!(g.len(), 5);
            assert!(g.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn recusa_chave_invalida() {
        assert!(safety_number("curta", &"a".repeat(64)).is_none());
        assert!(safety_number(&"z".repeat(64), &"a".repeat(64)).is_none());
    }
}
