//! Biblioteca de sons: os de fábrica, os criados pela pessoa e os que
//! chegaram de outra pessoa numa chamada. Usada pelo soundboard, pelas
//! conversas e pelas chamadas.

use jm_core::engine::Engine;
use jm_media::player::Player;
use std::sync::{Arc, Mutex};

/// Um som da lista.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundInfo {
    pub id: String,
    pub emoji: String,
    pub name: String,
    pub dur: f32,
    pub builtin: bool,
}

pub struct SoundBank {
    engine: Arc<Engine>,
    player: std::sync::RwLock<Option<Player>>,
    cache: Mutex<Vec<(String, Arc<Vec<f32>>)>>,
    /// Sons próprios de outras pessoas, recebidos numa chamada (até 30).
    remote: Mutex<Vec<(String, Vec<u8>)>>,
    /// Sons próprios (só os dados, sem o áudio) e os de fábrica escondidos.
    /// A tela pede a lista a cada desenho; ler o banco toda vez pesaria.
    lista: Mutex<Option<(Vec<SoundInfo>, Vec<String>)>>,
}

const CACHE: usize = 60;

impl SoundBank {
    pub fn new(engine: Arc<Engine>, player: Option<Player>) -> Arc<Self> {
        if let Some(p) = &player {
            p.set_volume(engine.sound_volume());
        }
        Arc::new(Self { engine, player: std::sync::RwLock::new(player), cache: Mutex::new(Vec::new()), remote: Mutex::new(Vec::new()), lista: Mutex::new(None) })
    }

    pub fn player(&self) -> Option<Player> {
        self.player.read().unwrap().clone()
    }

    /// Reabre a saída de som (trocou o dispositivo nos ajustes).
    pub fn reopen_player(&self) -> Result<(), String> {
        let antigo = self.player.write().unwrap().take();
        drop(antigo);
        let novo = Player::new().map_err(|e| e.to_string())?;
        novo.set_volume(self.engine.sound_volume());
        *self.player.write().unwrap() = Some(novo);
        Ok(())
    }

    /// Sons próprios e escondidos, lidos do banco só quando mudaram.
    fn stored(&self) -> (Vec<SoundInfo>, Vec<String>) {
        let mut g = self.lista.lock().unwrap();
        if g.is_none() {
            let proprios = self
                .engine
                .custom_sounds()
                .into_iter()
                .map(|s| SoundInfo { id: s.id, emoji: s.emoji, name: s.name, dur: s.dur as f32, builtin: false })
                .collect();
            *g = Some((proprios, self.engine.hidden_builtins()));
        }
        g.clone().unwrap_or_default()
    }

    /// Avisa que a lista mudou (som criado, editado, excluído ou restaurado).
    pub fn changed(&self) {
        *self.lista.lock().unwrap() = None;
    }

    /// A lista na ordem do celular: fábrica (menos os escondidos), depois os seus.
    pub fn library(&self) -> Vec<SoundInfo> {
        let (proprios, escondidos) = self.stored();
        let mut v: Vec<SoundInfo> = jm_core::sounds::BUILTIN
            .iter()
            .filter(|s| !escondidos.iter().any(|h| h == s.id))
            .map(builtin_info)
            .collect();
        v.extend(proprios);
        v
    }

    /// Sons de fábrica que a pessoa tirou da lista.
    pub fn hidden(&self) -> Vec<SoundInfo> {
        let (_, escondidos) = self.stored();
        jm_core::sounds::BUILTIN.iter().filter(|s| escondidos.iter().any(|h| h == s.id)).map(builtin_info).collect()
    }

    pub fn info(&self, id: &str) -> Option<SoundInfo> {
        if let Some(b) = jm_core::sounds::builtin(id) {
            return Some(builtin_info(b));
        }
        self.stored().0.into_iter().find(|s| s.id == id)
    }

