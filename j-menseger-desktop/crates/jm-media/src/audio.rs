//! Microfone e alto-falante.
//!
//! **Aviso honesto:** este módulo não pôde ser testado no ambiente onde foi
//! escrito, porque não havia placa de som. A lógica que não depende de
//! hardware (escolha de dispositivo, conversão de taxa, mistura de vozes) tem
//! teste; a captura e a reprodução em si só serão confirmadas em um computador
//! com áudio de verdade.

use crate::codec::{FRAME_SAMPLES, SAMPLE_RATE};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("nenhum {0} encontrado neste computador")]
    NoDevice(&'static str),
    #[error("o sistema recusou o dispositivo: {0}")]
    Device(String),
    #[error("não foi possível abrir o áudio: {0}")]
    Stream(String),
}

type Result<T> = std::result::Result<T, AudioError>;

/// Um microfone ou uma saída de som, como aparece nos ajustes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub name: String,
    pub default: bool,
}

/// Microfones disponíveis.
pub fn input_devices() -> Vec<Device> {
    let host = cpal::default_host();
    let padrao = host.default_input_device().and_then(|d| nome_de(&d));
    host.input_devices()
        .map(|ds| {
            ds.filter_map(|d| {
                let nome = nome_de(&d)?;
                Some(Device {
                    default: Some(&nome) == padrao.as_ref(),
                    name: nome,
                })
            })
            .collect()
        })
        .unwrap_or_default()
}

/// Saídas de som disponíveis.
pub fn output_devices() -> Vec<Device> {
    let host = cpal::default_host();
    let padrao = host.default_output_device().and_then(|d| nome_de(&d));
    host.output_devices()
        .map(|ds| {
            ds.filter_map(|d| {
                let nome = nome_de(&d)?;
                Some(Device {
                    default: Some(&nome) == padrao.as_ref(),
                    name: nome,
                })
            })
            .collect()
        })
        .unwrap_or_default()
}

/// Preferências de áudio escolhidas nos ajustes: valem para chamadas, salas,
/// gravações e sons. `None` é o dispositivo padrão do sistema.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioPrefs {
    pub input: Option<String>,
    pub output: Option<String>,
    /// Volume do microfone: 1.0 é o original, até 3.0.
    pub input_gain: f32,
    /// Volume de quem fala na chamada: 1.0 é o original, até 2.0.
    pub output_volume: f32,
}

impl Default for AudioPrefs {
    fn default() -> Self {
        Self { input: None, output: None, input_gain: 1.0, output_volume: 1.0 }
    }
}

static PREFS: std::sync::RwLock<Option<AudioPrefs>> = std::sync::RwLock::new(None);

/// Troca as preferências. Vale para o próximo microfone ou saída aberto.
pub fn set_prefs(p: AudioPrefs) {
    *PREFS.write().unwrap() = Some(p);
}

pub fn prefs() -> AudioPrefs {
    PREFS.read().unwrap().clone().unwrap_or_default()
}

/// Abre um fluxo de entrada no formato que o dispositivo usa (f32, i16, u16
/// ou i32), entregando sempre f32. Muitos microfones USB só falam i16: pedir
/// f32 a eles falhava e o microfone "não funcionava".
pub(crate) fn build_input(
    dev: &cpal::Device,
    cfg: &cpal::SupportedStreamConfig,
    mut f: impl FnMut(&[f32]) + Send + 'static,
) -> Result<cpal::Stream> {
    use cpal::SampleFormat as S;
    let erro = |e: cpal::Error| AudioError::Stream(e.to_string());
    let falha = |e| tracing::warn!("erro na captura de áudio: {e}");
    let c = cfg.config();
    let mut tmp: Vec<f32> = Vec::new();
    match cfg.sample_format() {
        S::F32 => dev.build_input_stream(c.clone(), move |d: &[f32], _: &_| f(d), falha, None).map_err(erro),
        S::I16 => dev
            .build_input_stream(
                c.clone(),
                move |d: &[i16], _: &_| {
                    tmp.clear();
                    tmp.extend(d.iter().map(|s| *s as f32 / 32768.0));
                    f(&tmp)
                },
                falha,
                None,
            )
            .map_err(erro),
        S::U16 => dev
            .build_input_stream(
                c.clone(),
                move |d: &[u16], _: &_| {
                    tmp.clear();
                    tmp.extend(d.iter().map(|s| (*s as f32 - 32768.0) / 32768.0));
                    f(&tmp)
                },
                falha,
                None,
            )
            .map_err(erro),
        S::I32 => dev
            .build_input_stream(
                c.clone(),
                move |d: &[i32], _: &_| {
                    tmp.clear();
                    tmp.extend(d.iter().map(|s| *s as f32 / 2_147_483_648.0));
                    f(&tmp)
                },
                falha,
                None,
            )
            .map_err(erro),
        outro => Err(AudioError::Device(format!("formato de áudio não suportado: {outro:?}"))),
    }
}

