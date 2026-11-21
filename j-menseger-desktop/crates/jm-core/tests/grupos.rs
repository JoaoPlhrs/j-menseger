//! Grupos no estilo servidor, com servidor de verdade no meio.
//!
//! Prova o que importa: a chave do grupo viaja no convite, quem aceita passa a
//! ler o canal, e quem não foi convidado não lê nada — porque não tem a chave.

use jm_core::engine::{Engine, EngineEvent, MessageBody};
use jm_core::groups::{ChannelKind, Template};
use jm_core::identity;
use jm_core::store::Store;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::timeout;

struct TestRelay {
    child: Child,
    url: String,
}

impl TestRelay {
    fn start(port: u16) -> Option<Self> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()?
            .parent()?
            .join("tools");
        if !dir.join("node_modules").exists() {
            eprintln!("pulando: rode `npm install` em tools/");
            return None;
        }
        let child = Command::new("node")
            .arg("testrelay.mjs")
            .arg(port.to_string())
            .current_dir(&dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        Some(Self { child, url: format!("ws://127.0.0.1:{port}") })
    }
}

impl Drop for TestRelay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn cliente(nome: &str, relay: &str) -> (Arc<Engine>, UnboundedReceiver<EngineEvent>, String) {
    let store = Store::in_memory().unwrap();
    let palavras = identity::new_words().unwrap();
    let keys = Engine::create_identity(&store, &palavras, nome).unwrap();
    let npub = identity::npub(&keys.public_key());
    let mut s = jm_core::engine::Settings::default();
    s.relays = vec![relay.to_string()];
    store.set_kv("settings", &s).unwrap();
    let (engine, rx) = Engine::open(keys, store).unwrap();
    engine.start().await.unwrap();
    (engine, rx, npub)
}

