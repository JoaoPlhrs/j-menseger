//! Sons de fábrica do soundboard.
//!
//! A lista, os nomes, os emojis e as durações são os do aplicativo de
//! celular. O que importa para a compatibilidade é o identificador: quando o
//! celular manda `{t:'snd', id:'aplausos'}`, o computador toca o próprio
//! arquivo `aplausos`, e vice-versa.

/// Um som que vem com o aplicativo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuiltinSound {
    pub id: &'static str,
    pub emoji: &'static str,
    pub name: &'static str,
    /// Duração em segundos.
    pub dur: f32,
}

pub const BUILTIN: [BuiltinSound; 14] = [
    BuiltinSound { id: "aplausos", emoji: "👏", name: "Aplausos", dur: 3.7 },
    BuiltinSound { id: "badumtss", emoji: "😆", name: "Ba dum tss", dur: 2.6 },
    BuiltinSound { id: "trombone", emoji: "😢", name: "Trombone triste", dur: 4.0 },
    BuiltinSound { id: "grilos", emoji: "🦗", name: "Grilos", dur: 3.4 },
    BuiltinSound { id: "suspense", emoji: "😱", name: "Dun dun duuun", dur: 4.6 },
    BuiltinSound { id: "rufar", emoji: "🥁", name: "Rufar de tambores", dur: 4.7 },
    BuiltinSound { id: "vitoria", emoji: "🏆", name: "Vitória", dur: 3.5 },
    BuiltinSound { id: "tcharam", emoji: "🎉", name: "Tcharam!", dur: 2.8 },
    BuiltinSound { id: "coro", emoji: "😇", name: "Aleluia", dur: 4.6 },
    BuiltinSound { id: "assobio", emoji: "😏", name: "Fiu-fiu", dur: 2.1 },
    BuiltinSound { id: "violino", emoji: "🎻", name: "Violino triste", dur: 4.4 },
    BuiltinSound { id: "impacto", emoji: "💥", name: "Impacto", dur: 2.6 },
    BuiltinSound { id: "sino", emoji: "🔔", name: "Acertou!", dur: 2.3 },
    BuiltinSound { id: "palmas", emoji: "🙃", name: "Palmas lentas", dur: 3.7 },
];

/// Nomes antigos (versão 0.3 do celular) que ainda podem chegar em conversas
/// antigas: tocam o som de hoje mais parecido.
pub fn legacy(id: &str) -> &str {
    match id {
        "tambor" => "badumtss",
        "ding" | "moeda" => "sino",
        "uau" => "coro",
        "risada" => "tcharam",
        "buzina" | "laser" => "impacto",
        outro => outro,
    }
}

/// Som de fábrica pelo identificador (aceita os nomes antigos).
pub fn builtin(id: &str) -> Option<&'static BuiltinSound> {
    let id = legacy(id);
    BUILTIN.iter().find(|s| s.id == id)
}

/// Emojis oferecidos ao nomear um som próprio (os mesmos do celular).
pub const SOUND_EMOJIS: [&str; 36] = [
    "🔊", "😂", "🤣", "😱", "😮", "🤯", "😭", "😎", "👏", "🎉", "🥁", "🎺", "🎸", "🎵", "📯", "🦆", "🐶",
    "🐱", "🐮", "🦗", "💥", "🔥", "💀", "👻", "🤖", "👽", "🚨", "⏰", "📞", "🍕", "💤", "❤️", "👍",
    "👎", "🙈", "🤡",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomes_antigos_ainda_tocam_algo() {
        assert_eq!(builtin("tambor").unwrap().id, "badumtss");
        assert_eq!(builtin("aplausos").unwrap().name, "Aplausos");
        assert!(builtin("inexistente").is_none());
    }

    #[test]
    fn ids_unicos() {
        let mut ids: Vec<_> = BUILTIN.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), BUILTIN.len());
    }
}
