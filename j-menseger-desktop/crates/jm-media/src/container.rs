//! Contêineres de áudio Opus: Ogg (ler e escrever) e WebM (ler).
//!
//! O celular grava mensagens de voz com o MediaRecorder do Android, que
//! entrega **WebM**; os sons próprios do soundboard ele gera em **Ogg** (com
//! um muxer próprio, em `opus.js`). O computador precisa abrir os dois e,
//! para mandar, escreve Ogg — que o celular toca sem problema.
//!
//! Os dois leitores aqui são mínimos de propósito: só o necessário para
//! achar os pacotes Opus em ordem. Arquivos estranhos dão erro, não pânico.

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ContainerError {
    #[error("arquivo de áudio inválido: {0}")]
    Invalid(&'static str),
    #[error("o áudio não está em Opus")]
    NotOpus,
}

type Result<T> = std::result::Result<T, ContainerError>;

/// Pacotes Opus de um arquivo, na ordem.
#[derive(Debug, Clone, PartialEq)]
pub struct OpusStream {
    pub channels: u8,
    /// Amostras do começo que o decodificador deve descartar (48 kHz).
    pub pre_skip: u16,
    pub packets: Vec<Vec<u8>>,
}

/* ---------------------------------------------------------------- Ogg */

/// CRC32 do Ogg: polinômio 0x04C11DB7, sem reflexão (o mesmo do celular).
fn crc_table() -> &'static [u32; 256] {
    use std::sync::OnceLock;
    static T: OnceLock<[u32; 256]> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let mut r = (i as u32) << 24;
            for _ in 0..8 {
                r = if r & 0x8000_0000 != 0 { (r << 1) ^ 0x04c1_1db7 } else { r << 1 };
            }
            *v = r;
        }
        t
    })
}

fn crc32(b: &[u8]) -> u32 {
    let t = crc_table();
    b.iter().fold(0u32, |c, &x| (c << 8) ^ t[(((c >> 24) ^ x as u32) & 0xff) as usize])
}

