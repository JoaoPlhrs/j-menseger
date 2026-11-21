//! Dois clientes trocando mensagens de verdade através de um servidor.
//!
//! O teste sobe o relay de teste (`tools/testrelay.mjs`), que imita as regras
//! dos servidores públicos, e faz dois clientes conversarem por ele. Nada aqui
//! é simulado: os envelopes são cifrados, publicados por WebSocket, entregues
//! pelo servidor e abertos do outro lado.

use jm_core::protocol::{self, WrapOptions};
use jm_core::relays::RelayPool;
use nostr::prelude::*;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

/// Sobe um relay local e o derruba quando sair de escopo.
struct TestRelay {
    child: Child,
    url: String,
}

impl TestRelay {
    fn start(port: u16, auth: bool) -> Option<Self> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()?
            .parent()?
            .join("tools");
        if !root.join("node_modules").exists() {
            eprintln!("pulando: rode `npm install` em tools/ para ter o relay de teste");
            return None;
        }
        let mut cmd = Command::new("node");
        cmd.arg("testrelay.mjs").arg(port.to_string());
        if auth {
            cmd.arg("--auth");
        }
        let child = cmd
            .current_dir(&root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        Some(Self {
            child,
            url: format!("ws://127.0.0.1:{port}"),
        })
    }
}

impl Drop for TestRelay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn conectado(pool: &RelayPool, url: &str) -> bool {
    pool.set_relays(&[url.to_string()]).await.is_ok()
        && pool.wait_connected(Duration::from_secs(8)).await
}

#[tokio::test(flavor = "multi_thread")]
async fn ana_manda_e_beto_recebe_pelo_servidor() {
    let Some(relay) = TestRelay::start(7811, false) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let ana = Keys::generate();
    let beto = Keys::generate();

    let pool_ana = RelayPool::new(ana.clone());
    let pool_beto = RelayPool::new(beto.clone());
    assert!(conectado(&pool_ana, &relay.url).await, "Ana não conectou");
    assert!(conectado(&pool_beto, &relay.url).await, "Beto não conectou");

    // Beto fica ouvindo o que for endereçado a ele.
    let mut caixa = pool_beto.listen();
    pool_beto
        .subscribe_inbox(&[beto.public_key()], Timestamp::now() - 3600u64)
        .await
        .expect("Beto não conseguiu assinar a caixa de entrada");
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Ana envia.
    let texto = "Oi Beto, isto passou por um servidor de verdade 🚀";
    let rumor = protocol::make_rumor(
        &ana,
        protocol::KIND_TEXT,
        texto,
        vec![Tag::public_key(beto.public_key())],
    );
    let envelope =
        protocol::wrap_rumor(&rumor, &ana, &beto.public_key(), WrapOptions::default()).unwrap();
    let aceitos = pool_ana.publish(&envelope).await.expect("servidor recusou");
    assert_eq!(aceitos, 1, "o servidor deveria ter confirmado");

    // Beto recebe e abre.
    let recebido = timeout(Duration::from_secs(10), caixa.recv())
        .await
        .expect("a mensagem não chegou em 10 s")
        .expect("o canal fechou");

    let aberto = protocol::unwrap_event(&recebido.event, &beto).expect("não abriu o envelope");
    assert_eq!(aberto.content, texto);
    assert_eq!(aberto.pubkey, ana.public_key(), "autoria preservada");
    assert_eq!(aberto.id, rumor.id, "é a mesma mensagem");

    // O servidor não tem como saber quem enviou.
    assert_ne!(recebido.event.pubkey, ana.public_key());
    assert!(!recebido.event.content.contains("Beto"));
}