/// Mesma ideia para a saída: o app gera f32 e converte para o formato da placa.
pub(crate) fn build_output(
    dev: &cpal::Device,
    cfg: &cpal::SupportedStreamConfig,
    mut f: impl FnMut(&mut [f32]) + Send + 'static,
) -> Result<cpal::Stream> {
    use cpal::SampleFormat as S;
    let erro = |e: cpal::Error| AudioError::Stream(e.to_string());
    let falha = |e| tracing::warn!("erro na reprodução de áudio: {e}");
    let c = cfg.config();
    let mut tmp: Vec<f32> = Vec::new();
    match cfg.sample_format() {
        S::F32 => dev.build_output_stream(c.clone(), move |d: &mut [f32], _: &_| f(d), falha, None).map_err(erro),
        S::I16 => dev
            .build_output_stream(
                c.clone(),
                move |d: &mut [i16], _: &_| {
                    tmp.resize(d.len(), 0.0);
                    f(&mut tmp);
                    for (o, s) in d.iter_mut().zip(&tmp) {
                        *o = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                    }
                },
                falha,
                None,
            )
            .map_err(erro),
        S::U16 => dev
            .build_output_stream(
                c.clone(),
                move |d: &mut [u16], _: &_| {
                    tmp.resize(d.len(), 0.0);
                    f(&mut tmp);
                    for (o, s) in d.iter_mut().zip(&tmp) {
                        *o = (s.clamp(-1.0, 1.0) * 32767.0 + 32768.0) as u16;
                    }
                },
                falha,
                None,
            )
            .map_err(erro),
        S::I32 => dev
            .build_output_stream(
                c.clone(),
                move |d: &mut [i32], _: &_| {
                    tmp.resize(d.len(), 0.0);
                    f(&mut tmp);
                    for (o, s) in d.iter_mut().zip(&tmp) {
                        *o = (s.clamp(-1.0, 1.0) as f64 * 2_147_483_647.0) as i32;
                    }
                },
                falha,
                None,
            )
            .map_err(erro),
        outro => Err(AudioError::Device(format!("formato de áudio não suportado: {outro:?}"))),
    }
}

/// Nome legível de um dispositivo de áudio.
fn nome_de(d: &cpal::Device) -> Option<String> {
    d.description().ok().map(|x| x.name().to_string())
}

/// Acha o microfone: o pedido, senão o escolhido nos ajustes, senão o
/// padrão. Um escolhido que sumiu (fone desconectado) cai no padrão.
pub(crate) fn find_input(nome: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    let escolhido = prefs().input;
    let nome = nome.or(escolhido.as_deref());
    if nome.is_some() {
        if let Ok(d) = find_input_exact(nome) {
            return Ok(d);
        }
        if escolhido.as_deref() == nome {
            tracing::warn!("microfone escolhido não encontrado; usando o padrão");
            return host.default_input_device().ok_or(AudioError::NoDevice("microfone"));
        }
    }
    find_input_exact(nome)
}

fn find_input_exact(nome: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    match nome {
        Some(n) => host
            .input_devices()
            .map_err(|e| AudioError::Device(e.to_string()))?
            .find(|d| nome_de(d).map(|x| x == n).unwrap_or(false))
            .ok_or(AudioError::NoDevice("microfone")),
        None => host.default_input_device().ok_or(AudioError::NoDevice("microfone")),
    }
}

