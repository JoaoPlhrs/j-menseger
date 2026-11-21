//! Gravador de voz: mensagens de voz e sons novos do soundboard.

use crate::audio::{i16_to_f32, Capture};
use std::sync::{Arc, Mutex};

/// Gravação em andamento.
pub struct Recorder {
    cap: Capture,
    dados: Arc<Mutex<Dados>>,
    inicio: std::time::Instant,
}

#[derive(Default)]
struct Dados {
    pcm: Vec<f32>,
    niveis: Vec<f32>,
    janela: Vec<f32>,
    pico_atual: f32,
}

/// O que sai de uma gravação.
#[derive(Debug, Clone)]
pub struct Recording {
    /// Mono, 48 kHz.
    pub pcm: Vec<f32>,
    pub dur: f64,
    /// Nível a cada 90 ms (0 a 1), para a forma de onda.
    pub levels: Vec<f32>,
}

/// Janela de medição: 90 ms, como no celular.
const JANELA: usize = 48_000 * 90 / 1000;

impl Recorder {
    /// Abre o microfone padrão e começa a gravar.
    pub fn start() -> Result<Self, crate::audio::AudioError> {
        let (cap, mut quadros) = Capture::start(None)?;
        let dados = Arc::new(Mutex::new(Dados::default()));
        let d = Arc::clone(&dados);
        std::thread::Builder::new()
            .name("gravador".into())
            .spawn(move || {
                while let Some(q) = quadros.blocking_recv() {
                    let f = i16_to_f32(&q);
                    let mut g = d.lock().unwrap();
                    g.pcm.extend_from_slice(&f);
                    for v in f {
                        g.janela.push(v);
                        if g.janela.len() >= JANELA {
                            let pico = g.janela.iter().fold(0f32, |m, x| m.max(x.abs()));
                            // O celular mede o pico em 8 bits e divide por 64:
                            // meia escala já é nível máximo.
                            let nivel = (pico * 2.0).min(1.0);
                            g.niveis.push(nivel);
                            g.pico_atual = nivel;
                            g.janela.clear();
                        }
                    }
                }
            })
            .map_err(|e| crate::audio::AudioError::Stream(e.to_string()))?;
        Ok(Self { cap, dados, inicio: std::time::Instant::now() })
    }

    pub fn elapsed(&self) -> f64 {
        self.inicio.elapsed().as_secs_f64()
    }

    /// Nível agora (0 a 1), para o medidor durante a gravação.
    pub fn level(&self) -> f32 {
        self.dados.lock().unwrap().pico_atual.max(self.cap.level())
    }

    /// Termina e devolve o áudio.
    pub fn stop(self) -> Recording {
        let dur = self.elapsed();
        drop(self.cap); // fecha o microfone; a thread termina sozinha
        let g = self.dados.lock().unwrap();
        Recording { pcm: g.pcm.clone(), dur: g.pcm.len() as f64 / 48_000.0, levels: g.niveis.clone() }
            .with_min_dur(dur)
    }
}

impl Recording {
    fn with_min_dur(mut self, medido: f64) -> Self {
        // A duração vale pelo áudio que chegou; o relógio só entra se nada
        // chegou (microfone mudo de fábrica, por exemplo).
        if self.pcm.is_empty() {
            self.dur = medido;
        }
        self
    }
}
