//! Retrato da conta: perfil, contatos, grupos (com as chaves), ajustes das
//! conversas e, no backup, as mensagens.
//!
//! O mesmo formato serve para duas coisas, no PC e no celular:
//!
//! - **Backup**: o retrato inteiro, compactado e cifrado num arquivo. A chave
//!   sai da chave da conta (HKDF), então só abre quem tem as palavras de
//!   recuperação — não existe senha extra para esquecer.
//! - **Mesma conta em dois aparelhos**: o retrato sem mensagens viaja da conta
//!   para ela mesma, cifrado como qualquer mensagem, e o outro aparelho junta
//!   o que não tinha.
//!
//! Fotos e áudios não entram no backup (ficariam enormes); fotos de perfil e
//! de grupo entram.

use super::*;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use std::io::{Read, Write};

/// Começo de todo arquivo de backup.
pub const MAGIC: &[u8; 5] = b"JMBK1";
const INFO: &[u8] = b"jmenseger-backup-v1";
/// Um backup não passa disto (descompactado), para um arquivo estranho não
/// estourar a memória.
const MAX_JSON: u64 = 256 * 1024 * 1024;

type HmacSha256 = Hmac<sha2::Sha256>;

/// HKDF-SHA256 com saída de 32 bytes (um bloco).
fn hkdf32(ikm: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let mut m = <HmacSha256 as Mac>::new_from_slice(salt).expect("hmac aceita qualquer chave");
    m.update(ikm);
    let prk = m.finalize().into_bytes();
    let mut m = <HmacSha256 as Mac>::new_from_slice(&prk).expect("hmac aceita qualquer chave");
    m.update(info);
    m.update(&[1u8]);
    m.finalize().into_bytes().into()
}

/// Cifra o JSON do backup com a chave da conta.
pub fn seal(sk: &[u8; 32], json: &[u8]) -> Result<Vec<u8>> {
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(json).map_err(|e| erro(e.to_string()))?;
    let compactado = gz.finish().map_err(|e| erro(e.to_string()))?;
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    rand::Rng::fill(&mut rand::rng(), &mut salt[..]);
    rand::Rng::fill(&mut rand::rng(), &mut nonce[..]);
    let chave = hkdf32(sk, &salt, INFO);
    let cifra = Aes256Gcm::new_from_slice(&chave).map_err(|e| erro(e.to_string()))?;
    let ct = cifra.encrypt(Nonce::from_slice(&nonce), compactado.as_ref()).map_err(|_| erro("falha ao cifrar"))?;
    let mut out = Vec::with_capacity(5 + 16 + 12 + ct.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Abre um backup. Erro quando o arquivo não é de backup ou é de outra conta.
pub fn open(sk: &[u8; 32], data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 5 + 16 + 12 + 16 || &data[..5] != MAGIC {
        return Err(erro("Este arquivo não é um backup do J-Menseger."));
    }
    let (salt, resto) = data[5..].split_at(16);
    let (nonce, ct) = resto.split_at(12);
    let chave = hkdf32(sk, salt, INFO);
    let cifra = Aes256Gcm::new_from_slice(&chave).map_err(|e| erro(e.to_string()))?;
    let compactado = cifra
        .decrypt(Nonce::from_slice(nonce), ct)
        .map_err(|_| erro("Este backup é de outra conta (ou o arquivo está corrompido)."))?;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(&compactado[..])
        .take(MAX_JSON)
        .read_to_end(&mut out)
        .map_err(|_| erro("Backup corrompido."))?;
    Ok(out)
}

/// O que um backup restaurado trouxe.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Restored {
    pub contacts: usize,
    pub groups: usize,
    pub messages: usize,
}

fn status_str(s: ContactStatus) -> &'static str {
    match s {
        ContactStatus::Ok => "ok",
        ContactStatus::Request => "request",
        ContactStatus::Blocked => "blocked",
    }
}

fn status_from(s: &str) -> Option<ContactStatus> {
    match s {
        "ok" => Some(ContactStatus::Ok),
        "request" => Some(ContactStatus::Request),
        "blocked" => Some(ContactStatus::Blocked),
        _ => None,
    }
}

