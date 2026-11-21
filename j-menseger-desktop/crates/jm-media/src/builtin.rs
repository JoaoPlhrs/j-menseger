//! Os 14 sons de fábrica, embutidos no executável.
//!
//! Gerados por `tools/sounds/render_sounds.py` (o mesmo script do celular) a
//! partir de gravações reais de instrumentos e plateia; licença em
//! `assets/sounds/LICENCA.txt`. O identificador é o mesmo do celular: quando
//! um lado manda `aplausos`, o outro toca o próprio `aplausos`.

/// Bytes Ogg Opus de um som de fábrica.
pub fn bytes(id: &str) -> Option<&'static [u8]> {
    match id {
        "aplausos" => Some(include_bytes!("../assets/sounds/aplausos.ogg")),
        "badumtss" => Some(include_bytes!("../assets/sounds/badumtss.ogg")),
        "trombone" => Some(include_bytes!("../assets/sounds/trombone.ogg")),
        "grilos" => Some(include_bytes!("../assets/sounds/grilos.ogg")),
        "suspense" => Some(include_bytes!("../assets/sounds/suspense.ogg")),
        "rufar" => Some(include_bytes!("../assets/sounds/rufar.ogg")),
        "vitoria" => Some(include_bytes!("../assets/sounds/vitoria.ogg")),
        "tcharam" => Some(include_bytes!("../assets/sounds/tcharam.ogg")),
        "coro" => Some(include_bytes!("../assets/sounds/coro.ogg")),
        "assobio" => Some(include_bytes!("../assets/sounds/assobio.ogg")),
        "violino" => Some(include_bytes!("../assets/sounds/violino.ogg")),
        "impacto" => Some(include_bytes!("../assets/sounds/impacto.ogg")),
        "sino" => Some(include_bytes!("../assets/sounds/sino.ogg")),
        "palmas" => Some(include_bytes!("../assets/sounds/palmas.ogg")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn todos_os_sons_abrem_e_tem_a_duracao_do_celular() {
        // Durações da lista do celular (sound.js), com a tolerância do
        // arredondamento a 0,1 s que ele usa.
        let lista = [
            ("aplausos", 3.7), ("badumtss", 2.6), ("trombone", 4.0), ("grilos", 3.4),
            ("suspense", 4.6), ("rufar", 4.7), ("vitoria", 3.5), ("tcharam", 2.8),
            ("coro", 4.6), ("assobio", 2.1), ("violino", 4.4), ("impacto", 2.6),
            ("sino", 2.3), ("palmas", 3.7),
        ];
        for (id, dur) in lista {
            let b = super::bytes(id).unwrap_or_else(|| panic!("falta {id}"));
            let pcm = crate::audiofile::decode(b).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!((pcm.duration() - dur).abs() < 0.08, "{id}: {:.2} s, esperado {dur}", pcm.duration());
        }
        assert!(super::bytes("inexistente").is_none());
    }
}