pub(crate) fn find_output(nome: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    let escolhido = prefs().output;
    let nome = nome.or(escolhido.as_deref());
    if nome.is_some() {
        if let Ok(d) = find_output_exact(nome) {
            return Ok(d);
        }
        if escolhido.as_deref() == nome {
            tracing::warn!("saída de som escolhida não encontrada; usando a padrão");
            return host.default_output_device().ok_or(AudioError::NoDevice("alto-falante"));
        }
    }
    find_output_exact(nome)
}

fn find_output_exact(nome: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    match nome {
        Some(n) => host
            .output_devices()
            .map_err(|e| AudioError::Device(e.to_string()))?
            .find(|d| nome_de(d).map(|x| x == n).unwrap_or(false))
            .ok_or(AudioError::NoDevice("alto-falante")),
        None => host.default_output_device().ok_or(AudioError::NoDevice("alto-falante")),
    }
}

/// Converte de uma taxa de amostragem para outra.
///
/// Placas de som raramente entregam exatamente 48 kHz em um canal, que é o que
/// o Opus espera aqui. Esta conversão é simples (vizinho mais próximo), que
/// basta para voz; música exigiria algo melhor.
pub fn resample(entrada: &[f32], de: u32, para: u32, canais_entrada: u16) -> Vec<f32> {
    if entrada.is_empty() {
        return Vec::new();
    }
    // Primeiro mistura os canais em um só.
    let mono: Vec<f32> = if canais_entrada <= 1 {
        entrada.to_vec()
    } else {
        entrada
            .chunks(canais_entrada as usize)
            .map(|c| c.iter().sum::<f32>() / c.len() as f32)
            .collect()
    };
    if de == para {
        return mono;
    }
    let destino = (mono.len() as u64 * para as u64 / de.max(1) as u64) as usize;
    (0..destino)
        .map(|i| {
            let origem = (i as u64 * de as u64 / para.max(1) as u64) as usize;
            mono[origem.min(mono.len() - 1)]
        })
        .collect()
}

/// Mistura várias vozes em uma saída só.
///
/// Numa sala com várias pessoas, somar as vozes direto estoura e vira
/// distorção. Aqui a soma é limitada de forma suave: quando passa do limite,
/// o som é comprimido em vez de cortado.
pub fn mix(vozes: &[Vec<i16>]) -> Vec<i16> {
    if vozes.is_empty() {
        return Vec::new();
    }
    let n = vozes.iter().map(|v| v.len()).max().unwrap_or(0);
    (0..n)
        .map(|i| {
            let soma: f32 = vozes
                .iter()
                .map(|v| v.get(i).copied().unwrap_or(0) as f32 / 32768.0)
                .sum();
            // Curva suave: perto de zero é linear, nos extremos satura.
            let limitado = soma.tanh();
            (limitado * 32767.0) as i16
        })
        .collect()
}

/// Converte amostras de -1..1 para o formato que o Opus recebe.
pub fn f32_to_i16(v: &[f32]) -> Vec<i16> {
    v.iter()
        .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect()
}

pub fn i16_to_f32(v: &[i16]) -> Vec<f32> {
    v.iter().map(|s| *s as f32 / 32768.0).collect()
}

/// Captura do microfone, entregando quadros prontos para o Opus.
pub struct Capture {
    _stream: cpal::Stream,
    muted: Arc<AtomicBool>,
    level: Arc<AtomicU32>,
}

