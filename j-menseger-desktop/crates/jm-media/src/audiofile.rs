//! Áudio em arquivo: abrir, gravar, analisar.
//!
//! Tudo trabalha com PCM mono em 48 kHz (`f32` de -1 a 1), que é como o
//! Opus e a placa de som se entendem. A decodificação aceita o que o celular
//! manda (Ogg e WebM com Opus) e o que uma pessoa pode querer importar para o
//! soundboard (MP3, M4A/AAC, WAV, FLAC, Ogg Vorbis).

use crate::container::{self, OpusStream};

pub const SR: u32 = 48_000;

#[derive(Debug, thiserror::Error)]
pub enum AudioFileError {
    #[error("não consegui ler esse áudio: {0}")]
    Decode(String),
    #[error("não consegui gravar o áudio: {0}")]
    Encode(String),
    #[error("o áudio está vazio")]
    Empty,
}

type Result<T> = std::result::Result<T, AudioFileError>;

/// Áudio decodificado.
#[derive(Debug, Clone)]
pub struct Pcm {
    /// Mono, 48 kHz.
    pub samples: Vec<f32>,
}

impl Pcm {
    pub fn duration(&self) -> f64 {
        self.samples.len() as f64 / SR as f64
    }
}

/// Decodifica qualquer áudio aceito.
pub fn decode(bytes: &[u8]) -> Result<Pcm> {
    let pcm = if bytes.starts_with(b"OggS") {
        match container::read_ogg_opus(bytes) {
            Ok(s) => decode_opus(&s)?,
            Err(container::ContainerError::NotOpus) => decode_symphonia(bytes)?,
            Err(e) => return Err(AudioFileError::Decode(e.to_string())),
        }
    } else if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        let s = container::read_webm_opus(bytes).map_err(|e| AudioFileError::Decode(e.to_string()))?;
        decode_opus(&s)?
    } else {
        decode_symphonia(bytes)?
    };
    if pcm.samples.is_empty() {
        return Err(AudioFileError::Empty);
    }
    Ok(pcm)
}

/// Decodifica pacotes Opus (quadros de até 120 ms, mono ou estéreo).
pub fn decode_opus(s: &OpusStream) -> Result<Pcm> {
    let canais = if s.channels >= 2 { opus::Channels::Stereo } else { opus::Channels::Mono };
    let nc = if s.channels >= 2 { 2 } else { 1 };
    let mut dec = opus::Decoder::new(SR, canais).map_err(|e| AudioFileError::Decode(e.to_string()))?;
    let mut buf = vec![0f32; 5760 * nc];
    let mut out = Vec::new();
    for p in &s.packets {
        let n = match dec.decode_float(p, &mut buf, false) {
            Ok(n) => n,
            Err(_) => continue, // pacote estragado: pula, como um player faria
        };
        for k in 0..n {
            let v = if nc == 2 { (buf[2 * k] + buf[2 * k + 1]) * 0.5 } else { buf[k] };
            out.push(v);
        }
    }
    let pular = (s.pre_skip as usize).min(out.len());
    out.drain(..pular);
    Ok(Pcm { samples: out })
}

fn decode_symphonia(bytes: &[u8]) -> Result<Pcm> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes.to_vec())), Default::default());
    let probed = symphonia::default::get_probe()
        .format(&Hint::new(), mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AudioFileError::Decode(e.to_string()))?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| AudioFileError::Decode("sem faixa de áudio".into()))?
        .clone();
    let taxa = track.codec_params.sample_rate.unwrap_or(SR);
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AudioFileError::Decode(e.to_string()))?;

    let mut mono = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break, // fim do arquivo (ou arquivo cortado: usa o que veio)
        };
        if packet.track_id() != track.id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(_) => break,
        };
        let spec = *decoded.spec();
        let nc = spec.channels.count().max(1);
        let mut sb = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sb.copy_interleaved_ref(decoded);
        for quadro in sb.samples().chunks(nc) {
            mono.push(quadro.iter().sum::<f32>() / nc as f32);
        }
        // Proteção: nada de importar uma hora de música para um som de 8 s.
        if mono.len() > taxa as usize * 60 * 10 {
            break;
        }
    }
    Ok(Pcm { samples: resample_linear(&mono, taxa, SR) })
}