/// Mensagem no formato neutro (o mesmo do celular). `None` para o que não
/// entra no backup (fotos, áudios, convites).
fn msg_to_json(m: &Message) -> Option<Value> {
    let mut v = json!({ "id": m.id, "chat": m.chat, "from": m.author, "ts": m.ts });
    match &m.body {
        MessageBody::Text(t) => {
            v["kind"] = "text".into();
            v["text"] = t.clone().into();
            if let Some(e) = m.edited {
                v["edited"] = e.into();
            }
        }
        MessageBody::System { sys, v: n, legacy } => {
            v["kind"] = "sys".into();
            v["sys"] = sys.clone().into();
            v["v"] = (*n).into();
            if let Some(t) = legacy {
                v["text"] = t.clone().into();
            }
        }
        MessageBody::Sound { id } => {
            v["kind"] = "sound".into();
            v["snd"] = id.clone().into();
        }
        MessageBody::Poll { q, o, multi, votes } => {
            v["kind"] = "poll".into();
            v["poll"] = json!({ "q": q, "o": o, "m": *multi as u8 });
            v["votes"] = json!(votes);
        }
        MessageBody::Event { title, at, desc, rsvp } => {
            v["kind"] = "event".into();
            v["evt"] = json!({ "ti": title, "at": at, "d": desc });
            v["rsvp"] = json!(rsvp);
        }
        MessageBody::Deleted => v["kind"] = "deleted".into(),
        MessageBody::Media(_) | MessageBody::Invite { .. } => return None,
    }
    if let Some(r) = &m.reply {
        v["reply"] = r.clone().into();
    }
    if !m.reacts.is_empty() {
        v["reacts"] = json!(m.reacts);
    }
    if let Some(s) = m.status {
        v["status"] = delivery_name(s).into();
    }
    if let Some(e) = m.exp {
        v["exp"] = e.into();
    }
    Some(v)
}

fn msg_from_json(v: &Value, eu: &str) -> Option<Message> {
    let id = v["id"].as_str().filter(|s| is_hex64(s))?.to_string();
    let chat = v["chat"].as_str().filter(|s| ChatKey::parse(s).is_some())?.to_string();
    let author = v["from"].as_str().filter(|s| is_hex64(s))?.to_string();
    let ts = v["ts"].as_i64()?;
    let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    let body = match v["kind"].as_str()? {
        "text" => MessageBody::Text(clamp_keep(&s("text"), 6000)),
        "sys" => MessageBody::System {
            sys: clamp(&s("sys"), 20),
            v: v["v"].as_i64().unwrap_or(0),
            legacy: v["text"].as_str().filter(|_| s("sys").is_empty()).map(|t| clamp(t, 200)),
        },
        "sound" => MessageBody::Sound { id: clamp(&s("snd"), 20) },
        "poll" => {
            let p = &v["poll"];
            let o: Vec<String> = serde_json::from_value::<Vec<String>>(p["o"].clone()).ok()?.iter().map(|x| clamp(x, 80)).take(10).collect();
            let votes: BTreeMap<String, Vec<u8>> = serde_json::from_value::<BTreeMap<String, Vec<i64>>>(v["votes"].clone())
                .unwrap_or_default()
                .into_iter()
                .filter(|(pk, _)| is_hex64(pk))
                .map(|(pk, xs)| (pk, xs.into_iter().filter(|x| *x >= 0 && (*x as usize) < o.len()).map(|x| x as u8).collect()))
                .collect();
            MessageBody::Poll { q: clamp(p["q"].as_str().unwrap_or_default(), 200), multi: p["m"].as_i64().unwrap_or(0) != 0, o, votes }
        }
        "event" => {
            let e = &v["evt"];
            let rsvp: BTreeMap<String, String> = serde_json::from_value::<BTreeMap<String, String>>(v["rsvp"].clone())
                .unwrap_or_default()
                .into_iter()
                .filter(|(pk, r)| is_hex64(pk) && matches!(r.as_str(), "y" | "m" | "n"))
                .collect();
            MessageBody::Event { title: clamp(e["ti"].as_str().unwrap_or_default(), 120), at: e["at"].as_i64().unwrap_or(0), desc: clamp(e["d"].as_str().unwrap_or_default(), 500), rsvp }
        }
        "deleted" => MessageBody::Deleted,
        _ => return None,
    };
    let reacts: BTreeMap<String, String> = serde_json::from_value::<BTreeMap<String, String>>(v["reacts"].clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|(pk, _)| is_hex64(pk))
        .map(|(pk, e)| (pk, clamp_keep(&e, 16)))
        .collect();
    Some(Message {
        fwd: false,
        lp: None,
        from_me: author == eu,
        status: v["status"].as_str().and_then(parse_delivery),
        reply: v["reply"].as_str().filter(|s| is_hex64(s)).map(|s| s.to_string()),
        exp: v["exp"].as_i64(),
        edited: v["edited"].as_i64(),
        id,
        chat,
        author,
        ts,
        body,
        reacts,
    })
}

