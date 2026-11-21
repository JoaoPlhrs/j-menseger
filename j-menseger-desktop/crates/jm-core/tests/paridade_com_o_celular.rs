//! Cada recurso do aplicativo de celular, funcionando entre dois (ou três)
//! clientes de verdade, pelo servidor local.
//!
//! As regras conferidas aqui são as do `engine.js` do celular (versão 0.4):
//! quem pode reagir, apagar, convidar; quando um recibo sai; como a mídia é
//! fatiada e conferida; como um grupo troca de chave ao remover alguém.

use jm_core::engine::{Engine, EngineEvent, MediaMeta, MessageBody};
use jm_core::groups::{ChannelKind, GroupPatch, Template};
use jm_core::identity;
use jm_core::messages::{MediaKind, SoundMeta};
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

type Cliente = (Arc<Engine>, UnboundedReceiver<EngineEvent>);

async fn cliente(nome: &str, relay: &str) -> Cliente {
    let store = Store::in_memory().unwrap();
    let palavras = identity::new_words().unwrap();
    let keys = Engine::create_identity(&store, &palavras, nome).unwrap();
    let mut s = jm_core::engine::Settings::default();
    s.relays = vec![relay.to_string()];
    store.set_kv("settings", &s).unwrap();
    let (engine, rx) = Engine::open(keys, store).unwrap();
    engine.start().await.expect("não conectou ao servidor");
    (engine, rx)
}

