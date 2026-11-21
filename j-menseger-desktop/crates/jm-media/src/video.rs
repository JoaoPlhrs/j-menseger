//! Codificação de vídeo VP8, para a transmissão de tela e a câmera.
//!
//! VP8 é o codec que todo cliente WebRTC entende, inclusive o navegador que
//! roda dentro do aplicativo do celular. Isso importa mais aqui do que ganhar
//! alguns por cento de compressão com um codec mais novo: de nada adianta
//! comprimir melhor se o outro lado não consegue abrir.
//!
//! A codificação por hardware (NVENC, AMF, Quick Sync) é o passo seguinte e
//! está anotada como tal: esta versão codifica por software, que funciona em
//! qualquer máquina mas gasta mais processador em 1440p.

use crate::screen::{Fps, Quality};
/// Ligações com o libvpx, geradas na compilação a partir dos cabeçalhos
/// instalados (veja `build.rs`).
#[allow(non_camel_case_types, non_upper_case_globals, non_snake_case, dead_code)]
mod vpx {
    include!(concat!(env!("OUT_DIR"), "/vpx.rs"));
}
use std::ptr;

unsafe extern "C" {
    /// Versão de ABI do libvpx deste sistema, lida do cabeçalho ao compilar.
    fn jm_vpx_encoder_abi_version() -> i32;
    fn jm_vpx_decoder_abi_version() -> i32;
}

// Estes dois valores entram em parâmetros que o C declara como `long` e
// `unsigned long`. Esses tipos têm 64 bits no Linux e 32 no Windows, então
// fixar um tamanho aqui faria o código compilar num sistema e falhar no outro.
// Usando os tipos das ligações geradas, cada sistema recebe a largura certa.
/// Pede que este quadro seja uma imagem inteira.
const VPX_EFLAG_FORCE_KF: vpx::vpx_enc_frame_flags_t = 1;
/// Prazo de codificação: tempo real, sem segurar quadros.
const VPX_DL_REALTIME: std::os::raw::c_ulong = 1;
/// Marca de quadro-chave na saída do codec.
const VPX_FRAME_IS_KEY: u32 = 0x1;

#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    #[error("não foi possível preparar o codificador de vídeo (código {0})")]
    Init(u32),
    #[error("a codificação falhou (código {0})")]
    Encode(u32),
    #[error("tamanho de quadro inválido: {0}×{1}")]
    Size(u32, u32),
}

type Result<T> = std::result::Result<T, VideoError>;

/// Um quadro já comprimido, pronto para ir pela conexão.
#[derive(Debug, Clone)]
pub struct EncodedFrame {
    pub data: Vec<u8>,
    /// Quadro-chave: a imagem inteira, que permite a quem chega depois
    /// começar a ver sem esperar.
    pub keyframe: bool,
}

/// Converte a imagem capturada (RGBA) para o formato que o codec usa (I420).
///
/// O olho humano enxerga muito mais detalhe em brilho do que em cor, então o
/// I420 guarda a cor em um quarto da resolução. É daí que vem boa parte da
/// compressão, antes mesmo do codec entrar em ação.
pub fn rgba_to_i420(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let tam_y = w * h;
    let tam_c = (w / 2) * (h / 2);
    let mut saida = vec![0u8; tam_y + tam_c * 2];

    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let (r, g, b) = (rgba[i] as i32, rgba[i + 1] as i32, rgba[i + 2] as i32);
            // Coeficientes da recomendação BT.601, que é o que o VP8 espera.
            saida[y * w + x] = (((66 * r + 129 * g + 25 * b + 128) >> 8) + 16).clamp(0, 255) as u8;
        }
    }

    // A cor é amostrada a cada dois pixels, nas duas direções.
    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let mut sr = 0i32;
            let mut sg = 0i32;
            let mut sb = 0i32;
            let mut n = 0i32;
            for dy in 0..2.min(h - y) {
                for dx in 0..2.min(w - x) {
                    let i = ((y + dy) * w + (x + dx)) * 4;
                    sr += rgba[i] as i32;
                    sg += rgba[i + 1] as i32;
                    sb += rgba[i + 2] as i32;
                    n += 1;
                }
            }
            let (r, g, b) = (sr / n, sg / n, sb / n);
            let cx = x / 2;
            let cy = y / 2;
            let u = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
            let v = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
            saida[tam_y + cy * (w / 2) + cx] = u.clamp(0, 255) as u8;
            saida[tam_y + tam_c + cy * (w / 2) + cx] = v.clamp(0, 255) as u8;
        }
    }
    saida
}