impl Engine {
    /// Identifica este aparelho (para não responder a si mesmo na sincronia).
    pub fn device_id(&self) -> String {
        let store = self.store.lock().unwrap();
        if let Ok(Some(d)) = store.get_kv::<String>("device_id") {
            return d;
        }
        let mut b = [0u8; 8];
        rand::Rng::fill(&mut rand::rng(), &mut b[..]);
        let d: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let _ = store.set_kv("device_id", &d);
        d
    }

    /// Retrato da conta. Com `msgs`, leva as mensagens (backup); sem, é o que
    /// vai para o outro aparelho (aí também sem fotos, que passariam do
    /// tamanho de uma mensagem — chegam depois pelo perfil de cada um).
    pub fn snapshot(&self, msgs: bool) -> Result<Value> {
        let me = self.me();
        let mut eu = json!({ "name": me.name, "note": me.note, "t": me.tone, "games": me.games });
        if msgs {
            eu["pic"] = json!(me.pic);
            eu["banner"] = json!(me.banner);
        }
        let contatos: Vec<Value> = self
            .contacts()
            .into_iter()
            .map(|c| {
                let mut v = json!({
                    "pk": c.pk, "name": c.name, "nick": c.nick, "note": c.note, "t": c.tone,
                    "status": status_str(c.status), "verified": c.verified, "relays": c.relays, "games": c.games,
                    "addedAt": c.added_at,
                });
                if msgs {
                    v["pic"] = json!(c.pic);
                    v["banner"] = json!(c.banner);
                }
                v
            })
            .collect();
        let grupos: Vec<Value> = self
            .groups()
            .into_iter()
            .map(|g| {
                let mut v = json!({
                    "lid": g.lid, "sk": g.sk, "old": g.old_keys.iter().map(|(_, sk)| sk.clone()).collect::<Vec<_>>(),
                    "meta": g.meta, "ver": g.ver, "picV": g.pic_v, "mute": g.muted, "gone": g.gone,
                });
                if msgs {
                    v["pic"] = json!(g.pic);
                }
                v
            })
            .collect();
        let (conversas, mensagens) = {
            let store = self.store.lock().unwrap();
            let mut st = store.raw().prepare(&format!("SELECT {CHAT_COLS} FROM chats WHERE deleted = 0")).map_err(crate::store::StoreError::from)?;
            let linhas: Vec<ChatRow> = st
                .query_map([], row_to_chat)
                .map_err(crate::store::StoreError::from)?
                .filter_map(|r| r.ok())
                .collect();
            let conversas: Vec<Value> = linhas
                .iter()
                .map(|c| {
                    let pins = store.get_kv::<Vec<String>>(&format!("pins:{}", c.key)).ok().flatten().unwrap_or_default();
                    json!({ "key": c.key, "ttl": c.ttl, "muted": c.muted, "pinned": c.pinned, "pins": pins })
                })
                .collect();
            let mut mensagens = Vec::new();
            if msgs {
                let eu = self.pk_hex();
                let mut st = store
                    .raw()
                    .prepare("SELECT id, chat, author, kind, ts, body, status, exp, reply, reacts FROM msgs ORDER BY ts")
                    .map_err(crate::store::StoreError::from)?;
                let todas: Vec<StoredMsg> = st
                    .query_map([], |r| {
                        Ok(StoredMsg {
                            id: r.get(0)?,
                            chat: r.get(1)?,
                            author: r.get(2)?,
                            kind: r.get(3)?,
                            ts: r.get(4)?,
                            body: r.get(5)?,
                            status: r.get(6)?,
                            exp: r.get(7)?,
                            reply: r.get(8)?,
                            reacts: r.get(9)?,
                        })
                    })
                    .map_err(crate::store::StoreError::from)?
                    .filter_map(|r| r.ok())
                    .collect();
                mensagens = todas.into_iter().filter_map(|sm| msg_to_json(&from_stored(sm, &eu))).collect();
            }
            (conversas, mensagens)
        };
        let s = self.settings();
        let mut v = json!({
            "v": 1, "app": "jmenseger", "by": "pc", "dev": self.device_id(), "at": now_ms(),
            "me": eu, "contacts": contatos, "groups": grupos, "chats": conversas,
            "settings": { "readReceipts": s.read_receipts, "typing": s.typing, "presence": s.presence, "ttlDefault": s.ttl_default, "relays": s.relays },
        });
        if msgs {
            v["msgs"] = Value::Array(mensagens);
        }
        Ok(v)
    }