/// Escreve um Ogg Opus mono a 48 kHz.
///
/// `counts[i]` é quantas amostras o pacote `i` representa (960 para 20 ms).
pub fn write_ogg_opus(packets: &[Vec<u8>], counts: &[u32], pre_skip: u16, total_samples: u64) -> Vec<u8> {
    let serial: u32 = rand_u32();
    let mut out = Vec::new();
    let mut seq = 0u32;
    let mut page = |pkts: &[&[u8]], gran: u64, flags: u8, out: &mut Vec<u8>| {
        let mut lacing = Vec::new();
        for p in pkts {
            let mut n = p.len();
            while n >= 255 {
                lacing.push(255u8);
                n -= 255;
            }
            lacing.push(n as u8);
        }
        let inicio = out.len();
        out.extend_from_slice(b"OggS");
        out.push(0);
        out.push(flags);
        out.extend_from_slice(&gran.to_le_bytes());
        out.extend_from_slice(&serial.to_le_bytes());
        out.extend_from_slice(&seq.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.push(lacing.len() as u8);
        out.extend_from_slice(&lacing);
        for p in pkts {
            out.extend_from_slice(p);
        }
        let crc = crc32(&out[inicio..]);
        out[inicio + 22..inicio + 26].copy_from_slice(&crc.to_le_bytes());
        seq += 1;
    };

    let mut head = Vec::with_capacity(19);
    head.extend_from_slice(b"OpusHead");
    head.push(1); // versão
    head.push(1); // canais
    head.extend_from_slice(&pre_skip.to_le_bytes());
    head.extend_from_slice(&48_000u32.to_le_bytes());
    head.extend_from_slice(&[0, 0, 0]); // ganho, mapeamento
    page(&[&head], 0, 0x02, &mut out);

    let vendor = b"J-Menseger";
    let mut tags = Vec::new();
    tags.extend_from_slice(b"OpusTags");
    tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    tags.extend_from_slice(vendor);
    tags.extend_from_slice(&0u32.to_le_bytes());
    page(&[&tags], 0, 0, &mut out);

    let mut gran: u64 = 0;
    let mut atual: Vec<&[u8]> = Vec::new();
    let mut segs = 0usize;
    for (i, p) in packets.iter().enumerate() {
        let nseg = p.len() / 255 + 1;
        if !atual.is_empty() && (segs + nseg > 255 || atual.len() >= 50) {
            page(&atual, pre_skip as u64 + gran, 0, &mut out);
            atual.clear();
            segs = 0;
        }
        atual.push(p);
        segs += nseg;
        gran += counts.get(i).copied().unwrap_or(960) as u64;
    }
    let fim = (pre_skip as u64 + total_samples).min(pre_skip as u64 + gran);
    page(&atual, fim, 0x04, &mut out);
    out
}

/// Lê os pacotes Opus de um Ogg.
pub fn read_ogg_opus(b: &[u8]) -> Result<OpusStream> {
    let mut packets: Vec<Vec<u8>> = Vec::new();
    let mut parcial: Vec<u8> = Vec::new();
    let mut serial: Option<u32> = None;
    let mut i = 0usize;
    while i + 27 <= b.len() {
        if &b[i..i + 4] != b"OggS" {
            return Err(ContainerError::Invalid("página Ogg sem assinatura"));
        }
        let s = u32::from_le_bytes([b[i + 14], b[i + 15], b[i + 16], b[i + 17]]);
        let nseg = b[i + 26] as usize;
        let tab = i + 27;
        if tab + nseg > b.len() {
            return Err(ContainerError::Invalid("página Ogg cortada"));
        }
        let lacing = &b[tab..tab + nseg];
        let mut p = tab + nseg;
        let corpo: usize = lacing.iter().map(|x| *x as usize).sum();
        if p + corpo > b.len() {
            return Err(ContainerError::Invalid("página Ogg cortada"));
        }
        let nosso = *serial.get_or_insert(s) == s;
        for &l in lacing {
            if nosso {
                parcial.extend_from_slice(&b[p..p + l as usize]);
                if l < 255 {
                    packets.push(std::mem::take(&mut parcial));
                }
            }
            p += l as usize;
        }
        i = p;
    }
    let mut it = packets.into_iter();
    let head = it.next().ok_or(ContainerError::Invalid("Ogg vazio"))?;
    if head.len() < 19 || &head[..8] != b"OpusHead" {
        return Err(ContainerError::NotOpus);
    }
    let channels = head[9].max(1);
    let pre_skip = u16::from_le_bytes([head[10], head[11]]);
    let tags = it.next();
    if !tags.is_some_and(|t| t.starts_with(b"OpusTags")) {
        return Err(ContainerError::Invalid("Ogg Opus sem OpusTags"));
    }
    Ok(OpusStream { channels, pre_skip, packets: it.filter(|p| !p.is_empty()).collect() })
}

/* ---------------------------------------------------------------- WebM */

/// Lê um inteiro de tamanho variável do EBML. Devolve (valor, bytes usados,
/// é "tamanho desconhecido").
fn vint(b: &[u8], i: usize, manter_marca: bool) -> Option<(u64, usize, bool)> {
    let primeiro = *b.get(i)?;
    if primeiro == 0 {
        return None;
    }
    let len = primeiro.leading_zeros() as usize + 1;
    if len > 8 || i + len > b.len() {
        return None;
    }
    let mut v: u64 = if manter_marca { primeiro as u64 } else { (primeiro as u64) & ((1u64 << (8 - len)) - 1) };
    for k in 1..len {
        v = (v << 8) | b[i + k] as u64;
    }
    let todos_um = !manter_marca && v == (1u64 << (7 * len)) - 1;
    Some((v, len, todos_um))
}

// Elementos que contêm outros e que precisamos atravessar.
const SEGMENT: u64 = 0x1853_8067;
const CLUSTER: u64 = 0x1F43_B675;
const TRACKS: u64 = 0x1654_AE6B;
const TRACK_ENTRY: u64 = 0xAE;
const AUDIO: u64 = 0xE1;
const BLOCK_GROUP: u64 = 0xA0;
// Folhas que interessam.
const TRACK_NUMBER: u64 = 0xD7;
const CODEC_ID: u64 = 0x86;
const CODEC_PRIVATE: u64 = 0x63A2;
const CHANNELS: u64 = 0x9F;
const SIMPLE_BLOCK: u64 = 0xA3;
const BLOCK: u64 = 0xA1;

/// Lê os pacotes Opus de um WebM (como os do MediaRecorder).
///
/// O MediaRecorder grava "ao vivo", com segmento e grupos de tamanho
/// desconhecido; por isso o arquivo é percorrido em sequência, entrando nos
/// contêineres em vez de pular por tamanho.
pub fn read_webm_opus(b: &[u8]) -> Result<OpusStream> {
    if b.len() < 4 || b[..4] != [0x1A, 0x45, 0xDF, 0xA3] {
        return Err(ContainerError::Invalid("não é WebM"));
    }
    let mut i = 0usize;
    let mut faixa_opus: Option<u64> = None;
    let mut faixa_atual: Option<u64> = None;
    let mut codec_atual: Option<String> = None;
    let mut channels = 1u8;
    let mut pre_skip = 0u16;
    let mut packets = Vec::new();

    while i < b.len() {
        let Some((id, idl, _)) = vint(b, i, true) else { break };
        let Some((size, szl, desconhecido)) = vint(b, i + idl, false) else { break };
        let dados = i + idl + szl;
        match id {
            SEGMENT | CLUSTER | TRACKS | BLOCK_GROUP | AUDIO => {
                i = dados; // entra
                continue;
            }
            TRACK_ENTRY => {
                faixa_atual = None;
                codec_atual = None;
                i = dados;
                continue;
            }
            _ => {}
        }
        if desconhecido {
            return Err(ContainerError::Invalid("elemento WebM sem tamanho"));
        }
        let fim = dados.checked_add(size as usize).filter(|f| *f <= b.len());
        let Some(fim) = fim else {
            // Gravação interrompida no meio de um bloco: usa o que veio.
            break;
        };
        let corpo = &b[dados..fim];
        match id {
            TRACK_NUMBER => {
                faixa_atual = Some(corpo.iter().fold(0u64, |a, x| (a << 8) | *x as u64));
                if codec_atual.as_deref() == Some("A_OPUS") {
                    faixa_opus = faixa_atual;
                }
            }
            CODEC_ID => {
                let c = String::from_utf8_lossy(corpo).trim_end_matches('\0').to_string();
                if c == "A_OPUS" && faixa_atual.is_some() {
                    faixa_opus = faixa_atual;
                }
                codec_atual = Some(c);
            }
            CODEC_PRIVATE => {
                if corpo.len() >= 12 && &corpo[..8] == b"OpusHead" {
                    channels = corpo[9].max(1);
                    pre_skip = u16::from_le_bytes([corpo[10], corpo[11]]);
                }
            }
            CHANNELS => {
                if codec_atual.as_deref() == Some("A_OPUS") {
                    channels = (corpo.iter().fold(0u64, |a, x| (a << 8) | *x as u64) as u8).max(1);
                }
            }
            SIMPLE_BLOCK | BLOCK => {
                if let Some((faixa, n, _)) = vint(corpo, 0, false) {
                    let alvo = faixa_opus.or(faixa_atual);
                    if alvo.is_none() || Some(faixa) == alvo {
                        block_frames(&corpo[n..], &mut packets)?;
                    }
                }
            }
            _ => {}
        }
        i = fim;
    }
    if codec_atual.is_some_and(|c| c != "A_OPUS") && faixa_opus.is_none() {
        return Err(ContainerError::NotOpus);
    }
    if packets.is_empty() {
        return Err(ContainerError::Invalid("WebM sem áudio"));
    }
    Ok(OpusStream { channels, pre_skip, packets })
}

/// Separa os quadros de um bloco (com ou sem "lacing").
fn block_frames(b: &[u8], out: &mut Vec<Vec<u8>>) -> Result<()> {
    if b.len() < 3 {
        return Err(ContainerError::Invalid("bloco curto"));
    }
    let flags = b[2];
    let resto = &b[3..];
    match (flags >> 1) & 0x03 {
        0 => out.push(resto.to_vec()),
        lacing => {
            let n = *resto.first().ok_or(ContainerError::Invalid("bloco sem quadros"))? as usize + 1;
            let mut p = 1usize;
            let mut tamanhos = Vec::with_capacity(n);
            match lacing {
                1 => {
                    // Xiph
                    for _ in 0..n - 1 {
                        let mut t = 0usize;
                        loop {
                            let x = *resto.get(p).ok_or(ContainerError::Invalid("lacing Xiph"))?;
                            p += 1;
                            t += x as usize;
                            if x != 255 {
                                break;
                            }
                        }
                        tamanhos.push(t);
                    }
                }
                3 => {
                    // EBML
                    let (primeiro, l, _) = vint(resto, p, false).ok_or(ContainerError::Invalid("lacing EBML"))?;
                    p += l;
                    let mut ultimo = primeiro as i64;
                    tamanhos.push(primeiro as usize);
                    for _ in 1..n - 1 {
                        let (v, l, _) = vint(resto, p, false).ok_or(ContainerError::Invalid("lacing EBML"))?;
                        p += l;
                        let bias = (1i64 << (7 * l - 1)) - 1;
                        ultimo += v as i64 - bias;
                        tamanhos.push(ultimo.max(0) as usize);
                    }
                }
                _ => {
                    // tamanho fixo
                    let total = resto.len() - p;
                    for _ in 0..n - 1 {
                        tamanhos.push(total / n);
                    }
                }
            }
            let usados: usize = tamanhos.iter().sum();
            let ultimo = resto.len().checked_sub(p + usados).ok_or(ContainerError::Invalid("lacing maior que o bloco"))?;
            tamanhos.push(ultimo);
            for t in tamanhos {
                let fim = p + t;
                if fim > resto.len() {
                    return Err(ContainerError::Invalid("quadro além do bloco"));
                }
                out.push(resto[p..fim].to_vec());
                p = fim;
            }
        }
    }
    Ok(())
}

fn rand_u32() -> u32 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    h.finish() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ogg_vai_e_volta() {
        let pacotes: Vec<Vec<u8>> = (0..120).map(|i| vec![i as u8; 30 + (i % 300)]).collect();
        let counts = vec![960u32; pacotes.len()];
        let ogg = write_ogg_opus(&pacotes, &counts, 312, 960 * 120);
        let lido = read_ogg_opus(&ogg).unwrap();
        assert_eq!(lido.channels, 1);
        assert_eq!(lido.pre_skip, 312);
        assert_eq!(lido.packets, pacotes, "pacotes grandes (>255) precisam atravessar as páginas inteiros");
    }

    #[test]
    fn crc_confere_com_o_ogg_de_verdade() {
        // A página de cabeçalho tem CRC calculado com o campo zerado; recalcular
        // por cima da página pronta precisa dar o mesmo valor.
        let ogg = write_ogg_opus(&[vec![1, 2, 3]], &[960], 0, 960);
        let mut pagina = ogg[..47].to_vec(); // 27 + 1 + 19
        let gravado = u32::from_le_bytes([pagina[22], pagina[23], pagina[24], pagina[25]]);
        pagina[22..26].copy_from_slice(&[0, 0, 0, 0]);
        assert_eq!(crc32(&pagina), gravado);
    }

    #[test]
    fn ogg_que_nao_e_opus_e_recusado() {
        let mut falso = write_ogg_opus(&[vec![1]], &[960], 0, 960);
        falso[28..36].copy_from_slice(b"VorbisXX");
        assert_eq!(read_ogg_opus(&falso), Err(ContainerError::NotOpus));
        assert!(read_ogg_opus(b"isto nao e ogg nenhum").is_err());
    }

    #[test]
    fn webm_lixo_nao_derruba() {
        assert!(read_webm_opus(&[0x1A, 0x45, 0xDF, 0xA3, 0xFF, 0xFF]).is_err());
        assert!(read_webm_opus(b"").is_err());
    }
}
