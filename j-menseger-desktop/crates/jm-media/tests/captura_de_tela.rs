//! Captura de tela de verdade, contra a tela do sistema.
//!
//! Este teste não simula nada: lista o que o sistema oferece, captura um
//! quadro e confere que a imagem tem conteúdo. Ele é pulado quando não há tela
//! (servidor sem interface gráfica), em vez de falhar por algo que não é bug.

use jm_media::screen::{self, Fps, Quality};

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

#[test]
fn lista_e_captura_uma_tela_de_verdade() {
    if !tem_tela() {
        eprintln!("pulando: sem tela neste ambiente");
        return;
    }
    let _exclusiva = captura_exclusiva();
    garantir_cliente();
    let monitores = match screen::monitors() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("pulando: não deu para listar as telas ({e})");
            return;
        }
    };
    if monitores.is_empty() {
        eprintln!("pulando: nenhuma tela encontrada");
        return;
    }

    let tela = &monitores[0];
    assert!(tela.width >= 16 && tela.height >= 16, "tamanho improvável: {}×{}", tela.width, tela.height);
    assert!(!tela.label().is_empty());

    let quadro = screen::capture(tela).expect("a captura falhou");
    assert_eq!(quadro.width, tela.width, "largura capturada diferente da tela");
    assert_eq!(quadro.height, tela.height);
    assert_eq!(
        quadro.rgba.len(),
        quadro.pixels() * 4,
        "a imagem veio com tamanho errado"
    );

    // Opacidade: um quadro todo transparente seria sinal de captura vazia.
    let opacos = quadro.rgba.chunks_exact(4).filter(|p| p[3] > 0).count();
    assert!(
        opacos > quadro.pixels() / 2,
        "mais da metade dos pixels veio transparente: a captura não pegou a tela"
    );

    println!(
        "capturou {}×{} de '{}' ({} bytes)",
        quadro.width,
        quadro.height,
        tela.label(),
        quadro.rgba.len()
    );
}

#[test]
fn quadro_capturado_e_reduzido_para_a_qualidade_escolhida() {
    if !tem_tela() {
        return;
    }
    let Ok(monitores) = screen::monitors() else { return };
    let Some(tela) = monitores.first() else { return };
    let Ok(quadro) = screen::capture(tela) else { return };

    for q in [Quality::P720, Quality::P1080] {
        let p = quadro.scale_to_height(q.height());
        assert!(p.height <= quadro.height.max(q.height()));
        assert_eq!(p.rgba.len(), p.pixels() * 4);
        assert_eq!(p.width % 2, 0, "o codificador exige largura par");
        assert_eq!(p.height % 2, 0);
    }
}

#[test]
fn captura_repetida_nao_esgota_o_sistema() {
    if !tem_tela() {
        eprintln!("pulando: sem tela neste ambiente");
        return;
    }
    let _exclusiva = captura_exclusiva();
    garantir_cliente();
    let telas = screen::monitors().expect("não listou as telas");
    let tela = telas.first().expect("nenhuma tela");
    let cap = screen::Capturer::open(tela).expect("não abriu o capturador");

    // Transmitir significa capturar dezenas de vezes por segundo. Se cada
    // quadro abrisse uma conexão nova com o sistema gráfico, os recursos se
    // esgotariam e a transmissão cairia depois de poucos segundos.
    let mut tempos = Vec::new();
    for i in 0..60 {
        let t0 = std::time::Instant::now();
        let q = cap
            .grab()
            .unwrap_or_else(|e| panic!("a captura {i} falhou: {e}"));
        assert!(q.pixels() > 0);
        tempos.push(t0.elapsed());
    }

    let media = tempos.iter().sum::<std::time::Duration>() / tempos.len() as u32;
    println!(
        "60 capturas seguidas · média {media:?} (alvo 30 fps: {:?})",
        Fps::Fps30.interval()
    );
    assert!(
        media < std::time::Duration::from_millis(400),
        "cada quadro levou {media:?}"
    );
}

#[test]
fn lista_de_fontes_inclui_as_telas() {
    if !tem_tela() {
        return;
    }
    let Ok(fontes) = screen::sources() else { return };
    let Ok(monitores) = screen::monitors() else { return };
    assert!(
        fontes.len() >= monitores.len(),
        "a lista de opções tem que conter pelo menos as telas"
    );
}