/// Reamostragem por interpolação linear (boa para 44,1 → 48 kHz).
pub fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let n = (input.len() as u64 * to as u64 / from as u64) as usize;
    let passo = from as f64 / to as f64;
    (0..n)
        .map(|i| {
            let x = i as f64 * passo;
            let k = x.floor() as usize;
            let f = (x - k as f64) as f32;
            let a = input[k.min(input.len() - 1)];
            let b = input[(k + 1).min(input.len() - 1)];
            a + (b - a) * f
        })
        .collect()
}

/// Comprime PCM mono 48 kHz em Ogg Opus.
///
/// Mensagens de voz usam 24 kbps (como o celular); sons do soundboard,
/// 64 kbps no modo "áudio", que preserva música e efeitos.
pub fn encode_ogg_opus(pcm: &[f32], bitrate: i32, music: bool) -> Result<Vec<u8>> {
    if pcm.is_empty() {
        return Err(AudioFileError::Empty);
    }
    let app = if music { opus::Application::Audio } else { opus::Application::Voip };
    let mut enc = opus::Encoder::new(SR, opus::Channels::Mono, app).map_err(|e| AudioFileError::Encode(e.to_string()))?;
    enc.set_bitrate(opus::Bitrate::Bits(bitrate)).map_err(|e| AudioFileError::Encode(e.to_string()))?;
    let pre_skip = enc.get_lookahead().unwrap_or(312).clamp(0, u16::MAX as i32) as u16;
    let mut packets = Vec::new();
    let mut counts = Vec::new();
    let mut buf = vec![0u8; 4000];
    for quadro in pcm.chunks(960) {
        let mut q = quadro.to_vec();
        q.resize(960, 0.0);
        let n = enc.encode_float(&q, &mut buf).map_err(|e| AudioFileError::Encode(e.to_string()))?;
        packets.push(buf[..n].to_vec());
        counts.push(960);
    }
    Ok(container::write_ogg_opus(&packets, &counts, pre_skip, pcm.len() as u64))
}

/// Forma de onda para a bolha de áudio: `n` barras de 2 a 31, a partir dos
/// níveis medidos durante a gravação (a mesma conta do celular).
pub fn waveform(levels: &[f32], n: usize) -> Vec<u8> {
    if levels.is_empty() {
        return vec![4; n];
    }
    (0..n)
        .map(|i| {
            let a = i * levels.len() / n;
            let b = ((i + 1) * levels.len() / n).max(a + 1);
            let m = levels[a..b.min(levels.len())].iter().copied().fold(0f32, f32::max);
            ((m.sqrt() * 31.0).round() as i32).clamp(2, 31) as u8
        })
        .collect()
}

/// Forma de onda calculada direto do áudio (para sons e áudios importados).
pub fn waveform_of(pcm: &[f32], n: usize) -> Vec<u8> {
    let janela = (SR as usize * 90 / 1000).max(1);
    let niveis: Vec<f32> = pcm
        .chunks(janela)
        .map(|c| (c.iter().map(|v| v.abs()).fold(0f32, f32::max) * 2.0).min(1.0))
        .collect();
    waveform(&niveis, n)
}

/// Picos por coluna, para desenhar o editor de corte.
pub fn peaks(pcm: &[f32], from: f64, to: f64, cols: usize) -> Vec<f32> {
    let s0 = ((from * SR as f64) as usize).min(pcm.len());
    let s1 = ((to * SR as f64) as usize).clamp(s0, pcm.len());
    let por = ((s1 - s0) / cols.max(1)).max(1);
    (0..cols)
        .map(|c| {
            let ini = s0 + c * por;
            let fim = (ini + por).min(s1);
            pcm.get(ini..fim).map(|x| x.iter().map(|v| v.abs()).fold(0f32, f32::max)).unwrap_or(0.0)
        })
        .collect()
}