#[tokio::test(flavor = "multi_thread")]
async fn evento_efemero_nao_fica_guardado() {
    let Some(relay) = TestRelay::start(7812, false) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let ana = Keys::generate();
    let beto = Keys::generate();
    let pool_ana = RelayPool::new(ana.clone());
    assert!(conectado(&pool_ana, &relay.url).await);

    // "digitando…" vai como efêmero: o servidor repassa, mas não guarda.
    let rumor = protocol::make_rumor(&ana, protocol::KIND_APP, r#"{"t":"typ"}"#, vec![]);
    let eph = protocol::wrap_rumor(
        &rumor,
        &ana,
        &beto.public_key(),
        WrapOptions { ephemeral: true, expiration: None },
    )
    .unwrap();
    assert!(pool_ana.publish(&eph).await.is_ok(), "servidor recusou o efêmero");

    // Beto entra depois. Como o efêmero não foi guardado, ele não recebe nada.
    let pool_beto = RelayPool::new(beto.clone());
    assert!(conectado(&pool_beto, &relay.url).await);
    let mut caixa = pool_beto.listen();
    pool_beto
        .subscribe_inbox(&[beto.public_key()], Timestamp::now() - 3600u64)
        .await
        .unwrap();

    let nada = timeout(Duration::from_secs(3), caixa.recv()).await;
    assert!(nada.is_err(), "o efêmero não deveria ter sido guardado");
}

#[tokio::test(flavor = "multi_thread")]
async fn servidor_que_exige_autenticacao_funciona() {
    let Some(relay) = TestRelay::start(7813, true) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let ana = Keys::generate();
    let beto = Keys::generate();
    let pool_ana = RelayPool::new(ana.clone());
    let pool_beto = RelayPool::new(beto.clone());
    assert!(conectado(&pool_ana, &relay.url).await);
    assert!(conectado(&pool_beto, &relay.url).await);

    let mut caixa = pool_beto.listen();
    pool_beto
        .subscribe_inbox(&[beto.public_key()], Timestamp::now() - 3600u64)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(800)).await;

    let rumor = protocol::make_rumor(&ana, protocol::KIND_TEXT, "com autenticação", vec![]);
    let env =
        protocol::wrap_rumor(&rumor, &ana, &beto.public_key(), WrapOptions::default()).unwrap();
    pool_ana
        .publish(&env)
        .await
        .expect("o servidor com AUTH recusou a publicação");

    let recebido = timeout(Duration::from_secs(10), caixa.recv())
        .await
        .expect("não chegou no servidor com AUTH")
        .unwrap();
    let aberto = protocol::unwrap_event(&recebido.event, &beto).unwrap();
    assert_eq!(aberto.content, "com autenticação");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_mesma_mensagem_por_dois_servidores_chega_uma_vez_so() {
    // Duas cópias do mesmo envelope, em servidores diferentes, não podem virar
    // duas mensagens na conversa. Quem garante isso é o registro de já vistos.
    let Some(r1) = TestRelay::start(7814, false) else {
        return;
    };
    let Some(r2) = TestRelay::start(7815, false) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let ana = Keys::generate();
    let beto = Keys::generate();
    let pool_ana = RelayPool::new(ana.clone());
    pool_ana
        .set_relays(&[r1.url.clone(), r2.url.clone()])
        .await
        .unwrap();
    assert!(pool_ana.wait_connected(Duration::from_secs(8)).await);
    assert_eq!(pool_ana.open_count().await, 2, "deveria falar com os dois");

    let pool_beto = RelayPool::new(beto.clone());
    pool_beto
        .set_relays(&[r1.url.clone(), r2.url.clone()])
        .await
        .unwrap();
    assert!(pool_beto.wait_connected(Duration::from_secs(8)).await);
    let mut caixa = pool_beto.listen();
    pool_beto
        .subscribe_inbox(&[beto.public_key()], Timestamp::now() - 3600u64)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let rumor = protocol::make_rumor(&ana, protocol::KIND_TEXT, "em dobro", vec![]);
    let env =
        protocol::wrap_rumor(&rumor, &ana, &beto.public_key(), WrapOptions::default()).unwrap();
    let n = pool_ana.publish(&env).await.unwrap();
    assert_eq!(n, 2, "foi para os dois servidores");

    let store = jm_core::store::Store::in_memory().unwrap();
    let mut novas = 0;
    // Junta o que chegar em 4 segundos e conta só o que é inédito.
    let coleta = tokio::time::timeout(Duration::from_secs(4), async {
        while let Some(inc) = caixa.recv().await {
            if store.mark_seen(&inc.event.id.to_hex(), 0).unwrap() {
                novas += 1;
            }
        }
    })
    .await;
    assert!(coleta.is_err(), "o canal deveria seguir aberto");
    assert_eq!(novas, 1, "a mesma mensagem não pode aparecer duas vezes");
}