impl Capture {
    /// Abre o microfone. Cada quadro entregue tem exatamente 20 ms.
    pub fn start(device: Option<&str>) -> Result<(Self, mpsc::UnboundedReceiver<Vec<i16>>)> {
        let dispositivo = find_input(device)?;
        let config = dispositivo
            .default_input_config()
            .map_err(|e| AudioError::Device(e.to_string()))?;
        let taxa = config.sample_rate();
        let canais = config.channels();

        let (tx, rx) = mpsc::unbounded_channel();
        let muted = Arc::new(AtomicBool::new(false));
        let level = Arc::new(AtomicU32::new(0));
        // A placa entrega pedaços de tamanho qualquer; aqui eles viram quadros
        // de 20 ms, que é o que o codec espera.
        let sobra: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));

        let m = Arc::clone(&muted);
        let lv = Arc::clone(&level);
        let sb = Arc::clone(&sobra);
        let ganho = prefs().input_gain.clamp(0.0, 3.0);

        let stream = build_input(&dispositivo, &config, move |dados: &[f32]| {
                    let mut mono = resample(dados, taxa, SAMPLE_RATE, canais);
                    if (ganho - 1.0).abs() > 0.01 {
                        mono.iter_mut().for_each(|s| *s = (*s * ganho).clamp(-1.0, 1.0));
                    }
                    let mut pcm = f32_to_i16(&mono);
                    if m.load(Ordering::Relaxed) {
                        // Mudo: manda silêncio em vez de parar o fluxo, para a
                        // conexão não achar que a chamada caiu.
                        pcm.iter_mut().for_each(|s| *s = 0);
                    }
                    // Som do PC (jogo, vídeo) quando a transmissão pede: vai
                    // mesmo com o microfone no mudo.
                    crate::sysaudio::mix_into(&mut pcm);
                    lv.store(
                        (crate::codec::level(&pcm) * 1000.0) as u32,
                        Ordering::Relaxed,
                    );
                    let mut buf = sb.lock().unwrap();
                    buf.extend_from_slice(&pcm);
                    while buf.len() >= FRAME_SAMPLES {
                        let quadro: Vec<i16> = buf.drain(..FRAME_SAMPLES).collect();
                        if tx.send(quadro).is_err() {
                            return;
                        }
                    }
                })?;

        stream.play().map_err(|e| AudioError::Stream(e.to_string()))?;
        Ok((Self { _stream: stream, muted, level }, rx))
    }

    /// Liga e desliga o microfone.
    pub fn set_muted(&self, mudo: bool) {
        self.muted.store(mudo, Ordering::Relaxed);
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    /// Volume atual, de 0 a 1, para o medidor da interface.
    pub fn level(&self) -> f32 {
        self.level.load(Ordering::Relaxed) as f32 / 1000.0
    }
}

/// Reprodução no alto-falante.
pub struct Playback {
    _stream: cpal::Stream,
    fila: Arc<Mutex<Vec<i16>>>,
    volume: Arc<AtomicU32>,
}

impl Playback {
    pub fn start(device: Option<&str>) -> Result<Self> {
        let dispositivo = find_output(device)?;
        let config = dispositivo
            .default_output_config()
            .map_err(|e| AudioError::Device(e.to_string()))?;
        let taxa = config.sample_rate();
        let canais = config.channels() as usize;

        let fila: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let volume = Arc::new(AtomicU32::new((prefs().output_volume.clamp(0.0, 2.0) * 1000.0) as u32));
        let f = Arc::clone(&fila);
        let v = Arc::clone(&volume);

        let stream = build_output(&dispositivo, &config, move |saida: &mut [f32]| {
                    let quadros = saida.len() / canais.max(1);
                    let ganho = v.load(Ordering::Relaxed) as f32 / 1000.0;
                    let mut buf = f.lock().unwrap();
                    // Converte da taxa do codec para a da placa.
                    let precisa = (quadros as u64 * SAMPLE_RATE as u64 / taxa.max(1) as u64) as usize;
                    let disponivel = buf.len().min(precisa);
                    let pedaco: Vec<i16> = buf.drain(..disponivel).collect();
                    // Evita a fila crescer sem fim se a rede adiantar.
                    if buf.len() > SAMPLE_RATE as usize {
                        let excesso = buf.len() - SAMPLE_RATE as usize / 2;
                        buf.drain(..excesso);
                    }
                    drop(buf);

                    let mono = resample(&i16_to_f32(&pedaco), SAMPLE_RATE, taxa, 1);
                    for (i, quadro) in saida.chunks_mut(canais.max(1)).enumerate() {
                        let amostra = mono.get(i).copied().unwrap_or(0.0) * ganho;
                        for canal in quadro.iter_mut() {
                            *canal = amostra;
                        }
                    }
                })?;

        stream.play().map_err(|e| AudioError::Stream(e.to_string()))?;
        Ok(Self { _stream: stream, fila, volume })
    }

    /// Entrega áudio recém-decodificado para tocar.
    pub fn push(&self, pcm: &[i16]) {
        self.fila.lock().unwrap().extend_from_slice(pcm);
    }

    /// Volume de 0 a 1.
    pub fn set_volume(&self, v: f32) {
        self.volume
            .store((v.clamp(0.0, 2.0) * 1000.0) as u32, Ordering::Relaxed);
    }

