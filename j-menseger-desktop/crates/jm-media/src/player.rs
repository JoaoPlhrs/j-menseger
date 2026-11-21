//! Tocador de sons: soundboard, mensagens de voz e toques de chamada.
//!
//! Um único fluxo de saída, com um mixer: até 3 efeitos ao mesmo tempo (o
//! quarto corta o mais antigo, como no celular), uma faixa com posição e
//! pausa para mensagens de voz, e um toque em repetição (chamando, tocando).
//!
//! O fluxo de áudio vive numa thread própria. A interface só mexe no mixer,
//! que é compartilhado; assim nada aqui depende de o fluxo do sistema poder
//! atravessar threads.

use cpal::traits::{DeviceTrait, StreamTrait};
use std::sync::{Arc, Mutex};

const SR: f64 = 48_000.0;
const MAX_EFEITOS: usize = 3;

#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("nenhuma saída de áudio neste computador")]
    NoDevice,
    #[error("não foi possível abrir a saída de áudio: {0}")]
    Stream(String),
}

struct Voz {
    pcm: Arc<Vec<f32>>,
    pos: f64,
    ganho: f32,
}

impl Voz {
    fn amostra(&self) -> f32 {
        let k = self.pos as usize;
        let f = (self.pos - k as f64) as f32;
        let a = self.pcm.get(k).copied().unwrap_or(0.0);
        let b = self.pcm.get(k + 1).copied().unwrap_or(0.0);
        (a + (b - a) * f) * self.ganho
    }
    fn acabou(&self) -> bool {
        // A posição anda em passos fracionários (taxa da placa ≠ 48 kHz) e
        // acumula erro de arredondamento: uma fração mínima de amostra conta
        // como fim.
        self.pos + 1e-6 >= self.pcm.len() as f64
    }
}

struct Faixa {
    id: String,
    voz: Voz,
    pausada: bool,
}

/// O mixer: o que está tocando agora.
#[derive(Default)]
pub struct Mixer {
    efeitos: Vec<Voz>,
    faixa: Option<Faixa>,
    toque: Option<Voz>,
    /// Volume do soundboard (0 a 1). O celular aplica 0,9 × volume.
    volume: f32,
}

impl Mixer {
    fn new() -> Self {
        Self { volume: 0.7, ..Default::default() }
    }

    /// Preenche `out` (mono) na taxa `taxa` da placa.
    pub fn render(&mut self, out: &mut [f32], taxa: u32) {
        let passo = SR / taxa.max(1) as f64;
        let mestre = self.volume * 0.9;
        for s in out.iter_mut() {
            let mut v = 0f32;
            for e in &mut self.efeitos {
                v += e.amostra() * mestre;
                e.pos += passo;
            }
            if let Some(t) = &mut self.toque {
                v += t.amostra();
                t.pos += passo;
                if t.acabou() {
                    t.pos = 0.0; // toque repete
                }
            }
            if let Some(f) = &mut self.faixa {
                if !f.pausada {
                    v += f.voz.amostra();
                    f.voz.pos += passo;
                }
            }
            // Saturação suave: vários sons juntos não estouram.
            *s = v.tanh();
        }
        self.efeitos.retain(|e| !e.acabou());
        if self.faixa.as_ref().is_some_and(|f| f.voz.acabou()) {
            self.faixa = None;
        }
    }
}

/// Tocador. Barato de clonar: todos os clones mexem no mesmo mixer.
#[derive(Clone)]
pub struct Player {
    mixer: Arc<Mutex<Mixer>>,
    _vivo: Arc<std::sync::mpsc::Sender<()>>,
}

