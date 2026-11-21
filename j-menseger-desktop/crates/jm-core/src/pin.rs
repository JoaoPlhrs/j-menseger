//! PIN de bloqueio do aplicativo.
//!
//! O PIN não é guardado: guarda-se o resultado de PBKDF2-HMAC-SHA256 com sal
//! aleatório e 150 000 rodadas, os mesmos parâmetros do celular. As rodadas
//! existem para que testar os 10 000 PINs possíveis custe tempo de verdade a
//! quem copiar o banco, em vez de um piscar de olhos.

use serde::{Deserialize, Serialize};
use sha2::Sha256;

const ROUNDS: u32 = 150_000;

/// O que fica gravado quando há PIN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinLock {
    /// Sal em hexadecimal (16 bytes).
    pub salt: String,
    /// PBKDF2 do PIN, em hexadecimal (32 bytes).
    pub hash: String,
    /// Quantos dígitos o PIN tem.
    pub len: u8,
    /// Depois de quantos segundos fora do aplicativo o PIN é pedido de novo.
    pub delay: u32,
}

impl PinLock {
    /// Cria o registro de um PIN novo, mantendo o tempo de espera anterior.
    pub fn new(pin: &str, delay: u32) -> Self {
        let mut salt = [0u8; 16];
        rand::Rng::fill(&mut rand::rng(), &mut salt);
        let salt_hex = hex::encode(salt);
        Self {
            hash: derive(pin, &salt),
            salt: salt_hex,
            len: pin.chars().count().min(255) as u8,
            delay,
        }
    }

    /// Confere um PIN digitado. A comparação não para no primeiro byte
    /// diferente, para o tempo de resposta não dar pistas.
    pub fn check(&self, pin: &str) -> bool {
        let Ok(salt) = hex::decode(&self.salt) else { return false };
        let a = derive(pin, &salt);
        let b = self.hash.to_ascii_lowercase();
        if a.len() != b.len() {
            return false;
        }
        a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
    }
}

fn derive(pin: &str, salt: &[u8]) -> String {
    let mut out = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(pin.as_bytes(), salt, ROUNDS, &mut out);
    hex::encode(out)
}

/// Um PIN válido tem exatamente 4 dígitos, como no celular.
pub fn valid_pin(pin: &str) -> bool {
    pin.len() == 4 && pin.bytes().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confere_o_pin_certo_e_recusa_o_errado() {
        let l = PinLock::new("2580", 60);
        assert!(l.check("2580"));
        assert!(!l.check("2581"));
        assert!(!l.check(""));
        assert_eq!(l.len, 4);
        assert_eq!(l.delay, 60);
    }

    #[test]
    fn mesmo_pin_com_sal_diferente_vira_outro_registro() {
        let a = PinLock::new("1234", 0);
        let b = PinLock::new("1234", 0);
        assert_ne!(a.salt, b.salt);
        assert_ne!(a.hash, b.hash, "sem sal, dois registros iguais revelariam o mesmo PIN");
    }

    #[test]
    fn derivacao_bate_com_o_padrao_pbkdf2() {
        // Vetor do RFC 7914 (PBKDF2-HMAC-SHA256, "passwd"/"salt", 1 rodada),
        // para provar que a função usada é a mesma do navegador do celular.
        let mut out = [0u8; 64];
        pbkdf2::pbkdf2_hmac::<Sha256>(b"passwd", b"salt", 1, &mut out);
        assert_eq!(
            hex::encode(&out[..16]),
            "55ac046e56e3089fec1691c22544b605"
        );
    }

    #[test]
    fn pin_precisa_de_quatro_digitos() {
        assert!(valid_pin("0000"));
        assert!(!valid_pin("123"));
        assert!(!valid_pin("12a4"));
        assert!(!valid_pin("12345"));
    }
}
