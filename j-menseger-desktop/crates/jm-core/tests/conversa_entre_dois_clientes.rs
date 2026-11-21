//! Duas contas conversando pelo motor completo, como dois computadores reais.
//!
//! Aqui já não é só o protocolo: passa pelo motor inteiro — banco, contatos,
//! lista de conversas, confirmação de leitura e avisos para a interface.

use jm_core::engine::{ContactStatus, Engine, EngineEvent, Message, MessageBody};
use jm_core::identity;
use jm_core::messages::Delivery;
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

/// Monta um cliente novo, com conta criada na hora e banco em memória.
async fn cliente(
    nome: &str,
    relay: &str,
) -> (Arc<Engine>, UnboundedReceiver<EngineEvent>, String) {
    let store = Store::in_memory().unwrap();
    let palavras = identity::new_words().unwrap();
    let keys = Engine::create_identity(&store, &palavras, nome).unwrap();
    let npub = identity::npub(&keys.public_key());

    // Aponta para o relay de teste antes de ligar.
    let mut s = jm_core::engine::Settings::default();
    s.relays = vec![relay.to_string()];
    store.set_kv("settings", &s).unwrap();

    let (engine, rx) = Engine::open(keys, store).unwrap();
    engine.start().await.expect("não conectou ao servidor");
    (engine, rx, npub)
}