impl Player {
    /// Abre a saída de áudio padrão.
    pub fn new() -> Result<Self, PlayerError> {
        let mixer = Arc::new(Mutex::new(Mixer::new()));
        let (pronto_tx, pronto_rx) = std::sync::mpsc::channel::<Result<(), PlayerError>>();
        let (parar_tx, parar_rx) = std::sync::mpsc::channel::<()>();
        let m = Arc::clone(&mixer);
        std::thread::Builder::new()
            .name("saida-de-som".into())
            .spawn(move || {
                let abrir = || -> Result<cpal::Stream, PlayerError> {
                    // A saída escolhida nos ajustes (ou a padrão).
                    let dev = crate::audio::find_output(None).map_err(|_| PlayerError::NoDevice)?;
                    let cfg = dev.default_output_config().map_err(|e| PlayerError::Stream(e.to_string()))?;
                    let taxa = cfg.sample_rate();
                    let canais = cfg.channels() as usize;
                    let mut mono = Vec::new();
                    let stream = crate::audio::build_output(&dev, &cfg, move |saida: &mut [f32]| {
                                let n = saida.len() / canais.max(1);
                                mono.resize(n, 0.0);
                                if let Ok(mut mx) = m.lock() {
                                    mx.render(&mut mono, taxa);
                                } else {
                                    mono.fill(0.0);
                                }
                                for (i, q) in saida.chunks_mut(canais.max(1)).enumerate() {
                                    q.fill(mono[i]);
                                }
                            })
                        .map_err(|e| PlayerError::Stream(e.to_string()))?;
                    stream.play().map_err(|e| PlayerError::Stream(e.to_string()))?;
                    Ok(stream)
                };
                match abrir() {
                    Ok(stream) => {
                        let _ = pronto_tx.send(Ok(()));
                        // Segura o fluxo vivo até o tocador ser descartado.
                        let _ = parar_rx.recv();
                        drop(stream);
                    }
                    Err(e) => {
                        let _ = pronto_tx.send(Err(e));
                    }
                }
            })
            .map_err(|e| PlayerError::Stream(e.to_string()))?;
        pronto_rx.recv().map_err(|_| PlayerError::NoDevice)??;
        Ok(Self { mixer, _vivo: Arc::new(parar_tx) })
    }

    /// Toca um efeito (som do soundboard, aviso).
    pub fn play(&self, pcm: Arc<Vec<f32>>) {
        let mut mx = self.mixer.lock().unwrap();
        if mx.efeitos.len() >= MAX_EFEITOS {
            mx.efeitos.remove(0);
        }
        mx.efeitos.push(Voz { pcm, pos: 0.0, ganho: 1.0 });
    }

    pub fn stop_effects(&self) {
        self.mixer.lock().unwrap().efeitos.clear();
    }

    pub fn set_volume(&self, v: f32) {
        self.mixer.lock().unwrap().volume = v.clamp(0.0, 1.0);
    }

    pub fn volume(&self) -> f32 {
        self.mixer.lock().unwrap().volume
    }

    /// Começa a tocar uma mensagem de voz (troca a anterior).
    pub fn play_track(&self, id: &str, pcm: Arc<Vec<f32>>) {
        self.mixer.lock().unwrap().faixa = Some(Faixa { id: id.into(), voz: Voz { pcm, pos: 0.0, ganho: 1.0 }, pausada: false });
    }

    pub fn toggle_pause(&self) {
        if let Some(f) = &mut self.mixer.lock().unwrap().faixa {
            f.pausada = !f.pausada;
        }
    }

    /// Pula para uma fração (0 a 1) da mensagem de voz.
    pub fn seek(&self, frac: f32) {
        if let Some(f) = &mut self.mixer.lock().unwrap().faixa {
            f.voz.pos = f.voz.pcm.len() as f64 * frac.clamp(0.0, 1.0) as f64;
        }
    }

    pub fn stop_track(&self) {
        self.mixer.lock().unwrap().faixa = None;
    }

    /// (id, posição em s, duração em s, pausada) da mensagem de voz tocando.
    pub fn track(&self) -> Option<(String, f32, f32, bool)> {
        let mx = self.mixer.lock().unwrap();
        mx.faixa.as_ref().map(|f| {
            (f.id.clone(), (f.voz.pos / SR) as f32, (f.voz.pcm.len() as f64 / SR) as f32, f.pausada)
        })
    }

    /// Toque em repetição: chamada chegando ou "chamando".
    pub fn start_loop(&self, kind: Tone) {
        self.mixer.lock().unwrap().toque = Some(Voz { pcm: Arc::new(tone(kind)), pos: 0.0, ganho: 1.0 });
    }

    pub fn stop_loop(&self) {
        self.mixer.lock().unwrap().toque = None;
    }

    /// Toque curto: aviso de mensagem ou fim de chamada.
    pub fn beep(&self, kind: Tone) {
        let mut mx = self.mixer.lock().unwrap();
        if mx.efeitos.len() >= MAX_EFEITOS {
            mx.efeitos.remove(0);
        }
        // Os toques não seguem o volume do soundboard.
        let ganho = 1.0 / (mx.volume * 0.9).max(0.05);
        mx.efeitos.push(Voz { pcm: Arc::new(tone(kind)), pos: 0.0, ganho });
    }
}

/// Os toques sintetizados do celular.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Chamada chegando (repete a cada 2,2 s).
    Ring,
    /// Você está chamando (repete a cada 3,5 s).
    Ringback,
    /// Mensagem nova.
    Blip,
    /// Chamada encerrada.
    Hangup,
}