/// Corta o silêncio do começo e do fim, como o editor do celular faz ao
/// abrir um áudio. Devolve (início, fim) em segundos.
pub fn auto_trim(pcm: &[f32]) -> (f64, f64) {
    let dur = pcm.len() as f64 / SR as f64;
    let mut pico = 0f32;
    for v in pcm.iter().step_by(4) {
        pico = pico.max(v.abs());
    }
    let limiar = 0.008f32.max(pico * 0.06);
    let alto = |i: usize| pcm[i].abs() > limiar;
    let mut i0 = 0usize;
    while i0 < pcm.len() && !alto(i0) {
        i0 += 16;
    }
    let mut i1 = pcm.len().saturating_sub(1);
    while i1 > i0 && !alto(i1) {
        i1 = i1.saturating_sub(16);
    }
    if i0 >= i1 {
        return (0.0, dur);
    }
    ((i0 as f64 / SR as f64 - 0.04).max(0.0), (i1 as f64 / SR as f64 + 0.12).min(dur))
}

/// Recorta um trecho, com rampas curtas nas pontas para não estalar.
pub fn clip(pcm: &[f32], from: f64, to: f64) -> Vec<f32> {
    let a = ((from * SR as f64) as usize).min(pcm.len());
    let b = ((to * SR as f64) as usize).clamp(a, pcm.len());
    let mut v = pcm[a..b].to_vec();
    let rampa = (SR as usize / 250).min(v.len() / 2); // 4 ms
    for k in 0..rampa {
        let g = k as f32 / rampa as f32;
        v[k] *= g;
        let j = v.len() - 1 - k;
        v[j] *= g;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seno(freq: f32, secs: f32) -> Vec<f32> {
        (0..(SR as f32 * secs) as usize)
            .map(|i| (i as f32 * freq * std::f32::consts::TAU / SR as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn opus_em_ogg_vai_e_volta_com_a_mesma_duracao() {
        let original = seno(440.0, 1.5);
        let ogg = encode_ogg_opus(&original, 64_000, true).unwrap();
        assert!(ogg.len() < 20_000, "1,5 s a 64 kbps não deveria passar de 20 KB: {}", ogg.len());
        let volta = decode(&ogg).unwrap();
        let dif = (volta.duration() - 1.5).abs();
        assert!(dif < 0.03, "duração mudou: {:.3} s", volta.duration());
        // O tom continua lá: a energia não sumiu.
        let rms = (volta.samples.iter().map(|v| v * v).sum::<f32>() / volta.samples.len() as f32).sqrt();
        assert!(rms > 0.2, "o som saiu apagado (rms {rms})");
    }

    #[test]
    fn forma_de_onda_igual_a_do_celular() {
        assert_eq!(waveform(&[], 5), vec![4; 5]);
        let w = waveform(&[0.0, 1.0, 0.25, 0.0], 4);
        assert_eq!(w, vec![2, 31, 16, 2]);
    }

    #[test]
    fn corte_automatico_tira_o_silencio() {
        let mut pcm = vec![0f32; SR as usize]; // 1 s de silêncio
        pcm.extend(seno(300.0, 0.5));
        pcm.extend(vec![0f32; SR as usize]);
        let (a, b) = auto_trim(&pcm);
        assert!((0.9..=1.0).contains(&a), "começo {a}");
        assert!((1.5..=1.7).contains(&b), "fim {b}");
    }

    #[test]
    fn reamostragem_mantem_a_duracao() {
        let v = vec![0.5f32; 44_100];
        let r = resample_linear(&v, 44_100, 48_000);
        assert_eq!(r.len(), 48_000);
        assert!(r.iter().all(|x| (*x - 0.5).abs() < 1e-6));
    }

    #[test]
    fn lixo_vira_erro_e_nao_panico() {
        assert!(decode(b"definitivamente nao e audio").is_err());
        assert!(decode(&[]).is_err());
    }
}