async fn ate_que(segundos: u64, mut cond: impl FnMut() -> bool) -> bool {
    let limite = std::time::Instant::now() + Duration::from_secs(segundos);
    while std::time::Instant::now() < limite {
        if cond() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

async fn esperar_texto(
    rx: &mut UnboundedReceiver<EngineEvent>,
    segundos: u64,
    trecho: &str,
) -> bool {
    timeout(Duration::from_secs(segundos), async {
        while let Some(ev) = rx.recv().await {
            if let EngineEvent::Message { message, .. } = ev {
                if let MessageBody::Text(t) = &message.body {
                    if t.contains(trecho) {
                        return true;
                    }
                }
            }
        }
        false
    })
    .await
    .unwrap_or(false)
}

#[tokio::test(flavor = "multi_thread")]
async fn grupo_com_canais_funciona_de_ponta_a_ponta() {
    let Some(relay) = TestRelay::start(7831) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let (ana, _ev_ana, _) = cliente("Ana", &relay.url).await;
    let (beto, mut ev_beto, npub_beto) = cliente("Beto", &relay.url).await;
    let (caio, mut ev_caio, _) = cliente("Caio", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Ana cria o grupo.
    let g = ana.create_group("Squad Beta", Template::Amigos).await.unwrap();
    assert_eq!(g.meta.text_channels().count(), 2);
    assert_eq!(g.meta.voice_channels().count(), 1);
    assert!(g.meta.is_admin(&ana.public_key().to_hex()));

    // Os canais de texto já aparecem na lista de conversas dela.
    let conversas = ana.chat_list().unwrap();
    assert_eq!(conversas.len(), 2, "um por canal de texto");
    assert!(conversas.iter().all(|c| c.is_group));
    assert!(
        conversas.iter().any(|c| c.title.contains("Squad Beta") && c.title.contains("#geral")),
        "títulos: {:?}",
        conversas.iter().map(|c| &c.title).collect::<Vec<_>>()
    );

    // Convida Beto.
    ana.add_contact(&npub_beto).await.unwrap();
    // Convite só vale de contato aceito (regra do celular).
    beto.accept_contact(&ana.public_key().to_hex()).await.unwrap();
    ana.invite_to_group(&g.lid, &beto.public_key().to_hex()).await.unwrap();

    // Beto recebe o convite como mensagem, mas ainda não participa.
    let recebeu = timeout(Duration::from_secs(12), async {
        while let Some(ev) = ev_beto.recv().await {
            if let EngineEvent::Message { message, .. } = ev {
                if let MessageBody::Invite { lid, name, .. } = &message.body {
                    return Some((lid.clone(), name.clone()));
                }
            }
        }
        None
    })
    .await
    .ok()
    .flatten();
    let (lid, nome) = recebeu.expect("o convite não chegou");
    assert_eq!(lid, g.lid);
    assert_eq!(nome, "Squad Beta");
    assert!(
        beto.chat_list().unwrap().iter().all(|c| !c.is_group),
        "não deve haver canal antes de aceitar"
    );

    // Beto aceita e os canais aparecem.
    let gb = beto.accept_invite(&lid).await.expect("não aceitou");
    assert_eq!(gb.pk, g.pk, "mesma chave de grupo");
    assert!(beto.chat_list().unwrap().iter().filter(|c| c.is_group).count() == 2);

    // Conversa no canal, nos dois sentidos.
    let canal = g.main_channel().unwrap().id.clone();
    let chave_canal = format!("g:{}:{}", g.lid, canal);
    ana.send_text(&chave_canal, "Bem-vindo ao grupo, Beto!").await.unwrap();
    assert!(
        esperar_texto(&mut ev_beto, 12, "Bem-vindo ao grupo").await,
        "a mensagem do canal não chegou em Beto"
    );

    beto.send_text(&chave_canal, "Valeu, Ana!").await.unwrap();
    let viu = ate_que(12, || {
        ana.messages(&chave_canal, i64::MAX, 50)
            .unwrap()
            .iter()
            .any(|m| matches!(&m.body, MessageBody::Text(t) if t.contains("Valeu")))
    })
    .await;
    assert!(viu, "a resposta de Beto não chegou em Ana");

    // Ana passa a ver Beto como membro, pelo aviso de entrada.
    assert!(
        ate_que(10, || {
            ana.group(&g.lid)
                .is_some_and(|x| x.meta.members.contains(&beto.public_key().to_hex()))
        })
        .await,
        "Beto não entrou na lista de membros de Ana"
    );

    // Caio não foi convidado: não tem a chave, então não lê nada.
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert!(caio.groups().is_empty(), "Caio não deveria ter o grupo");
    assert!(
        caio.messages(&chave_canal, i64::MAX, 50).unwrap().is_empty(),
        "quem não tem a chave não pode ler o canal"
    );
    let nada = timeout(Duration::from_secs(2), ev_caio.recv()).await;
    assert!(
        nada.is_err() || !matches!(nada, Ok(Some(EngineEvent::Message { .. }))),
        "Caio recebeu algo do grupo"
    );

    ana.shutdown().await;
    beto.shutdown().await;
    caio.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sair_do_grupo_apaga_os_canais_deste_aparelho() {
    let Some(relay) = TestRelay::start(7832) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let (ana, _e1, _) = cliente("Ana", &relay.url).await;
    let (beto, mut ev_beto, npub_beto) = cliente("Beto", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let g = ana.create_group("Temporário", Template::Branco).await.unwrap();
    ana.add_contact(&npub_beto).await.unwrap();
    // Convite só vale de contato aceito (regra do celular).
    beto.accept_contact(&ana.public_key().to_hex()).await.unwrap();
    ana.invite_to_group(&g.lid, &beto.public_key().to_hex()).await.unwrap();

    let chegou = timeout(Duration::from_secs(12), async {
        while let Some(ev) = ev_beto.recv().await {
            if let EngineEvent::Message { message, .. } = ev {
                if let MessageBody::Invite { lid, .. } = &message.body {
                    return Some(lid.clone());
                }
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
    .expect("convite não chegou");

    beto.accept_invite(&chegou).await.unwrap();
    let canal = g.main_channel().unwrap().id.clone();
    let chave = format!("g:{}:{}", g.lid, canal);
    ana.send_text(&chave, "oi grupo").await.unwrap();
    assert!(esperar_texto(&mut ev_beto, 12, "oi grupo").await);
    assert!(!beto.messages(&chave, i64::MAX, 10).unwrap().is_empty());

    // Beto sai.
    beto.leave_group(&g.lid).await.unwrap();
    assert!(beto.groups().is_empty());
    assert!(
        beto.messages(&chave, i64::MAX, 10).unwrap().is_empty(),
        "as mensagens do grupo deveriam sair deste aparelho"
    );
    assert!(beto.chat_list().unwrap().iter().all(|c| !c.is_group));

    // Ana fica sabendo que ele saiu.
    assert!(
        ate_que(12, || {
            ana.group(&g.lid)
                .is_some_and(|x| !x.meta.members.contains(&beto.public_key().to_hex()))
        })
        .await,
        "Ana continua achando que Beto está no grupo"
    );

    ana.shutdown().await;
    beto.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn so_admin_muda_os_dados_do_grupo() {
    let Some(relay) = TestRelay::start(7833) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let (ana, _e1, _) = cliente("Ana", &relay.url).await;
    let (beto, mut ev_beto, npub_beto) = cliente("Beto", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let g = ana.create_group("Regras", Template::Branco).await.unwrap();
    ana.add_contact(&npub_beto).await.unwrap();
    // Convite só vale de contato aceito (regra do celular).
    beto.accept_contact(&ana.public_key().to_hex()).await.unwrap();
    ana.invite_to_group(&g.lid, &beto.public_key().to_hex()).await.unwrap();
    let lid = timeout(Duration::from_secs(12), async {
        while let Some(ev) = ev_beto.recv().await {
            if let EngineEvent::Message { message, .. } = ev {
                if let MessageBody::Invite { lid, .. } = &message.body {
                    return Some(lid.clone());
                }
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
    .expect("convite não chegou");
    beto.accept_invite(&lid).await.unwrap();

    // Beto é membro comum.
    let gb = beto.group(&lid).unwrap();
    assert!(!gb.meta.is_admin(&beto.public_key().to_hex()));
    assert!(gb.meta.is_admin(&ana.public_key().to_hex()));

    // O grupo nasce com um canal de texto e um de voz.
    assert_eq!(gb.meta.channels.iter().filter(|c| c.kind == ChannelKind::Voice).count(), 1);

    ana.shutdown().await;
    beto.shutdown().await;
}