/// Uma nota senoidal com ataque de 12 ms e queda exponencial, como o
/// `tone()` de `sound.js`.
fn nota(buf: &mut [f32], freq: f32, t0: f32, dur: f32, ganho: f32) {
    let i0 = (t0 * SR as f32) as usize;
    let n = ((dur + 0.05) * SR as f32) as usize;
    for k in 0..n {
        let Some(s) = buf.get_mut(i0 + k) else { break };
        let t = k as f32 / SR as f32;
        let env = if t < 0.012 {
            0.0001 * (ganho / 0.0001).powf(t / 0.012)
        } else if t < dur {
            ganho * (0.0001 / ganho).powf((t - 0.012) / (dur - 0.012).max(0.001))
        } else {
            0.0
        };
        *s += (t * freq * std::f32::consts::TAU).sin() * env;
    }
}

/// O áudio de um toque (para os que repetem: um ciclo inteiro, com o
/// silêncio até a próxima vez).
pub fn tone(kind: Tone) -> Vec<f32> {
    let secs = match kind {
        Tone::Ring => 2.2,
        Tone::Ringback => 3.5,
        Tone::Blip => 0.4,
        Tone::Hangup => 0.55,
    };
    let mut b = vec![0f32; (secs * SR) as usize];
    let t = 0.02;
    match kind {
        Tone::Ring => {
            for i in 0..2 {
                let o = t + i as f32 * 0.42;
                nota(&mut b, 523.0, o, 0.35, 0.35);
                nota(&mut b, 659.0, o + 0.12, 0.3, 0.25);
                nota(&mut b, 784.0, o + 0.22, 0.26, 0.2);
            }
        }
        Tone::Ringback => nota(&mut b, 425.0, t, 1.0, 0.08),
        Tone::Blip => {
            nota(&mut b, 880.0, t, 0.12, 0.16);
            nota(&mut b, 1320.0, t + 0.09, 0.2, 0.12);
        }
        Tone::Hangup => {
            nota(&mut b, 480.0, t, 0.18, 0.15);
            nota(&mut b, 380.0, t + 0.2, 0.26, 0.15);
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_corta_o_efeito_mais_antigo_no_quarto() {
        let mut mx = Mixer::new();
        for i in 0..4 {
            if mx.efeitos.len() >= MAX_EFEITOS {
                mx.efeitos.remove(0);
            }
            mx.efeitos.push(Voz { pcm: Arc::new(vec![i as f32 * 0.1; 100]), pos: 0.0, ganho: 1.0 });
        }
        assert_eq!(mx.efeitos.len(), 3);
        assert!((mx.efeitos[0].pcm[0] - 0.1).abs() < 1e-6, "o primeiro (0) deveria ter saído");
    }

    #[test]
    fn efeito_acaba_e_sai_do_mixer() {
        let mut mx = Mixer::new();
        mx.efeitos.push(Voz { pcm: Arc::new(vec![0.5; 480]), pos: 0.0, ganho: 1.0 });
        let mut out = vec![0f32; 1000];
        mx.render(&mut out, 48_000);
        assert!(out[0] > 0.0);
        assert_eq!(out[999], 0.0);
        assert!(mx.efeitos.is_empty());
    }

    #[test]
    fn taxa_da_placa_diferente_mantem_a_duracao() {
        let mut mx = Mixer::new();
        mx.faixa = Some(Faixa { id: "a".into(), voz: Voz { pcm: Arc::new(vec![0.3; 48_000]), pos: 0.0, ganho: 1.0 }, pausada: false });
        let mut out = vec![0f32; 44_100]; // 1 s numa placa de 44,1 kHz
        mx.render(&mut out, 44_100);
        assert!(mx.faixa.is_none(), "1 s de áudio deveria ter acabado em 1 s de placa");
    }

    #[test]
    fn faixa_pausada_nao_anda() {
        let mut mx = Mixer::new();
        mx.faixa = Some(Faixa { id: "a".into(), voz: Voz { pcm: Arc::new(vec![0.3; 48_000]), pos: 0.0, ganho: 1.0 }, pausada: true });
        let mut out = vec![0f32; 1000];
        mx.render(&mut out, 48_000);
        assert_eq!(mx.faixa.as_ref().unwrap().voz.pos, 0.0);
        assert!(out.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn toques_tem_som_e_duracao_certa() {
        let ring = tone(Tone::Ring);
        assert_eq!(ring.len(), (2.2 * SR) as usize);
        let pico = ring.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(pico > 0.2 && pico < 1.0, "pico {pico}");
        assert!(tone(Tone::Blip).iter().any(|v| v.abs() > 0.05));
        // O "chamando" tem som só no primeiro segundo.
        let rb = tone(Tone::Ringback);
        assert!(rb[(2.0 * SR) as usize..].iter().all(|v| v.abs() < 1e-3));
    }
}
