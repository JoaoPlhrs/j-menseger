//! Codec de áudio Opus.
//!
//! As configurações aqui não são escolha livre: elas precisam bater com o que
//! o aplicativo do celular usa, senão a chamada entre PC e celular sai com
//! ruído ou não sai. 48 kHz e um canal são o padrão do WebRTC; os quadros de
//! 20 ms são o compromisso usual entre atraso e eficiência.

/// Taxa de amostragem das chamadas.
pub const SAMPLE_RATE: u32 = 48_000;
/// Duração de cada quadro enviado, em milissegundos.
pub const FRAME_MS: u32 = 20;
/// Amostras por quadro, em um canal.
pub const FRAME_SAMPLES: usize = (SAMPLE_RATE * FRAME_MS / 1000) as usize; // 960

/// Qualidade do áudio da chamada, em bits por segundo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioQuality {
    /// Internet ruim: ainda inteligível, gasta pouco.
    Low,
    /// Padrão para conversa.
    Normal,
    /// Boa conexão: voz bem mais limpa.
    High,
}

impl AudioQuality {
    pub fn bitrate(&self) -> i32 {
        match self {
            AudioQuality::Low => 16_000,
            AudioQuality::Normal => 32_000,
            AudioQuality::High => 64_000,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AudioQuality::Low => "Economia",
            AudioQuality::Normal => "Normal",
            AudioQuality::High => "Alta",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("opus: {0}")]
    Opus(String),
    #[error("quadro com tamanho errado: {0} amostras, esperado {1}")]
    FrameSize(usize, usize),
}

type Result<T> = std::result::Result<T, CodecError>;

/// Comprime áudio para enviar.
pub struct Encoder {
    inner: opus::Encoder,
}

impl Encoder {
    pub fn new(quality: AudioQuality) -> Result<Self> {
        // `Voip` liga o modelo de fala do Opus: para conversa, soa melhor que
        // o modo genérico na mesma taxa de bits.
        let mut inner = opus::Encoder::new(SAMPLE_RATE, opus::Channels::Mono, opus::Application::Voip)
            .map_err(|e| CodecError::Opus(e.to_string()))?;
        inner
            .set_bitrate(opus::Bitrate::Bits(quality.bitrate()))
            .map_err(|e| CodecError::Opus(e.to_string()))?;
        // Em rede ruim, o Opus embute parte do quadro anterior para reconstruir
        // o que se perdeu, em vez de deixar um buraco audível.
        inner.set_inband_fec(true).map_err(|e| CodecError::Opus(e.to_string()))?;
        inner.set_packet_loss_perc(10).map_err(|e| CodecError::Opus(e.to_string()))?;
        // Taxa variável: a banda acompanha o que está sendo dito.
        inner.set_vbr(true).map_err(|e| CodecError::Opus(e.to_string()))?;
        // Durante o silêncio o Opus praticamente para de mandar, em vez de
        // gastar a mesma banda transmitindo nada.
        inner.set_dtx(true).map_err(|e| CodecError::Opus(e.to_string()))?;
        Ok(Self { inner })
    }

    pub fn set_quality(&mut self, q: AudioQuality) -> Result<()> {
        self.inner
            .set_bitrate(opus::Bitrate::Bits(q.bitrate()))
            .map_err(|e| CodecError::Opus(e.to_string()))
    }

    /// Comprime um quadro de 20 ms.
    pub fn encode(&mut self, pcm: &[i16]) -> Result<Vec<u8>> {
        if pcm.len() != FRAME_SAMPLES {
            return Err(CodecError::FrameSize(pcm.len(), FRAME_SAMPLES));
        }
        let mut saida = vec![0u8; 4000];
        let n = self
            .inner
            .encode(pcm, &mut saida)
            .map_err(|e| CodecError::Opus(e.to_string()))?;
        saida.truncate(n);
        Ok(saida)
    }
}

/// Descomprime o áudio que chega.
pub struct Decoder {
    inner: opus::Decoder,
}

impl Decoder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: opus::Decoder::new(SAMPLE_RATE, opus::Channels::Mono)
                .map_err(|e| CodecError::Opus(e.to_string()))?,
        })
    }

    pub fn decode(&mut self, packet: &[u8]) -> Result<Vec<i16>> {
        let mut pcm = vec![0i16; FRAME_SAMPLES];
        let n = self
            .inner
            .decode(packet, &mut pcm, false)
            .map_err(|e| CodecError::Opus(e.to_string()))?;
        pcm.truncate(n);
        Ok(pcm)
    }

    /// Reconstrói um quadro que se perdeu no caminho.
    ///
    /// Sem isto, cada pacote perdido vira um estalo no áudio; o Opus prefere
    /// inventar algo parecido com o som anterior, que passa despercebido.
    pub fn decode_lost(&mut self) -> Result<Vec<i16>> {
        let mut pcm = vec![0i16; FRAME_SAMPLES];
        let n = self
            .inner
            .decode(&[], &mut pcm, true)
            .map_err(|e| CodecError::Opus(e.to_string()))?;
        pcm.truncate(n);
        Ok(pcm)
    }
}