    pub fn volume(&self) -> f32 {
        self.volume.load(Ordering::Relaxed) as f32 / 1000.0
    }

    /// Quanto áudio está esperando para tocar, em milissegundos.
    pub fn buffered_ms(&self) -> u32 {
        (self.fila.lock().unwrap().len() as u32 * 1000) / SAMPLE_RATE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mistura_canais_em_um_so() {
        // Dois canais iguais viram um canal com o mesmo valor.
        let estereo = vec![0.5, 0.5, -0.5, -0.5];
        let mono = resample(&estereo, 48_000, 48_000, 2);
        assert_eq!(mono, vec![0.5, -0.5]);
    }

    #[test]
    fn converte_a_taxa_de_amostragem() {
        let entrada: Vec<f32> = (0..44_100).map(|i| (i % 100) as f32 / 100.0).collect();
        let saida = resample(&entrada, 44_100, 48_000, 1);
        // Um segundo de som continua sendo um segundo.
        assert!(
            (saida.len() as i64 - 48_000).abs() < 50,
            "esperava perto de 48000 amostras, veio {}",
            saida.len()
        );

        let menor = resample(&entrada, 44_100, 16_000, 1);
        assert!((menor.len() as i64 - 16_000).abs() < 50);
    }

    #[test]
    fn taxa_igual_nao_mexe_no_som() {
        let v = vec![0.1, -0.2, 0.3];
        assert_eq!(resample(&v, 48_000, 48_000, 1), v);
        assert!(resample(&[], 48_000, 16_000, 1).is_empty());
    }

    #[test]
    fn mistura_de_vozes_nao_estoura() {
        // Quatro pessoas falando alto ao mesmo tempo.
        let alto = vec![30_000i16; 100];
        let m = mix(&[alto.clone(), alto.clone(), alto.clone(), alto.clone()]);
        assert_eq!(m.len(), 100);
        assert!(
            m.iter().all(|s| *s < 32_767 && *s > 0),
            "a soma deveria saturar suavemente, não estourar"
        );

        // Uma voz só passa quase intacta.
        let sozinha = vec![10_000i16; 10];
        let m1 = mix(&[sozinha.clone()]);
        assert!(
            (m1[0] as i32 - 10_000).abs() < 1500,
            "uma voz só não deveria ser alterada tanto: {} virou {}",
            10_000,
            m1[0]
        );
    }

    #[test]
    fn mistura_lida_com_tamanhos_diferentes() {
        let a = vec![1000i16; 50];
        let b = vec![1000i16; 20];
        let m = mix(&[a, b]);
        assert_eq!(m.len(), 50, "o resultado tem o tamanho da voz mais longa");
    }

    #[test]
    fn mistura_vazia_nao_quebra() {
        assert!(mix(&[]).is_empty());
    }

    #[test]
    fn converte_entre_os_formatos_de_amostra() {
        let orig = vec![0i16, 16_384, -16_384, 32_767];
        let f = i16_to_f32(&orig);
        let volta = f32_to_i16(&f);
        for (a, b) in orig.iter().zip(volta.iter()) {
            assert!((a - b).abs() <= 1, "{a} virou {b}");
        }
        // Valores fora da faixa são limitados em vez de dar a volta.
        assert_eq!(f32_to_i16(&[5.0, -5.0]), vec![32_767, -32_767]);
    }

    #[test]
    fn lista_dispositivos_sem_quebrar_sem_placa_de_som() {
        // Em um servidor sem áudio a lista vem vazia, e isso não é erro.
        let entradas = input_devices();
        let saidas = output_devices();
        for d in entradas.iter().chain(saidas.iter()) {
            assert!(!d.name.is_empty());
        }
        // No máximo um padrão de cada lado.
        assert!(entradas.iter().filter(|d| d.default).count() <= 1);
        assert!(saidas.iter().filter(|d| d.default).count() <= 1);
    }

    #[test]
    fn abrir_microfone_inexistente_da_erro_claro() {
        let r = Capture::start(Some("microfone que não existe"));
        assert!(r.is_err());
        if let Err(e) = r {
            let txt = e.to_string();
            assert!(
                txt.contains("microfone") || txt.contains("recusou"),
                "mensagem pouco clara: {txt}"
            );
        }
    }
}