/// Codificador VP8.
pub struct VideoEncoder {
    ctx: Box<vpx::vpx_codec_ctx_t>,
    image: Box<vpx::vpx_image_t>,
    width: u32,
    height: u32,
    frame: i64,
    fps: Fps,
    /// Buffer reaproveitado entre quadros, para não alocar 60 vezes por segundo.
    i420: Vec<u8>,
}

impl VideoEncoder {
    /// Prepara o codificador para um tamanho e uma qualidade.
    pub fn new(width: u32, height: u32, quality: Quality, fps: Fps) -> Result<Self> {
        if width < 2 || height < 2 || width % 2 != 0 || height % 2 != 0 {
            return Err(VideoError::Size(width, height));
        }
        unsafe {
            let iface = vpx::vpx_codec_vp8_cx();
            let mut cfg: vpx::vpx_codec_enc_cfg_t = std::mem::zeroed();
            let r = vpx::vpx_codec_enc_config_default(iface, &mut cfg, 0);
            if r != vpx::vpx_codec_err_t_VPX_CODEC_OK {
                return Err(VideoError::Init(r));
            }

            cfg.g_w = width;
            cfg.g_h = height;
            cfg.g_timebase.num = 1;
            cfg.g_timebase.den = fps.value() as i32;
            cfg.rc_target_bitrate = quality.bitrate(fps) / 1000; // em kbps
            cfg.rc_end_usage = vpx::vpx_rc_mode_VPX_CBR;
            // Sem atraso: numa transmissão ao vivo não dá para segurar quadros
            // esperando os próximos para comprimir melhor.
            cfg.g_lag_in_frames = 0;
            cfg.g_pass = vpx::vpx_enc_pass_VPX_RC_ONE_PASS;
            // Permite ao decodificador se recuperar sozinho de pacote perdido,
            // em vez de congelar a imagem até o próximo quadro-chave.
            cfg.g_error_resilient = 1;
            cfg.g_threads = num_threads();
            // Um quadro-chave a cada poucos segundos: quem entra no meio da
            // transmissão não fica olhando para uma tela preta.
            cfg.kf_max_dist = fps.value() * 4;
            // Prefere descartar quadro a borrar a imagem quando a banda cai.
            cfg.rc_dropframe_thresh = 25;

            let mut ctx: Box<vpx::vpx_codec_ctx_t> = Box::new(std::mem::zeroed());
            let r = vpx::vpx_codec_enc_init_ver(
                &mut *ctx,
                iface,
                &mut cfg,
                0,
                jm_vpx_encoder_abi_version(),
            );
            if r != vpx::vpx_codec_err_t_VPX_CODEC_OK {
                return Err(VideoError::Init(r));
            }

            let mut image: Box<vpx::vpx_image_t> = Box::new(std::mem::zeroed());
            if vpx::vpx_img_alloc(&mut *image, vpx::vpx_img_fmt_VPX_IMG_FMT_I420, width, height, 1).is_null() {
                return Err(VideoError::Init(0));
            }

            Ok(Self {
                ctx,
                image,
                width,
                height,
                frame: 0,
                fps,
                i420: Vec::new(),
            })
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Comprime um quadro RGBA. `force_key` pede uma imagem inteira.
    pub fn encode(&mut self, rgba: &[u8], force_key: bool) -> Result<Vec<EncodedFrame>> {
        let esperado = (self.width as usize) * (self.height as usize) * 4;
        if rgba.len() < esperado {
            return Err(VideoError::Size(self.width, self.height));
        }

        self.i420 = rgba_to_i420(&rgba[..esperado], self.width, self.height);

        unsafe {
            // Copia os três planos para a imagem do codec, respeitando o
            // espaçamento que ele escolheu para cada linha.
            let w = self.width as usize;
            let h = self.height as usize;
            let tam_y = w * h;
            let tam_c = (w / 2) * (h / 2);

            copiar_plano(&mut self.image, 0, &self.i420[..tam_y], w, h);
            copiar_plano(&mut self.image, 1, &self.i420[tam_y..tam_y + tam_c], w / 2, h / 2);
            copiar_plano(
                &mut self.image,
                2,
                &self.i420[tam_y + tam_c..tam_y + tam_c * 2],
                w / 2,
                h / 2,
            );

            let flags = if force_key { VPX_EFLAG_FORCE_KF } else { 0 };
            let r = vpx::vpx_codec_encode(
                &mut *self.ctx,
                &mut *self.image,
                self.frame,
                1,
                flags,
                VPX_DL_REALTIME,
            );
            if r != vpx::vpx_codec_err_t_VPX_CODEC_OK {
                return Err(VideoError::Encode(r));
            }
            self.frame += 1;

            let mut saida = Vec::new();
            let mut iter: vpx::vpx_codec_iter_t = ptr::null();
            loop {
                let pkt = vpx::vpx_codec_get_cx_data(&mut *self.ctx, &mut iter);
                if pkt.is_null() {
                    break;
                }
                if (*pkt).kind != vpx::vpx_codec_cx_pkt_kind_VPX_CODEC_CX_FRAME_PKT {
                    continue;
                }
                let quadro = (*pkt).data.frame;
                let dados =
                    std::slice::from_raw_parts(quadro.buf as *const u8, quadro.sz).to_vec();
                saida.push(EncodedFrame {
                    keyframe: (quadro.flags & VPX_FRAME_IS_KEY) != 0,
                    data: dados,
                });
            }
            Ok(saida)
        }
    }

    /// Duração de um quadro, para enviar junto com a amostra.
    pub fn frame_duration(&self) -> std::time::Duration {
        self.fps.interval()
    }
}

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        unsafe {
            vpx::vpx_img_free(&mut *self.image);
            vpx::vpx_codec_destroy(&mut *self.ctx);
        }
    }
}

