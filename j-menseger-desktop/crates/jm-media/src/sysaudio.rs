//! Áudio do computador (jogo, vídeo, música) junto com a transmissão de tela.
//!
//! No Windows 10 (2004) ou mais novo, captura tudo o que o PC toca **menos o
//! próprio J-Menseger** ("process loopback"): os outros ouvem o jogo, mas não
//! ouvem a própria voz voltando. O som é misturado ao microfone antes de ir
//! para o codec, então vale tanto em chamada quanto em sala de voz — e
//! continua indo mesmo com o microfone no mudo.
//!
//! Em Linux e macOS não há um jeito simples de excluir o próprio app da
//! captura; lá a opção fica indisponível.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::codec::SAMPLE_RATE;

/// Volume do som do PC em relação à voz.
const GANHO: f32 = 0.7;

struct Estado {
    fila: Mutex<VecDeque<i16>>,
    parar: Mutex<Option<Arc<AtomicBool>>>,
}

fn estado() -> &'static Estado {
    static E: OnceLock<Estado> = OnceLock::new();
    E.get_or_init(|| Estado { fila: Mutex::new(VecDeque::new()), parar: Mutex::new(None) })
}

/// Este sistema consegue mandar o áudio do PC?
pub fn supported() -> bool {
    imp::supported()
}

/// Está mandando agora?
pub fn is_on() -> bool {
    estado().parar.lock().unwrap().is_some()
}

/// Começa a capturar o áudio do PC.
pub fn start() -> Result<(), String> {
    if is_on() {
        return Ok(());
    }
    let parar = Arc::new(AtomicBool::new(false));
    estado().fila.lock().unwrap().clear();
    imp::start(Arc::clone(&parar))?;
    *estado().parar.lock().unwrap() = Some(parar);
    Ok(())
}

/// Para de capturar.
pub fn stop() {
    if let Some(p) = estado().parar.lock().unwrap().take() {
        p.store(true, Ordering::Relaxed);
    }
    estado().fila.lock().unwrap().clear();
}

/// Entrega áudio capturado (mono, 48 kHz).
#[cfg_attr(not(windows), allow(dead_code))]
fn push(amostras: &[i16]) {
    let mut f = estado().fila.lock().unwrap();
    f.extend(amostras.iter().copied());
    // Nunca mais de 300 ms guardados: atraso pior que perder um pedaço.
    let max = SAMPLE_RATE as usize * 3 / 10;
    if f.len() > max {
        let sobra = f.len() - max;
        f.drain(..sobra);
    }
}

/// Mistura o som do PC no quadro do microfone (chamado pela captura do microfone).
pub fn mix_into(pcm: &mut [i16]) {
    if !is_on() {
        return;
    }
    let mut f = estado().fila.lock().unwrap();
    for s in pcm.iter_mut() {
        let Some(x) = f.pop_front() else { break };
        *s = (*s as f32 + x as f32 * GANHO).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
    }
}

#[cfg(windows)]
mod imp {
    use super::push;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use wasapi::{initialize_mta, AudioClient, Direction, SampleType, StreamMode, WaveFormat};

    pub fn supported() -> bool {
        // Process loopback chegou no Windows 10 2004 (build 19041).
        let v = windows_version::OsVersion::current();
        v.major > 10 || (v.major == 10 && v.build >= 19041)
    }

    pub fn start(parar: Arc<AtomicBool>) -> Result<(), String> {
        if !supported() {
            return Err("Mandar o áudio do PC precisa do Windows 10 (versão 2004) ou mais novo.".into());
        }
        let (pronto_tx, pronto_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        std::thread::Builder::new()
            .name("audio-do-pc".into())
            .spawn(move || {
                let r = (|| -> Result<(), String> {
                    initialize_mta().ok().map_err(|e| e.to_string())?;
                    let formato = WaveFormat::new(32, 32, SampleType::Float, 48000, 2, None);
                    // Tudo menos este processo: a voz dos outros (que o app toca) não volta para eles.
                    let mut cliente = AudioClient::new_application_loopback_client(std::process::id(), false).map_err(|e| e.to_string())?;
                    cliente
                        .initialize_client(&formato, Direction::Capture, StreamMode::EventsShared { autoconvert: true, buffer_duration_hns: 0 }, None)
                        .map_err(|e| e.to_string())?;
                    let evento = cliente.set_get_eventhandle().map_err(|e| e.to_string())?;
                    let captura = cliente.get_audiocaptureclient().map_err(|e| e.to_string())?;
                    cliente.start_stream().map_err(|e| e.to_string())?;
                    let _ = pronto_tx.send(Ok(()));
                    let mut bytes: VecDeque<u8> = VecDeque::new();
                    while !parar.load(Ordering::Relaxed) {
                        if evento.wait_for_event(500).is_err() {
                            continue; // silêncio: nada tocando
                        }
                        if captura.read_from_device_to_deque(&mut bytes).is_err() {
                            break;
                        }
                        // f32 estéreo → i16 mono
                        let quadros = bytes.len() / 8;
                        let mut mono = Vec::with_capacity(quadros);
                        for _ in 0..quadros {
                            let mut b = [0u8; 8];
                            for x in b.iter_mut() {
                                *x = bytes.pop_front().unwrap_or(0);
                            }
                            let l = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                            let r = f32::from_le_bytes([b[4], b[5], b[6], b[7]]);
                            mono.push((((l + r) * 0.5).clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
                        }
                        push(&mono);
                    }
                    let _ = cliente.stop_stream();
                    Ok(())
                })();
                if let Err(e) = r {
                    tracing::warn!("áudio do PC: {e}");
                    let _ = pronto_tx.send(Err(e));
                }
            })
            .map_err(|e| e.to_string())?;
        match pronto_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(r) => r,
            Err(_) => Err("O Windows não respondeu ao pedido de áudio.".into()),
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    pub fn supported() -> bool {
        false
    }

    pub fn start(_: Arc<AtomicBool>) -> Result<(), String> {
        Err("Mandar o áudio do PC por enquanto só funciona no Windows.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mistura_sem_estourar() {
        // Sem captura ligada, não mexe em nada.
        let mut q = vec![1000i16; 4];
        mix_into(&mut q);
        assert_eq!(q, vec![1000; 4]);
        // Com som na fila, soma (e satura em vez de virar ruído).
        *estado().parar.lock().unwrap() = Some(Arc::new(AtomicBool::new(false)));
        push(&[i16::MAX, 100, -100]);
        let mut q = vec![i16::MAX, 0, 0, 7];
        mix_into(&mut q);
        assert_eq!(q[0], i16::MAX);
        assert_eq!(q[1], 70);
        assert_eq!(q[2], -70);
        assert_eq!(q[3], 7, "acabou a fila: o resto fica como estava");
        stop();
    }
}