/// Quão alto está o som, de 0 a 1.
///
/// Usado para acender o indicador de quem está falando e para o medidor do
/// microfone nos ajustes.
pub fn level(pcm: &[i16]) -> f32 {
    if pcm.is_empty() {
        return 0.0;
    }
    let soma: f64 = pcm.iter().map(|s| (*s as f64 / 32768.0).powi(2)).sum();
    ((soma / pcm.len() as f64).sqrt() as f32).clamp(0.0, 1.0)
}

/// Detector simples de fala, para o indicador de "está falando".
///
/// Guarda um pouco de histórico porque uma pausa entre palavras não pode
/// apagar o indicador: ele ficaria piscando o tempo todo.
pub struct VoiceDetector {
    limiar: f32,
    quadros_ativos: u32,
    quadros_silencio: u32,
    falando: bool,
}

impl Default for VoiceDetector {
    fn default() -> Self {
        Self::new(0.02)
    }
}

impl VoiceDetector {
    pub fn new(limiar: f32) -> Self {
        Self {
            limiar,
            quadros_ativos: 0,
            quadros_silencio: 0,
            falando: false,
        }
    }

    /// Ajusta a sensibilidade (0 a 1). Valor maior exige falar mais alto.
    pub fn set_threshold(&mut self, v: f32) {
        self.limiar = v.clamp(0.001, 0.5);
    }

    /// Alimenta um quadro e diz se a pessoa está falando agora.
    pub fn push(&mut self, pcm: &[i16]) -> bool {
        if level(pcm) >= self.limiar {
            self.quadros_ativos += 1;
            self.quadros_silencio = 0;
            // Dois quadros (40 ms) evitam que um estalo acenda o indicador.
            if self.quadros_ativos >= 2 {
                self.falando = true;
            }
        } else {
            self.quadros_silencio += 1;
            self.quadros_ativos = 0;
            // Meio segundo de silêncio antes de apagar.
            if self.quadros_silencio >= 25 {
                self.falando = false;
            }
        }
        self.falando
    }