// O codificador é usado por uma thread de cada vez, e os ponteiros do libvpx
// pertencem a ele; não há estado global compartilhado.
unsafe impl Send for VideoEncoder {}

unsafe fn copiar_plano(
    image: &mut vpx::vpx_image_t,
    plano: usize,
    origem: &[u8],
    largura: usize,
    altura: usize,
) {
    let destino = image.planes[plano];
    let passo = image.stride[plano] as usize;
    for y in 0..altura {
        let de = y * largura;
        let para = destino.add(y * passo);
        ptr::copy_nonoverlapping(origem[de..de + largura].as_ptr(), para, largura);
    }
}

/// Threads de codificação, sem engolir a máquina inteira.
fn num_threads() -> u32 {
    let n = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(2);
    n.clamp(1, 8).saturating_sub(1).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadro_colorido(w: u32, h: u32, deslocamento: u8) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                v[i] = ((x * 255 / w.max(1)) as u8).wrapping_add(deslocamento);
                v[i + 1] = (y * 255 / h.max(1)) as u8;
                v[i + 2] = 90;
                v[i + 3] = 255;
            }
        }
        v
    }

    #[test]
    fn converte_cores_para_o_formato_do_codec() {
        let w = 4;
        let h = 4;
        // Branco puro.
        let branco = vec![255u8; (w * h * 4) as usize];
        let i420 = rgba_to_i420(&branco, w, h);
        assert_eq!(i420.len(), (w * h + (w / 2) * (h / 2) * 2) as usize);
        // Brilho perto do máximo, cor neutra.
        assert!(i420[0] > 230, "branco deveria ser claro: {}", i420[0]);
        let inicio_cor = (w * h) as usize;
        assert!(
            (i420[inicio_cor] as i32 - 128).abs() < 6,
            "branco não deveria ter cor: {}",
            i420[inicio_cor]
        );

        // Preto puro.
        let mut preto = vec![0u8; (w * h * 4) as usize];
        for p in preto.chunks_mut(4) {
            p[3] = 255;
        }
        let i420 = rgba_to_i420(&preto, w, h);
        assert!(i420[0] < 30, "preto deveria ser escuro: {}", i420[0]);
    }

    #[test]
    fn vermelho_e_azul_ficam_em_lados_opostos_da_cor() {
        let w = 2;
        let h = 2;
        let mut vermelho = vec![0u8; 16];
        let mut azul = vec![0u8; 16];
        for i in 0..4 {
            vermelho[i * 4] = 255;
            vermelho[i * 4 + 3] = 255;
            azul[i * 4 + 2] = 255;
            azul[i * 4 + 3] = 255;
        }
        let iv = rgba_to_i420(&vermelho, w, h);
        let ia = rgba_to_i420(&azul, w, h);
        let base = (w * h) as usize;
        let tam_c = ((w / 2) * (h / 2)) as usize;
        // No vermelho o canal V sobe; no azul, o U.
        assert!(iv[base + tam_c] > 180, "V do vermelho: {}", iv[base + tam_c]);
        assert!(ia[base] > 180, "U do azul: {}", ia[base]);
    }

    #[test]
    fn comprime_um_quadro_de_verdade() {
        let (w, h) = (320u32, 240u32);
        let mut enc = VideoEncoder::new(w, h, Quality::P720, Fps::Fps30).expect("não preparou");
        assert_eq!(enc.size(), (w, h));

        let pacotes = enc.encode(&quadro_colorido(w, h, 0), false).unwrap();
        assert!(!pacotes.is_empty(), "o primeiro quadro não saiu");
        assert!(pacotes[0].keyframe, "o primeiro quadro tem que ser completo");
        assert!(!pacotes[0].data.is_empty());

        let bruto = (w * h * 4) as usize;
        assert!(
            pacotes[0].data.len() < bruto / 10,
            "deveria comprimir bastante: {} de {bruto} bytes",
            pacotes[0].data.len()
        );
    }

    #[test]
    fn quadro_parecido_com_o_anterior_fica_bem_menor() {
        let (w, h) = (320u32, 240u32);
        let mut enc = VideoEncoder::new(w, h, Quality::P720, Fps::Fps30).unwrap();
        let base = quadro_colorido(w, h, 0);

        let chave = enc.encode(&base, false).unwrap();
        let tam_chave: usize = chave.iter().map(|p| p.data.len()).sum();

        // Mesma imagem de novo: quase nada muda.
        let repetido = enc.encode(&base, false).unwrap();
        let tam_repetido: usize = repetido.iter().map(|p| p.data.len()).sum();

        assert!(
            tam_repetido * 5 < tam_chave,
            "quadro repetido deveria ser muito menor: {tam_repetido} contra {tam_chave}"
        );
        assert!(
            repetido.iter().all(|p| !p.keyframe),
            "não deveria mandar imagem inteira de novo"
        );
    }

    #[test]
    fn pedir_quadro_chave_manda_a_imagem_inteira() {
        let (w, h) = (160u32, 120u32);
        let mut enc = VideoEncoder::new(w, h, Quality::P720, Fps::Fps30).unwrap();
        let base = quadro_colorido(w, h, 0);
        enc.encode(&base, false).unwrap();
        enc.encode(&base, false).unwrap();

        let forcado = enc.encode(&base, true).unwrap();
        assert!(
            forcado.iter().any(|p| p.keyframe),
            "pedimos imagem inteira e não veio"
        );
    }

    #[test]
    fn tamanho_impar_e_recusado() {
        assert!(matches!(
            VideoEncoder::new(321, 240, Quality::P720, Fps::Fps30),
            Err(VideoError::Size(321, 240))
        ));
        assert!(VideoEncoder::new(0, 0, Quality::P720, Fps::Fps30).is_err());
    }

    #[test]
    fn quadro_pequeno_demais_e_recusado() {
        let mut enc = VideoEncoder::new(64, 64, Quality::P720, Fps::Fps30).unwrap();
        let erro = enc.encode(&vec![0u8; 100], false).unwrap_err();
        assert!(matches!(erro, VideoError::Size(64, 64)));
    }

    #[test]
    fn qualidade_maior_gera_mais_dados() {
        let (w, h) = (320u32, 240u32);
        let medir = |q: Quality| {
            let mut enc = VideoEncoder::new(w, h, q, Fps::Fps30).unwrap();
            let mut total = 0;
            for i in 0..20 {
                // Imagem sempre mudando, para o codec não economizar à toa.
                for p in enc.encode(&quadro_colorido(w, h, (i * 9) as u8), false).unwrap() {
                    total += p.data.len();
                }
            }
            total
        };
        let baixa = medir(Quality::P720);
        let alta = medir(Quality::P1440);
        assert!(
            alta > baixa,
            "mais qualidade deveria gastar mais: {baixa} contra {alta}"
        );
    }
}

