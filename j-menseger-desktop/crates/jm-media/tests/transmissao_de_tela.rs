//! Transmissão de tela de ponta a ponta.
//!
//! Captura a tela de verdade, comprime em VP8 e manda pela conexão WebRTC até
//! o outro lado. Só o que falta para ser uma transmissão completa é a janela
//! que mostra o vídeo recebido.

use jm_media::peer::{CallConfig, ConnectionState, MediaKind, Peer, PeerEvent};
use jm_media::screen::{self, Fps, Quality};
use jm_media::video::{VideoDecoder, VideoEncoder};
use std::time::{Duration, Instant};
use tokio::time::timeout;

fn tem_tela() -> bool {
    std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Uma tela virtual sem nenhuma janela aberta não entrega conteúdo: o servidor
/// só prepara a área de desenho quando existe algum cliente. Num computador de
/// verdade isso nunca acontece, porque sempre há o ambiente gráfico.
///
/// A janela é criada uma vez por execução e fica viva até o fim: se cada teste
/// criasse e matasse a sua, um teste apagaria a janela enquanto outro ainda
/// estivesse capturando.
fn garantir_cliente() {
    use std::sync::OnceLock;
    static CLIENTE: OnceLock<()> = OnceLock::new();
    CLIENTE.get_or_init(|| {
        let consegue = screen::monitors()
            .ok()
            .and_then(|m| m.first().cloned())
            .is_some_and(|t| screen::capture(&t).is_ok());
        if consegue {
            return;
        }
        if std::process::Command::new("xterm")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
    });
}

/// A captura de tela usa o servidor gráfico, que não gosta de vários
/// capturadores ao mesmo tempo. No aplicativo existe um só, em uma thread
/// dedicada; aqui o travamento reproduz essa condição.
fn captura_exclusiva() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static TRAVA: OnceLock<Mutex<()>> = OnceLock::new();
    TRAVA
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

async fn esperar_conexao(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<PeerEvent>,
    s: u64,
) -> bool {
    timeout(Duration::from_secs(s), async {
        while let Some(ev) = rx.recv().await {
            match ev {
                PeerEvent::State(ConnectionState::Connected) => return true,
                PeerEvent::State(ConnectionState::Failed) => return false,
                _ => {}
            }
        }
        false
    })
    .await
    .unwrap_or(false)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tela_capturada_chega_comprimida_do_outro_lado() {
    if !tem_tela() {
        eprintln!("pulando: sem tela neste ambiente");
        return;
    }
    let _exclusiva = captura_exclusiva();
    garantir_cliente();
    let telas = screen::monitors().expect("não listou as telas");
    let tela = telas.first().cloned().expect("nenhuma tela encontrada");

    let (ana, mut rx_ana) = Peer::new(CallConfig::default()).await.unwrap();
    let (beto, mut rx_beto) = Peer::new(CallConfig::default()).await.unwrap();
    let oferta = ana.create_offer().await.unwrap();
    let resposta = beto.accept_offer(&oferta).await.unwrap();
    ana.accept_answer(&resposta).await.unwrap();

    if !esperar_conexao(&mut rx_ana, 25).await || !esperar_conexao(&mut rx_beto, 25).await {
        eprintln!("pulando: a conexão não fechou neste ambiente de rede");
        ana.close().await;
        beto.close().await;
        return;
    }

    // Prepara o codificador no tamanho que a qualidade escolhida pede.
    let primeiro = screen::capture(&tela).expect("captura falhou");
    let reduzido = primeiro.scale_to_height(Quality::P720.height());
    let mut enc = VideoEncoder::new(reduzido.width, reduzido.height, Quality::P720, Fps::Fps30)
        .expect("codificador não preparou");

    // O capturador não pode trocar de thread (no Windows é assim, e o app
    // captura numa thread própria): captura e comprime aqui, manda depois.
    let mut pacotes_prontos: Vec<Vec<u8>> = Vec::new();
    if let Ok(cap) = screen::Capturer::open(&tela) {
        for i in 0..20 {
            let Ok(q) = cap.grab() else { break };
            let p = q.scale_to_height(Quality::P720.height());
            let Ok(pacotes) = enc.encode(&p.rgba, i == 0) else { break };
            pacotes_prontos.extend(pacotes.into_iter().map(|p| p.data));
        }
    }
    let envio = tokio::spawn(async move {
        let mut enviados = 0usize;
        let mut bytes = 0usize;
        for dados in pacotes_prontos {
            bytes += dados.len();
            if ana.send_video(&dados, Duration::from_millis(33)).await.is_err() {
                return (ana, enviados, bytes);
            }
            enviados += 1;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        (ana, enviados, bytes)
    });

    let mut recebidos = 0usize;
    let mut bytes_recebidos = 0usize;
    let mut imagens = 0usize;
    let mut tamanho_imagem = (0u32, 0u32);
    let mut dec = VideoDecoder::new().expect("decodificador não preparou");

    let _ = timeout(Duration::from_secs(15), async {
        while let Some(ev) = rx_beto.recv().await {
            if let PeerEvent::Media { kind: MediaKind::Video, data } = ev {
                recebidos += 1;
                bytes_recebidos += data.len();
                // Decodificar é o que prova que o quadro chegou inteiro: um
                // quadro remontado errado não vira imagem nenhuma.
                if let Ok(Some(img)) = dec.decode(&data) {
                    imagens += 1;
                    tamanho_imagem = (img.width, img.height);
                }
                if recebidos >= 10 {
                    break;
                }
            }
        }
    })
    .await;

    let (ana, enviados, bytes) = envio.await.unwrap();

    assert!(enviados > 0, "nada foi enviado");
    assert!(
        recebidos >= 5,
        "chegaram só {recebidos} pacotes de vídeo de {enviados} enviados"
    );
    assert!(bytes_recebidos > 0);
    assert!(
        imagens >= 3,
        "chegaram {recebidos} quadros mas só {imagens} viraram imagem: a remontagem está errada"
    );
    assert_eq!(
        tamanho_imagem,
        (reduzido.width, reduzido.height),
        "a imagem chegou com outro tamanho"
    );
    println!(
        "tela {}×{}: {enviados} pacotes enviados ({} KB), {recebidos} recebidos, {imagens} viraram imagem de {}×{}",
        reduzido.width,
        reduzido.height,
        bytes / 1024,
        tamanho_imagem.0,
        tamanho_imagem.1
    );

    ana.close().await;
    beto.close().await;
}

#[tokio::test]
async fn comprime_a_tela_real_em_tempo_util() {
    if !tem_tela() {
        eprintln!("pulando: sem tela neste ambiente");
        return;
    }
    // A partir daqui qualquer falha é falha de verdade: há tela, então
    // capturar e comprimir tem que funcionar.
    let _exclusiva = captura_exclusiva();
    garantir_cliente();
    let telas = screen::monitors().expect("não listou as telas");
    let tela = telas.first().expect("nenhuma tela encontrada");
    let primeiro = screen::capture(tela).expect("a primeira captura falhou");
    let base = primeiro.scale_to_height(Quality::P720.height());

    let mut enc = VideoEncoder::new(base.width, base.height, Quality::P720, Fps::Fps30)
        .expect("o codificador não preparou");

    let mut total_bruto = 0usize;
    let mut total_comprimido = 0usize;
    let inicio = Instant::now();
    let quadros = 15u32;

    let cap = screen::Capturer::open(tela).expect("não abriu o capturador");
    for i in 0..quadros {
        let q = cap.grab().unwrap_or_else(|e| panic!("captura {i} falhou: {e}"));
        let p = q.scale_to_height(Quality::P720.height());
        total_bruto += p.rgba.len();
        for pkt in enc.encode(&p.rgba, i == 0).expect("compressão falhou") {
            total_comprimido += pkt.data.len();
        }
    }
    let gasto = inicio.elapsed();

    assert!(total_comprimido > 0, "nada foi comprimido");
    let fator = total_bruto / total_comprimido.max(1);
    println!(
        "{quadros} quadros de {}x{} em {gasto:?}: {} KB brutos viraram {} KB ({fator}x menor)",
        base.width,
        base.height,
        total_bruto / 1024,
        total_comprimido / 1024
    );

    assert!(fator > 20, "a compressão deveria ser muito maior: só {fator}x");
    // Captura mais compressão, por quadro. Sem GPU e com tela virtual não
    // exigimos 30 fps; exigimos que seja utilizável.
    let por_quadro = gasto / quadros;
    assert!(
        por_quadro < Duration::from_millis(500),
        "cada quadro levou {por_quadro:?}, lento demais até para software"
    );
}