    pub fn is_speaking(&self) -> bool {
        self.falando
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um tom simples, do jeito que a voz chegaria do microfone.
    fn tom(freq: f32, amplitude: f32) -> Vec<i16> {
        (0..FRAME_SAMPLES)
            .map(|i| {
                let t = i as f32 / SAMPLE_RATE as f32;
                ((t * freq * std::f32::consts::TAU).sin() * amplitude * 32767.0) as i16
            })
            .collect()
    }

    #[test]
    fn um_quadro_tem_vinte_milissegundos() {
        assert_eq!(FRAME_SAMPLES, 960);
        assert_eq!(FRAME_SAMPLES as u32 * 1000 / SAMPLE_RATE, FRAME_MS);
    }

    #[test]
    fn comprime_e_descomprime_mantendo_o_som() {
        let mut enc = Encoder::new(AudioQuality::Normal).unwrap();
        let mut dec = Decoder::new().unwrap();

        let original = tom(440.0, 0.5);
        // O Opus precisa de alguns quadros para estabilizar.
        for _ in 0..3 {
            let p = enc.encode(&original).unwrap();
            dec.decode(&p).unwrap();
        }

        let pacote = enc.encode(&original).unwrap();
        assert!(!pacote.is_empty(), "o pacote não pode sair vazio");
        assert!(pacote.len() < 200, "20 ms não deveriam passar de 200 bytes: {}", pacote.len());

        let volta = dec.decode(&pacote).unwrap();
        assert_eq!(volta.len(), FRAME_SAMPLES, "tem que voltar o quadro inteiro");

        // O Opus tem perdas, então não comparamos amostra a amostra: o que
        // precisa sobreviver é a energia do som.
        let antes = level(&original);
        let depois = level(&volta);
        assert!(
            (antes - depois).abs() < 0.15,
            "o volume mudou demais: {antes:.3} virou {depois:.3}"
        );
    }

    #[test]
    fn silencio_gasta_muito_menos_banda_que_fala() {
        let medir = |amostras: &[i16]| {
            let mut enc = Encoder::new(AudioQuality::Normal).unwrap();
            // Os primeiros quadros são de aquecimento do codec.
            for _ in 0..10 {
                enc.encode(amostras).unwrap();
            }
            (0..50).map(|_| enc.encode(amostras).unwrap().len()).sum::<usize>()
        };

        let mudo = medir(&vec![0i16; FRAME_SAMPLES]);
        let falando = medir(&tom(300.0, 0.6));

        assert!(
            mudo * 3 < falando,
            "o silêncio deveria custar bem menos que a fala: {mudo} contra {falando} bytes"
        );
    }

    #[test]
    fn qualidade_maior_gasta_mais_banda() {
        let mut tamanhos = Vec::new();
        for q in [AudioQuality::Low, AudioQuality::Normal, AudioQuality::High] {
            let mut enc = Encoder::new(q).unwrap();
            let fala = tom(220.0, 0.6);
            let mut total = 0;
            for _ in 0..25 {
                total += enc.encode(&fala).unwrap().len();
            }
            tamanhos.push(total);
        }
        assert!(
            tamanhos[0] < tamanhos[1] && tamanhos[1] < tamanhos[2],
            "as taxas deveriam crescer: {tamanhos:?}"
        );
    }

    #[test]
    fn quadro_do_tamanho_errado_e_recusado() {
        let mut enc = Encoder::new(AudioQuality::Normal).unwrap();
        let erro = enc.encode(&vec![0i16; 100]).unwrap_err();
        assert!(matches!(erro, CodecError::FrameSize(100, 960)));
    }

    #[test]
    fn pacote_perdido_vira_som_inventado_em_vez_de_buraco() {
        let mut enc = Encoder::new(AudioQuality::Normal).unwrap();
        let mut dec = Decoder::new().unwrap();
        let fala = tom(330.0, 0.5);
        for _ in 0..5 {
            dec.decode(&enc.encode(&fala).unwrap()).unwrap();
        }
        let recuperado = dec.decode_lost().unwrap();
        assert_eq!(recuperado.len(), FRAME_SAMPLES);
        assert!(
            level(&recuperado) > 0.0,
            "deveria inventar algo parecido, não devolver silêncio"
        );
    }

    #[test]
    fn mede_o_volume_do_som() {
        assert_eq!(level(&[]), 0.0);
        assert_eq!(level(&vec![0i16; 960]), 0.0);
        let baixo = level(&tom(440.0, 0.1));
        let alto = level(&tom(440.0, 0.8));
        assert!(alto > baixo * 4.0, "baixo {baixo:.3}, alto {alto:.3}");
        assert!(alto <= 1.0);
    }

    #[test]
    fn detector_de_fala_ignora_estalo_e_segura_a_pausa() {
        let mut vad = VoiceDetector::new(0.02);
        let silencio = vec![0i16; FRAME_SAMPLES];
        let voz = tom(300.0, 0.4);

        assert!(!vad.push(&silencio));
        // Um único quadro alto é estalo, não fala.
        assert!(!vad.push(&voz), "um quadro só não deveria acender");
        assert!(vad.push(&voz), "dois quadros seguidos já são fala");

        // Pausa curta entre palavras não apaga o indicador.
        for _ in 0..10 {
            assert!(vad.push(&silencio), "a pausa curta apagou o indicador");
        }
        // Silêncio longo apaga.
        for _ in 0..20 {
            vad.push(&silencio);
        }
        assert!(!vad.is_speaking(), "depois de meio segundo deveria apagar");
    }

    #[test]
    fn sensibilidade_fica_em_faixa_util() {
        let mut vad = VoiceDetector::default();
        vad.set_threshold(10.0);
        vad.set_threshold(-5.0);
        // Com o limiar no mínimo, um sinal fraco já conta como fala.
        let fraco = tom(300.0, 0.02);
        vad.push(&fraco);
        assert!(vad.push(&fraco));
    }
}