    /// Junta um retrato com o que já existe aqui. Nada é apagado: contatos e
    /// grupos novos entram, grupos com versão mais nova são atualizados, e as
    /// mensagens que faltam são acrescentadas.
    pub async fn merge_snapshot(self: &Arc<Self>, v: &Value, backup: bool) -> Result<Restored> {
        if v["v"].as_i64() != Some(1) {
            return Err(erro("Versão de backup desconhecida."));
        }
        let mut feito = Restored::default();
        let eu = self.pk_hex();

        // Perfil: no backup, volta tudo; na sincronia, só se aqui estiver vazio.
        if let Some(me) = v["me"].as_object() {
            let atual = self.me();
            let nome = me.get("name").and_then(|x| x.as_str()).unwrap_or_default();
            if backup || atual.name.trim().is_empty() {
                let pic = me.get("pic").and_then(|x| x.as_str()).filter(|_| atual.pic.is_none());
                let _ = self
                    .update_profile(
                        (!nome.is_empty()).then_some(nome),
                        me.get("note").and_then(|x| x.as_str()),
                        me.get("t").and_then(|x| x.as_u64()).map(|t| t as u8),
                        pic,
                    )
                    .await;
            }
        }

        // Contatos.
        for c in v["contacts"].as_array().into_iter().flatten() {
            let Some(pk) = c["pk"].as_str().filter(|p| is_hex64(p) && *p != eu) else { continue };
            let status = c["status"].as_str().and_then(status_from).unwrap_or(ContactStatus::Request);
            let existe = self.contact(pk);
            let mut ct = existe.clone().unwrap_or(Contact {
                cstatus: None,
                pk: pk.into(),
                name: String::new(),
                nick: String::new(),
                note: String::new(),
                pic: None,
                tone: 0,
                status,
                verified: false,
                relays: Vec::new(),
                prof_sent: false,
                added_at: c["addedAt"].as_i64().unwrap_or_else(now_ms),
                banner: None,
                games: Vec::new(),
            });
            let s = |k: &str, n: usize| c[k].as_str().map(|x| clamp(x, n)).unwrap_or_default();
            if ct.name.is_empty() {
                ct.name = s("name", 40);
            }
            if ct.note.is_empty() {
                ct.note = s("note", 80);
            }
            if !s("nick", 40).is_empty() {
                ct.nick = s("nick", 40);
            }
            ct.tone = c["t"].as_u64().map(|t| (t % 6) as u8).unwrap_or(ct.tone);
            // O mais "decidido" vence: bloqueado > aceito > pedido.
            let peso = |x: ContactStatus| match x {
                ContactStatus::Request => 0,
                ContactStatus::Ok => 1,
                ContactStatus::Blocked => 2,
            };
            if existe.is_none() || peso(status) > peso(ct.status) {
                ct.status = status;
            }
            ct.verified |= c["verified"].as_bool().unwrap_or(false);
            if ct.relays.is_empty() {
                ct.relays = c["relays"].as_array().into_iter().flatten().filter_map(|u| u.as_str()).filter_map(normalize_url).take(5).collect();
            }
            if ct.games.is_empty() {
                if let Some(g) = c["games"].as_array() {
                    ct.games = media::clean_games(&g.iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>());
                }
            }
            if ct.pic.is_none() {
                if let Some(Pic::Set(p)) = c["pic"].as_str().map(media::clean_pic) {
                    ct.pic = Some(p);
                }
            }
            if ct.banner.is_none() {
                if let Some(Pic::Set(b)) = c["banner"].as_str().map(media::clean_banner) {
                    ct.banner = Some(b);
                }
            }
            self.save_contact(&ct)?;
            self.state.lock().unwrap().contacts.insert(pk.to_string(), ct);
            if existe.is_none() {
                feito.contacts += 1;
            }
        }

        // Grupos (com as chaves).
        for g in v["groups"].as_array().into_iter().flatten() {
            let Some(sk) = g["sk"].as_str() else { continue };
            let Ok(chaves) = Keys::parse(sk) else { continue };
            let pk = chaves.public_key().to_hex();
            let lid = g["lid"].as_str().filter(|l| is_hex64(l)).unwrap_or(&pk).to_string();
            let ver = g["ver"].as_u64().unwrap_or(1);
            let atual = self.group(&lid);
            if atual.as_ref().is_some_and(|a| a.ver >= ver) {
                continue;
            }
            let Some(meta) = clean_meta(g["meta"].clone(), "") else { continue };
            let old_keys: Vec<(String, String)> = g["old"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|x| x.as_str())
                .filter_map(|s| Keys::parse(s).ok().map(|k| (k.public_key().to_hex(), k.secret_key().to_secret_hex())))
                .take(20)
                .collect();
            let novo = atual.is_none();
            let grupo = Group {
                lid: lid.clone(),
                pk,
                sk: chaves.secret_key().to_secret_hex(),
                old_keys,
                meta,
                ver,
                pic: atual.as_ref().and_then(|a| a.pic.clone()).or_else(|| match g["pic"].as_str().map(media::clean_pic) {
                    Some(Pic::Set(p)) => Some(p),
                    _ => None,
                }),
                pic_v: g["picV"].as_u64().unwrap_or(0),
                pics: atual.as_ref().map(|a| a.pics.clone()).unwrap_or_default(),
                gone: g["gone"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).take(50).collect(),
                muted: g["mute"].as_bool().unwrap_or(false),
                joined_at: atual.as_ref().map(|a| a.joined_at).unwrap_or_else(now_ms),
            };
            self.save_group(&grupo)?;
            if novo {
                feito.groups += 1;
            }
            let _ = self.events.send(EngineEvent::GroupsChanged);
        }
        if feito.groups > 0 {
            // Passa a ouvir os grupos novos.
            self.resubscribe().await;
        }

        // Mensagens (só no backup).
        let mut tocadas = std::collections::BTreeSet::new();
        for m in v["msgs"].as_array().into_iter().flatten() {
            let Some(msg) = msg_from_json(m, &eu) else { continue };
            let chat = msg.chat.clone();
            let novo = self.store.lock().unwrap().put_msg(&to_stored(&msg))?;
            if novo {
                feito.messages += 1;
                tocadas.insert(chat);
            }
        }

        // Ajustes de cada conversa.
        for c in v["chats"].as_array().into_iter().flatten() {
            let Some(key) = c["key"].as_str().filter(|k| ChatKey::parse(k).is_some()) else { continue };
            {
                let store = self.store.lock().unwrap();
                let mut row = chat_for(&store, key);
                row.ttl = c["ttl"].as_u64().map(|t| t.min(28 * DAY as u64) as u32).unwrap_or(row.ttl);
                row.muted = c["muted"].as_bool().unwrap_or(row.muted);
                row.pinned = c["pinned"].as_bool().unwrap_or(row.pinned);
                save_chat(&store, &row)?;
                if let Some(p) = c["pins"].as_array().filter(|p| !p.is_empty()) {
                    let ids: Vec<String> = p.iter().filter_map(|x| x.as_str()).filter(|x| is_hex64(x)).take(50).map(String::from).collect();
                    store.set_kv(&format!("pins:{key}"), &ids)?;
                }
            }
            tocadas.insert(key.to_string());
        }
        for key in &tocadas {
            let _ = self.refresh_last(key);
        }

        // Ajustes do app (só no backup).
        if backup {
            let s = &v["settings"];
            let _ = self
                .update_settings(|x| {
                    if let Some(b) = s["readReceipts"].as_bool() {
                        x.read_receipts = b;
                    }
                    if let Some(b) = s["typing"].as_bool() {
                        x.typing = b;
                    }
                    if let Some(b) = s["presence"].as_bool() {
                        x.presence = b;
                    }
                    if let Some(t) = s["ttlDefault"].as_u64() {
                        x.ttl_default = t as u32;
                    }
                })
                .await;
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        let _ = self.events.send(EngineEvent::ProfileChanged);
        Ok(feito)
    }

    fn secret_bytes(&self) -> [u8; 32] {
        self.keys.secret_key().secret_bytes()
    }

    /// Arquivo de backup cifrado, pronto para gravar.
    pub fn export_backup(&self) -> Result<Vec<u8>> {
        let v = self.snapshot(true)?;
        seal(&self.secret_bytes(), &serde_json::to_vec(&v).map_err(|e| erro(e.to_string()))?)
    }

    /// Restaura um arquivo de backup desta conta.
    pub async fn import_backup(self: &Arc<Self>, data: &[u8]) -> Result<Restored> {
        let json = open(&self.secret_bytes(), data)?;
        let v: Value = serde_json::from_slice(&json).map_err(|_| erro("Backup corrompido."))?;
        self.merge_snapshot(&v, true).await
    }
}

/// Tamanho máximo de cada pedaço da sincronia (o envelope cifrado duas
/// vezes em base64 tem de caber nos 64 KB dos servidores).
const SYNC_PEDACO: usize = 24_000;

impl Engine {
    /// Recebeu algo que a própria conta mandou, de outro aparelho.
    pub(super) fn on_self_app(self: &Arc<Self>, app: AppMessage) {
        let meu = self.device_id();
        match app {
            AppMessage::Device { dev, n, p } if dev != meu && dev.len() <= 32 => {
                {
                    let store = self.store.lock().unwrap();
                    let mut lista: std::collections::BTreeMap<String, Value> = store.get_kv("devices").ok().flatten().unwrap_or_default();
                    let tirados: Vec<String> = store.get_kv("devices_off").ok().flatten().unwrap_or_default();
                    if tirados.contains(&dev) {
                        return;
                    }
                    lista.insert(dev, json!({ "n": clamp(&n, 40), "p": clamp(&p, 20), "last": now_ms() }));
                    let _ = store.set_kv("devices", &lista);
                }
                let _ = self.events.send(EngineEvent::Devices);
            }
            AppMessage::DeviceOff { dev } => {
                if dev == meu {
                    // Outro aparelho da conta mandou este sair.
                    let _ = self.events.send(EngineEvent::RemoteLogout);
                } else {
                    let store = self.store.lock().unwrap();
                    let mut lista: std::collections::BTreeMap<String, Value> = store.get_kv("devices").ok().flatten().unwrap_or_default();
                    lista.remove(&dev);
                    let _ = store.set_kv("devices", &lista);
                    drop(store);
                    let _ = self.events.send(EngineEvent::Devices);
                }
            }
            AppMessage::SyncReq { dev } if dev != meu => {
                let me = Arc::clone(self);
                tokio::spawn(async move { me.sync_push(true).await });
            }
            AppMessage::Sync { dev, d } if dev != meu => {
                let me = Arc::clone(self);
                tokio::spawn(async move {
                    match me.merge_snapshot(&d, false).await {
                        Ok(r) => {
                            if r.contacts + r.groups > 0 {
                                tracing::info!("sincronia: {} contatos e {} grupos novos de outro aparelho", r.contacts, r.groups);
                                let _ = me.events.send(EngineEvent::Synced { contacts: r.contacts, groups: r.groups });
                            }
                        }
                        Err(e) => tracing::debug!("sincronia: {e}"),
                    }
                });
            }
            _ => {}
        }
    }