/// Espera por um aviso específico, ignorando os outros.
async fn esperar<T>(
    rx: &mut UnboundedReceiver<EngineEvent>,
    segundos: u64,
    mut filtro: impl FnMut(&EngineEvent) -> Option<T>,
) -> Option<T> {
    timeout(Duration::from_secs(segundos), async {
        while let Some(ev) = rx.recv().await {
            if let Some(v) = filtro(&ev) {
                return Some(v);
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
}

fn texto_recebido(ev: &EngineEvent) -> Option<Message> {
    match ev {
        EngineEvent::Message { message, .. }
            if !message.from_me && matches!(message.body, MessageBody::Text(_)) =>
        {
            Some((**message).clone())
        }
        _ => None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ana_e_beto_conversam_de_verdade() {
    let Some(relay) = TestRelay::start(7821) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let (ana, mut ev_ana, npub_ana) = cliente("Ana", &relay.url).await;
    let (beto, mut ev_beto, npub_beto) = cliente("Beto", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Ana adiciona Beto pelo código dele e manda a primeira mensagem.
    let contato = ana.add_contact(&npub_beto).await.expect("não adicionou");
    assert_eq!(contato.status, ContactStatus::Ok);

    let chat_ana = format!("dm:{}", beto.public_key().to_hex());
    let enviada = ana
        .send_text(&chat_ana, "Oi Beto! Mandando do computador.")
        .await
        .expect("não enviou");
    // Como no celular: com um servidor aceitando, a mensagem já sai como enviada.
    assert_eq!(enviada.status, Some(Delivery::Sent));

    // Beto recebe.
    let recebida = esperar(&mut ev_beto, 12, texto_recebido)
        .await
        .expect("a mensagem não chegou em Beto");
    assert_eq!(
        recebida.body,
        MessageBody::Text("Oi Beto! Mandando do computador.".into())
    );
    assert_eq!(recebida.author, ana.public_key().to_hex());

    // Para Beto, Ana entra como pedido de conversa, não como contato aceito.
    let ana_para_beto = beto
        .contact(&ana.public_key().to_hex())
        .expect("Ana deveria ter virado um contato em Beto");
    assert_eq!(ana_para_beto.status, ContactStatus::Request);

    // O perfil de Ana chega junto, então Beto já vê o nome dela.
    let viu_nome = esperar(&mut ev_beto, 8, |e| match e {
        EngineEvent::ContactChanged { pk } if *pk == ana.public_key().to_hex() => Some(()),
        _ => None,
    })
    .await;
    if viu_nome.is_some() {
        assert_eq!(beto.name_of(&ana.public_key().to_hex()), "Ana");
    }

    // A conversa aparece na lista dos dois.
    let lista_beto = beto.chat_list().unwrap();
    assert_eq!(lista_beto.len(), 1);
    assert_eq!(lista_beto[0].unread, 1, "deveria ter 1 não lida");
    assert!(lista_beto[0].last_text.contains("Mandando do computador"));
    assert!(!lista_beto[0].last_from_me);

    let lista_ana = ana.chat_list().unwrap();
    assert_eq!(lista_ana.len(), 1);
    assert!(lista_ana[0].last_from_me);

    // Ana ainda é um pedido de conversa para Beto: a chegada não gera recibo
    // de entrega automático (a mesma sequência do teste do celular).
    let recibo_cedo = esperar(&mut ev_ana, 3, |e| match e {
        EngineEvent::MessageUpdated { id, status, .. } if *id == enviada.id && *status >= Delivery::Delivered => Some(()),
        _ => None,
    })
    .await;
    assert!(recibo_cedo.is_none(), "pedido de conversa não manda recibo de entrega sozinho");

    // Beto aceita: Ana vê "entregue".
    beto.accept_contact(&ana.public_key().to_hex()).await.unwrap();
    let entregue = esperar(&mut ev_ana, 12, |e| match e {
        EngineEvent::MessageUpdated { id, status, .. } if *id == enviada.id && *status >= Delivery::Delivered => Some(()),
        _ => None,
    })
    .await;
    assert!(entregue.is_some(), "Ana não viu a entrega depois do aceite");

    // Beto abre a conversa: zera o não lido e confirma a leitura.
    let chat_beto = format!("dm:{}", ana.public_key().to_hex());
    beto.set_active(Some(&chat_beto)).await.unwrap();
    assert_eq!(beto.chat_list().unwrap()[0].unread, 0);

    // Ana vê o visto de leitura.
    let lida = esperar(&mut ev_ana, 12, |e| match e {
        EngineEvent::MessageUpdated { id, status, .. }
            if *id == enviada.id && *status == Delivery::Read =>
        {
            Some(*status)
        }
        _ => None,
    })
    .await;
    assert_eq!(lida, Some(Delivery::Read), "Ana não recebeu a confirmação de leitura");

    // Beto responde e Ana recebe.
    beto.send_text(&chat_beto, "Chegou aqui! 👋").await.unwrap();
    let resposta = esperar(&mut ev_ana, 12, texto_recebido)
        .await
        .expect("a resposta não chegou em Ana");
    assert_eq!(resposta.body, MessageBody::Text("Chegou aqui! 👋".into()));

    // O histórico está guardado dos dois lados, na ordem certa.
    let msgs_ana = ana.messages(&chat_ana, i64::MAX, 50).unwrap();
    assert_eq!(msgs_ana.len(), 2);
    assert!(msgs_ana[0].from_me, "a primeira é a que Ana mandou");
    assert!(!msgs_ana[1].from_me);
    assert!(msgs_ana[0].ts <= msgs_ana[1].ts);

    assert!(!npub_ana.is_empty());
    ana.shutdown().await;
    beto.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn quem_esta_bloqueado_nao_entrega_nada() {
    let Some(relay) = TestRelay::start(7822) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let (ana, _ev_ana, _) = cliente("Ana", &relay.url).await;
    let (beto, mut ev_beto, npub_beto) = cliente("Beto", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(600)).await;

    ana.add_contact(&npub_beto).await.unwrap();
    let chat = format!("dm:{}", beto.public_key().to_hex());

    // Beto bloqueia Ana antes de ela falar.
    beto.set_contact_status(&ana.public_key().to_hex(), ContactStatus::Blocked)
        .ok();
    // Garante que o contato existe para poder ficar bloqueado.
    let _ = beto.contact(&ana.public_key().to_hex());
    beto.set_contact_status(&ana.public_key().to_hex(), ContactStatus::Blocked)
        .unwrap();

    ana.send_text(&chat, "deixa eu entrar").await.unwrap();

    let chegou = esperar(&mut ev_beto, 5, texto_recebido).await;
    if beto.contact(&ana.public_key().to_hex()).map(|c| c.status) == Some(ContactStatus::Blocked) {
        assert!(chegou.is_none(), "mensagem de bloqueado não pode chegar");
    }
    ana.shutdown().await;
    beto.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_conta_restaurada_recupera_a_conversa() {
    // O mesmo que trocar de computador: entrar com as 12 palavras e ver o
    // histórico voltar dos servidores.
    let Some(relay) = TestRelay::start(7823) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let store = Store::in_memory().unwrap();
    let palavras = identity::new_words().unwrap();
    let keys_ana = Engine::create_identity(&store, &palavras, "Ana").unwrap();
    let mut s = jm_core::engine::Settings::default();
    s.relays = vec![relay.url.clone()];
    store.set_kv("settings", &s).unwrap();
    let (ana, _ev, _) = {
        let (e, rx) = Engine::open(keys_ana.clone(), store).unwrap();
        e.start().await.unwrap();
        (e, rx, ())
    };

    let (beto, _ev_beto, npub_beto) = cliente("Beto", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(600)).await;

    ana.add_contact(&npub_beto).await.unwrap();
    let chat = format!("dm:{}", beto.public_key().to_hex());
    ana.send_text(&chat, "esta mensagem precisa voltar").await.unwrap();
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // Em outro "computador": banco vazio, mesmas palavras.
    let store2 = Store::in_memory().unwrap();
    let keys2 = Engine::restore_identity(&store2, &palavras, "Ana").unwrap();
    assert_eq!(keys2.public_key(), keys_ana.public_key(), "mesma conta");
    let mut s2 = jm_core::engine::Settings::default();
    s2.relays = vec![relay.url.clone()];
    store2.set_kv("settings", &s2).unwrap();
    let (ana2, mut ev2) = Engine::open(keys2, store2).unwrap();
    ana2.start().await.unwrap();

    // A cópia que Ana guardou para si mesma traz a conversa de volta.
    let voltou = esperar(&mut ev2, 15, |e| match e {
        EngineEvent::Message { message, .. } => match &message.body {
            MessageBody::Text(t) if t.contains("precisa voltar") => Some(t.clone()),
            _ => None,
        },
        _ => None,
    })
    .await;
    assert_eq!(
        voltou.as_deref(),
        Some("esta mensagem precisa voltar"),
        "o histórico não voltou no aparelho novo"
    );

    ana.shutdown().await;
    ana2.shutdown().await;
    beto.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn adicionar_contato_ja_abre_a_conversa() {
    // Sem isto a pessoa adiciona alguém pelo código e fica sem onde escrever:
    // a conversa só nascia quando a primeira mensagem chegava.
    let Some(relay) = TestRelay::start(7824) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let (ana, _ev, _) = cliente("Ana", &relay.url).await;
    let (beto, _ev_b, npub_beto) = cliente("Beto", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(ana.chat_list().unwrap().is_empty(), "começa sem conversas");

    ana.add_contact(&npub_beto).await.unwrap();

    let lista = ana.chat_list().unwrap();
    assert_eq!(lista.len(), 1, "a conversa deveria aparecer logo após adicionar");
    assert_eq!(lista[0].key, format!("dm:{}", beto.public_key().to_hex()));
    assert_eq!(lista[0].unread, 0);
    assert!(lista[0].last_text.is_empty(), "ainda sem mensagens");

    // E dá para escrever nela na hora.
    ana.send_text(&lista[0].key, "primeira").await.unwrap();
    assert_eq!(ana.messages(&lista[0].key, i64::MAX, 10).unwrap().len(), 1);

    ana.shutdown().await;
    beto.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn os_dois_lados_passam_a_ver_o_nome_um_do_outro() {
    // Sem a troca de perfis a conversa fica com um código npub no lugar do nome.
    let Some(relay) = TestRelay::start(7825) else {
        return;
    };
    tokio::time::sleep(Duration::from_millis(900)).await;

    let (ana, mut ev_ana, _) = cliente("Ana Souza", &relay.url).await;
    let (beto, mut ev_beto, npub_beto) = cliente("Beto Lima", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(600)).await;

    let pk_ana = ana.public_key().to_hex();
    let pk_beto = beto.public_key().to_hex();

    ana.add_contact(&npub_beto).await.unwrap();
    ana.send_text(&format!("dm:{pk_beto}"), "oi").await.unwrap();

    // Beto recebe e passa a ver "Ana Souza".
    esperar(&mut ev_beto, 12, texto_recebido).await.expect("não chegou");
    assert!(
        ate_que(12, || beto.name_of(&pk_ana) == "Ana Souza").await,
        "Beto continua vendo o código no lugar do nome de Ana: {}",
        beto.name_of(&pk_ana)
    );

    // Ana pede o perfil de Beto ao receber a resposta dele.
    beto.send_text(&format!("dm:{pk_ana}"), "oi de volta").await.unwrap();
    esperar(&mut ev_ana, 12, texto_recebido).await.expect("resposta não chegou");
    assert!(
        ate_que(12, || ana.name_of(&pk_beto) == "Beto Lima").await,
        "Ana continua vendo o código no lugar do nome de Beto: {}",
        ana.name_of(&pk_beto)
    );

    ana.shutdown().await;
    beto.shutdown().await;
}

/// Tenta a condição por alguns segundos antes de desistir.
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
