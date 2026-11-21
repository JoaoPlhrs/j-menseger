//! Backup cifrado e a mesma conta em dois aparelhos, com servidor de verdade.
//!
//! - O backup de uma conta, restaurado numa instalação nova (entrando com as
//!   mesmas palavras), traz contatos, grupos com as chaves e as mensagens.
//! - Outra conta não abre o arquivo.
//! - Um aparelho novo da mesma conta pede a sincronia e recebe contatos e
//!   grupos do aparelho que já estava ligado, sem arquivo nenhum.

use jm_core::engine::{ContactStatus, Engine, EngineEvent, MessageBody};
use jm_core::groups::Template;
use jm_core::identity;
use jm_core::store::Store;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

struct TestRelay {
    child: Child,
    url: String,
}

impl TestRelay {
    fn start(port: u16) -> Option<Self> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.parent()?.join("tools");
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

async fn abrir(store: Store, keys: nostr::prelude::Keys, relay: &str) -> (Arc<Engine>, UnboundedReceiver<EngineEvent>) {
    let mut s = jm_core::engine::Settings::default();
    s.relays = vec![relay.to_string()];
    store.set_kv("settings", &s).unwrap();
    let (engine, rx) = Engine::open(keys, store).unwrap();
    engine.start().await.unwrap();
    (engine, rx)
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

#[tokio::test(flavor = "multi_thread")]
async fn backup_restaura_e_aparelho_novo_sincroniza() {
    let Some(relay) = TestRelay::start(7843) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    // Ana (com as palavras guardadas) e Beto.
    let palavras = identity::new_words().unwrap();
    let st = Store::in_memory().unwrap();
    let keys = Engine::create_identity(&st, &palavras, "Ana").unwrap();
    let (ana, _ev) = abrir(st, keys, &relay.url).await;
    let st = Store::in_memory().unwrap();
    let kb = Engine::create_identity(&st, &identity::new_words().unwrap(), "Beto").unwrap();
    let beto_pk = kb.public_key().to_hex();
    let (beto, _evb) = abrir(st, kb, &relay.url).await;

    ana.add_contact(&identity::npub(&beto.public_key())).await.unwrap();
    let dm = format!("dm:{beto_pk}");
    let m = ana.send_text(&dm, "olá, isto vai pro backup").await.unwrap();
    ana.edit_message(&dm, &m.id, "olá, isto vai pro backup (editada)").await.unwrap();
    let g = ana.create_group("Turma do Backup", Template::Amigos).await.unwrap();
    let canal = format!("g:{}:{}", g.lid, g.meta.text_channels().next().unwrap().id);
    ana.send_poll(&canal, "Pizza ou hambúrguer?", &["Pizza".into(), "Hambúrguer".into()], false).await.unwrap();
    ana.pin_message(&dm, &m.id, true).await.unwrap();

    // Backup.
    let arquivo = ana.export_backup().unwrap();
    assert_eq!(&arquivo[..5], b"JMBK1");
    assert!(!String::from_utf8_lossy(&arquivo).contains("olá"), "o arquivo não pode ter texto aberto");

    // Outra conta não abre.
    assert!(beto.import_backup(&arquivo).await.is_err());

    // Instalação nova, entrando com as mesmas palavras.
    let st = Store::in_memory().unwrap();
    let k2 = Engine::restore_identity(&st, &palavras, "").unwrap();
    let (ana2, _ev2) = abrir(st, k2, &relay.url).await;
    let r = ana2.import_backup(&arquivo).await.expect("backup não abriu");
    assert!(r.contacts >= 1 && r.groups == 1 && r.messages >= 2, "restaurou pouco: {r:?}");
    assert_eq!(ana2.contact(&beto_pk).map(|c| c.status), Some(ContactStatus::Ok));
    assert_eq!(ana2.groups().len(), 1);
    assert_eq!(ana2.groups()[0].sk, g.sk, "a chave do grupo veio junto");
    let msgs = ana2.messages(&dm, i64::MAX, 50).unwrap();
    let texto = msgs.iter().find(|x| x.id == m.id).expect("mensagem não voltou");
    assert!(matches!(&texto.body, MessageBody::Text(t) if t.ends_with("(editada)")));
    assert!(texto.edited.is_some());
    assert_eq!(ana2.pins(&dm), vec![m.id.clone()]);
    assert!(ana2.messages(&canal, i64::MAX, 50).unwrap().iter().any(|x| matches!(&x.body, MessageBody::Poll { q, .. } if q.starts_with("Pizza"))));
    // Restaurar de novo não duplica nada.
    let r2 = ana2.import_backup(&arquivo).await.unwrap();
    assert_eq!((r2.contacts, r2.groups, r2.messages), (0, 0, 0));

    // Terceiro aparelho da Ana, sem arquivo: pede a sincronia e a Ana original responde.
    let st = Store::in_memory().unwrap();
    let k3 = Engine::restore_identity(&st, &palavras, "").unwrap();
    let (ana3, _ev3) = abrir(st, k3, &relay.url).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    ana3.sync_request().await;
    assert!(
        ate_que(20, || ana3.groups().len() == 1 && ana3.contact(&beto_pk).is_some()).await,
        "o aparelho novo não recebeu contatos e grupos"
    );
    assert_eq!(ana3.groups()[0].sk, g.sk);
}