/* ---------------- decodificação ---------------- */

/// Decodificador VP8, para mostrar o que a outra pessoa transmite.
pub struct VideoDecoder {
    ctx: Box<vpx::vpx_codec_ctx_t>,
}

impl VideoDecoder {
    pub fn new() -> Result<Self> {
        unsafe {
            let iface = vpx::vpx_codec_vp8_dx();
            let mut cfg: vpx::vpx_codec_dec_cfg_t = std::mem::zeroed();
            // Deixa o codec usar mais de um núcleo em telas grandes.
            cfg.threads = num_threads();
            let mut ctx: Box<vpx::vpx_codec_ctx_t> = Box::new(std::mem::zeroed());
            let r = vpx::vpx_codec_dec_init_ver(
                &mut *ctx,
                iface,
                &mut cfg,
                0,
                jm_vpx_decoder_abi_version(),
            );
            if r != vpx::vpx_codec_err_t_VPX_CODEC_OK {
                return Err(VideoError::Init(r));
            }
            Ok(Self { ctx })
        }
    }

    /// Descomprime um pacote e devolve a imagem em RGBA, se houver uma pronta.
    pub fn decode(&mut self, data: &[u8]) -> Result<Option<DecodedFrame>> {
        unsafe {
            let r = vpx::vpx_codec_decode(
                &mut *self.ctx,
                data.as_ptr(),
                data.len() as u32,
                ptr::null_mut(),
                0,
            );
            if r != vpx::vpx_codec_err_t_VPX_CODEC_OK {
                return Err(VideoError::Encode(r));
            }
            let mut iter: vpx::vpx_codec_iter_t = ptr::null();
            let img = vpx::vpx_codec_get_frame(&mut *self.ctx, &mut iter);
            if img.is_null() {
                // Pacote que sozinho ainda não forma uma imagem.
                return Ok(None);
            }
            Ok(Some(i420_to_rgba(&*img)))
        }
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        unsafe {
            vpx::vpx_codec_destroy(&mut *self.ctx);
        }
    }
}