    /// Bytes de um som: de fábrica, seu ou recebido numa chamada.
    pub fn bytes(&self, id: &str) -> Option<Vec<u8>> {
        let id = jm_core::sounds::legacy(id);
        if let Some(b) = jm_media::builtin::bytes(id) {
            return Some(b.to_vec());
        }
        if let Some(s) = self.engine.custom_sounds().into_iter().find(|s| s.id == id) {
            return Some(s.bytes);
        }
        self.remote_bytes(id)
    }

    /// Áudio pronto para tocar (decodificado uma vez e guardado).
    pub fn pcm(&self, key: &str, bytes: impl FnOnce() -> Option<Vec<u8>>) -> Option<Arc<Vec<f32>>> {
        if let Some((_, p)) = self.cache.lock().unwrap().iter().find(|(k, _)| k == key) {
            return Some(Arc::clone(p));
        }
        let pcm = Arc::new(jm_media::audiofile::decode(&bytes()?).ok()?.samples);
        let mut c = self.cache.lock().unwrap();
        c.push((key.to_string(), Arc::clone(&pcm)));
        if c.len() > CACHE {
            c.remove(0);
        }
        Some(pcm)
    }

    /// Toca um som pelo id. Devolve a duração (0 se não tocou).
    pub fn play(&self, id: &str) -> f32 {
        let id = jm_core::sounds::legacy(id).to_string();
        let Some(player) = self.player() else { return 0.0 };
        match self.pcm(&id, || self.bytes(&id)) {
            Some(p) => {
                let d = p.len() as f32 / 48_000.0;
                player.play(p);
                d
            }
            None => 0.0,
        }
    }

    /// Toca bytes quaisquer (som próprio recebido numa conversa).
    pub fn play_bytes(&self, key: &str, bytes: &[u8]) -> f32 {
        let Some(player) = self.player() else { return 0.0 };
        match self.pcm(key, || Some(bytes.to_vec())) {
            Some(p) => {
                let d = p.len() as f32 / 48_000.0;
                player.play(p);
                d
            }
            None => 0.0,
        }
    }

    pub fn add_remote(&self, id: &str, bytes: Vec<u8>) {
        let mut r = self.remote.lock().unwrap();
        r.retain(|(k, _)| k != id);
        r.push((id.to_string(), bytes));
        if r.len() > 30 {
            r.remove(0);
        }
    }

    pub fn remote_bytes(&self, id: &str) -> Option<Vec<u8>> {
        self.remote.lock().unwrap().iter().find(|(k, _)| k == id).map(|(_, b)| b.clone())
    }

    pub fn set_volume(&self, v: f32) {
        if let Some(p) = self.player() {
            p.set_volume(v);
        }
        let _ = self.engine.set_sound_volume(v);
    }

    pub fn volume(&self) -> f32 {
        self.engine.sound_volume()
    }

    pub fn forget(&self, id: &str) {
        self.cache.lock().unwrap().retain(|(k, _)| k != id);
    }
}

fn builtin_info(b: &jm_core::sounds::BuiltinSound) -> SoundInfo {
    SoundInfo { id: b.id.into(), emoji: b.emoji.into(), name: b.name.into(), dur: b.dur, builtin: true }
}

/// Emoji aceitável vindo de outra pessoa (como o EMOJI_OK do celular): poucos
/// caracteres e nada de letras ou símbolos de texto.
pub fn emoji_ok(e: &str) -> bool {
    let n = e.chars().count();
    (1..=8).contains(&n) && e.chars().all(|c| !c.is_alphanumeric() && !c.is_ascii() || c == '\u{200d}' || c == '\u{fe0f}')
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_de_outra_pessoa_e_conferido() {
        assert!(emoji_ok("🎉"));
        assert!(emoji_ok("👍🏽"));
        assert!(emoji_ok("❤️"));
        assert!(!emoji_ok("abc"));
        assert!(!emoji_ok("<script>"));
        assert!(!emoji_ok(""));
        assert!(!emoji_ok("🎉🎉🎉🎉🎉🎉🎉🎉🎉"));
    }
}