/// Dois clientes que já são contatos aceitos um do outro.
async fn dupla(relay: &TestRelay) -> (Cliente, Cliente) {
    tokio::time::sleep(Duration::from_millis(900)).await;
    let a = cliente("Ana", &relay.url).await;
    let b = cliente("Beto", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    a.0.add_contact(&b.0.npub()).await.unwrap();
    b.0.add_contact(&a.0.npub()).await.unwrap();
    (a, b)
}

fn dm(e: &Engine) -> String {
    format!("dm:{}", e.pk_hex())
}

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

/// Aceita o convite assim que ele chegar.
async fn entrar(quem: &Arc<Engine>, lid: &str) {
    for _ in 0..60 {
        if quem.accept_invite(lid).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("o convite não chegou");
}

async fn ate_que(segundos: u64, mut cond: impl FnMut() -> bool) -> bool {
    let fim = std::time::Instant::now() + Duration::from_secs(segundos);
    while std::time::Instant::now() < fim {
        if cond() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    cond()
}

/* ---------------------------------------------------------------- */

#[test]
fn numero_de_seguranca_igual_ao_do_celular() {
    #[derive(serde::Deserialize)]
    struct V {
        a: String,
        b: String,
        n: Vec<String>,
    }
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/seguranca-do-mobile.json");
    let vetores: Vec<V> = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    assert!(vetores.len() >= 20);
    for v in vetores {
        let n = jm_core::safety::safety_number(&v.a, &v.b).unwrap();
        assert_eq!(n.to_vec(), v.n, "número diferente do celular para {} / {}", v.a, v.b);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn reacao_resposta_e_apagar_para_todos() {
    let Some(relay) = TestRelay::start(7841) else { return };
    let ((ana, mut ev_a), (beto, mut ev_b)) = dupla(&relay).await;

    let m = ana.send_text(&dm(&beto), "Vamos jogar hoje?").await.unwrap();
    let chegou = esperar(&mut ev_b, 12, |e| match e {
        EngineEvent::Message { message, .. } if message.id == m.id => Some(()),
        _ => None,
    })
    .await;
    assert!(chegou.is_some(), "a mensagem não chegou");

    // Reação: aparece do outro lado com o emoji certo.
    beto.send_reaction(&dm(&ana), &m.id, "👍").await.unwrap();
    let reagiu = ate_que(12, || {
        ana.message(&m.id).is_some_and(|x| x.reacts.get(&beto.pk_hex()).map(|s| s.as_str()) == Some("👍"))
    })
    .await;
    assert!(reagiu, "a reação não chegou");

    // Reagir de novo com o mesmo emoji tira a reação (vai como "-").
    beto.send_reaction(&dm(&ana), &m.id, "👍").await.unwrap();
    let tirou = ate_que(12, || ana.message(&m.id).is_some_and(|x| x.reacts.is_empty())).await;
    assert!(tirou, "a reação não foi retirada");

    // Resposta: a mensagem de Beto aponta para a de Ana.
    let r = beto.send_text_with(&dm(&ana), "Bora!", Some(&m.id)).await.unwrap();
    let resposta = esperar(&mut ev_a, 12, |e| match e {
        EngineEvent::Message { message, .. } if message.id == r.id => Some(message.reply.clone()),
        _ => None,
    })
    .await;
    assert_eq!(resposta, Some(Some(m.id.clone())), "a resposta não veio ligada à mensagem original");

    // Beto tenta apagar a mensagem de Ana: não pode.
    beto.delete_for_all(&dm(&ana), &[m.id.clone()]).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(
        matches!(ana.message(&m.id).unwrap().body, MessageBody::Text(_)),
        "ninguém apaga para todos a mensagem de outra pessoa"
    );
    assert!(matches!(beto.message(&m.id).unwrap().body, MessageBody::Text(_)));

    // Ana apaga a própria: some dos dois lados, deixando o aviso.
    ana.delete_for_all(&dm(&beto), &[m.id.clone()]).await.unwrap();
    assert_eq!(ana.message(&m.id).unwrap().body, MessageBody::Deleted);
    let apagou = ate_que(12, || beto.message(&m.id).is_some_and(|x| x.body == MessageBody::Deleted)).await;
    assert!(apagou, "Beto ainda vê a mensagem apagada");

    // Apagar para mim tira só deste aparelho.
    beto.delete_for_me(&dm(&ana), &[r.id.clone()]).unwrap();
    assert!(beto.message(&r.id).is_none());
    assert!(ana.message(&r.id).is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn mensagens_temporarias_somem_nos_dois_lados() {
    let Some(relay) = TestRelay::start(7842) else { return };
    let ((ana, _ev_a), (beto, mut ev_b)) = dupla(&relay).await;

    ana.set_ttl(&dm(&beto), 3600).await.unwrap();
    let avisou = esperar(&mut ev_b, 12, |e| match e {
        EngineEvent::Message { message, .. } => match &message.body {
            MessageBody::System { sys, v, .. } if sys == "ttl" => Some(*v),
            _ => None,
        },
        _ => None,
    })
    .await;
    assert_eq!(avisou, Some(3600), "o aviso de mensagens temporárias não chegou");
    assert_eq!(beto.chat_ttl(&dm(&ana)), 3600, "o prazo não foi aplicado à conversa do outro lado");

    let m = ana.send_text(&dm(&beto), "isto some em 1 hora").await.unwrap();
    let exp = esperar(&mut ev_b, 12, |e| match e {
        EngineEvent::Message { message, .. } if message.id == m.id => Some(message.exp),
        _ => None,
    })
    .await
    .flatten()
    .expect("a mensagem chegou sem prazo");
    let falta = exp - jm_core::engine::now_ms();
    assert!((3_500_000..=3_610_000).contains(&falta), "prazo errado: {falta} ms");

    // Prazo curto, para ver a limpeza acontecer.
    ana.set_ttl(&dm(&beto), 2).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let curta = ana.send_text(&dm(&beto), "relâmpago").await.unwrap();
    assert!(ate_que(10, || beto.message(&curta.id).is_some()).await, "a mensagem curta não chegou");
    tokio::time::sleep(Duration::from_secs(3)).await;
    beto.purge_expired();
    ana.purge_expired();
    assert!(beto.message(&curta.id).is_none(), "a mensagem vencida continua no aparelho de Beto");
    assert!(ana.message(&curta.id).is_none(), "a mensagem vencida continua no aparelho de Ana");
}

#[tokio::test(flavor = "multi_thread")]
async fn foto_chega_inteira_e_conferida() {
    let Some(relay) = TestRelay::start(7843) else { return };
    let ((ana, _ev_a), (beto, mut ev_b)) = dupla(&relay).await;

    // 55 KB: três blocos de até 20 000 bytes.
    let bytes: Vec<u8> = (0..55_000u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8).collect();
    let meta = MediaMeta {
        kind: Some(MediaKind::Image),
        mime: "image/jpeg".into(),
        w: Some(800),
        h: Some(600),
        th: Some("data:image/jpeg;base64,/9j/AA==".into()),
        ..Default::default()
    };
    let m = ana.send_media(&dm(&beto), bytes.clone(), meta).await.unwrap();

    let mid = esperar(&mut ev_b, 15, |e| match e {
        EngineEvent::Message { message, .. } if message.id == m.id => match &message.body {
            MessageBody::Media(md) => Some(md.mid.clone()),
            _ => None,
        },
        _ => None,
    })
    .await
    .expect("o cabeçalho da foto não chegou");

    let pronta = ate_que(20, || match beto.message(&m.id).map(|x| x.body) {
        Some(MessageBody::Media(md)) => md.ready,
        _ => false,
    })
    .await;
    assert!(pronta, "a foto não terminou de chegar");
    assert_eq!(beto.media_bytes(&mid).unwrap(), bytes, "a foto chegou diferente");
    match beto.message(&m.id).unwrap().body {
        MessageBody::Media(md) => {
            assert_eq!(md.kind, MediaKind::Image);
            assert_eq!((md.w, md.h), (Some(800), Some(600)));
            assert!(md.th.is_some(), "a miniatura não veio no cabeçalho");
            assert!(!md.err);
        }
        outro => panic!("virou outra coisa: {outro:?}"),
    }
    assert_eq!(beto.chat_list().unwrap()[0].last_text, "Foto");
}

#[tokio::test(flavor = "multi_thread")]
async fn som_proprio_e_som_de_fabrica() {
    let Some(relay) = TestRelay::start(7844) else { return };
    let ((ana, _ev_a), (beto, mut ev_b)) = dupla(&relay).await;

    // Som de fábrica: vai só o nome.
    let s = ana.send_sound(&dm(&beto), "aplausos").await.unwrap();
    let veio = esperar(&mut ev_b, 12, |e| match e {
        EngineEvent::Message { message, .. } if message.id == s.id => Some(message.body.clone()),
        _ => None,
    })
    .await;
    assert_eq!(veio, Some(MessageBody::Sound { id: "aplausos".into() }));

    // Som próprio: vai o áudio, com nome e emoji.
    let audio: Vec<u8> = b"OggS".iter().copied().chain((0..3000u32).map(|i| (i % 251) as u8)).collect();
    let meta = MediaMeta {
        kind: Some(MediaKind::Audio),
        mime: "audio/ogg".into(),
        dur: Some(2.0),
        sb: Some(SoundMeta { n: "Buzina do Zé".into(), e: "📯".into() }),
        ..Default::default()
    };
    let m = ana.send_media(&dm(&beto), audio.clone(), meta).await.unwrap();
    let pronto = ate_que(15, || match beto.message(&m.id).map(|x| x.body) {
        Some(MessageBody::Media(md)) => md.ready && md.sb.as_ref().is_some_and(|sb| sb.n == "Buzina do Zé" && sb.e == "📯"),
        _ => false,
    })
    .await;
    assert!(pronto, "o som próprio não chegou com nome e emoji");
    assert_eq!(beto.chat_list().unwrap()[0].last_text, "Som: Buzina do Zé");

    // Beto guarda no próprio soundboard: o id vem dos bytes, igual ao do celular.
    let rec = beto.save_custom_sound(&audio, "audio/ogg", "Buzina do Zé", "📯", 2.0).unwrap();
    assert!(jm_core::media::is_custom_sound_id(&rec.id));
    assert_eq!(rec.id, jm_core::media::sound_id(&audio));
    assert_eq!(beto.custom_sounds().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn foto_de_perfil_e_chamada_perdida() {
    let Some(relay) = TestRelay::start(7845) else { return };
    let ((ana, _ev_a), (beto, mut ev_b)) = dupla(&relay).await;

    let foto = "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQ==";
    ana.update_profile(Some("Ana Souza"), Some("no trabalho"), Some(3), Some(foto)).await.unwrap();
    let chegou = ate_que(12, || {
        beto.contact(&ana.pk_hex())
            .is_some_and(|c| c.pic.as_deref() == Some(foto) && c.name == "Ana Souza" && c.note == "no trabalho" && c.tone == 3)
    })
    .await;
    assert!(chegou, "a foto e os dados novos do perfil não chegaram");
    assert_eq!(beto.pic_of(&ana.pk_hex()).as_deref(), Some(foto));

    // Tirar a foto também chega.
    ana.update_profile(None, None, None, Some("")).await.unwrap();
    assert!(ate_que(12, || beto.contact(&ana.pk_hex()).is_some_and(|c| c.pic.is_none())).await);

    // Chamada perdida no formato do celular, sem identificador de chamada.
    ana.report_missed_call(&beto.pk_hex(), true).await.unwrap();
    let perdida = esperar(&mut ev_b, 12, |e| match e {
        EngineEvent::MissedCall { from, video, .. } if *from == ana.pk_hex() => Some(*video),
        _ => None,
    })
    .await;
    assert_eq!(perdida, Some(true));
    let aviso = beto
        .messages(&dm(&ana), i64::MAX, 20)
        .unwrap()
        .into_iter()
        .find(|m| matches!(&m.body, MessageBody::System { sys, .. } if sys == "missed"))
        .expect("a chamada perdida não ficou no histórico");
    assert_eq!(beto.system_text(&aviso), "Chamada de vídeo perdida");
}

#[tokio::test(flavor = "multi_thread")]
async fn convite_de_estranho_e_ignorado() {
    let Some(relay) = TestRelay::start(7846) else { return };
    tokio::time::sleep(Duration::from_millis(900)).await;
    let (ana, _ev_a) = cliente("Ana", &relay.url).await;
    let (beto, mut ev_b) = cliente("Beto", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Ana conhece Beto, mas Beto nunca aceitou Ana.
    ana.add_contact(&beto.npub()).await.unwrap();
    let g = ana.create_group("Grupo suspeito", Template::Branco).await.unwrap();
    ana.invite_to_group(&g.lid, &beto.pk_hex()).await.unwrap();
    let convite = esperar(&mut ev_b, 5, |e| match e {
        EngineEvent::Message { message, .. } if matches!(message.body, MessageBody::Invite { .. }) => Some(()),
        _ => None,
    })
    .await;
    assert!(convite.is_none(), "convite de quem não é contato aceito não pode aparecer");
}

#[tokio::test(flavor = "multi_thread")]
async fn grupo_editado_canais_e_foto_chegam_aos_membros() {
    let Some(relay) = TestRelay::start(7847) else { return };
    let ((ana, _ev_a), (beto, _ev_b)) = dupla(&relay).await;

    let g = ana.create_group("Turma", Template::Amigos).await.unwrap();
    ana.invite_to_group(&g.lid, &beto.pk_hex()).await.unwrap();
    entrar(&beto, &g.lid).await;
    // Ana (admin) vê Beto entrar e o põe na lista de membros.
    assert!(ate_que(12, || ana.group(&g.lid).is_some_and(|x| x.meta.members.contains(&beto.pk_hex()))).await);

    ana.edit_group(
        &g.lid,
        &[
            GroupPatch::Name("Turma da Faculdade".into()),
            GroupPatch::AddChannel { name: "memes".into(), kind: ChannelKind::Text },
            GroupPatch::EditChannel { id: "geral".into(), name: None, topic: Some("assuntos gerais".into()) },
        ],
    )
    .await
    .unwrap();
    let mudou = ate_que(12, || {
        beto.group(&g.lid).is_some_and(|x| {
            x.meta.name == "Turma da Faculdade"
                && x.meta.channels.iter().any(|c| c.id == "memes")
                && x.meta.channels.iter().any(|c| c.id == "geral" && c.topic.as_deref() == Some("assuntos gerais"))
        })
    })
    .await;
    assert!(mudou, "as mudanças do grupo não chegaram a Beto");
    assert!(beto.chat_list().unwrap().iter().any(|c| c.key == format!("g:{}:memes", g.lid)), "o canal novo não apareceu");

    // Apagar um canal some com ele (e com as mensagens) dos dois lados.
    ana.send_text(&format!("g:{}:fotos", g.lid), "foto da festa").await.unwrap();
    assert!(ate_que(12, || !beto.messages(&format!("g:{}:fotos", g.lid), i64::MAX, 5).unwrap().is_empty()).await);
    ana.edit_group(&g.lid, &[GroupPatch::RemoveChannel("fotos".into())]).await.unwrap();
    let sumiu = ate_que(12, || {
        beto.group(&g.lid).is_some_and(|x| !x.meta.channels.iter().any(|c| c.id == "fotos"))
            && beto.messages(&format!("g:{}:fotos", g.lid), i64::MAX, 5).unwrap().is_empty()
    })
    .await;
    assert!(sumiu, "o canal apagado continua em Beto");

    // Foto do grupo.
    let foto = "data:image/webp;base64,UklGRg==";
    ana.set_group_pic(&g.lid, foto).await.unwrap();
    assert!(ate_que(12, || beto.group(&g.lid).is_some_and(|x| x.pic.as_deref() == Some(foto))).await, "a foto do grupo não chegou");

    // Beto não é admin: não muda nada.
    assert!(beto.edit_group(&g.lid, &[GroupPatch::Name("Hackeado".into())]).await.is_err());
    assert!(beto.set_group_pic(&g.lid, "").await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn remover_membro_troca_a_chave_do_grupo() {
    let Some(relay) = TestRelay::start(7848) else { return };
    tokio::time::sleep(Duration::from_millis(900)).await;
    let (ana, _ev_a) = cliente("Ana", &relay.url).await;
    let (beto, _ev_b) = cliente("Beto", &relay.url).await;
    let (caio, _ev_c) = cliente("Caio", &relay.url).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    for outro in [&beto, &caio] {
        ana.add_contact(&outro.npub()).await.unwrap();
        outro.add_contact(&ana.npub()).await.unwrap();
    }
    beto.add_contact(&caio.npub()).await.unwrap();
    caio.add_contact(&beto.npub()).await.unwrap();

    let g = ana.create_group("Projeto", Template::Branco).await.unwrap();
    for outro in [&beto, &caio] {
        ana.invite_to_group(&g.lid, &outro.pk_hex()).await.unwrap();
        entrar(outro, &g.lid).await;
    }
    assert!(ate_que(12, || ana.group(&g.lid).is_some_and(|x| x.meta.members.len() == 3)).await);
    let canal = format!("g:{}:geral", g.lid);

    ana.remove_member(&g.lid, &caio.pk_hex()).await.unwrap();
    let nova = ana.group(&g.lid).unwrap().pk;
    assert_ne!(nova, g.pk, "a chave do grupo não mudou");
    assert!(ate_que(12, || beto.group(&g.lid).is_some_and(|x| x.pk == nova)).await, "Beto não recebeu a chave nova");
    assert_eq!(caio.group(&g.lid).unwrap().pk, g.pk, "Caio não pode receber a chave nova");

    let m = ana.send_text(&canal, "só para quem ficou").await.unwrap();
    assert!(ate_que(12, || beto.message(&m.id).is_some()).await, "quem ficou deveria ler");
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(caio.message(&m.id).is_none(), "quem foi removido leu uma mensagem nova");

    // O histórico antigo continua legível para quem ficou (chave antiga guardada).
    assert!(beto.group(&g.lid).unwrap().old_keys.iter().any(|(pk, _)| *pk == g.pk));
}

#[tokio::test(flavor = "multi_thread")]
async fn presenca_na_sala_de_voz() {
    let Some(relay) = TestRelay::start(7849) else { return };
    let ((ana, _ev_a), (beto, mut ev_b)) = dupla(&relay).await;
    let g = ana.create_group("Jogatina", Template::Jogos).await.unwrap();
    ana.invite_to_group(&g.lid, &beto.pk_hex()).await.unwrap();
    entrar(&beto, &g.lid).await;
    let voz = g.meta.voice_channels().next().unwrap().id.clone();

    ana.room_presence(&g.lid, &voz, true).await.unwrap();
    let viu = esperar(&mut ev_b, 12, |e| match e {
        EngineEvent::Room { lid, ch, members } if *lid == g.lid && *ch == voz && members.contains(&ana.pk_hex()) => Some(()),
        _ => None,
    })
    .await;
    assert!(viu.is_some(), "Beto não viu Ana entrar na sala de voz");
    assert_eq!(beto.room_members(&g.lid, &voz), vec![ana.pk_hex()]);

    ana.room_presence(&g.lid, &voz, false).await.unwrap();
    let saiu = esperar(&mut ev_b, 12, |e| match e {
        EngineEvent::Room { lid, members, .. } if *lid == g.lid && members.is_empty() => Some(()),
        _ => None,
    })
    .await;
    assert!(saiu.is_some(), "Beto não viu Ana sair da sala");
}

/// Código de convite curto: Ana publica, Beto (que não a conhece) encontra a
/// chave dela pelo código e adiciona.
#[tokio::test(flavor = "multi_thread")]
async fn codigo_curto_encontra_a_pessoa() {
    let Some(relay) = TestRelay::start(7841) else { return };
    tokio::time::sleep(Duration::from_millis(900)).await;
    let (ana, _ra) = cliente("Ana", &relay.url).await;
    let (beto, _rb) = cliente("Beto", &relay.url).await;
    let (codigo, vence) = ana.create_short_code().await.expect("publicar código");
    assert_eq!(codigo.len(), 9);
    assert!(vence > jm_core::engine::now_secs() + 6 * 86_400);
    // Mesmo código ao pedir de novo (enquanto vale).
    assert_eq!(ana.create_short_code().await.unwrap().0, codigo);
    let digitado = codigo.to_lowercase().replace('-', " ");
    let pk = beto.resolve_short_code(&digitado).await.expect("achar código");
    assert_eq!(pk, ana.pk_hex());
    beto.add_contact(&pk).await.expect("adicionar pela chave achada");
    // Código que ninguém publicou.
    assert!(beto.resolve_short_code("ZZZZ-ZZZZ").await.is_err());
}

/// Banner e jogos favoritos chegam ao contato; tirar o banner também.
#[tokio::test(flavor = "multi_thread")]
async fn banner_e_jogos_favoritos_chegam() {
    let Some(relay) = TestRelay::start(7842) else { return };
    let ((ana, _ra), (beto, _rb)) = dupla(&relay).await;
    let banner = format!("data:image/jpeg;base64,{}", "A".repeat(4000));
    let jogos = vec!["Minecraft".to_string(), "Valorant".into(), "minecraft".into(), "  ".into()];
    ana.update_profile_extras(Some(&banner), Some(&jogos)).await.unwrap();
    assert_eq!(ana.me().games, vec!["Minecraft".to_string(), "Valorant".into()]);
    let pk = ana.pk_hex();
    assert!(
        ate_que(10, || beto.contact(&pk).is_some_and(|c| c.banner.as_deref() == Some(banner.as_str()) && c.games.len() == 2)).await,
        "banner e jogos não chegaram"
    );
    ana.update_profile_extras(Some(""), None).await.unwrap();
    assert!(ate_que(10, || beto.contact(&pk).is_some_and(|c| c.banner.is_none() && c.games.len() == 2)).await, "remoção do banner não chegou");
    // Banner grande demais é recusado antes de sair.
    assert!(ana.update_profile_extras(Some(&format!("data:image/jpeg;base64,{}", "A".repeat(30_000))), None).await.is_err());
}

/// "Ouvindo agora": chega ao contato, a mesma música não é reenviada à toa,
/// e parar de tocar some do outro lado.
#[tokio::test(flavor = "multi_thread")]
async fn ouvindo_agora_chega_ao_contato() {
    use jm_core::messages::Listening;
    let Some(relay) = TestRelay::start(7843) else { return };
    let ((ana, _ra), (beto, _rb)) = dupla(&relay).await;
    let pk = ana.pk_hex();
    let musica = Listening { s: "Evidências".into(), a: "Chitãozinho & Xororó".into(), st: jm_core::engine::now_ms(), d: Some(280), ..Default::default() };
    ana.set_listening(Some(musica.clone())).await;
    assert!(ate_que(10, || beto.listening_of(&pk).is_some_and(|l| l.s == "Evidências")).await, "música não chegou");
    assert_eq!(ana.my_listening().unwrap().a, "Chitãozinho & Xororó");
    ana.set_listening(None).await;
    assert!(ate_que(10, || beto.listening_of(&pk).is_none()).await, "parar não chegou");
}