    async fn send_self(&self, app: &AppMessage) -> Result<()> {
        let chat = ChatKey::Direct { peer: self.pk_hex() }.as_string();
        let o = BuildOpts { no_ttl: true, no_self_copy: true, expires_in: Some(2 * DAY), ..Default::default() };
        self.send_app(&chat, app, o).await.map(|_| ())
    }

    /// Conta aos outros aparelhos que este existe (nome e sistema). Repetir de
    /// tempos em tempos mantém o "visto por último" em dia.
    pub async fn announce_device(self: &Arc<Self>, nome: &str, sistema: &str) {
        let app = AppMessage::Device { dev: self.device_id(), n: clamp(nome, 40), p: clamp(sistema, 20) };
        let chat = ChatKey::Direct { peer: self.pk_hex() }.as_string();
        // Dura 7 dias no servidor: um aparelho que volta depois de uma semana ainda se vê na lista.
        let o = BuildOpts { no_ttl: true, no_self_copy: true, expires_in: Some(7 * DAY), ..Default::default() };
        let _ = self.send_app(&chat, &app, o).await;
    }

    /// Outros aparelhos da conta: (id, nome, sistema, visto por último em ms).
    pub fn devices(&self) -> Vec<(String, String, String, i64)> {
        let lista: std::collections::BTreeMap<String, Value> = self.store.lock().unwrap().get_kv("devices").ok().flatten().unwrap_or_default();
        let mut v: Vec<_> = lista
            .into_iter()
            .map(|(id, x)| (id, x["n"].as_str().unwrap_or("Aparelho").to_string(), x["p"].as_str().unwrap_or("").to_string(), x["last"].as_i64().unwrap_or(0)))
            .collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.3));
        v
    }

    /// Manda um aparelho da conta sair (ele apaga a conta de lá ao receber).
    pub async fn remove_device(self: &Arc<Self>, dev: &str) -> Result<()> {
        {
            let store = self.store.lock().unwrap();
            let mut lista: std::collections::BTreeMap<String, Value> = store.get_kv("devices").ok().flatten().unwrap_or_default();
            lista.remove(dev);
            store.set_kv("devices", &lista)?;
            let mut tirados: Vec<String> = store.get_kv("devices_off").ok().flatten().unwrap_or_default();
            if !tirados.iter().any(|x| x == dev) {
                tirados.push(dev.to_string());
            }
            store.set_kv("devices_off", &tirados)?;
        }
        let chat = ChatKey::Direct { peer: self.pk_hex() }.as_string();
        let o = BuildOpts { no_ttl: true, no_self_copy: true, expires_in: Some(30 * DAY), ..Default::default() };
        self.send_app(&chat, &AppMessage::DeviceOff { dev: dev.into() }, o).await?;
        let _ = self.events.send(EngineEvent::Devices);
        Ok(())
    }

    /// Pede aos outros aparelhos da conta o que eles têm (uma vez por instalação).
    pub async fn sync_request(self: &Arc<Self>) {
        let ja = self.store.lock().unwrap().get_kv::<bool>("sync_req").ok().flatten().unwrap_or(false);
        if ja {
            return;
        }
        if self.send_self(&AppMessage::SyncReq { dev: self.device_id() }).await.is_ok() {
            let _ = self.store.lock().unwrap().set_kv("sync_req", &true);
        }
    }

    /// Manda contatos, grupos e ajustes para os outros aparelhos da conta,
    /// em pedaços. Sem `force`, só quando algo mudou desde a última vez.
    pub async fn sync_push(self: &Arc<Self>, force: bool) {
        let Ok(mut v) = self.snapshot(false) else { return };
        let mut sem_hora = v.clone();
        sem_hora["at"] = Value::Null;
        let hash: String = {
            use sha2::Digest;
            sha2::Sha256::digest(sem_hora.to_string().as_bytes()).iter().take(12).map(|x| format!("{x:02x}")).collect()
        };
        let antes = self.store.lock().unwrap().get_kv::<String>("sync_hash").ok().flatten();
        if !force && antes.as_deref() == Some(hash.as_str()) {
            return;
        }
        // Só vale a pena se a conta tiver alguma coisa.
        let vazio = |k: &str| v[k].as_array().is_none_or(|a| a.is_empty());
        if vazio("contacts") && vazio("groups") && !force {
            let _ = self.store.lock().unwrap().set_kv("sync_hash", &hash);
            return;
        }
        let listas: Vec<(&str, Vec<Value>)> = ["contacts", "groups", "chats"]
            .iter()
            .map(|k| (*k, v[*k].as_array().cloned().unwrap_or_default()))
            .collect();
        for (k, _) in &listas {
            v[*k] = Value::Array(Vec::new());
        }
        let base = v;
        let mut pedacos: Vec<Value> = Vec::new();
        let mut atual = base.clone();
        for (k, itens) in listas {
            for item in itens {
                let tam = item.to_string().len();
                if tam > SYNC_PEDACO {
                    continue; // um grupo gigante não trava o resto
                }
                if atual.to_string().len() + tam > SYNC_PEDACO {
                    pedacos.push(std::mem::replace(&mut atual, base.clone()));
                }
                if let Some(a) = atual[k].as_array_mut() {
                    a.push(item);
                }
            }
        }
        pedacos.push(atual);
        let dev = self.device_id();
        for d in pedacos {
            if self.send_self(&AppMessage::Sync { dev: dev.clone(), d }).await.is_err() {
                return;
            }
        }
        let _ = self.store.lock().unwrap().set_kv("sync_hash", &hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_so_abre_com_a_mesma_conta() {
        let a = [7u8; 32];
        let b = [8u8; 32];
        let dados = br#"{"v":1,"oi":"mundo"}"#;
        let f = seal(&a, dados).unwrap();
        assert_eq!(&f[..5], MAGIC);
        assert_eq!(open(&a, &f).unwrap(), dados.to_vec());
        assert!(open(&b, &f).is_err(), "outra conta não abre");
        let mut estragado = f.clone();
        let n = estragado.len() - 1;
        estragado[n] ^= 1;
        assert!(open(&a, &estragado).is_err(), "um bit trocado não passa");
        assert!(open(&a, b"qualquer coisa").is_err());
    }

    #[test]
    fn hkdf_confere_com_o_vetor_da_rfc5869() {
        // RFC 5869, caso 1 (primeiros 32 bytes).
        let ikm = [0x0bu8; 22];
        let salt: Vec<u8> = (0u8..=0x0c).collect();
        let info: Vec<u8> = (0xf0u8..=0xf9).collect();
        let okm = hkdf32(&ikm, &salt, &info);
        let esperado = "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf";
        assert_eq!(okm.iter().map(|x| format!("{x:02x}")).collect::<String>(), esperado);
    }

    #[test]
    fn mensagem_ida_e_volta_no_formato_neutro() {
        let eu = "a".repeat(64);
        let m = Message {
            fwd: false,
            lp: None,
            id: "b".repeat(64),
            chat: format!("dm:{}", "c".repeat(64)),
            author: eu.clone(),
            from_me: true,
            ts: 5,
            body: MessageBody::Poll { q: "Dia?".into(), o: vec!["Sáb".into(), "Dom".into()], multi: false, votes: BTreeMap::from([(eu.clone(), vec![1])]) },
            status: Some(Delivery::Read),
            reply: None,
            reacts: BTreeMap::new(),
            exp: None,
            edited: None,
        };
        let v = msg_to_json(&m).unwrap();
        assert_eq!(msg_from_json(&v, &eu).unwrap(), m);
    }
}