unsafe impl Send for VideoDecoder {}

/// Uma imagem recebida, pronta para desenhar.
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Converte a imagem do codec de volta para cores de tela.
unsafe fn i420_to_rgba(img: &vpx::vpx_image_t) -> DecodedFrame {
    let w = img.d_w as usize;
    let h = img.d_h as usize;
    let mut rgba = vec![0u8; w * h * 4];

    let (py, pu, pv) = (img.planes[0], img.planes[1], img.planes[2]);
    let (sy, su, sv) = (
        img.stride[0] as usize,
        img.stride[1] as usize,
        img.stride[2] as usize,
    );

    for y in 0..h {
        for x in 0..w {
            let luz = *py.add(y * sy + x) as i32;
            // A cor vem em metade da resolução nas duas direções.
            let u = *pu.add((y / 2) * su + (x / 2)) as i32 - 128;
            let v = *pv.add((y / 2) * sv + (x / 2)) as i32 - 128;
            let c = (luz - 16).max(0) * 298;

            let i = (y * w + x) * 4;
            rgba[i] = ((c + 409 * v + 128) >> 8).clamp(0, 255) as u8;
            rgba[i + 1] = ((c - 100 * u - 208 * v + 128) >> 8).clamp(0, 255) as u8;
            rgba[i + 2] = ((c + 516 * u + 128) >> 8).clamp(0, 255) as u8;
            rgba[i + 3] = 255;
        }
    }
    DecodedFrame {
        width: w as u32,
        height: h as u32,
        rgba,
    }
}

