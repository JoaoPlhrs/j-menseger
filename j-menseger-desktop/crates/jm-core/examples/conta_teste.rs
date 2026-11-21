//! Ferramenta de teste: abre uma conta pela chave (hex ou nsec) num banco
//! novo e faz uma das coisas:
//!
//!   conta_teste <chave> abrir <arquivo.jmbackup>        mostra o que tem no backup
//!   conta_teste <chave> selar <arquivo.json> <saida>     cifra um JSON como backup
//!   conta_teste <chave> sincronia <relay> <segundos>     aparelho novo: pede a sincronia e espera
use jm_core::engine::{open_backup, seal_backup, Engine};
use jm_core::store::Store;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let a: Vec<String> = std::env::args().collect();
    let chave = &a[1];
    let keys = jm_core::identity::keys_from_any(chave).expect("chave inválida");
    let sk = keys.secret_key().secret_bytes();
    match a[2].as_str() {
        "abrir" => {
            let dados = std::fs::read(&a[3]).unwrap();
            let json = open_backup(&sk, &dados).expect("não abriu");
            let v: serde_json::Value = serde_json::from_slice(&json).unwrap();
            println!(
                "backup de {:?} ({}): {} contatos, {} grupos, {} conversas, {} mensagens; nome {:?}",
                v["by"],
                v["v"],
                v["contacts"].as_array().map(|x| x.len()).unwrap_or(0),
                v["groups"].as_array().map(|x| x.len()).unwrap_or(0),
                v["chats"].as_array().map(|x| x.len()).unwrap_or(0),
                v["msgs"].as_array().map(|x| x.len()).unwrap_or(0),
                v["me"]["name"]
            );
        }
        "selar" => {
            let json = std::fs::read(&a[3]).unwrap();
            std::fs::write(&a[4], seal_backup(&sk, &json).unwrap()).unwrap();
            println!("ok");
        }
        "sincronia" => {
            let store = Store::in_memory().unwrap();
            let keys = Engine::restore_identity(&store, chave, "").unwrap();
            let mut s = jm_core::engine::Settings::default();
            s.relays = vec![a[3].clone()];
            store.set_kv("settings", &s).unwrap();
            let (e, _rx) = Engine::open(keys, store).unwrap();
            e.start().await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            e.sync_request().await;
            println!("pedido de sincronia enviado; esperando…");
            let fim = std::time::Instant::now() + Duration::from_secs(a[4].parse().unwrap_or(30));
            while std::time::Instant::now() < fim {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if !e.groups().is_empty() && !e.contacts().is_empty() {
                    break;
                }
            }
            println!(
                "recebido: {} contatos ({}), {} grupos ({})",
                e.contacts().len(),
                e.contacts().iter().map(|c| c.display_name()).collect::<Vec<_>>().join(", "),
                e.groups().len(),
                e.groups().iter().map(|g| g.meta.name.clone()).collect::<Vec<_>>().join(", ")
            );
            // Fica mais um pouco ligado: responde ao pedido do celular, se vier.
            e.sync_push(true).await;
            tokio::time::sleep(Duration::from_secs(8)).await;
        }
        "aparelho" => {
            let store = Store::in_memory().unwrap();
            let keys = Engine::restore_identity(&store, chave, "").unwrap();
            let mut s = jm_core::engine::Settings::default();
            s.relays = vec![a[3].clone()];
            store.set_kv("settings", &s).unwrap();
            let (e, mut rx) = Engine::open(keys, store).unwrap();
            e.start().await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            e.announce_device("PC de teste", "Windows").await;
            println!("aparelho anunciado; esperando ordem de sair…");
            let fim = tokio::time::Instant::now() + Duration::from_secs(a[4].parse().unwrap_or(40));
            loop {
                match tokio::time::timeout_at(fim, rx.recv()).await {
                    Ok(Some(jm_core::engine::EngineEvent::RemoteLogout)) => {
                        println!("RECEBIDO: ordem de sair (o app apagaria a conta aqui)");
                        break;
                    }
                    Ok(Some(_)) => {}
                    _ => {
                        println!("tempo esgotado sem ordem de sair");
                        break;
                    }
                }
            }
        }
        _ => eprintln!("modo desconhecido"),
    }
}