#[cfg(test)]
mod testes_decodificacao {
    use super::*;

    fn quadro(w: u32, h: u32, cor: [u8; 3]) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for p in v.chunks_mut(4) {
            p[0] = cor[0];
            p[1] = cor[1];
            p[2] = cor[2];
            p[3] = 255;
        }
        v
    }

    #[test]
    fn a_imagem_volta_depois_de_comprimida() {
        let (w, h) = (160u32, 120u32);
        let mut enc = VideoEncoder::new(w, h, Quality::P720, Fps::Fps30).unwrap();
        let mut dec = VideoDecoder::new().expect("decodificador não preparou");

        let original = quadro(w, h, [200, 60, 40]);
        let pacotes = enc.encode(&original, true).unwrap();
        assert!(!pacotes.is_empty());

        let mut recebido = None;
        for p in &pacotes {
            if let Some(img) = dec.decode(&p.data).unwrap() {
                recebido = Some(img);
            }
        }
        let img = recebido.expect("nenhuma imagem saiu do decodificador");
        assert_eq!((img.width, img.height), (w, h), "tamanho mudou no caminho");
        assert_eq!(img.rgba.len(), (w * h * 4) as usize);

        // O VP8 tem perdas, então a cor chega parecida, não idêntica.
        let (r, g, b) = (img.rgba[0] as i32, img.rgba[1] as i32, img.rgba[2] as i32);
        assert!(
            (r - 200).abs() < 25 && (g - 60).abs() < 25 && (b - 40).abs() < 25,
            "cor muito diferente: esperava perto de (200,60,40), veio ({r},{g},{b})"
        );
        assert_eq!(img.rgba[3], 255, "deveria vir opaco");
    }

    #[test]
    fn sequencia_de_quadros_continua_decodificando() {
        let (w, h) = (160u32, 120u32);
        let mut enc = VideoEncoder::new(w, h, Quality::P720, Fps::Fps30).unwrap();
        let mut dec = VideoDecoder::new().unwrap();

        let mut imagens = 0;
        for i in 0..10u8 {
            let cor = [30 + i * 20, 80, 150];
            for p in enc.encode(&quadro(w, h, cor), i == 0).unwrap() {
                if dec.decode(&p.data).unwrap().is_some() {
                    imagens += 1;
                }
            }
        }
        assert!(imagens >= 8, "só {imagens} de 10 quadros saíram do decodificador");
    }

    #[test]
    fn pacote_corrompido_nao_derruba_o_decodificador() {
        let mut dec = VideoDecoder::new().unwrap();
        // Lixo: tem que virar erro tratado, não pânico.
        let r = dec.decode(&[0xff, 0x00, 0x13, 0x37]);
        assert!(r.is_err() || r.unwrap().is_none());

        // E o decodificador continua utilizável depois.
        let mut enc = VideoEncoder::new(64, 64, Quality::P720, Fps::Fps30).unwrap();
        let bom = enc.encode(&quadro(64, 64, [10, 220, 90]), true).unwrap();
        let mut ok = false;
        for p in &bom {
            if dec.decode(&p.data).unwrap_or(None).is_some() {
                ok = true;
            }
        }
        assert!(ok, "o decodificador não se recuperou depois do pacote ruim");
    }
}
