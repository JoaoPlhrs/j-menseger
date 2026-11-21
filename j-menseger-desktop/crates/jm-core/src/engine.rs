//! O motor: amarra identidade, servidores e banco, e avisa a interface.
//!
//! A interface nunca fala com a rede nem com o banco diretamente. Ela chama
//! métodos daqui e escuta [`EngineEvent`]. Assim a janela nunca trava
//! esperando um servidor responder.
//!
//! Este arquivo é o porte do `engine.js` do aplicativo de celular (versão
//! 0.4). Cada regra de recebimento — quem pode mandar o quê, o que é
//! descartado, quando um recibo é enviado — segue a do celular, porque os
//! dois conversam entre si e qualquer diferença vira mensagem perdida ou
//! comportamento estranho do outro lado.

use crate::groups::{ChannelKind, Group, GroupMeta, GroupPatch, Template};
use crate::identity;
use crate::media::{self, MediaInfo, Pic};
use crate::messages::{
    AppMessage, CallAction, CallSignal, ChatKey, Delivery, GroupInvite, MediaHeader, MediaKind,
    Listening, Presence, ReceiptKind, SoundMeta,
};
use crate::pin::PinLock;
use crate::protocol::{self, WrapOptions, CHUNK, KIND_APP, KIND_REACTION, KIND_TEXT, KIND_WRAP_EPH};
use crate::relays::{normalize_url, RelayPool, RelayStatus};
use crate::store::{CallLog, SoundRecord, Store, StoredMsg};
use base64::Engine as _;
use nostr::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc;

const DAY: i64 = 86_400;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Store(#[from] crate::store::StoreError),
    #[error(transparent)]
    Identity(#[from] identity::IdentityError),
    #[error("{0}")]
    Protocol(String),
    #[error("servidores: {0}")]
    Relay(String),
    #[error("ainda não há uma conta neste aparelho")]
    NoIdentity,
    #[error("conversa desconhecida: {0}")]
    UnknownChat(String),
}

type Result<T> = std::result::Result<T, EngineError>;

fn erro(msg: impl Into<String>) -> EngineError {
    EngineError::Protocol(msg.into())
}

/* ======================================================================
 *  Tipos que a interface usa
 * ====================================================================== */

/// Seu próprio perfil.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub tone: u8,
    #[serde(default)]
    pub pic: Option<String>,
    #[serde(default)]
    pub created: i64,
    #[serde(default)]
    pub restored: bool,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub games: Vec<String>,
}

/// Situação de um contato.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContactStatus {
    /// Mandou mensagem mas ainda não foi aceito.
    Request,
    /// Contato aceito: pode mandar mensagem e ligar.
    Ok,
    /// Bloqueado: nada do que ele manda chega.
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub pk: String,
    #[serde(default)]
    pub name: String,
    /// Apelido que você deu. Tem prioridade sobre o nome que a pessoa escolheu.
    #[serde(default)]
    pub nick: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub pic: Option<String>,
    #[serde(default)]
    pub tone: u8,
    pub status: ContactStatus,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub relays: Vec<String>,
    #[serde(default)]
    pub prof_sent: bool,
    #[serde(default)]
    pub added_at: i64,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub games: Vec<String>,
}

impl Contact {
    /// Como o nome aparece na tela.
    pub fn display_name(&self) -> String {
        if !self.nick.is_empty() {
            self.nick.clone()
        } else if !self.name.is_empty() {
            self.name.clone()
        } else {
            PublicKey::from_hex(&self.pk)
                .map(|p| identity::short_id(&p))
                .unwrap_or_else(|_| self.pk[..8.min(self.pk.len())].to_string())
        }
    }
}

/// Uma conversa na lista.
#[derive(Debug, Clone)]
pub struct ChatSummary {
    pub key: String,
    pub title: String,
    pub last_text: String,
    pub last_ts: i64,
    pub last_status: Option<Delivery>,
    pub last_from_me: bool,
    pub last_kind: String,
    pub unread: u32,
    pub pinned: bool,
    pub muted: bool,
    pub is_group: bool,
    pub peer: Option<String>,
    pub ttl: u32,
}

/// Uma mensagem como a interface precisa dela.
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub chat: String,
    pub author: String,
    pub from_me: bool,
    pub ts: i64,
    pub body: MessageBody,
    pub status: Option<Delivery>,
    pub reply: Option<String>,
    /// Quem reagiu com o quê: `{chave pública: emoji}`.
    pub reacts: BTreeMap<String, String>,
    /// Quando some (mensagem temporária), em milissegundos.
    pub exp: Option<i64>,
}

/// Dados de um convite, guardados na própria mensagem até a pessoa aceitar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InviteData {
    pub sk: String,
    pub pk: String,
    pub lid: String,
    pub meta: GroupMeta,
    pub ver: u64,
    #[serde(default)]
    pub pic: String,
    #[serde(default, rename = "picV")]
    pub pic_v: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageBody {
    Text(String),
    /// Aviso do sistema. `sys` usa os códigos do celular: `missed`, `ttl`,
    /// `created`, `joined`, `join`, `leave`, `removed`, `kicked`, `gpic`,
    /// `gpicdel`. `legacy` guarda o texto pronto de versões antigas do PC.
    System { sys: String, v: i64, legacy: Option<String> },
    /// Som de fábrica do soundboard.
    Sound { id: String },
    /// Foto, áudio ou som próprio.
    Media(MediaInfo),
    /// Convite para um grupo, com botão de entrar.
    Invite { lid: String, name: String, joined: bool, data: Option<Box<InviteData>> },
    Deleted,
}

impl MessageBody {
    fn kind_name(&self) -> &'static str {
        match self {
            MessageBody::Text(_) => "text",
            MessageBody::System { .. } => "sys",
            MessageBody::Sound { .. } => "sound",
            MessageBody::Media(_) => "media",
            MessageBody::Invite { .. } => "invite",
            MessageBody::Deleted => "deleted",
        }
    }

    fn to_json(&self) -> String {
        match self {
            MessageBody::Text(t) => serde_json::json!({ "text": t }).to_string(),
            MessageBody::System { sys, v, legacy } => match legacy {
                Some(t) => serde_json::json!({ "text": t }).to_string(),
                None => serde_json::json!({ "sys": sys, "v": v }).to_string(),
            },
            MessageBody::Sound { id } => serde_json::json!({ "id": id }).to_string(),
            MessageBody::Media(m) => serde_json::to_string(m).unwrap_or_default(),
            MessageBody::Invite { lid, name, joined, data } => {
                serde_json::json!({ "lid": lid, "text": name, "joined": joined, "inv": data }).to_string()
            }
            MessageBody::Deleted => "{}".into(),
        }
    }

    fn from_stored(kind: &str, body: &str) -> MessageBody {
        let v: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
        let text = v["text"].as_str().unwrap_or_default().to_string();
        match kind {
            "text" => MessageBody::Text(text),
            "sys" => match v["sys"].as_str() {
                Some(code) => MessageBody::System {
                    sys: code.to_string(),
                    v: v["v"].as_i64().unwrap_or(0),
                    legacy: None,
                },
                None => MessageBody::System { sys: String::new(), v: 0, legacy: Some(text) },
            },
            "sound" => MessageBody::Sound { id: v["id"].as_str().unwrap_or_default().to_string() },
            "media" => match serde_json::from_value::<MediaInfo>(v) {
                Ok(m) => MessageBody::Media(m),
                Err(_) => MessageBody::Deleted,
            },
            "invite" => MessageBody::Invite {
                lid: v["lid"].as_str().unwrap_or_default().to_string(),
                name: text,
                joined: v["joined"].as_bool().unwrap_or(false),
                data: serde_json::from_value::<InviteData>(v["inv"].clone()).ok().map(Box::new),
            },
            _ => MessageBody::Deleted,
        }
    }

    pub fn is_system(&self) -> bool {
        matches!(self, MessageBody::System { .. })
    }
}

/// O que o motor avisa para a interface.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Chegou ou saiu uma mensagem.
    Message { chat: String, message: Box<Message> },
    /// Uma mensagem que já está na tela mudou (reação, mídia chegando,
    /// apagada para todos).
    MessageChanged { chat: String, message: Box<Message> },
    /// Mudou só o estado de entrega.
    MessageUpdated { chat: String, id: String, status: Delivery },
    /// Uma mensagem sumiu deste aparelho (apagada para mim, ou venceu).
    MessageDeleted { chat: String, id: String },
    /// Chegou mensagem de outra pessoa: serve para avisos e notificações.
    Incoming { chat: String, message: Box<Message>, visible: bool },
    /// Progresso do envio de uma mídia.
    Progress { chat: String, id: String, done: u32, total: u32 },
    /// A lista de conversas mudou.
    ChatsChanged,
    /// Uma conversa deixou de existir (canal apagado, grupo deixado).
    ChatGone { chat: String },
    /// Um contato mudou (nome, foto, situação).
    ContactChanged { pk: String },
    /// Alguém está digitando nesta conversa.
    Typing { chat: String, who: String },
    /// Mudou a conexão com os servidores.
    Relays(Vec<RelayStatus>),
    /// A lista de grupos ou os dados de um grupo mudaram.
    GroupsChanged,
    /// Chegou sinalização de chamada (ou de sala de voz) de alguém.
    CallSignal { from: String, signal: CallSignal },
    /// Uma chamada que você não atendeu.
    MissedCall { from: String, video: bool, ts: i64 },
    /// Quem está numa sala de voz mudou.
    Room { lid: String, ch: String, members: Vec<String> },
    /// O histórico de chamadas mudou.
    CallsChanged,
    /// Seu perfil mudou.
    ProfileChanged,
    /// Mudou o que um contato está ouvindo.
    Listening { pk: String },
    /// Os ajustes mudaram.
    SettingsChanged,
}

/// O que aparece na notificação quando chega mensagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NotifPreview {
    /// Nome e mensagem.
    Full,
    /// Só o nome.
    #[default]
    Name,
    /// Só "Nova mensagem".
    None,
}

/// Quem pode ligar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CallsFrom {
    #[default]
    Contacts,
    Nobody,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnServer {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub pass: String,
}

/// Ajustes do aplicativo (os mesmos do celular).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub relays: Vec<String>,
    #[serde(default = "sim")]
    pub read_receipts: bool,
    #[serde(default = "sim")]
    pub typing: bool,
    #[serde(default)]
    pub ttl_default: u32,
    #[serde(default)]
    pub notif_preview: NotifPreview,
    #[serde(default)]
    pub calls_from: CallsFrom,
    #[serde(default)]
    pub turn: TurnServer,
    /// Toda chamada passa pelo servidor TURN: o outro lado não vê seu IP.
    #[serde(default)]
    pub force_relay: bool,
    #[serde(default = "sim")]
    pub notifications: bool,
    #[serde(default = "sim")]
    pub sounds: bool,
    /// Esconder o conteúdo da janela em capturas de tela (Windows).
    #[serde(default)]
    pub lock_screenshots: bool,
}

fn sim() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            relays: protocol::DEFAULT_RELAYS.iter().map(|s| s.to_string()).collect(),
            read_receipts: true,
            typing: true,
            ttl_default: 0,
            notif_preview: NotifPreview::Name,
            calls_from: CallsFrom::Contacts,
            turn: TurnServer::default(),
            force_relay: false,
            notifications: true,
            sounds: true,
            lock_screenshots: false,
        }
    }
}

/// Metadados de uma mídia a enviar.
#[derive(Debug, Clone, Default)]
pub struct MediaMeta {
    pub kind: Option<MediaKind>,
    pub mime: String,
    /// Duração em segundos (áudio).
    pub dur: Option<f64>,
    /// Forma de onda, valores de 0 a 31.
    pub wf: Option<Vec<u8>>,
    pub w: Option<u32>,
    pub h: Option<u32>,
    /// Miniatura em data URL.
    pub th: Option<String>,
    /// Quando é um som próprio do soundboard.
    pub sb: Option<SoundMeta>,
}

/// Contas e chaves exportáveis.
#[derive(Debug, Clone)]
pub struct ExportedKeys {
    pub npub: String,
    pub nsec: String,
    pub words: Option<String>,
}

/* ======================================================================
 *  Estado interno
 * ====================================================================== */

#[derive(Default)]
struct ReceiptQueue {
    d: BTreeSet<String>,
    r: BTreeSet<String>,
}

struct State {
    contacts: HashMap<String, Contact>,
    groups: HashMap<String, Group>,
    /// Quando pedimos o perfil de alguém pela última vez, para não insistir.
    prof_req_at: HashMap<String, i64>,
    /// Conversa aberta na tela.
    active: Option<String>,
    /// A janela está à vista (em foco).
    visible: bool,
    typing_at: HashMap<String, i64>,
    /// Salas de voz: (grupo, canal) → quem está lá e quando foi visto.
    rooms: HashMap<(String, String), HashMap<String, i64>>,
    /// Recibos a mandar, por contato.
    rq: HashMap<String, ReceiptQueue>,
    rq_scheduled: bool,
    /// O que cada contato está ouvindo e quando avisou.
    listening: HashMap<String, (Listening, i64)>,
    /// O que eu estou ouvindo e quando avisei os contatos.
    my_listening: Option<(Listening, i64)>,
}

/// Uma conversa como fica na tabela `chats`.
#[derive(Debug, Clone, Default)]
struct ChatRow {
    key: String,
    kind: String,
    peer: Option<String>,
    lid: Option<String>,
    channel: Option<String>,
    last_ts: i64,
    last_id: Option<String>,
    last_from: Option<String>,
    last_kind: Option<String>,
    last_text: String,
    last_status: Option<String>,
    unread: u32,
    ttl: u32,
    pinned: bool,
    muted: bool,
    deleted: bool,
    pending_read: Vec<String>,
    existed: bool,
}

/// Uma mensagem pronta para sair: o rumor e os envelopes.
struct Built {
    id: String,
    created_ms: i64,
    wraps: Vec<Event>,
    relays: Vec<String>,
    exp_ms: Option<i64>,
}

#[derive(Default, Clone, Copy)]
struct BuildOpts {
    /// Não aplicar o prazo de mensagem temporária da conversa.
    no_ttl: bool,
    ephemeral: bool,
    /// Não mandar cópia para mim mesmo (outros aparelhos da mesma conta).
    no_self_copy: bool,
    /// Prazo do envelope no servidor, em segundos a partir de agora.
    expires_in: Option<i64>,
}

pub struct Engine {
    keys: Keys,
    me: RwLock<Profile>,
    settings: RwLock<Settings>,
    store: Arc<Mutex<Store>>,
    pool: Arc<RelayPool>,
    state: Arc<Mutex<State>>,
    events: mpsc::UnboundedSender<EngineEvent>,
    started: AtomicBool,
    tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl Engine {
    /* ---------------- abertura e identidade ---------------- */

    /// Abre o motor com uma conta já existente.
    pub fn open(keys: Keys, store: Store) -> Result<(Arc<Self>, mpsc::UnboundedReceiver<EngineEvent>)> {
        let me: Profile = store.get_kv("me")?.unwrap_or_default();
        let settings: Settings = store.get_kv("settings")?.unwrap_or_default();
        let contacts = load_contacts(&store)?;
        let groups = load_groups(&store)?;
        // A chave da conta passa a ficar guardada, para abrir direto da
        // próxima vez. Contas antigas do PC já a tinham.
        if store.get_kv::<String>("account_sk")?.is_none() {
            store.set_kv("account_sk", &keys.secret_key().to_secret_hex())?;
        }

        let (tx, rx) = mpsc::unbounded_channel();
        let engine = Arc::new(Self {
            pool: Arc::new(RelayPool::new(keys.clone())),
            keys,
            me: RwLock::new(me),
            settings: RwLock::new(settings),
            store: Arc::new(Mutex::new(store)),
            state: Arc::new(Mutex::new(State {
                contacts,
                groups,
                prof_req_at: HashMap::new(),
                active: None,
                visible: true,
                typing_at: HashMap::new(),
                rooms: HashMap::new(),
                rq: HashMap::new(),
                rq_scheduled: false,
                listening: HashMap::new(),
                my_listening: None,
            })),
            events: tx,
            started: AtomicBool::new(false),
            tasks: Mutex::new(Vec::new()),
        });
        Ok((engine, rx))
    }

    /// Cria uma conta nova a partir de 12 palavras.
    pub fn create_identity(store: &Store, words: &str, name: &str) -> Result<Keys> {
        Self::create_identity_full(store, words, name, "", None)
    }

    /// Cria uma conta nova, com recado e cor do avatar.
    pub fn create_identity_full(
        store: &Store,
        words: &str,
        name: &str,
        note: &str,
        tone: Option<u8>,
    ) -> Result<Keys> {
        let keys = identity::keys_from_words(words)?;
        let me = Profile {
            name: clamp(name, 40),
            note: clamp(note, 80),
            tone: tone.unwrap_or_else(|| rand::random::<u8>() % 6) % 6,
            pic: None,
            created: now_ms(),
            restored: false,
            ..Default::default()
        };
        store.set_kv("account_sk", &keys.secret_key().to_secret_hex())?;
        if let Some(w) = identity::valid_words(words) {
            store.set_kv("words", &w)?;
        }
        store.set_kv("me", &me)?;
        store.set_kv("last_sync", &now_secs())?;
        Ok(keys)
    }

    /// Entra em uma conta que já existe, em outro aparelho.
    ///
    /// Busca o último mês guardado nos servidores, que é o que eles costumam
    /// manter: o histórico mais antigo fica só no aparelho de origem.
    pub fn restore_identity(store: &Store, input: &str, name: &str) -> Result<Keys> {
        let keys = identity::keys_from_any(input)?;
        let me = Profile {
            name: clamp(name, 40),
            tone: rand::random::<u8>() % 6,
            created: now_ms(),
            restored: true,
            ..Default::default()
        };
        store.set_kv("account_sk", &keys.secret_key().to_secret_hex())?;
        if let Some(w) = identity::valid_words(input) {
            store.set_kv("words", &w)?;
        }
        store.set_kv("me", &me)?;
        store.set_kv("last_sync", &(now_secs() - 30 * DAY))?;
        Ok(keys)
    }

    pub fn public_key(&self) -> PublicKey {
        self.keys.public_key()
    }

    pub fn pk_hex(&self) -> String {
        self.keys.public_key().to_hex()
    }

    pub fn npub(&self) -> String {
        identity::npub(&self.keys.public_key())
    }

    pub fn me(&self) -> Profile {
        self.me.read().unwrap().clone()
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    /// Chaves e palavras, para a tela de recuperação.
    pub fn export_keys(&self) -> ExportedKeys {
        let words = self.store.lock().unwrap().get_kv::<String>("words").ok().flatten();
        ExportedKeys {
            npub: self.npub(),
            nsec: identity::nsec(&self.keys),
            words,
        }
    }

    /// Estas palavras (ou chave) são desta conta? Usado no "esqueci o PIN".
    pub fn verify_recovery(&self, input: &str) -> bool {
        identity::keys_from_any(input)
            .map(|k| k.public_key() == self.keys.public_key())
            .unwrap_or(false)
    }

    /// As palavras foram anotadas? (Aviso na tela de ajustes.)
    pub fn words_pending(&self) -> bool {
        self.store.lock().unwrap().get_kv::<bool>("words_pending").ok().flatten().unwrap_or(false)
    }

    pub fn set_words_pending(&self, pending: bool) -> Result<()> {
        self.store.lock().unwrap().set_kv("words_pending", &pending)?;
        Ok(())
    }

    /* ---------------- PIN ---------------- */

    pub fn pin_lock(&self) -> Option<PinLock> {
        self.store.lock().unwrap().get_kv::<PinLock>("lock").ok().flatten()
    }

    /// Define (ou troca) o PIN, mantendo o tempo de espera.
    pub fn set_pin(&self, pin: &str) -> Result<PinLock> {
        if !crate::pin::valid_pin(pin) {
            return Err(erro("o PIN precisa ter 4 dígitos"));
        }
        let delay = self.pin_lock().map(|l| l.delay).unwrap_or(60);
        let lock = PinLock::new(pin, delay);
        self.store.lock().unwrap().set_kv("lock", &lock)?;
        Ok(lock)
    }

    pub fn remove_pin(&self) -> Result<()> {
        self.store.lock().unwrap().raw().execute("DELETE FROM kv WHERE k = 'lock'", []).map_err(crate::store::StoreError::from)?;
        Ok(())
    }

    pub fn set_pin_delay(&self, secs: u32) -> Result<()> {
        if let Some(mut l) = self.pin_lock() {
            l.delay = secs;
            self.store.lock().unwrap().set_kv("lock", &l)?;
        }
        Ok(())
    }

    pub fn check_pin(&self, pin: &str) -> bool {
        self.pin_lock().is_some_and(|l| l.check(pin))
    }

    /* ---------------- conexão ---------------- */

    /// Conecta aos servidores e começa a receber.
    pub async fn start(self: &Arc<Self>) -> Result<()> {
        let relays = self.settings().relays;
        self.pool.set_relays(&relays).await.map_err(|e| EngineError::Relay(e.to_string()))?;
        self.subscribe().await?;
        if self.started.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let mut tarefas = Vec::new();

        let mut inbox = self.pool.listen();
        let me = Arc::clone(self);
        tarefas.push(tokio::spawn(async move {
            while let Some(incoming) = inbox.recv().await {
                if let Err(e) = me.handle_envelope(&incoming.event) {
                    tracing::debug!("envelope descartado: {e}");
                }
            }
        }));

        // Estado das conexões, para a barra da interface.
        let me = Arc::clone(self);
        tarefas.push(tokio::spawn(async move {
            let mut anterior: Vec<RelayStatus> = Vec::new();
            loop {
                let agora = me.pool.status().await;
                if agora != anterior {
                    let _ = me.events.send(EngineEvent::Relays(agora.clone()));
                    anterior = agora;
                }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }));

        // A cada 15 s: marca até onde já foi sincronizado e apaga o que venceu.
        let me = Arc::clone(self);
        tarefas.push(tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            me.purge_expired();
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                if me.pool.open_count().await > 0 {
                    let _ = me.store.lock().unwrap().set_kv("last_sync", &now_secs());
                }
                me.purge_expired();
            }
        }));

        // Reenvio do que ficou na fila sem internet.
        let me = Arc::clone(self);
        tarefas.push(tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            loop {
                me.flush_outbox().await;
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        }));

        // Quem sumiu das salas de voz sem avisar.
        let me = Arc::clone(self);
        tarefas.push(tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                me.room_sweep();
            }
        }));

        // Lista pública de servidores de caixa de entrada (NIP-17).
        let me = Arc::clone(self);
        tarefas.push(tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            me.publish_relay_list().await;
        }));

        // Servidores preferidos dos contatos já conhecidos.
        let extras: Vec<String> = {
            let st = self.state.lock().unwrap();
            st.contacts.values().flat_map(|c| c.relays.clone()).collect()
        };
        if !extras.is_empty() {
            self.pool.add_extra_relays(&extras).await;
        }

        self.tasks.lock().unwrap().extend(tarefas);
        Ok(())
    }

    async fn subscribe(self: &Arc<Self>) -> Result<()> {
        let since: i64 = self
            .store
            .lock()
            .unwrap()
            .get_kv("last_sync")?
            .unwrap_or_else(|| now_secs() - DAY);
        self.pool
            .subscribe_inbox(&self.inbox_keys(), Timestamp::from_secs(since.max(0) as u64))
            .await
            .map_err(|e| EngineError::Relay(e.to_string()))
    }

    /// Reassina a caixa de entrada: usado quando a lista de grupos muda.
    async fn resubscribe(self: &Arc<Self>) {
        if let Err(e) = self.subscribe().await {
            tracing::warn!("não foi possível renovar a assinatura: {e}");
        }
    }

    /// Publica a lista de servidores de caixa de entrada (kind 10050), para
    /// outros aplicativos NIP-17 saberem onde entregar mensagens para você.
    async fn publish_relay_list(&self) {
        let relays = self.settings().relays;
        let marca = relays.join(",");
        let ja: Option<String> = self.store.lock().unwrap().get_kv("pub10050").ok().flatten();
        if ja.as_deref() == Some(marca.as_str()) {
            return;
        }
        let tags: Vec<Tag> = relays
            .iter()
            .take(6)
            .map(|u| Tag::custom("relay", [u.clone()]))
            .collect();
        let Ok(ev) = EventBuilder::new(Kind::Custom(10050), "").tags(tags).finalize(&self.keys) else {
            return;
        };
        if self.pool.publish(&ev).await.is_ok() {
            let _ = self.store.lock().unwrap().set_kv("pub10050", &marca);
        }
    }

    pub async fn relay_status(&self) -> Vec<RelayStatus> {
        self.pool.status().await
    }

    pub async fn open_relays(&self) -> usize {
        self.pool.open_count().await
    }

    /// Força reconectar (volta do modo de espera, rede mudou).
    pub async fn wake(&self) {
        self.pool.wake().await;
    }

    pub async fn shutdown(&self) {
        for t in self.tasks.lock().unwrap().drain(..) {
            t.abort();
        }
        self.pool.shutdown().await;
    }

    /* ---------------- recebimento ---------------- */

    /// Abre um envelope que chegou e decide o que fazer com ele.
    fn handle_envelope(self: &Arc<Self>, event: &Event) -> Result<()> {
        let id = event.id.to_hex();
        // A mesma mensagem chega por vários servidores: só a primeira conta.
        if !self.store.lock().unwrap().mark_seen(&id, now_secs())? {
            return Ok(());
        }

        let destino = protocol::tag_value_event(event, "p").unwrap_or_default().to_string();
        let grupo = self.group_for_pubkey(&destino);
        let abridor = match &grupo {
            Some((_, keys)) => keys.clone(),
            None => {
                if destino != self.pk_hex() {
                    return Ok(());
                }
                self.keys.clone()
            }
        };

        let mut rumor = protocol::unwrap_event(event, &abridor).map_err(|e| erro(e.to_string()))?;
        // Horário no futuro (relógio adiantado ou malícia) vira "agora".
        if rumor.created_at.as_secs() > now_secs() as u64 + 600 {
            rumor.created_at = Timestamp::now();
        }
        let eph = event.kind.as_u16() == KIND_WRAP_EPH;

        match grupo {
            Some((lid, _)) => self.on_group_rumor(&rumor, &lid, eph),
            None => self.on_dm_rumor(&rumor, eph),
        }
    }

    fn on_dm_rumor(self: &Arc<Self>, r: &UnsignedEvent, eph: bool) -> Result<()> {
        let eu = self.pk_hex();
        let from = r.pubkey.to_hex();
        let mine = from == eu;
        let peer = if mine { protocol::tag_value(r, "p").map(|s| s.to_string()) } else { Some(from.clone()) };
        let Some(peer) = peer else { return Ok(()) };
        if !is_hex64(&peer) || peer == eu {
            return Ok(());
        }
        let known = self.contact(&peer);
        // Quem está bloqueado não entrega nada.
        if known.as_ref().is_some_and(|c| c.status == ContactStatus::Blocked) {
            return Ok(());
        }
        let key = ChatKey::Direct { peer: peer.clone() }.as_string();
        let rid = r.id.map(|i| i.to_hex()).unwrap_or_default();
        let ts = r.created_at.as_secs() as i64 * 1000;

        match r.kind.as_u16() {
            KIND_TEXT => {
                self.ensure_contact(&peer, if mine { ContactStatus::Ok } else { ContactStatus::Request })?;
                self.add_msg(
                    &key,
                    Message {
                        id: rid,
                        chat: key.clone(),
                        author: from.clone(),
                        from_me: mine,
                        ts,
                        body: MessageBody::Text(clamp_keep(&r.content, 6000)),
                        status: mine.then_some(Delivery::Sent),
                        reply: reply_ref(r),
                        reacts: BTreeMap::new(),
                        exp: exp_of(r),
                    },
                    !mine,
                )?;
                if !mine {
                    self.maybe_prof_req(&peer);
                }
                Ok(())
            }
            KIND_REACTION => self.apply_reaction(&key, r, &from),
            KIND_APP => {
                let Some(app) = AppMessage::parse(&r.content) else { return Ok(()) };
                self.on_dm_app(app, r, &key, &peer, &from, mine, eph, known.as_ref())
            }
            _ => Ok(()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn on_dm_app(
        self: &Arc<Self>,
        app: AppMessage,
        r: &UnsignedEvent,
        key: &str,
        peer: &str,
        from: &str,
        mine: bool,
        eph: bool,
        known: Option<&Contact>,
    ) -> Result<()> {
        let rid = r.id.map(|i| i.to_hex()).unwrap_or_default();
        let ts = r.created_at.as_secs() as i64 * 1000;
        let aceito = known.is_some_and(|c| c.status == ContactStatus::Ok);
        match app {
            AppMessage::ProfileBanner { b } if !mine => {
                if known.is_none() {
                    return Ok(());
                }
                let snap = {
                    let mut st = self.state.lock().unwrap();
                    let Some(ct) = st.contacts.get_mut(peer) else { return Ok(()) };
                    match media::clean_banner(&b) {
                        Pic::Set(x) => ct.banner = Some(x),
                        Pic::Remove => ct.banner = None,
                        Pic::Invalid => return Ok(()),
                    }
                    ct.clone()
                };
                self.save_contact(&snap)?;
                let _ = self.events.send(EngineEvent::ContactChanged { pk: peer.into() });
            }
            AppMessage::Profile { n, s, c, p, r: relays, g } if !mine => {
                self.ensure_contact(peer, ContactStatus::Request)?;
                let snapshot = {
                    let mut st = self.state.lock().unwrap();
                    let Some(ct) = st.contacts.get_mut(peer) else { return Ok(()) };
                    ct.name = clamp(&n, 40);
                    ct.note = clamp(&s, 80);
                    if c < 6 {
                        ct.tone = c;
                    }
                    match p.as_deref().map(media::clean_pic) {
                        Some(Pic::Set(pic)) => ct.pic = Some(pic),
                        Some(Pic::Remove) => ct.pic = None,
                        _ => {}
                    }
                    ct.relays = relays.iter().filter_map(|u| normalize_url(u)).take(5).collect();
                    if let Some(g) = &g {
                        ct.games = media::clean_games(g);
                    }
                    ct.clone()
                };
                self.save_contact(&snapshot)?;
                if !snapshot.relays.is_empty() {
                    let pool = Arc::clone(&self.pool);
                    let rel = snapshot.relays.clone();
                    tokio::spawn(async move { pool.add_extra_relays(&rel).await });
                }
                let _ = self.events.send(EngineEvent::ContactChanged { pk: peer.into() });
                let _ = self.events.send(EngineEvent::ChatsChanged);
            }
            AppMessage::ProfileRequest if !mine => {
                // Só contatos aceitos recebem o perfil: um desconhecido não
                // descobre seu nome e sua foto só por pedir.
                if aceito {
                    let me = Arc::clone(self);
                    let p = peer.to_string();
                    tokio::spawn(async move {
                        let _ = me.send_profile(&p).await;
                    });
                }
            }
            AppMessage::Receipt { s, ids } if !mine => {
                let status = match s {
                    ReceiptKind::Delivered => Delivery::Delivered,
                    ReceiptKind::Read => Delivery::Read,
                };
                self.apply_receipts(key, &ids.into_iter().take(200).collect::<Vec<_>>(), status)?;
            }
            AppMessage::Listening(l) if !mine => {
                if !eph || !aceito {
                    return Ok(());
                }
                {
                    let mut st = self.state.lock().unwrap();
                    match l.clean() {
                        Some(l) => st.listening.insert(peer.to_string(), (l, now_ms())),
                        None => st.listening.remove(peer),
                    };
                }
                let _ = self.events.send(EngineEvent::Listening { pk: peer.into() });
            }
            AppMessage::Typing if !mine => {
                if eph && self.settings().typing {
                    let _ = self.events.send(EngineEvent::Typing { chat: key.into(), who: peer.into() });
                }
            }
            AppMessage::Delete { ids } => {
                self.apply_delete(key, &ids.into_iter().take(100).collect::<Vec<_>>(), from)?;
            }
            AppMessage::Ttl { v } => {
                let v = (v as i64).clamp(0, 28 * DAY) as u32;
                {
                    let store = self.store.lock().unwrap();
                    let mut c = chat_for(&store, key);
                    c.ttl = v;
                    save_chat(&store, &c)?;
                }
                self.add_msg(
                    key,
                    sys_msg(&rid, key, from, mine, ts, "ttl", v as i64),
                    false,
                )?;
            }
            AppMessage::MediaHeader(h) => self.on_media_header(key, r, &h, from, mine)?,
            AppMessage::MediaChunk { mid, i, d } => self.on_chunk(&mid, i, &d)?,
            AppMessage::Sound { id } => {
                if !mine {
                    self.ensure_contact(peer, ContactStatus::Request)?;
                }
                self.add_msg(
                    key,
                    Message {
                        id: rid,
                        chat: key.into(),
                        author: from.into(),
                        from_me: mine,
                        ts,
                        body: MessageBody::Sound { id: clamp(&id, 20) },
                        status: mine.then_some(Delivery::Sent),
                        reply: None,
                        reacts: BTreeMap::new(),
                        exp: exp_of(r),
                    },
                    !mine,
                )?;
            }
            AppMessage::GroupInvite { g } => {
                // Convite só vale de contato aceito: um estranho não consegue
                // enfiar você num grupo.
                if !mine && !aceito {
                    return Ok(());
                }
                let Some(inv) = parse_invite(&g) else { return Ok(()) };
                self.add_msg(
                    key,
                    Message {
                        id: rid,
                        chat: key.into(),
                        author: from.into(),
                        from_me: mine,
                        ts,
                        body: MessageBody::Invite {
                            lid: inv.lid.clone(),
                            name: inv.meta.name.clone(),
                            joined: false,
                            data: Some(Box::new(inv)),
                        },
                        status: mine.then_some(Delivery::Sent),
                        reply: None,
                        reacts: BTreeMap::new(),
                        exp: None,
                    },
                    !mine,
                )?;
            }
            AppMessage::GroupRekey { lid, old, sk, meta, ver } if !mine => {
                self.apply_rekey(from, &lid, &old, &sk, meta, ver);
            }
            AppMessage::Call(sig) if !mine => {
                if eph {
                    let _ = self.events.send(EngineEvent::CallSignal { from: peer.into(), signal: sig });
                } else if sig.a == CallAction::Missed {
                    let video = sig.v.unwrap_or(0) == 1;
                    self.add_msg(key, sys_msg(&rid, key, from, false, ts, "missed", video as i64), true)?;
                    let _ = self.events.send(EngineEvent::MissedCall { from: peer.into(), video, ts });
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_group_rumor(self: &Arc<Self>, r: &UnsignedEvent, lid: &str, eph: bool) -> Result<()> {
        let Some(g) = self.group(lid) else { return Ok(()) };
        let eu = self.pk_hex();
        let from = r.pubkey.to_hex();
        let mine = from == eu;
        let ch_tag = protocol::tag_value(r, "ch").map(|s| s.to_string());
        let canal = match &ch_tag {
            Some(c) if g.meta.text_channels().any(|x| &x.id == c) => Some(c.clone()),
            _ => g.meta.text_channels().next().map(|c| c.id.clone()),
        };
        let key = canal.map(|c| ChatKey::Channel { lid: lid.into(), channel: c }.as_string());
        // Canal apagado por um admin: o que chegar atrasado para ele some.
        let gone = ch_tag.as_ref().is_some_and(|c| g.gone.contains(c));
        let rid = r.id.map(|i| i.to_hex()).unwrap_or_default();
        let ts = r.created_at.as_secs() as i64 * 1000;

        match r.kind.as_u16() {
            KIND_TEXT => {
                let Some(key) = key.filter(|_| !gone) else { return Ok(()) };
                self.add_msg(
                    &key,
                    Message {
                        id: rid,
                        chat: key.clone(),
                        author: from,
                        from_me: mine,
                        ts,
                        body: MessageBody::Text(clamp_keep(&r.content, 6000)),
                        status: mine.then_some(Delivery::Sent),
                        reply: reply_ref(r),
                        reacts: BTreeMap::new(),
                        exp: exp_of(r),
                    },
                    !mine,
                )?;
                Ok(())
            }
            KIND_REACTION => match key.filter(|_| !gone) {
                Some(key) => self.apply_reaction(&key, r, &from),
                None => Ok(()),
            },
            KIND_APP => {
                let Some(app) = AppMessage::parse(&r.content) else { return Ok(()) };
                self.on_group_app(app, g, r, key, gone, &from, mine, eph, &rid, ts)
            }
            _ => Ok(()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn on_group_app(
        self: &Arc<Self>,
        app: AppMessage,
        mut g: Group,
        r: &UnsignedEvent,
        key: Option<String>,
        gone: bool,
        from: &str,
        mine: bool,
        eph: bool,
        rid: &str,
        ts: i64,
    ) -> Result<()> {
        let eu = self.pk_hex();
        match app {
            AppMessage::GroupMeta { meta, ver } => {
                // Só quem manda no grupo muda os dados dele, e a versão tem que
                // ser mais nova: senão uma mensagem antiga desfaria uma mudança.
                if mine || !g.meta.is_admin(from) || ver <= g.ver {
                    return Ok(());
                }
                let Some(nova) = clean_meta(meta, &g.meta.owner) else { return Ok(()) };
                if nova.owner != g.meta.owner && from != g.meta.owner {
                    return Ok(());
                }
                let era_membro = g.meta.members.contains(&eu);
                let removidos: Vec<String> = g
                    .meta
                    .text_channels()
                    .filter(|c| !nova.channels.iter().any(|x| x.id == c.id))
                    .map(|c| c.id.clone())
                    .collect();
                g.meta = nova;
                g.ver = ver;
                self.save_group(&g)?;
                for ch in removidos {
                    self.purge_channel(&g.lid, &ch)?;
                }
                if era_membro && !g.meta.members.contains(&eu) {
                    self.sys_group(&g.lid, "kicked", from, ts, None)?;
                }
                self.ensure_group_chats(&g.lid)?;
                let _ = self.events.send(EngineEvent::GroupsChanged);
            }
            AppMessage::GroupPic { p, v } => {
                if mine || !g.meta.is_admin(from) || v <= g.pic_v {
                    return Ok(());
                }
                let pic = match media::clean_pic(&p) {
                    Pic::Set(s) => Some(s),
                    Pic::Remove => None,
                    Pic::Invalid => return Ok(()),
                };
                let tem = pic.is_some();
                g.pic = pic;
                g.pic_v = v;
                self.save_group(&g)?;
                self.sys_group(&g.lid, if tem { "gpic" } else { "gpicdel" }, from, ts, Some(rid))?;
                let _ = self.events.send(EngineEvent::GroupsChanged);
            }
            AppMessage::GroupJoin { n, p } => {
                let nome = clamp(&n, 40);
                let novo = !g.meta.members.iter().any(|m| m == from);
                if !nome.is_empty() {
                    g.meta.names.insert(from.into(), nome);
                }
                if !mine {
                    match p.as_deref().map(media::clean_pic) {
                        Some(Pic::Set(pic)) => {
                            g.pics.insert(from.into(), pic);
                        }
                        Some(Pic::Remove) => {
                            g.pics.remove(from);
                        }
                        _ => {}
                    }
                }
                // Quem entra aparece na lista quando um admin publica os dados
                // novos; o admin que vê a entrada é quem faz isso.
                if novo && g.meta.is_admin(&eu) && g.meta.members.len() < 64 {
                    g.meta.members.push(from.into());
                    g.ver += 1;
                    self.save_group(&g)?;
                    let me = Arc::clone(self);
                    let lid = g.lid.clone();
                    let foto = g.pic.clone().map(|p| (p, g.pic_v.max(1)));
                    tokio::spawn(async move {
                        let _ = me.publish_meta(&lid).await;
                        // Quem acabou de entrar recebe a foto atual do grupo.
                        if let Some((p, v)) = foto {
                            let _ = me.send_group_app(&lid, &AppMessage::GroupPic { p, v }, BuildOpts::default()).await;
                        }
                    });
                } else {
                    self.save_group(&g)?;
                }
                if novo && !mine {
                    self.sys_group(&g.lid, "join", from, ts, Some(rid))?;
                }
                let _ = self.events.send(EngineEvent::GroupsChanged);
            }
            AppMessage::GroupLeave => {
                if mine || !g.meta.members.iter().any(|m| m == from) {
                    return Ok(());
                }
                g.meta.members.retain(|m| m != from);
                g.meta.admins.retain(|m| m != from);
                if g.meta.is_admin(&eu) {
                    g.ver += 1;
                    self.save_group(&g)?;
                    let me = Arc::clone(self);
                    let lid = g.lid.clone();
                    tokio::spawn(async move {
                        let _ = me.publish_meta(&lid).await;
                    });
                } else {
                    self.save_group(&g)?;
                }
                self.sys_group(&g.lid, "leave", from, ts, Some(rid))?;
                let _ = self.events.send(EngineEvent::GroupsChanged);
            }
            AppMessage::MediaHeader(h) => {
                if let Some(key) = key.filter(|_| !gone) {
                    self.on_media_header(&key, r, &h, from, mine)?;
                }
            }
            AppMessage::MediaChunk { mid, i, d } => self.on_chunk(&mid, i, &d)?,
            AppMessage::Sound { id } => {
                if let Some(key) = key.filter(|_| !gone) {
                    self.add_msg(
                        &key,
                        Message {
                            id: rid.into(),
                            chat: key.clone(),
                            author: from.into(),
                            from_me: mine,
                            ts,
                            body: MessageBody::Sound { id: clamp(&id, 20) },
                            status: mine.then_some(Delivery::Sent),
                            reply: None,
                            reacts: BTreeMap::new(),
                            exp: None,
                        },
                        !mine,
                    )?;
                }
            }
            AppMessage::Delete { ids } => {
                if let Some(key) = key.filter(|_| !gone) {
                    self.apply_delete(&key, &ids.into_iter().take(100).collect::<Vec<_>>(), from)?;
                }
            }
            AppMessage::Room { ch, a } => {
                if eph && !mine {
                    self.on_room(&g, &ch, from, a);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Guarda uma mensagem, atualiza a conversa e cuida dos recibos.
    ///
    /// Devolve `false` se a mensagem já existia ou já venceu.
    fn add_msg(&self, key: &str, msg: Message, incoming: bool) -> Result<bool> {
        if msg.exp.is_some_and(|e| e <= now_ms()) {
            return Ok(false);
        }
        let stored = to_stored(&msg);
        let (visible, peer_ok, peer) = {
            let st = self.state.lock().unwrap();
            let visible = st.active.as_deref() == Some(key) && st.visible;
            let peer = match ChatKey::parse(key) {
                Some(ChatKey::Direct { peer }) => Some(peer),
                _ => None,
            };
            let peer_ok = peer
                .as_ref()
                .and_then(|p| st.contacts.get(p))
                .is_some_and(|c| c.status == ContactStatus::Ok);
            (visible, peer_ok, peer)
        };
        let conta = incoming && !msg.body.is_system();
        let mut fila_d = false;
        let mut fila_r = false;
        {
            let store = self.store.lock().unwrap();
            if !store.put_msg(&stored)? {
                return Ok(false);
            }
            let mut c = chat_for(&store, key);
            if msg.ts >= c.last_ts {
                c.last_ts = msg.ts;
                c.last_from = Some(msg.author.clone());
                c.last_id = Some(msg.id.clone());
                c.last_kind = Some(msg.body.kind_name().into());
                c.last_text = self.preview(&msg);
                c.last_status = msg.status.map(|s| delivery_name(s).into());
            }
            if conta && !visible {
                c.unread += 1;
            }
            if conta && peer.is_some() {
                // Pedidos de conversa não recebem recibo até você aceitar.
                if peer_ok {
                    fila_d = true;
                }
                if peer_ok && visible && self.settings().read_receipts {
                    fila_r = true;
                } else {
                    c.pending_read.push(msg.id.clone());
                    let excesso = c.pending_read.len().saturating_sub(200);
                    c.pending_read.drain(..excesso);
                }
            }
            c.deleted = false;
            save_chat(&store, &c)?;
        }
        if let Some(p) = &peer {
            if fila_d {
                self.queue_receipt(p, &msg.id, false);
            }
            if fila_r {
                self.queue_receipt(p, &msg.id, true);
            }
        }
        let chat = key.to_string();
        let _ = self.events.send(EngineEvent::Message { chat: chat.clone(), message: Box::new(msg.clone()) });
        if conta {
            let _ = self.events.send(EngineEvent::Incoming { chat, message: Box::new(msg), visible });
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(true)
    }

    /// Texto curto para a lista de conversas e para notificações.
    pub fn preview(&self, m: &Message) -> String {
        match &m.body {
            MessageBody::Text(t) => t.clone(),
            MessageBody::Media(md) => match (&md.sb, md.kind) {
                (Some(sb), _) => format!("Som: {}", sb.n),
                (None, MediaKind::Audio) => "Mensagem de voz".into(),
                (None, MediaKind::Image) if md.mime.to_ascii_lowercase().starts_with("image/gif") => "GIF".into(),
                (None, MediaKind::Image) => "Foto".into(),
            },
            MessageBody::Sound { .. } => "Som do soundboard".into(),
            MessageBody::Invite { name, .. } => format!("Convite para {name}"),
            MessageBody::Deleted => "Mensagem apagada".into(),
            MessageBody::System { sys, v, legacy } => {
                if let Some(t) = legacy {
                    return t.clone();
                }
                let quem = self.name_of(&m.author);
                match sys.as_str() {
                    "missed" => "Chamada perdida".into(),
                    "ttl" => {
                        if *v > 0 {
                            "Mensagens temporárias ativadas".into()
                        } else {
                            "Mensagens temporárias desativadas".into()
                        }
                    }
                    "created" => "Grupo criado".into(),
                    "joined" => "Você entrou no grupo".into(),
                    "join" => format!("{quem} entrou no grupo"),
                    "leave" => format!("{quem} saiu do grupo"),
                    "removed" => format!("{quem} foi removido"),
                    "kicked" => "Você foi removido do grupo".into(),
                    "gpic" => format!("{quem} mudou a foto do grupo"),
                    "gpicdel" => format!("{quem} removeu a foto do grupo"),
                    _ => "Aviso".into(),
                }
            }
        }
    }

    /// Texto completo de um aviso do sistema, para a conversa.
    pub fn system_text(&self, m: &Message) -> String {
        if let MessageBody::System { sys, v, legacy: None } = &m.body {
            match sys.as_str() {
                "missed" => {
                    return if *v == 1 { "Chamada de vídeo perdida".into() } else { "Chamada de voz perdida".into() };
                }
                "ttl" => {
                    let quem = if m.from_me { "Você".to_string() } else { self.name_of(&m.author) };
                    return if *v > 0 {
                        format!("{quem} ativou mensagens temporárias: {}", ttl_label(*v as u32))
                    } else {
                        format!("{quem} desativou as mensagens temporárias")
                    };
                }
                _ => {}
            }
        }
        self.preview(m)
    }

    fn apply_reaction(&self, key: &str, r: &UnsignedEvent, from: &str) -> Result<()> {
        let Some(alvo) = protocol::tag_value(r, "e").filter(|t| is_hex64(t)) else { return Ok(()) };
        let msg = {
            let store = self.store.lock().unwrap();
            let Some(mut sm) = store.get_msg(alvo)? else { return Ok(()) };
            if sm.chat != key {
                return Ok(());
            }
            let mut reacts: BTreeMap<String, String> = serde_json::from_str(&sm.reacts).unwrap_or_default();
            let e = clamp_keep(&r.content, 16);
            if e.is_empty() || e == "-" {
                reacts.remove(from);
            } else {
                reacts.insert(from.into(), e);
            }
            sm.reacts = serde_json::to_string(&reacts).unwrap_or_else(|_| "{}".into());
            store.update_msg(&sm)?;
            from_stored(sm, &self.pk_hex())
        };
        let _ = self.events.send(EngineEvent::MessageChanged { chat: key.into(), message: Box::new(msg) });
        Ok(())
    }

    fn apply_receipts(&self, key: &str, ids: &[String], status: Delivery) -> Result<()> {
        let eu = self.pk_hex();
        for id in ids {
            if !is_hex64(id) {
                continue;
            }
            let store = self.store.lock().unwrap();
            let Some(m) = store.get_msg(id)? else { continue };
            if m.chat != key || m.author != eu {
                continue;
            }
            let atual = m.status.as_deref().and_then(parse_delivery).unwrap_or(Delivery::Pending);
            if atual >= status {
                continue;
            }
            store.set_msg_status(id, delivery_name(status))?;
            let mut c = chat_for(&store, key);
            if c.last_id.as_deref() == Some(id.as_str()) {
                c.last_status = Some(delivery_name(status).into());
                save_chat(&store, &c)?;
            }
            drop(store);
            let _ = self.events.send(EngineEvent::MessageUpdated { chat: key.into(), id: id.clone(), status });
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /// Apaga para todos: a mensagem vira "Mensagem apagada" — só se quem pediu
    /// for quem escreveu.
    fn apply_delete(&self, key: &str, ids: &[String], from: &str) -> Result<()> {
        let eu = self.pk_hex();
        for id in ids {
            if !is_hex64(id) {
                continue;
            }
            let msg = {
                let store = self.store.lock().unwrap();
                let Some(m) = store.get_msg(id)? else { continue };
                if m.chat != key || m.author != from {
                    continue;
                }
                if let MessageBody::Media(md) = MessageBody::from_stored(&m.kind, &m.body) {
                    store.delete_media(&md.mid)?;
                }
                let t = StoredMsg {
                    kind: "deleted".into(),
                    body: "{}".into(),
                    reacts: "{}".into(),
                    ..m
                };
                store.update_msg(&t)?;
                let mut c = chat_for(&store, key);
                if c.last_id.as_deref() == Some(id.as_str()) {
                    c.last_text = "Mensagem apagada".into();
                    c.last_kind = Some("deleted".into());
                    save_chat(&store, &c)?;
                }
                from_stored(t, &eu)
            };
            let _ = self.events.send(EngineEvent::MessageChanged { chat: key.into(), message: Box::new(msg) });
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /* ---------------- recibos ---------------- */

    fn queue_receipt(&self, pk: &str, id: &str, read: bool) {
        let agendar = {
            let mut st = self.state.lock().unwrap();
            let q = st.rq.entry(pk.to_string()).or_default();
            if read {
                q.r.insert(id.to_string());
            } else {
                q.d.insert(id.to_string());
            }
            let ag = !st.rq_scheduled;
            st.rq_scheduled = true;
            ag
        };
        if agendar {
            // Um recibo por mensagem seria um envelope por mensagem: juntam-se
            // os de 1,2 s num só, como no celular.
            let state = Arc::clone(&self.state);
            let store = Arc::clone(&self.store);
            let pool = Arc::clone(&self.pool);
            let keys = self.keys.clone();
            let read_ok = self.settings().read_receipts;
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
                let fila: Vec<(String, ReceiptQueue)> = {
                    let mut st = state.lock().unwrap();
                    st.rq_scheduled = false;
                    let todos: Vec<(String, ReceiptQueue)> = st.rq.drain().collect();
                    todos
                        .into_iter()
                        .filter(|(pk, _)| st.contacts.get(pk).is_some_and(|c| c.status != ContactStatus::Blocked))
                        .collect()
                };
                for (pk, q) in fila {
                    let Ok(peer) = PublicKey::from_hex(&pk) else { continue };
                    for (kind, ids) in [(ReceiptKind::Delivered, q.d), (ReceiptKind::Read, q.r)] {
                        if ids.is_empty() || (kind == ReceiptKind::Read && !read_ok) {
                            continue;
                        }
                        let app = AppMessage::Receipt { s: kind, ids: ids.into_iter().collect() };
                        let rumor = protocol::make_rumor(&keys, KIND_APP, app.to_json(), vec![Tag::public_key(peer)]);
                        let opts = WrapOptions {
                            ephemeral: false,
                            expiration: Some(Timestamp::from_secs((now_secs() + 3 * DAY) as u64)),
                        };
                        if let Ok(w) = protocol::wrap_rumor(&rumor, &keys, &peer, opts) {
                            if pool.publish(&w).await.is_err() {
                                let wraps = serde_json::to_string(&vec![w.as_json()]).unwrap_or_default();
                                let id = rumor.id.map(|i| i.to_hex()).unwrap_or_default();
                                let _ = store.lock().unwrap().queue_outbox(&id, &wraps, "[]", now_secs());
                            }
                        }
                    }
                }
            });
        }
    }

    fn maybe_prof_req(self: &Arc<Self>, pk: &str) {
        {
            let st = self.state.lock().unwrap();
            match st.contacts.get(pk) {
                Some(c) if c.name.is_empty() && c.status == ContactStatus::Ok => {}
                _ => return,
            }
        }
        let agora = now_ms();
        {
            let mut st = self.state.lock().unwrap();
            let ultimo = st.prof_req_at.get(pk).copied().unwrap_or(0);
            if agora - ultimo < 6 * 3600 * 1000 {
                return;
            }
            st.prof_req_at.insert(pk.to_string(), agora);
        }
        let me = Arc::clone(self);
        let chat = ChatKey::Direct { peer: pk.to_string() }.as_string();
        tokio::spawn(async move {
            let _ = me
                .send_app(&chat, &AppMessage::ProfileRequest, BuildOpts { no_self_copy: true, no_ttl: true, expires_in: Some(3 * DAY), ..Default::default() })
                .await;
        });
    }

    /* ---------------- envio ---------------- */

    /// Monta o rumor e os envelopes de uma mensagem para uma conversa.
    fn build(&self, chat_key: &str, kind: u16, content: String, extra: Vec<Tag>, o: BuildOpts) -> Result<Built> {
        let key = ChatKey::parse(chat_key).ok_or_else(|| EngineError::UnknownChat(chat_key.into()))?;
        let (recip, mut tags, relays, dm) = match &key {
            ChatKey::Direct { peer } => {
                let pk = PublicKey::from_hex(peer).map_err(|_| EngineError::UnknownChat(chat_key.into()))?;
                let rel = self.contact(peer).map(|c| c.relays).unwrap_or_default();
                (pk, vec![Tag::public_key(pk)], rel, true)
            }
            ChatKey::Channel { lid, channel } => {
                let g = self.group(lid).ok_or_else(|| erro("Grupo indisponível"))?;
                let pk = PublicKey::from_hex(&g.pk).map_err(|_| erro("chave do grupo inválida"))?;
                (pk, vec![Tag::public_key(pk), Tag::custom("ch", [channel.clone()])], Vec::new(), false)
            }
        };
        tags.extend(extra);
        let ttl = if o.no_ttl {
            0
        } else {
            let store = self.store.lock().unwrap();
            chat_for(&store, chat_key).ttl as i64
        };
        let exp = (ttl > 0).then(|| now_secs() + ttl);
        if let Some(e) = exp {
            tags.push(Tag::expiration(Timestamp::from_secs(e as u64)));
        }
        let rumor = protocol::make_rumor(&self.keys, kind, content, tags);
        let id = rumor.id.map(|i| i.to_hex()).unwrap_or_default();
        let wrap_exp = match o.expires_in {
            Some(s) => Some(now_secs() + s),
            None => exp.map(|e| e + 3600),
        }
        .map(|s| Timestamp::from_secs(s as u64));
        let opts = WrapOptions { ephemeral: o.ephemeral, expiration: wrap_exp };
        let mut wraps = vec![protocol::wrap_rumor(&rumor, &self.keys, &recip, opts).map_err(|e| erro(e.to_string()))?];
        if !o.no_self_copy && dm && !o.ephemeral {
            wraps.push(
                protocol::wrap_rumor(&rumor, &self.keys, &self.keys.public_key(), WrapOptions { ephemeral: false, expiration: wrap_exp })
                    .map_err(|e| erro(e.to_string()))?,
            );
        }
        Ok(Built {
            id,
            created_ms: rumor.created_at.as_secs() as i64 * 1000,
            wraps,
            relays,
            exp_ms: exp.map(|e| e * 1000),
        })
    }

    /// Publica. Se nenhum servidor aceitar, guarda para reenviar.
    async fn publish(&self, b: &Built) -> bool {
        if !b.relays.is_empty() {
            self.pool.add_extra_relays(&b.relays).await;
        }
        let ok = self.pool.publish(&b.wraps[0]).await.is_ok();
        for w in &b.wraps[1..] {
            let pool = Arc::clone(&self.pool);
            let w = w.clone();
            tokio::spawn(async move {
                let _ = pool.publish(&w).await;
            });
        }
        if !ok {
            let wraps = serde_json::to_string(&b.wraps.iter().map(|w| w.as_json()).collect::<Vec<_>>()).unwrap_or_default();
            let relays = serde_json::to_string(&b.relays).unwrap_or_else(|_| "[]".into());
            let _ = self.store.lock().unwrap().queue_outbox(&b.id, &wraps, &relays, now_secs());
        }
        ok
    }

    async fn send_app(&self, chat_key: &str, app: &AppMessage, o: BuildOpts) -> Result<String> {
        let b = self.build(chat_key, KIND_APP, app.to_json(), Vec::new(), o)?;
        if o.ephemeral {
            let _ = self.pool.publish(&b.wraps[0]).await;
        } else {
            self.publish(&b).await;
        }
        Ok(b.id)
    }

    /// Guarda como "enviando", publica e marca como enviada se saiu.
    async fn local_send(&self, chat_key: &str, mut msg: Message, b: &Built) -> Result<Message> {
        msg.status = Some(Delivery::Pending);
        msg.exp = b.exp_ms;
        self.add_msg(chat_key, msg.clone(), false)?;
        if self.publish(b).await {
            self.set_status(chat_key, &b.id, Delivery::Sent)?;
            msg.status = Some(Delivery::Sent);
        }
        Ok(msg)
    }

    fn set_status(&self, chat_key: &str, id: &str, status: Delivery) -> Result<()> {
        let store = self.store.lock().unwrap();
        let Some(m) = store.get_msg(id)? else { return Ok(()) };
        let atual = m.status.as_deref().and_then(parse_delivery).unwrap_or(Delivery::Pending);
        if atual >= status {
            return Ok(());
        }
        store.set_msg_status(id, delivery_name(status))?;
        let mut c = chat_for(&store, chat_key);
        if c.last_id.as_deref() == Some(id) {
            c.last_status = Some(delivery_name(status).into());
            save_chat(&store, &c)?;
        }
        drop(store);
        let _ = self.events.send(EngineEvent::MessageUpdated { chat: chat_key.into(), id: id.into(), status });
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /// Reenvia o que ficou na fila por falta de internet.
    pub async fn flush_outbox(&self) {
        if self.pool.open_count().await == 0 {
            return;
        }
        let fila = self.store.lock().unwrap().outbox().unwrap_or_default();
        for (id, wraps_json, relays_json) in fila {
            let wraps: Vec<String> = serde_json::from_str(&wraps_json).unwrap_or_default();
            let relays: Vec<String> = serde_json::from_str(&relays_json).unwrap_or_default();
            let evs: Vec<Event> = wraps.iter().filter_map(|j| Event::from_json(j).ok()).collect();
            let Some(primeiro) = evs.first() else {
                let _ = self.store.lock().unwrap().drop_outbox(&id);
                continue;
            };
            if !relays.is_empty() {
                self.pool.add_extra_relays(&relays).await;
            }
            if self.pool.publish(primeiro).await.is_ok() {
                for w in &evs[1..] {
                    let _ = self.pool.publish(w).await;
                }
                let _ = self.store.lock().unwrap().drop_outbox(&id);
                let chat = self.store.lock().unwrap().get_msg(&id).ok().flatten().map(|m| m.chat);
                if let Some(chat) = chat {
                    let _ = self.set_status(&chat, &id, Delivery::Sent);
                }
            }
        }
    }

    /// Manda uma mensagem de texto.
    pub async fn send_text(self: &Arc<Self>, chat_key: &str, text: &str) -> Result<Message> {
        self.send_text_with(chat_key, text, None).await
    }

    /// Manda texto, respondendo a outra mensagem.
    pub async fn send_text_with(self: &Arc<Self>, chat_key: &str, text: &str, reply: Option<&str>) -> Result<Message> {
        let text: String = text.trim().chars().take(6000).collect();
        if text.is_empty() {
            return Err(erro("mensagem vazia"));
        }
        self.check_group_joined(chat_key)?;
        let tags = match reply.filter(|r| is_hex64(r)) {
            Some(r) => vec![Tag::parse(["e", r, "", "reply"]).map_err(|e| erro(e.to_string()))?],
            None => Vec::new(),
        };
        let b = self.build(chat_key, KIND_TEXT, text.clone(), tags, BuildOpts::default())?;
        self.ensure_dm_contact(chat_key).await?;
        let msg = Message {
            id: b.id.clone(),
            chat: chat_key.into(),
            author: self.pk_hex(),
            from_me: true,
            ts: b.created_ms,
            body: MessageBody::Text(text),
            status: None,
            reply: reply.map(|s| s.to_string()),
            reacts: BTreeMap::new(),
            exp: None,
        };
        self.local_send(chat_key, msg, &b).await
    }

    fn check_group_joined(&self, chat_key: &str) -> Result<()> {
        if let Some(ChatKey::Channel { lid, .. }) = ChatKey::parse(chat_key) {
            if self.group(&lid).is_none() {
                return Err(erro("Grupo indisponível"));
            }
        }
        Ok(())
    }

    /// Ao mandar a primeira mensagem para alguém, a pessoa vira contato e
    /// recebe o seu perfil: é assim que ela vê seu nome em vez do código.
    async fn ensure_dm_contact(self: &Arc<Self>, chat_key: &str) -> Result<()> {
        let Some(ChatKey::Direct { peer }) = ChatKey::parse(chat_key) else { return Ok(()) };
        let c = self.ensure_contact(&peer, ContactStatus::Ok)?;
        if c.status == ContactStatus::Request {
            self.set_contact_status(&peer, ContactStatus::Ok)?;
        }
        if !c.prof_sent {
            let _ = self.send_profile(&peer).await;
        }
        Ok(())
    }

    /// Manda um som de fábrica do soundboard para a conversa.
    pub async fn send_sound(self: &Arc<Self>, chat_key: &str, id: &str) -> Result<Message> {
        self.check_group_joined(chat_key)?;
        let b = self.build(chat_key, KIND_APP, AppMessage::Sound { id: id.into() }.to_json(), Vec::new(), BuildOpts::default())?;
        self.ensure_dm_contact(chat_key).await?;
        let msg = Message {
            id: b.id.clone(),
            chat: chat_key.into(),
            author: self.pk_hex(),
            from_me: true,
            ts: b.created_ms,
            body: MessageBody::Sound { id: id.into() },
            status: None,
            reply: None,
            reacts: BTreeMap::new(),
            exp: None,
        };
        self.local_send(chat_key, msg, &b).await
    }

    /// Reage a uma mensagem. Reagir de novo com o mesmo emoji tira a reação.
    pub async fn send_reaction(self: &Arc<Self>, chat_key: &str, msg_id: &str, emoji: &str) -> Result<()> {
        let eu = self.pk_hex();
        let (msg, conteudo) = {
            let store = self.store.lock().unwrap();
            let Some(mut sm) = store.get_msg(msg_id)? else { return Ok(()) };
            let mut reacts: BTreeMap<String, String> = serde_json::from_str(&sm.reacts).unwrap_or_default();
            let conteudo = if emoji.is_empty() || reacts.get(&eu).map(|s| s.as_str()) == Some(emoji) {
                reacts.remove(&eu);
                "-".to_string()
            } else {
                reacts.insert(eu.clone(), emoji.to_string());
                emoji.to_string()
            };
            sm.reacts = serde_json::to_string(&reacts).unwrap_or_else(|_| "{}".into());
            store.update_msg(&sm)?;
            (from_stored(sm, &eu), conteudo)
        };
        let _ = self.events.send(EngineEvent::MessageChanged { chat: chat_key.into(), message: Box::new(msg) });
        let tags = vec![
            Tag::parse(["e", msg_id]).map_err(|e| erro(e.to_string()))?,
            Tag::parse(["k", "14"]).map_err(|e| erro(e.to_string()))?,
        ];
        let b = self.build(chat_key, KIND_REACTION, conteudo, tags, BuildOpts { no_ttl: true, ..Default::default() })?;
        self.publish(&b).await;
        Ok(())
    }

    /// Apaga para todos (só as suas mensagens).
    pub async fn delete_for_all(self: &Arc<Self>, chat_key: &str, ids: &[String]) -> Result<()> {
        self.apply_delete(chat_key, ids, &self.pk_hex())?;
        self.send_app(chat_key, &AppMessage::Delete { ids: ids.to_vec() }, BuildOpts { no_ttl: true, ..Default::default() })
            .await?;
        Ok(())
    }

    /// Apaga só deste aparelho.
    pub fn delete_for_me(&self, chat_key: &str, ids: &[String]) -> Result<()> {
        for id in ids {
            let store = self.store.lock().unwrap();
            let Some(m) = store.get_msg(id)? else { continue };
            if m.chat != chat_key {
                continue;
            }
            if let MessageBody::Media(md) = MessageBody::from_stored(&m.kind, &m.body) {
                store.delete_media(&md.mid)?;
            }
            store.delete_msg(id)?;
            drop(store);
            let _ = self.events.send(EngineEvent::MessageDeleted { chat: chat_key.into(), id: id.clone() });
        }
        self.refresh_last(chat_key)?;
        Ok(())
    }

    fn refresh_last(&self, chat_key: &str) -> Result<()> {
        let eu = self.pk_hex();
        let store = self.store.lock().unwrap();
        let mut c = chat_for(&store, chat_key);
        if !c.existed {
            return Ok(());
        }
        match store.last_msg(chat_key)? {
            Some(sm) => {
                let m = from_stored(sm, &eu);
                c.last_ts = m.ts;
                c.last_id = Some(m.id.clone());
                c.last_from = Some(m.author.clone());
                c.last_kind = Some(m.body.kind_name().into());
                c.last_status = m.status.map(|s| delivery_name(s).into());
                drop(store);
                c.last_text = self.preview(&m);
                let store = self.store.lock().unwrap();
                save_chat(&store, &c)?;
            }
            None => {
                c.last_text.clear();
                c.last_id = None;
                save_chat(&store, &c)?;
            }
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /// Liga ou desliga mensagens temporárias numa conversa (segundos; 0 desliga).
    pub async fn set_ttl(self: &Arc<Self>, chat_key: &str, secs: u32) -> Result<()> {
        {
            let store = self.store.lock().unwrap();
            let mut c = chat_for(&store, chat_key);
            c.ttl = secs;
            save_chat(&store, &c)?;
        }
        if chat_key.starts_with("dm:") {
            let id = self
                .send_app(chat_key, &AppMessage::Ttl { v: secs }, BuildOpts { no_ttl: true, ..Default::default() })
                .await?;
            self.add_msg(chat_key, sys_msg(&id, chat_key, &self.pk_hex(), true, now_ms(), "ttl", secs as i64), false)?;
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    pub fn chat_ttl(&self, chat_key: &str) -> u32 {
        let store = self.store.lock().unwrap();
        chat_for(&store, chat_key).ttl
    }

    /// Avisa que você está digitando (no máximo a cada 4 s).
    pub async fn send_typing(self: &Arc<Self>, chat_key: &str) {
        if !self.settings().typing || !chat_key.starts_with("dm:") {
            return;
        }
        {
            let mut st = self.state.lock().unwrap();
            let ultimo = st.typing_at.get(chat_key).copied().unwrap_or(0);
            if now_ms() - ultimo < 4000 {
                return;
            }
            st.typing_at.insert(chat_key.into(), now_ms());
        }
        let _ = self
            .send_app(chat_key, &AppMessage::Typing, BuildOpts { ephemeral: true, no_ttl: true, no_self_copy: true, ..Default::default() })
            .await;
    }

    /// Diz qual conversa está aberta e se a janela está à vista. Abrir uma
    /// conversa visível marca como lida.
    pub async fn set_active(self: &Arc<Self>, chat: Option<&str>) -> Result<()> {
        let visible = self.state.lock().unwrap().visible;
        self.set_view(chat, visible)
    }

    pub fn set_view(&self, chat: Option<&str>, visible: bool) -> Result<()> {
        {
            let mut st = self.state.lock().unwrap();
            st.active = chat.map(|s| s.to_string());
            st.visible = visible;
        }
        if let (Some(c), true) = (chat, visible) {
            self.mark_read(c)?;
        }
        Ok(())
    }

    pub fn mark_read(&self, chat_key: &str) -> Result<()> {
        let (peer, pend) = {
            let store = self.store.lock().unwrap();
            let mut c = chat_for(&store, chat_key);
            if !c.existed {
                return Ok(());
            }
            let had = c.unread > 0 || !c.pending_read.is_empty();
            let pend = std::mem::take(&mut c.pending_read);
            c.unread = 0;
            if had {
                save_chat(&store, &c)?;
            }
            (c.peer.clone(), pend)
        };
        // Como no celular: abrir a conversa confirma a leitura do que estava
        // pendente (o envio pula só os bloqueados). O que os pedidos de
        // conversa não recebem é o recibo automático de entrega.
        if let Some(p) = peer {
            if self.settings().read_receipts {
                for id in pend {
                    self.queue_receipt(&p, &id, true);
                }
            }
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    pub async fn send_profile(&self, pk: &str) -> Result<()> {
        let me = self.me();
        let app = AppMessage::Profile {
            n: me.name.clone(),
            s: me.note.clone(),
            c: me.tone,
            p: Some(me.pic.clone().unwrap_or_default()),
            r: self.settings().relays.iter().take(5).cloned().collect(),
            g: Some(me.games.clone()),
        };
        let chat = ChatKey::Direct { peer: pk.to_string() }.as_string();
        self.send_app(&chat, &app, BuildOpts { no_self_copy: true, no_ttl: true, ..Default::default() }).await?;
        if let Some(b) = &me.banner {
            let ban = AppMessage::ProfileBanner { b: b.clone() };
            self.send_app(&chat, &ban, BuildOpts { no_self_copy: true, no_ttl: true, ..Default::default() }).await?;
        }
        // Quem acabou de virar contato já vê o que está tocando.
        if let Some(l) = self.my_listening() {
            let _ = self.send_app(&chat, &AppMessage::Listening(l), BuildOpts { ephemeral: true, no_ttl: true, no_self_copy: true, ..Default::default() }).await;
        }
        let snap = {
            let mut st = self.state.lock().unwrap();
            st.contacts.get_mut(pk).filter(|c| !c.prof_sent).map(|c| {
                c.prof_sent = true;
                c.clone()
            })
        };
        if let Some(c) = snap {
            self.save_contact(&c)?;
        }
        Ok(())
    }

    /// Muda nome, recado, cor ou foto, e avisa contatos e grupos.
    pub async fn update_profile(
        self: &Arc<Self>,
        name: Option<&str>,
        note: Option<&str>,
        tone: Option<u8>,
        pic: Option<&str>,
    ) -> Result<()> {
        {
            let mut me = self.me.write().unwrap();
            if let Some(n) = name {
                me.name = clamp(n, 40);
            }
            if let Some(n) = note {
                me.note = clamp(n, 80);
            }
            if let Some(t) = tone {
                me.tone = t % 6;
            }
            if let Some(p) = pic {
                match media::clean_pic(p) {
                    Pic::Set(s) => me.pic = Some(s),
                    Pic::Remove => me.pic = None,
                    Pic::Invalid => return Err(erro("foto inválida")),
                }
            }
            self.store.lock().unwrap().set_kv("me", &*me)?;
        }
        let _ = self.events.send(EngineEvent::ProfileChanged);
        let contatos: Vec<String> = self
            .contacts()
            .into_iter()
            .filter(|c| c.status == ContactStatus::Ok)
            .map(|c| c.pk)
            .collect();
        let grupos: Vec<String> = self.groups().into_iter().map(|g| g.lid).collect();
        let me = Arc::clone(self);
        tokio::spawn(async move {
            for pk in contatos {
                let _ = me.send_profile(&pk).await;
            }
            let p = me.me();
            for lid in grupos {
                let _ = me
                    .send_group_app(&lid, &AppMessage::GroupJoin { n: p.name.clone(), p: Some(p.pic.clone().unwrap_or_default()) }, BuildOpts::default())
                    .await;
            }
        });
        Ok(())
    }

    /// Banner e jogos favoritos. `banner` vazio tira o banner. Os contatos
    /// aceitos recebem a mudança.
    pub async fn update_profile_extras(self: &Arc<Self>, banner: Option<&str>, games: Option<&[String]>) -> Result<()> {
        let tirou_banner;
        {
            let mut me = self.me.write().unwrap();
            let antes = me.banner.is_some();
            if let Some(b) = banner {
                match media::clean_banner(b) {
                    Pic::Set(s) => me.banner = Some(s),
                    Pic::Remove => me.banner = None,
                    Pic::Invalid => return Err(erro("banner inválido")),
                }
            }
            if let Some(g) = games {
                me.games = media::clean_games(g);
            }
            tirou_banner = antes && me.banner.is_none();
            self.store.lock().unwrap().set_kv("me", &*me)?;
        }
        let _ = self.events.send(EngineEvent::ProfileChanged);
        let contatos: Vec<String> = self.contacts().into_iter().filter(|c| c.status == ContactStatus::Ok).map(|c| c.pk).collect();
        let me = Arc::clone(self);
        tokio::spawn(async move {
            for pk in contatos {
                let _ = me.send_profile(&pk).await;
                if tirou_banner {
                    let chat = ChatKey::Direct { peer: pk.clone() }.as_string();
                    let _ = me
                        .send_app(&chat, &AppMessage::ProfileBanner { b: String::new() }, BuildOpts { no_self_copy: true, no_ttl: true, ..Default::default() })
                        .await;
                }
            }
        });
        Ok(())
    }

    /* ---------------- ouvindo agora ---------------- */

    /// O que um contato está ouvindo (some sozinho se ele parar de avisar).
    pub fn listening_of(&self, pk: &str) -> Option<Listening> {
        let st = self.state.lock().unwrap();
        let (l, quando) = st.listening.get(pk)?;
        listening_alive(l, *quando, now_ms()).then(|| l.clone())
    }

    /// O que eu estou mostrando agora.
    pub fn my_listening(&self) -> Option<Listening> {
        self.state.lock().unwrap().my_listening.as_ref().map(|x| x.0.clone())
    }

    /// Conta aos contatos aceitos o que você está ouvindo (`None`: parou ou
    /// não quer mostrar). Pode chamar sempre: só manda quando muda a música
    /// ou, tocando a mesma, a cada [`LISTENING_HEARTBEAT_MS`].
    pub async fn set_listening(self: &Arc<Self>, l: Option<Listening>) {
        let l = l.and_then(|x| x.clean());
        let agora = now_ms();
        let enviar = {
            let mut st = self.state.lock().unwrap();
            let antes = st.my_listening.clone();
            match (&antes, &l) {
                (None, None) => None,
                (Some(_), None) => {
                    st.my_listening = None;
                    Some(Listening::default())
                }
                (Some((a, quando)), Some(n)) if a.same_track(n) && agora - quando < LISTENING_HEARTBEAT_MS => None,
                (Some((a, _)), Some(n)) if a.same_track(n) => {
                    let mesma = a.clone();
                    st.my_listening = Some((mesma.clone(), agora));
                    Some(mesma)
                }
                (_, Some(n)) => {
                    st.my_listening = Some((n.clone(), agora));
                    Some(n.clone())
                }
            }
        };
        let Some(msg) = enviar else { return };
        let _ = self.events.send(EngineEvent::ProfileChanged);
        let contatos: Vec<String> = self.contacts().into_iter().filter(|c| c.status == ContactStatus::Ok).map(|c| c.pk).collect();
        let app = AppMessage::Listening(msg);
        for pk in contatos {
            let chat = ChatKey::Direct { peer: pk }.as_string();
            let _ = self.send_app(&chat, &app, BuildOpts { ephemeral: true, no_ttl: true, no_self_copy: true, ..Default::default() }).await;
        }
    }

    /* ---------------- mídia ---------------- */

    /// Manda foto, áudio ou som próprio. O cabeçalho sai na hora; os blocos
    /// seguem em segundo plano, avisando o progresso.
    pub async fn send_media(self: &Arc<Self>, chat_key: &str, bytes: Vec<u8>, meta: MediaMeta) -> Result<Message> {
        if bytes.len() > media::MAX_MEDIA {
            return Err(erro("Arquivo grande demais"));
        }
        if !media::mime_ok(&meta.mime) {
            return Err(erro("Formato não suportado"));
        }
        let kind = meta.kind.ok_or_else(|| erro("tipo de mídia não informado"))?;
        self.check_group_joined(chat_key)?;
        self.ensure_dm_contact(chat_key).await?;

        let mid = random_hex(16);
        let n = bytes.len().div_ceil(CHUNK) as u32;
        let sha = {
            use sha2::{Digest, Sha256};
            hex::encode(Sha256::digest(&bytes))
        };
        let sb = if kind == MediaKind::Audio { meta.sb.as_ref().map(|s| media::clean_sound_meta(&s.n, &s.e)) } else { None };
        let head = MediaHeader {
            mid: mid.clone(),
            kind,
            mime: meta.mime.clone(),
            size: bytes.len() as u64,
            n,
            sha: sha.clone(),
            dur: meta.dur.map(|d| d.round().max(0.0) as u32),
            wf: meta.wf.as_ref().map(|v| v.iter().take(48).map(|x| i64::from(*x)).collect()),
            w: meta.w.filter(|_| meta.h.is_some()),
            h: meta.h.filter(|_| meta.w.is_some()),
            th: meta.th.clone().filter(|t| t.len() < media::THUMB_MAX),
            sb: sb.clone(),
        };
        let b = self.build(chat_key, KIND_APP, AppMessage::MediaHeader(head.clone()).to_json(), Vec::new(), BuildOpts::default())?;
        self.store.lock().unwrap().put_media(&mid, &b.id, chat_key, &meta.mime, &bytes)?;
        let info = MediaInfo {
            mid: mid.clone(),
            kind,
            mime: meta.mime.clone(),
            size: bytes.len() as u64,
            n,
            sha,
            dur: head.dur,
            wf: meta.wf.as_ref().map(|v| v.iter().take(48).map(|x| (*x).min(31)).collect()),
            w: head.w,
            h: head.h,
            th: head.th.clone(),
            sb,
            got: n,
            ready: true,
            err: false,
        };
        let msg = Message {
            id: b.id.clone(),
            chat: chat_key.into(),
            author: self.pk_hex(),
            from_me: true,
            ts: b.created_ms,
            body: MessageBody::Media(info),
            status: None,
            reply: None,
            reacts: BTreeMap::new(),
            exp: None,
        };
        let msg = self.local_send(chat_key, msg, &b).await?;

        let me = Arc::clone(self);
        let chat = chat_key.to_string();
        let msg_id = b.id.clone();
        tokio::spawn(async move {
            for (i, pedaco) in bytes.chunks(CHUNK).enumerate() {
                let app = AppMessage::MediaChunk {
                    mid: mid.clone(),
                    i: i as u32,
                    d: base64::engine::general_purpose::STANDARD.encode(pedaco),
                };
                if let Ok(cb) = me.build(&chat, KIND_APP, app.to_json(), Vec::new(), BuildOpts { no_self_copy: true, ..Default::default() }) {
                    me.publish(&cb).await;
                }
                let _ = me.events.send(EngineEvent::Progress { chat: chat.clone(), id: msg_id.clone(), done: i as u32 + 1, total: n });
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;
            }
        });
        Ok(msg)
    }

    fn on_media_header(&self, key: &str, r: &UnsignedEvent, h: &MediaHeader, from: &str, mine: bool) -> Result<()> {
        let Some(mut info) = media::validate_header(h) else { return Ok(()) };
        if !mine && key.starts_with("dm:") {
            self.ensure_contact(from, ContactStatus::Request)?;
        }
        let rid = r.id.map(|i| i.to_hex()).unwrap_or_default();
        {
            let store = self.store.lock().unwrap();
            match store.media_record(&info.mid)? {
                Some((_, _, true)) => {
                    info.ready = true;
                    info.got = info.n;
                }
                Some(_) => {}
                None => store.put_media_pending(&info.mid, &rid, key, &info.mime)?,
            }
        }
        let ready = info.ready;
        let mid = info.mid.clone();
        let added = self.add_msg(
            key,
            Message {
                id: rid,
                chat: key.into(),
                author: from.into(),
                from_me: mine,
                ts: r.created_at.as_secs() as i64 * 1000,
                body: MessageBody::Media(info),
                status: mine.then_some(Delivery::Sent),
                reply: None,
                reacts: BTreeMap::new(),
                exp: exp_of(r),
            },
            !mine,
        )?;
        if added && !ready {
            self.try_assemble(&mid)?;
        }
        Ok(())
    }

    fn on_chunk(&self, mid: &str, i: u32, d: &str) -> Result<()> {
        if !media::chunk_ok(mid, i, d) {
            return Ok(());
        }
        {
            let store = self.store.lock().unwrap();
            if matches!(store.media_record(mid)?, Some((_, _, true))) {
                return Ok(());
            }
            store.put_chunk(mid, i, d)?;
        }
        self.try_assemble(mid)
    }

    /// Junta os blocos quando todos chegaram e confere o hash.
    fn try_assemble(&self, mid: &str) -> Result<()> {
        let eu = self.pk_hex();
        let store = self.store.lock().unwrap();
        let Some((Some(msg_id), _, false)) = store.media_record(mid)? else { return Ok(()) };
        let Some(mut sm) = store.get_msg(&msg_id)? else { return Ok(()) };
        let MessageBody::Media(mut info) = MessageBody::from_stored(&sm.kind, &sm.body) else { return Ok(()) };
        let chunks = store.chunks_of(mid)?;
        let n = info.n;
        info.got = (chunks.len() as u32).min(n);
        if (chunks.len() as u32) < n {
            sm.body = MessageBody::Media(info).to_json();
            store.update_msg(&sm)?;
            let chat = sm.chat.clone();
            let msg = from_stored(sm, &eu);
            drop(store);
            let _ = self.events.send(EngineEvent::MessageChanged { chat, message: Box::new(msg) });
            return Ok(());
        }
        let mut todos = Vec::with_capacity(info.size as usize);
        let mut esperado = 0u32;
        for (i, d) in &chunks {
            if *i != esperado {
                continue;
            }
            match base64::engine::general_purpose::STANDARD.decode(d) {
                Ok(b) => todos.extend_from_slice(&b),
                Err(_) => break,
            }
            esperado += 1;
            if esperado == n {
                break;
            }
        }
        let hash_ok = info.sha.is_empty() || {
            use sha2::{Digest, Sha256};
            hex::encode(Sha256::digest(&todos)) == info.sha
        };
        if esperado < n || !hash_ok {
            info.err = true;
        } else {
            store.put_media(mid, &msg_id, &sm.chat, &info.mime, &todos)?;
            info.ready = true;
            info.got = n;
        }
        sm.body = MessageBody::Media(info).to_json();
        store.update_msg(&sm)?;
        let chat = sm.chat.clone();
        let msg = from_stored(sm, &eu);
        drop(store);
        let _ = self.events.send(EngineEvent::MessageChanged { chat, message: Box::new(msg) });
        Ok(())
    }

    /// Bytes de uma mídia já completa.
    pub fn media_bytes(&self, mid: &str) -> Option<Vec<u8>> {
        self.store.lock().unwrap().media_bytes(mid).ok().flatten()
    }

    /* ---------------- mensagens temporárias ---------------- */

    /// Apaga deste aparelho as mensagens temporárias que já venceram.
    pub fn purge_expired(&self) {
        let vencidas = self.store.lock().unwrap().msgs_expired(now_ms()).unwrap_or_default();
        let mut tocadas = BTreeSet::new();
        for m in vencidas {
            {
                let store = self.store.lock().unwrap();
                if let MessageBody::Media(md) = MessageBody::from_stored(&m.kind, &m.body) {
                    let _ = store.delete_media(&md.mid);
                }
                let _ = store.delete_msg(&m.id);
            }
            let _ = self.events.send(EngineEvent::MessageDeleted { chat: m.chat.clone(), id: m.id.clone() });
            tocadas.insert(m.chat);
        }
        for k in tocadas {
            let _ = self.refresh_last(&k);
        }
    }

    /* ---------------- contatos e conversas ---------------- */

    /// Adiciona alguém pelo código e já manda o seu perfil.
    pub async fn add_contact(self: &Arc<Self>, input: &str) -> Result<Contact> {
        self.add_contact_with(input, "").await
    }

    pub async fn add_contact_with(self: &Arc<Self>, input: &str, nick: &str) -> Result<Contact> {
        let pk = identity::parse_pubkey(input).ok_or_else(|| erro("Código inválido. Confira e tente de novo."))?;
        if pk == self.keys.public_key() {
            return Err(erro("Esse é o seu próprio código."));
        }
        let hex = pk.to_hex();
        self.ensure_contact(&hex, ContactStatus::Ok)?;
        let c = {
            let mut st = self.state.lock().unwrap();
            let c = st.contacts.get_mut(&hex).ok_or(EngineError::NoIdentity)?;
            c.status = ContactStatus::Ok;
            if !nick.trim().is_empty() {
                c.nick = clamp(nick, 40);
            }
            c.clone()
        };
        self.save_contact(&c)?;
        let _ = self.events.send(EngineEvent::ContactChanged { pk: hex.clone() });
        let key = ChatKey::Direct { peer: hex.clone() }.as_string();
        {
            let ttl_padrao = self.settings().ttl_default;
            let store = self.store.lock().unwrap();
            let mut chat = chat_for(&store, &key);
            if chat.ttl == 0 && ttl_padrao > 0 {
                chat.ttl = ttl_padrao;
            }
            if chat.last_ts == 0 {
                chat.last_ts = placeholder_ts();
            }
            chat.deleted = false;
            save_chat(&store, &chat)?;
        }
        let _ = self.send_profile(&hex).await;
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(c)
    }

    /// Aceita um pedido de conversa: manda os recibos que estavam guardados.
    pub async fn accept_contact(self: &Arc<Self>, pk: &str) -> Result<()> {
        self.set_contact_status(pk, ContactStatus::Ok)?;
        let _ = self.send_profile(pk).await;
        let key = ChatKey::Direct { peer: pk.to_string() }.as_string();
        let pend = {
            let store = self.store.lock().unwrap();
            chat_for(&store, &key).pending_read
        };
        for id in pend {
            self.queue_receipt(pk, &id, false);
        }
        self.mark_read(&key)?;
        Ok(())
    }

    pub fn set_contact_status(&self, pk: &str, status: ContactStatus) -> Result<()> {
        let c = self.ensure_contact(pk, status)?;
        let snap = {
            let mut st = self.state.lock().unwrap();
            let c2 = st.contacts.get_mut(pk).map(|x| {
                x.status = status;
                x.clone()
            });
            c2.unwrap_or(c)
        };
        self.save_contact(&snap)?;
        let _ = self.events.send(EngineEvent::ContactChanged { pk: pk.into() });
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /// Apelido (vazio tira) e marca de verificado.
    pub fn update_contact(&self, pk: &str, nick: Option<&str>, verified: Option<bool>) -> Result<()> {
        let snap = {
            let mut st = self.state.lock().unwrap();
            let Some(c) = st.contacts.get_mut(pk) else { return Ok(()) };
            if let Some(n) = nick {
                c.nick = clamp(n, 40);
            }
            if let Some(v) = verified {
                c.verified = v;
            }
            c.clone()
        };
        self.save_contact(&snap)?;
        let _ = self.events.send(EngineEvent::ContactChanged { pk: pk.into() });
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /// Número de segurança com um contato (o mesmo que aparece no celular dele).
    pub fn safety_number(&self, pk: &str) -> Option<[String; 12]> {
        crate::safety::safety_number(&self.pk_hex(), pk)
    }

    /// Apaga a conversa deste aparelho. Ela volta se chegar mensagem nova.
    pub fn delete_chat(&self, key: &str) -> Result<()> {
        {
            let store = self.store.lock().unwrap();
            store.delete_chat_msgs(key)?;
            let mut c = chat_for(&store, key);
            c.last_text.clear();
            c.last_id = None;
            c.unread = 0;
            c.deleted = true;
            c.pending_read.clear();
            save_chat(&store, &c)?;
        }
        let _ = self.events.send(EngineEvent::ChatGone { chat: key.into() });
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    pub fn set_chat_pinned(&self, key: &str, pinned: bool) -> Result<()> {
        {
            let store = self.store.lock().unwrap();
            let mut c = chat_for(&store, key);
            c.pinned = pinned;
            save_chat(&store, &c)?;
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    pub fn set_chat_muted(&self, key: &str, muted: bool) -> Result<()> {
        {
            let store = self.store.lock().unwrap();
            let mut c = chat_for(&store, key);
            c.muted = muted;
            save_chat(&store, &c)?;
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    pub fn chat_muted(&self, key: &str) -> bool {
        let store = self.store.lock().unwrap();
        chat_for(&store, key).muted
    }

    /// Garante que a conversa aparece na lista, mesmo sem nenhuma mensagem.
    pub fn ensure_chat(&self, chat_key: &str) -> Result<()> {
        if ChatKey::parse(chat_key).is_none() {
            return Err(EngineError::UnknownChat(chat_key.into()));
        }
        {
            let store = self.store.lock().unwrap();
            let mut c = chat_for(&store, chat_key);
            if c.last_ts == 0 {
                c.last_ts = placeholder_ts();
            }
            c.deleted = false;
            save_chat(&store, &c)?;
        }
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    fn ensure_contact(&self, pk: &str, status: ContactStatus) -> Result<Contact> {
        let (snap, novo) = {
            let mut st = self.state.lock().unwrap();
            let novo = !st.contacts.contains_key(pk);
            let entry = st.contacts.entry(pk.to_string()).or_insert_with(|| new_contact(pk, status));
            (entry.clone(), novo)
        };
        if novo {
            self.save_contact(&snap)?;
            let _ = self.events.send(EngineEvent::ContactChanged { pk: pk.into() });
        }
        Ok(snap)
    }

    fn save_contact(&self, c: &Contact) -> Result<()> {
        let store = self.store.lock().unwrap();
        store.raw().execute(
            "INSERT INTO contacts (pk, name, nick, note, pic, tone, status, verified, relays, prof_sent, added_at, extra)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(pk) DO UPDATE SET name=?2, nick=?3, note=?4, pic=?5, tone=?6, status=?7,
                                           verified=?8, relays=?9, prof_sent=?10, extra=?12",
            rusqlite::params![
                c.pk, c.name, c.nick, c.note, c.pic, c.tone,
                status_name(c.status), c.verified as i64,
                serde_json::to_string(&c.relays).unwrap_or_else(|_| "[]".into()),
                c.prof_sent as i64, c.added_at,
                serde_json::json!({ "b": c.banner, "g": c.games }).to_string()
            ],
        ).map_err(crate::store::StoreError::from)?;
        Ok(())
    }

    pub fn contact(&self, pk: &str) -> Option<Contact> {
        self.state.lock().unwrap().contacts.get(pk).cloned()
    }

    pub fn contacts(&self) -> Vec<Contact> {
        let mut v: Vec<Contact> = self.state.lock().unwrap().contacts.values().cloned().collect();
        v.sort_by_key(|c| c.display_name().to_lowercase());
        v
    }

    /// Foto de alguém: a sua, a que o contato mandou, ou a que a pessoa
    /// mostrou num grupo em comum.
    pub fn pic_of(&self, pk: &str) -> Option<String> {
        if pk == self.pk_hex() {
            return self.me().pic;
        }
        let st = self.state.lock().unwrap();
        if let Some(c) = st.contacts.get(pk) {
            if c.status == ContactStatus::Blocked {
                return None;
            }
            if c.pic.is_some() {
                return c.pic.clone();
            }
        }
        st.groups.values().find_map(|g| g.pics.get(pk).cloned())
    }

    /// Cor do avatar de alguém (0 a 5).
    pub fn tone_of(&self, pk: &str) -> u8 {
        if pk == self.pk_hex() {
            return self.me().tone;
        }
        self.contact(pk)
            .map(|c| c.tone)
            .unwrap_or_else(|| u8::from_str_radix(pk.get(..2).unwrap_or("0"), 16).unwrap_or(0) % 6)
    }

    /// Nome de alguém, seja contato ou não.
    pub fn name_of(&self, pk: &str) -> String {
        if pk == self.pk_hex() {
            let n = self.me().name;
            return if n.is_empty() { "Você".into() } else { n };
        }
        let st = self.state.lock().unwrap();
        if let Some(c) = st.contacts.get(pk) {
            if !c.nick.is_empty() || !c.name.is_empty() {
                return c.display_name();
            }
        }
        for g in st.groups.values() {
            if let Some(n) = g.meta.names.get(pk) {
                return n.clone();
            }
        }
        drop(st);
        PublicKey::from_hex(pk)
            .map(|p| identity::short_id(&p))
            .unwrap_or_else(|_| pk[..8.min(pk.len())].to_string())
    }

    /// Pedidos de conversa ainda não respondidos.
    pub fn requests(&self) -> Vec<Contact> {
        self.contacts().into_iter().filter(|c| c.status == ContactStatus::Request).collect()
    }

    pub fn blocked(&self) -> Vec<Contact> {
        self.contacts().into_iter().filter(|c| c.status == ContactStatus::Blocked).collect()
    }

    /// Conversas, as fixadas primeiro e depois por data.
    pub fn chat_list(&self) -> Result<Vec<ChatSummary>> {
        let rows: Vec<ChatRow> = {
            let store = self.store.lock().unwrap();
            let mut stmt = store
                .raw()
                .prepare(&format!("SELECT {CHAT_COLS} FROM chats WHERE deleted = 0 OR unread > 0 ORDER BY pinned DESC, last_ts DESC"))
                .map_err(crate::store::StoreError::from)?;
            let v: Vec<ChatRow> = stmt
                .query_map([], row_to_chat)
                .map_err(crate::store::StoreError::from)?
                .collect::<std::result::Result<_, _>>()
                .map_err(crate::store::StoreError::from)?;
            v
        };
        let eu = self.pk_hex();
        Ok(rows
            .into_iter()
            .filter_map(|c| {
                let key = ChatKey::parse(&c.key)?;
                let title = match &key {
                    ChatKey::Direct { peer } => self.name_of(peer),
                    ChatKey::Channel { lid, channel } => {
                        let g = self.group(lid)?;
                        let canal = g.meta.channels.iter().find(|x| &x.id == channel)?;
                        format!("{} · #{}", g.meta.name, canal.name)
                    }
                };
                Some(ChatSummary {
                    title,
                    is_group: key.is_group(),
                    last_from_me: c.last_from.as_deref() == Some(eu.as_str()),
                    last_kind: c.last_kind.unwrap_or_default(),
                    last_text: c.last_text,
                    last_ts: c.last_ts,
                    last_status: c.last_status.as_deref().and_then(parse_delivery),
                    unread: c.unread,
                    pinned: c.pinned,
                    muted: c.muted,
                    peer: c.peer,
                    ttl: c.ttl,
                    key: c.key,
                })
            })
            .collect())
    }

    /// Página de mensagens de uma conversa.
    pub fn messages(&self, chat: &str, before_ts: i64, limit: usize) -> Result<Vec<Message>> {
        let eu = self.pk_hex();
        let store = self.store.lock().unwrap();
        Ok(store
            .msgs_by_chat(chat, before_ts, limit)?
            .into_iter()
            .map(|m| from_stored(m, &eu))
            .collect())
    }

    pub fn message(&self, id: &str) -> Option<Message> {
        let eu = self.pk_hex();
        self.store.lock().unwrap().get_msg(id).ok().flatten().map(|m| from_stored(m, &eu))
    }

    /* ---------------- grupos ---------------- */

    /// Todas as chaves que este aparelho precisa escutar: a minha e a de cada
    /// grupo, incluindo as chaves antigas dos grupos que trocaram de chave.
    fn inbox_keys(&self) -> Vec<PublicKey> {
        let mut v = vec![self.keys.public_key()];
        let st = self.state.lock().unwrap();
        for g in st.groups.values() {
            for (pk, _) in g.all_keys() {
                if let Ok(p) = PublicKey::from_hex(&pk) {
                    v.push(p);
                }
            }
        }
        v
    }

    fn group_for_pubkey(&self, pk_hex: &str) -> Option<(String, Keys)> {
        let st = self.state.lock().unwrap();
        for g in st.groups.values() {
            for (pk, keys) in g.all_keys() {
                if pk == pk_hex {
                    return Some((g.lid.clone(), keys));
                }
            }
        }
        None
    }

    pub fn group(&self, lid: &str) -> Option<Group> {
        self.state.lock().unwrap().groups.get(lid).cloned()
    }

    pub fn groups(&self) -> Vec<Group> {
        let mut v: Vec<Group> = self.state.lock().unwrap().groups.values().cloned().collect();
        v.sort_by_key(|g| g.meta.name.to_lowercase());
        v
    }

    pub fn is_group_admin(&self, lid: &str) -> bool {
        self.group(lid).is_some_and(|g| g.meta.is_admin(&self.pk_hex()))
    }

    /// Mensagens não lidas em todos os canais de um grupo.
    pub fn group_unread(&self, lid: &str) -> u32 {
        let store = self.store.lock().unwrap();
        store
            .raw()
            .query_row(
                "SELECT COALESCE(SUM(unread), 0) FROM chats WHERE key LIKE ?1",
                rusqlite::params![format!("g:{lid}:%")],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as u32
    }

    fn ensure_group_chats(&self, lid: &str) -> Result<()> {
        let Some(g) = self.group(lid) else { return Ok(()) };
        let store = self.store.lock().unwrap();
        for ch in g.meta.text_channels() {
            let key = ChatKey::Channel { lid: lid.into(), channel: ch.id.clone() }.as_string();
            let mut c = chat_for(&store, &key);
            if !c.existed {
                c.last_ts = placeholder_ts();
                save_chat(&store, &c)?;
            }
        }
        drop(store);
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /// Cria um grupo e já abre os canais dele na lista de conversas.
    pub async fn create_group(self: &Arc<Self>, name: &str, template: Template) -> Result<Group> {
        let g = Group::create(name, &self.keys.public_key(), &self.me().name, template);
        self.save_group(&g)?;
        self.resubscribe().await;
        self.ensure_group_chats(&g.lid)?;
        self.sys_group(&g.lid, "created", &self.pk_hex(), now_ms(), None)?;
        let _ = self.events.send(EngineEvent::GroupsChanged);
        Ok(g)
    }

    /// Convida alguém: manda a chave do grupo por mensagem privada.
    pub async fn invite_to_group(self: &Arc<Self>, lid: &str, peer: &str) -> Result<Message> {
        let g = self.group(lid).ok_or_else(|| EngineError::UnknownChat(lid.into()))?;
        let c = self.ensure_contact(peer, ContactStatus::Ok)?;
        if c.status != ContactStatus::Ok {
            self.set_contact_status(peer, ContactStatus::Ok)?;
        }
        if !c.prof_sent {
            let _ = self.send_profile(peer).await;
        }
        let convite = GroupInvite {
            sk: g.sk.clone(),
            meta: serde_json::to_value(&g.meta).unwrap_or_default(),
            ver: g.ver,
            pic: g.pic.clone().unwrap_or_default(),
            pic_v: g.pic_v,
        };
        let chat = ChatKey::Direct { peer: peer.into() }.as_string();
        let b = self.build(&chat, KIND_APP, AppMessage::GroupInvite { g: convite }.to_json(), Vec::new(), BuildOpts { no_ttl: true, ..Default::default() })?;
        let data = InviteData {
            sk: g.sk.clone(),
            pk: g.pk.clone(),
            lid: g.lid.clone(),
            meta: g.meta.clone(),
            ver: g.ver,
            pic: g.pic.clone().unwrap_or_default(),
            pic_v: g.pic_v,
        };
        let msg = Message {
            id: b.id.clone(),
            chat: chat.clone(),
            author: self.pk_hex(),
            from_me: true,
            ts: b.created_ms,
            body: MessageBody::Invite { lid: g.lid.clone(), name: g.meta.name.clone(), joined: true, data: Some(Box::new(data)) },
            status: None,
            reply: None,
            reacts: BTreeMap::new(),
            exp: None,
        };
        self.local_send(&chat, msg, &b).await
    }

    /// Entra num grupo a partir de um convite recebido. Aceita o id da
    /// mensagem de convite ou o id do grupo.
    pub async fn accept_invite(self: &Arc<Self>, msg_or_lid: &str) -> Result<Group> {
        let eu = self.pk_hex();
        // Procura a mensagem de convite.
        let achado: Option<(String, InviteData)> = {
            let store = self.store.lock().unwrap();
            let por_id = store.get_msg(msg_or_lid)?.and_then(|m| match MessageBody::from_stored(&m.kind, &m.body) {
                MessageBody::Invite { data: Some(d), .. } => Some((m.id.clone(), *d)),
                _ => None,
            });
            match por_id {
                Some(x) => Some(x),
                None => {
                    let mut stmt = store
                        .raw()
                        .prepare("SELECT id, body FROM msgs WHERE kind = 'invite' ORDER BY ts DESC")
                        .map_err(crate::store::StoreError::from)?;
                    let linhas: Vec<(String, String)> = stmt
                        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                        .map_err(crate::store::StoreError::from)?
                        .collect::<std::result::Result<_, _>>()
                        .map_err(crate::store::StoreError::from)?;
                    linhas.into_iter().find_map(|(id, body)| match MessageBody::from_stored("invite", &body) {
                        MessageBody::Invite { data: Some(d), lid, .. } if lid == msg_or_lid => Some((id, *d)),
                        _ => None,
                    })
                }
            }
        };
        let (msg_id, inv) = achado.ok_or_else(|| EngineError::UnknownChat(msg_or_lid.into()))?;

        if let Some(g) = self.group(&inv.lid) {
            if g.pk == inv.pk {
                return Ok(g);
            }
        }
        let mut meta = inv.meta.clone();
        if !meta.members.contains(&eu) {
            meta.members.push(eu.clone());
        }
        meta.names.insert(eu.clone(), self.me().name);
        let g = Group {
            lid: inv.lid.clone(),
            pk: inv.pk.clone(),
            sk: inv.sk.clone(),
            old_keys: Vec::new(),
            meta,
            ver: inv.ver,
            pic: match media::clean_pic(&inv.pic) {
                Pic::Set(p) => Some(p),
                _ => None,
            },
            pic_v: inv.pic_v,
            pics: HashMap::new(),
            gone: Vec::new(),
            muted: false,
            joined_at: now_ms(),
        };
        self.save_group(&g)?;
        self.resubscribe().await;
        // O convite passa a mostrar "Você entrou".
        let alterada = {
            let store = self.store.lock().unwrap();
            store.get_msg(&msg_id)?.map(|mut sm| {
                if let MessageBody::Invite { lid, name, data, .. } = MessageBody::from_stored(&sm.kind, &sm.body) {
                    sm.body = MessageBody::Invite { lid, name, joined: true, data }.to_json();
                    let _ = store.update_msg(&sm);
                }
                from_stored(sm, &eu)
            })
        };
        if let Some(m) = alterada {
            let _ = self.events.send(EngineEvent::MessageChanged { chat: m.chat.clone(), message: Box::new(m) });
        }
        self.ensure_group_chats(&g.lid)?;
        self.sys_group(&g.lid, "joined", &eu, now_ms(), None)?;
        let me = self.me();
        let _ = self
            .send_group_app(&g.lid, &AppMessage::GroupJoin { n: me.name, p: Some(me.pic.unwrap_or_default()) }, BuildOpts::default())
            .await;
        let _ = self.events.send(EngineEvent::GroupsChanged);
        Ok(g)
    }

    /// Sai do grupo e apaga as conversas dele deste aparelho.
    pub async fn leave_group(self: &Arc<Self>, lid: &str) -> Result<()> {
        let g = self.group(lid).ok_or_else(|| EngineError::UnknownChat(lid.into()))?;
        let _ = self.send_group_app(lid, &AppMessage::GroupLeave, BuildOpts::default()).await;
        {
            let store = self.store.lock().unwrap();
            for ch in g.meta.channels.iter() {
                let key = ChatKey::Channel { lid: lid.into(), channel: ch.id.clone() }.as_string();
                store.delete_chat_msgs(&key)?;
                store
                    .raw()
                    .execute("DELETE FROM chats WHERE key = ?1", rusqlite::params![key])
                    .map_err(crate::store::StoreError::from)?;
            }
            store
                .raw()
                .execute("DELETE FROM groups WHERE lid = ?1", rusqlite::params![lid])
                .map_err(crate::store::StoreError::from)?;
        }
        self.state.lock().unwrap().groups.remove(lid);
        self.resubscribe().await;
        for ch in g.meta.text_channels() {
            let key = ChatKey::Channel { lid: lid.into(), channel: ch.id.clone() }.as_string();
            let _ = self.events.send(EngineEvent::ChatGone { chat: key });
        }
        let _ = self.events.send(EngineEvent::GroupsChanged);
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /// Muda o grupo (só admin). Vale para todos: os dados novos são
    /// publicados e canais de texto apagados levam as mensagens junto.
    pub async fn edit_group(self: &Arc<Self>, lid: &str, patches: &[GroupPatch]) -> Result<()> {
        let mut g = self.group(lid).ok_or_else(|| EngineError::UnknownChat(lid.into()))?;
        if !g.meta.is_admin(&self.pk_hex()) {
            return Err(erro("Só administradores mudam o grupo."));
        }
        let mut removidos = Vec::new();
        for p in patches {
            removidos.extend(g.meta.apply(p));
        }
        g.ver += 1;
        self.save_group(&g)?;
        self.publish_meta(lid).await?;
        for c in removidos {
            if c.kind == ChannelKind::Text {
                self.purge_channel(lid, &c.id)?;
            }
        }
        self.ensure_group_chats(lid)?;
        let _ = self.events.send(EngineEvent::GroupsChanged);
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /// Tira alguém do grupo (só o dono). A chave do grupo é trocada e a nova
    /// vai só para quem fica: quem saiu não lê mais nada novo.
    pub async fn remove_member(self: &Arc<Self>, lid: &str, pk: &str) -> Result<()> {
        let eu = self.pk_hex();
        let mut g = self.group(lid).ok_or_else(|| EngineError::UnknownChat(lid.into()))?;
        if g.meta.owner != eu || pk == eu {
            return Err(erro("Só o dono do grupo remove membros."));
        }
        let antiga = g.pk.clone();
        g.rekey();
        g.meta.members.retain(|m| m != pk);
        g.meta.admins.retain(|m| m != pk);
        self.save_group(&g)?;
        self.resubscribe().await;
        for m in g.meta.members.clone() {
            if m == eu {
                continue;
            }
            self.ensure_contact(&m, ContactStatus::Ok)?;
            let chat = ChatKey::Direct { peer: m.clone() }.as_string();
            let app = AppMessage::GroupRekey {
                lid: g.lid.clone(),
                old: antiga.clone(),
                sk: g.sk.clone(),
                meta: serde_json::to_value(&g.meta).unwrap_or_default(),
                ver: g.ver,
            };
            let _ = self.send_app(&chat, &app, BuildOpts { no_self_copy: true, no_ttl: true, ..Default::default() }).await;
        }
        self.sys_group(lid, "removed", pk, now_ms(), None)?;
        let _ = self.events.send(EngineEvent::GroupsChanged);
        Ok(())
    }

    fn apply_rekey(self: &Arc<Self>, from: &str, lid: &str, old: &str, sk: &str, meta: serde_json::Value, ver: u64) {
        if !is_hex64(sk) {
            return;
        }
        let g = {
            let st = self.state.lock().unwrap();
            st.groups.values().find(|g| g.pk == old || g.lid == lid).cloned()
        };
        let Some(mut g) = g else { return };
        if g.meta.owner != from {
            return;
        }
        let Some(meta) = clean_meta(meta, &g.meta.owner) else { return };
        let Ok(novas) = Keys::parse(sk) else { return };
        g.old_keys.push((g.pk.clone(), g.sk.clone()));
        if g.old_keys.len() > 5 {
            g.old_keys.remove(0);
        }
        g.pk = novas.public_key().to_hex();
        g.sk = sk.to_ascii_lowercase();
        g.meta = meta;
        g.ver = if ver > 0 { ver } else { g.ver + 1 };
        if self.save_group(&g).is_ok() {
            let me = Arc::clone(self);
            tokio::spawn(async move { me.resubscribe().await });
            let _ = self.events.send(EngineEvent::GroupsChanged);
        }
    }

    /// Muda (ou tira, com texto vazio) a foto do grupo. Só admin.
    pub async fn set_group_pic(self: &Arc<Self>, lid: &str, pic: &str) -> Result<()> {
        let mut g = self.group(lid).ok_or_else(|| EngineError::UnknownChat(lid.into()))?;
        if !g.meta.is_admin(&self.pk_hex()) {
            return Err(erro("Só administradores mudam a foto."));
        }
        let p = match media::clean_pic(pic) {
            Pic::Set(s) => Some(s),
            Pic::Remove => None,
            Pic::Invalid => return Err(erro("foto inválida")),
        };
        let tem = p.is_some();
        g.pic = p.clone();
        g.pic_v = (now_ms() as u64).max(g.pic_v + 1);
        self.save_group(&g)?;
        self.send_group_app(lid, &AppMessage::GroupPic { p: p.unwrap_or_default(), v: g.pic_v }, BuildOpts::default()).await?;
        self.sys_group(lid, if tem { "gpic" } else { "gpicdel" }, &self.pk_hex(), now_ms(), None)?;
        let _ = self.events.send(EngineEvent::GroupsChanged);
        Ok(())
    }

    pub fn set_group_muted(&self, lid: &str, muted: bool) -> Result<()> {
        if let Some(mut g) = self.group(lid) {
            g.muted = muted;
            self.save_group(&g)?;
            let _ = self.events.send(EngineEvent::GroupsChanged);
        }
        Ok(())
    }

    /// Apaga deste aparelho as mensagens de um canal que não existe mais.
    fn purge_channel(&self, lid: &str, ch: &str) -> Result<()> {
        if let Some(mut g) = self.group(lid) {
            if !g.gone.iter().any(|x| x == ch) {
                g.gone.push(ch.into());
                let excesso = g.gone.len().saturating_sub(50);
                g.gone.drain(..excesso);
            }
            self.save_group(&g)?;
        }
        let key = ChatKey::Channel { lid: lid.into(), channel: ch.into() }.as_string();
        {
            let store = self.store.lock().unwrap();
            store.delete_chat_msgs(&key)?;
            store
                .raw()
                .execute("DELETE FROM chats WHERE key = ?1", rusqlite::params![key])
                .map_err(crate::store::StoreError::from)?;
        }
        let _ = self.events.send(EngineEvent::ChatGone { chat: key });
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(())
    }

    /// Aviso do sistema no primeiro canal de texto do grupo.
    fn sys_group(&self, lid: &str, sys: &str, who: &str, ts_ms: i64, id: Option<&str>) -> Result<()> {
        let Some(g) = self.group(lid) else { return Ok(()) };
        let Some(ch) = g.main_channel() else { return Ok(()) };
        let key = ChatKey::Channel { lid: lid.into(), channel: ch.id.clone() }.as_string();
        let id = id.map(|s| s.to_string()).unwrap_or_else(|| random_hex(32));
        self.add_msg(&key, sys_msg(&id, &key, who, who == self.pk_hex(), ts_ms, sys, 0), false)?;
        Ok(())
    }

    async fn send_group_app(&self, lid: &str, app: &AppMessage, o: BuildOpts) -> Result<String> {
        let g = self.group(lid).ok_or_else(|| erro("Grupo indisponível"))?;
        let ch = g.main_channel().ok_or_else(|| erro("grupo sem canal de texto"))?;
        let key = ChatKey::Channel { lid: lid.into(), channel: ch.id.clone() }.as_string();
        self.send_app(&key, app, BuildOpts { no_ttl: true, ..o }).await
    }

    async fn publish_meta(&self, lid: &str) -> Result<()> {
        let g = self.group(lid).ok_or_else(|| erro("Grupo indisponível"))?;
        let app = AppMessage::GroupMeta { meta: serde_json::to_value(&g.meta).unwrap_or_default(), ver: g.ver };
        self.send_group_app(lid, &app, BuildOpts::default()).await?;
        Ok(())
    }

    fn save_group(&self, g: &Group) -> Result<()> {
        {
            let store = self.store.lock().unwrap();
            store
                .raw()
                .execute(
                    "INSERT INTO groups (lid, pk, sk, old_keys, meta, ver, pic, pic_v, pics, gone, muted, joined_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
                     ON CONFLICT(lid) DO UPDATE SET pk=?2, sk=?3, old_keys=?4, meta=?5, ver=?6,
                                                    pic=?7, pic_v=?8, pics=?9, gone=?10, muted=?11, joined_at=?12",
                    rusqlite::params![
                        g.lid, g.pk, g.sk,
                        serde_json::to_string(&g.old_keys).unwrap_or_else(|_| "[]".into()),
                        serde_json::to_string(&g.meta).unwrap_or_default(),
                        g.ver as i64, g.pic, g.pic_v as i64,
                        serde_json::to_string(&g.pics).unwrap_or_else(|_| "{}".into()),
                        serde_json::to_string(&g.gone).unwrap_or_else(|_| "[]".into()),
                        g.muted as i64, g.joined_at
                    ],
                )
                .map_err(crate::store::StoreError::from)?;
        }
        self.state.lock().unwrap().groups.insert(g.lid.clone(), g.clone());
        Ok(())
    }

    /* ---------------- salas de voz e chamadas ---------------- */

    fn on_room(&self, g: &Group, ch: &str, pk: &str, a: Presence) {
        if !g.meta.voice_channels().any(|c| c.id == ch) {
            return;
        }
        let (mudou, membros) = {
            let mut st = self.state.lock().unwrap();
            let sala = st.rooms.entry((g.lid.clone(), ch.to_string())).or_default();
            let tinha = sala.contains_key(pk);
            match a {
                Presence::Leave => {
                    sala.remove(pk);
                }
                Presence::Here => {
                    sala.insert(pk.to_string(), now_ms());
                }
            }
            (tinha != (a == Presence::Here), sala.keys().cloned().collect::<Vec<_>>())
        };
        if mudou {
            let _ = self.events.send(EngineEvent::Room { lid: g.lid.clone(), ch: ch.into(), members: membros });
        }
    }

    /// Tira das salas quem não dá sinal há mais de 26 s.
    fn room_sweep(&self) {
        let t = now_ms();
        let mut mudancas = Vec::new();
        {
            let mut st = self.state.lock().unwrap();
            for ((lid, ch), sala) in st.rooms.iter_mut() {
                let antes = sala.len();
                sala.retain(|_, visto| t - *visto <= 26_000);
                if sala.len() != antes {
                    mudancas.push((lid.clone(), ch.clone(), sala.keys().cloned().collect::<Vec<_>>()));
                }
            }
        }
        for (lid, ch, members) in mudancas {
            let _ = self.events.send(EngineEvent::Room { lid, ch, members });
        }
    }

    /// Quem está numa sala de voz agora (sem contar você).
    pub fn room_members(&self, lid: &str, ch: &str) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .rooms
            .get(&(lid.to_string(), ch.to_string()))
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Avisa o grupo que você está (ou saiu) da sala de voz.
    pub async fn room_presence(&self, lid: &str, ch: &str, here: bool) -> Result<()> {
        let a = if here { Presence::Here } else { Presence::Leave };
        self.send_group_app(lid, &AppMessage::Room { ch: ch.into(), a }, BuildOpts { ephemeral: true, ..Default::default() })
            .await?;
        Ok(())
    }

    /// Manda sinalização de chamada (ou de sala de voz) para alguém.
    pub async fn send_call_signal(&self, peer: &str, signal: CallSignal) -> Result<()> {
        let chat = ChatKey::Direct { peer: peer.into() }.as_string();
        self.send_app(&chat, &AppMessage::Call(signal), BuildOpts { ephemeral: true, no_self_copy: true, no_ttl: true, ..Default::default() })
            .await?;
        Ok(())
    }

    /// Registra no histórico do outro lado que ele não atendeu.
    pub async fn report_missed_call(&self, peer: &str, video: bool) -> Result<()> {
        let chat = ChatKey::Direct { peer: peer.into() }.as_string();
        let sig = CallSignal { v: Some(video as u8), ..CallSignal::new("", CallAction::Missed) };
        self.send_app(&chat, &AppMessage::Call(sig), BuildOpts { no_self_copy: true, no_ttl: true, expires_in: Some(7 * DAY), ..Default::default() })
            .await?;
        Ok(())
    }

    pub fn log_call(&self, c: &CallLog) -> Result<()> {
        self.store.lock().unwrap().log_call(c)?;
        let _ = self.events.send(EngineEvent::CallsChanged);
        Ok(())
    }

    pub fn calls(&self, limit: usize) -> Vec<CallLog> {
        self.store.lock().unwrap().calls(limit).unwrap_or_default()
    }

    pub fn clear_calls(&self) -> Result<()> {
        self.store.lock().unwrap().clear_calls()?;
        let _ = self.events.send(EngineEvent::CallsChanged);
        Ok(())
    }

    /* ---------------- soundboard ---------------- */

    pub fn custom_sounds(&self) -> Vec<SoundRecord> {
        self.store.lock().unwrap().sounds().unwrap_or_default()
    }

    /// Guarda um som próprio. O id vem dos bytes, como no celular.
    pub fn save_custom_sound(&self, bytes: &[u8], mime: &str, name: &str, emoji: &str, dur: f64) -> Result<SoundRecord> {
        if bytes.len() > media::MAX_SOUND_BYTES {
            return Err(erro("som grande demais"));
        }
        let meta = media::clean_sound_meta(name, emoji);
        let rec = SoundRecord {
            id: media::sound_id(bytes),
            name: meta.n,
            emoji: meta.e,
            dur: (dur * 10.0).round() / 10.0,
            mime: mime.into(),
            bytes: bytes.to_vec(),
            ts: now_ms(),
        };
        self.store.lock().unwrap().put_sound(&rec)?;
        Ok(rec)
    }

    pub fn edit_custom_sound(&self, id: &str, name: Option<&str>, emoji: Option<&str>) -> Result<()> {
        let atual = self.custom_sounds().into_iter().find(|s| s.id == id);
        if let Some(mut s) = atual {
            let meta = media::clean_sound_meta(name.unwrap_or(&s.name), emoji.unwrap_or(&s.emoji));
            s.name = meta.n;
            s.emoji = meta.e;
            self.store.lock().unwrap().put_sound(&s)?;
        }
        Ok(())
    }

    /// Tira um som da lista. Os de fábrica só ficam escondidos (dá para
    /// restaurar); os criados por você são apagados.
    pub fn delete_sound(&self, id: &str) -> Result<()> {
        if crate::sounds::builtin(id).is_some() {
            let mut h = self.hidden_builtins();
            if !h.iter().any(|x| x == id) {
                h.push(id.into());
            }
            self.store.lock().unwrap().set_kv("sb_hidden", &h)?;
        } else {
            self.store.lock().unwrap().delete_sound(id)?;
        }
        Ok(())
    }

    pub fn hidden_builtins(&self) -> Vec<String> {
        self.store.lock().unwrap().get_kv("sb_hidden").ok().flatten().unwrap_or_default()
    }

    pub fn restore_builtins(&self) -> Result<()> {
        self.store.lock().unwrap().set_kv("sb_hidden", &Vec::<String>::new())?;
        Ok(())
    }

    pub fn sound_volume(&self) -> f32 {
        self.store.lock().unwrap().get_kv("sb_volume").ok().flatten().unwrap_or(0.7)
    }

    pub fn set_sound_volume(&self, v: f32) -> Result<()> {
        self.store.lock().unwrap().set_kv("sb_volume", &v.clamp(0.0, 1.0))?;
        Ok(())
    }

    /* ---------------- código de convite curto ---------------- */

    /// O código curto atual e quando vence (segundos), se ainda vale.
    pub fn short_code(&self) -> Option<(String, i64)> {
        let c: Option<(String, i64)> = self.store.lock().unwrap().get_kv("short_code").ok().flatten();
        c.filter(|(_, exp)| *exp > now_secs() + 3600)
    }

    /// Gera (ou reaproveita) o código curto e publica nos servidores.
    pub async fn create_short_code(&self) -> Result<(String, i64)> {
        if let Some(c) = self.short_code() {
            // Publica de novo: um servidor novo pode não ter o registro.
            self.publish_short_code(&c.0, c.1).await?;
            return Ok(c);
        }
        let code = crate::shortcode::generate();
        let exp = now_secs() + crate::shortcode::VALIDADE_DIAS * DAY;
        self.publish_short_code(&code, exp).await?;
        self.store.lock().unwrap().set_kv("short_code", &(code.clone(), exp))?;
        Ok((code, exp))
    }

    async fn publish_short_code(&self, code: &str, exp: i64) -> Result<()> {
        let ev = EventBuilder::new(Kind::Custom(crate::shortcode::KIND), "")
            .tags([Tag::identifier(crate::shortcode::tag(code)), Tag::expiration(Timestamp::from_secs(exp as u64))])
            .finalize(&self.keys)
            .map_err(|e| erro(e.to_string()))?;
        self.pool.publish(&ev).await.map_err(|e| EngineError::Relay(e.to_string()))?;
        Ok(())
    }

    /// Descobre de quem é um código curto. Devolve a chave pública (hex).
    pub async fn resolve_short_code(&self, input: &str) -> Result<String> {
        let code = crate::shortcode::parse(input).ok_or_else(|| erro("Código inválido."))?;
        let ident = crate::shortcode::tag(&code);
        let filtro = Filter::new().kind(Kind::Custom(crate::shortcode::KIND)).identifier(ident.clone());
        let evs = self.pool.fetch(filtro, std::time::Duration::from_secs(6)).await.map_err(|e| EngineError::Relay(e.to_string()))?;
        let agora = Timestamp::now();
        let mut autores: BTreeSet<String> = BTreeSet::new();
        for ev in evs {
            if ev.kind != Kind::Custom(crate::shortcode::KIND) || ev.verify().is_err() || ev.is_expired_at(agora) {
                continue;
            }
            if ev.tags.identifier().map(|s| s.to_string()) != Some(ident.clone()) {
                continue;
            }
            autores.insert(ev.pubkey.to_hex());
        }
        match autores.len() {
            0 => Err(erro("Código não encontrado ou vencido. Peça um código novo.")),
            1 => Ok(autores.into_iter().next().unwrap_or_default()),
            _ => Err(erro("Esse código está duplicado nos servidores. Peça o código completo (npub1…).")),
        }
    }

    /// Preferência só da interface (aparência e afins). Fica num espaço de
    /// chaves próprio (`ui.`), separado do que o protocolo usa.
    pub fn ui_pref<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.store.lock().unwrap().get_kv(&format!("ui.{key}")).ok().flatten()
    }

    pub fn set_ui_pref<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.store.lock().unwrap().set_kv(&format!("ui.{key}"), value)?;
        Ok(())
    }

    /* ---------------- ajustes, armazenamento e limpeza ---------------- */

    /// Muda os ajustes. Trocar a lista de servidores reconecta na hora.
    pub async fn update_settings(self: &Arc<Self>, f: impl FnOnce(&mut Settings)) -> Result<Settings> {
        let (novo, relays_mudou) = {
            let mut s = self.settings.write().unwrap();
            let antes = s.relays.clone();
            f(&mut s);
            s.relays = s.relays.iter().filter_map(|u| normalize_url(u)).collect();
            s.relays.dedup();
            if s.relays.is_empty() {
                s.relays = Settings::default().relays;
            }
            s.ttl_default = s.ttl_default.min(28 * DAY as u32);
            self.store.lock().unwrap().set_kv("settings", &*s)?;
            (s.clone(), antes != s.relays)
        };
        if relays_mudou {
            self.pool.set_relays(&novo.relays).await.map_err(|e| EngineError::Relay(e.to_string()))?;
            self.resubscribe().await;
            self.publish_relay_list().await;
        }
        let _ = self.events.send(EngineEvent::SettingsChanged);
        Ok(novo)
    }

    /// `(tamanho do banco, espaço de mídia)`, em bytes.
    pub fn storage_usage(&self) -> (i64, i64) {
        let store = self.store.lock().unwrap();
        (store.db_size().unwrap_or(0), store.media_size().unwrap_or(0))
    }

    /// Apaga fotos e áudios deste aparelho. Os textos continuam.
    pub fn clear_media(&self) -> Result<usize> {
        let n = self.store.lock().unwrap().clear_media()?;
        let _ = self.events.send(EngineEvent::ChatsChanged);
        Ok(n)
    }

    /// Apaga a conta e tudo deste aparelho.
    pub async fn wipe(&self) -> Result<()> {
        self.shutdown().await;
        self.store.lock().unwrap().wipe()?;
        Ok(())
    }
}

/* ======================================================================
 *  Apoio
 * ====================================================================== */

fn sys_msg(id: &str, chat: &str, who: &str, from_me: bool, ts: i64, sys: &str, v: i64) -> Message {
    Message {
        id: id.into(),
        chat: chat.into(),
        author: who.into(),
        from_me,
        ts,
        body: MessageBody::System { sys: sys.into(), v, legacy: None },
        status: None,
        reply: None,
        reacts: BTreeMap::new(),
        exp: None,
    }
}

fn reply_ref(r: &UnsignedEvent) -> Option<String> {
    r.tags.iter().find_map(|t| {
        let s = t.as_slice();
        (s.first().map(|x| x.as_str()) == Some("e") && s.get(1).is_some_and(|v| is_hex64(v))).then(|| s[1].clone())
    })
}

fn exp_of(r: &UnsignedEvent) -> Option<i64> {
    protocol::expiration_of(r).map(|t| t.as_secs() as i64 * 1000)
}

fn parse_invite(g: &GroupInvite) -> Option<InviteData> {
    if !is_hex64(&g.sk) {
        return None;
    }
    let meta = clean_meta(g.meta.clone(), "")?;
    let keys = Keys::parse(&g.sk).ok()?;
    let pk = keys.public_key().to_hex();
    Some(InviteData {
        sk: g.sk.to_ascii_lowercase(),
        lid: meta.lid.clone().filter(|l| is_hex64(l)).unwrap_or_else(|| pk.clone()),
        pk,
        meta,
        ver: g.ver.max(1),
        pic: match media::clean_pic(&g.pic) {
            Pic::Set(p) => p,
            _ => String::new(),
        },
        pic_v: g.pic_v,
    })
}

fn clean_meta(v: serde_json::Value, owner_fallback: &str) -> Option<GroupMeta> {
    let meta: GroupMeta = serde_json::from_value(v).ok()?;
    meta.sanitize(owner_fallback)
}

fn to_stored(m: &Message) -> StoredMsg {
    StoredMsg {
        id: m.id.clone(),
        chat: m.chat.clone(),
        author: m.author.clone(),
        kind: m.body.kind_name().into(),
        ts: m.ts,
        body: m.body.to_json(),
        status: m.status.map(|s| delivery_name(s).to_string()),
        exp: m.exp,
        reply: m.reply.clone(),
        reacts: serde_json::to_string(&m.reacts).unwrap_or_else(|_| "{}".into()),
    }
}

fn from_stored(m: StoredMsg, eu: &str) -> Message {
    Message {
        from_me: m.author == eu,
        body: MessageBody::from_stored(&m.kind, &m.body),
        status: m.status.as_deref().and_then(parse_delivery),
        reacts: serde_json::from_str(&m.reacts).unwrap_or_default(),
        id: m.id,
        chat: m.chat,
        author: m.author,
        ts: m.ts,
        reply: m.reply,
        exp: m.exp,
    }
}

const CHAT_COLS: &str = "key, kind, peer, lid, channel, last_ts, last_id, last_from, last_kind, last_text, last_status, unread, ttl, pinned, muted, deleted, pending_read";

fn row_to_chat(r: &rusqlite::Row) -> rusqlite::Result<ChatRow> {
    Ok(ChatRow {
        key: r.get(0)?,
        kind: r.get(1)?,
        peer: r.get(2)?,
        lid: r.get(3)?,
        channel: r.get(4)?,
        last_ts: r.get(5)?,
        last_id: r.get(6)?,
        last_from: r.get(7)?,
        last_kind: r.get(8)?,
        last_text: r.get(9)?,
        last_status: r.get(10)?,
        unread: r.get::<_, i64>(11)? as u32,
        ttl: r.get::<_, i64>(12)? as u32,
        pinned: r.get::<_, i64>(13)? != 0,
        muted: r.get::<_, i64>(14)? != 0,
        deleted: r.get::<_, i64>(15)? != 0,
        pending_read: serde_json::from_str(&r.get::<_, String>(16)?).unwrap_or_default(),
        existed: true,
    })
}

/// A conversa, ou uma nova em branco se ainda não existe (sem gravar).
fn chat_for(store: &Store, key: &str) -> ChatRow {
    let achada = store
        .raw()
        .query_row(&format!("SELECT {CHAT_COLS} FROM chats WHERE key = ?1"), rusqlite::params![key], row_to_chat)
        .ok();
    achada.unwrap_or_else(|| {
        let (kind, peer, lid, channel) = match ChatKey::parse(key) {
            Some(ChatKey::Direct { peer }) => ("dm", Some(peer), None, None),
            Some(ChatKey::Channel { lid, channel }) => ("group", None, Some(lid), Some(channel)),
            None => ("dm", None, None, None),
        };
        ChatRow { key: key.into(), kind: kind.into(), peer, lid, channel, ..Default::default() }
    })
}

fn save_chat(store: &Store, c: &ChatRow) -> Result<()> {
    store
        .raw()
        .execute(
            "INSERT INTO chats (key, kind, peer, lid, channel, last_ts, last_id, last_from, last_kind, last_text,
                                last_status, unread, ttl, pinned, muted, deleted, pending_read)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
             ON CONFLICT(key) DO UPDATE SET last_ts=?6, last_id=?7, last_from=?8, last_kind=?9, last_text=?10,
                 last_status=?11, unread=?12, ttl=?13, pinned=?14, muted=?15, deleted=?16, pending_read=?17",
            rusqlite::params![
                c.key, c.kind, c.peer, c.lid, c.channel, c.last_ts, c.last_id, c.last_from, c.last_kind,
                c.last_text, c.last_status, c.unread as i64, c.ttl as i64, c.pinned as i64, c.muted as i64,
                c.deleted as i64, serde_json::to_string(&c.pending_read).unwrap_or_else(|_| "[]".into())
            ],
        )
        .map_err(crate::store::StoreError::from)?;
    Ok(())
}

fn new_contact(pk: &str, status: ContactStatus) -> Contact {
    let tone = u8::from_str_radix(pk.get(..2).unwrap_or("0"), 16).unwrap_or(0) % 6;
    Contact {
        pk: pk.to_string(),
        name: String::new(),
        nick: String::new(),
        note: String::new(),
        pic: None,
        tone,
        status,
        verified: false,
        relays: Vec::new(),
        prof_sent: false,
        added_at: now_ms(),
        banner: None,
        games: Vec::new(),
    }
}

fn load_contacts(store: &Store) -> Result<HashMap<String, Contact>> {
    let mut stmt = store
        .raw()
        .prepare("SELECT pk, name, nick, note, pic, tone, status, verified, relays, prof_sent, added_at, extra FROM contacts")
        .map_err(crate::store::StoreError::from)?;
    let rows: Vec<Contact> = stmt
        .query_map([], |r| {
            Ok(Contact {
                pk: r.get(0)?,
                name: r.get(1)?,
                nick: r.get(2)?,
                note: r.get(3)?,
                pic: r.get(4)?,
                tone: r.get::<_, i64>(5)? as u8,
                status: parse_status(&r.get::<_, String>(6)?),
                verified: r.get::<_, i64>(7)? != 0,
                relays: serde_json::from_str(&r.get::<_, String>(8)?).unwrap_or_default(),
                prof_sent: r.get::<_, i64>(9)? != 0,
                added_at: r.get(10)?,
                banner: None,
                games: Vec::new(),
            })
            .map(|mut c| {
                let extra: serde_json::Value = r.get::<_, String>(11).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
                c.banner = extra["b"].as_str().map(str::to_string);
                c.games = extra["g"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
                c
            })
        })
        .map_err(crate::store::StoreError::from)?
        .collect::<std::result::Result<_, _>>()
        .map_err(crate::store::StoreError::from)?;
    Ok(rows.into_iter().map(|c| (c.pk.clone(), c)).collect())
}

fn load_groups(store: &Store) -> Result<HashMap<String, Group>> {
    // Versões antigas do PC guardavam convites não aceitos como grupos com
    // `joined_at = 0`. Agora o convite fica na mensagem; esses registros não
    // são grupos de verdade e não podem receber mensagens.
    store
        .raw()
        .execute("DELETE FROM groups WHERE joined_at = 0", [])
        .map_err(crate::store::StoreError::from)?;
    let mut stmt = store
        .raw()
        .prepare("SELECT lid, pk, sk, old_keys, meta, ver, pic, pic_v, muted, joined_at, pics, gone FROM groups")
        .map_err(crate::store::StoreError::from)?;
    let linhas: Vec<Option<Group>> = stmt
        .query_map([], |r| {
            let meta_json: String = r.get(4)?;
            let old_json: String = r.get(3)?;
            let pics_json: String = r.get(10).unwrap_or_else(|_| "{}".into());
            let gone_json: String = r.get(11).unwrap_or_else(|_| "[]".into());
            let meta: Option<GroupMeta> = serde_json::from_str(&meta_json).ok();
            Ok(meta.map(|meta| Group {
                lid: r.get(0).unwrap_or_default(),
                pk: r.get(1).unwrap_or_default(),
                sk: r.get(2).unwrap_or_default(),
                old_keys: serde_json::from_str(&old_json).unwrap_or_default(),
                meta,
                ver: r.get::<_, i64>(5).unwrap_or(1) as u64,
                pic: r.get(6).unwrap_or(None),
                pic_v: r.get::<_, i64>(7).unwrap_or(0) as u64,
                pics: serde_json::from_str(&pics_json).unwrap_or_default(),
                gone: serde_json::from_str(&gone_json).unwrap_or_default(),
                muted: r.get::<_, i64>(8).unwrap_or(0) != 0,
                joined_at: r.get(9).unwrap_or(0),
            }))
        })
        .map_err(crate::store::StoreError::from)?
        .collect::<std::result::Result<_, _>>()
        .map_err(crate::store::StoreError::from)?;
    Ok(linhas.into_iter().flatten().map(|g| (g.lid.clone(), g)).collect())
}

fn status_name(s: ContactStatus) -> &'static str {
    match s {
        ContactStatus::Request => "request",
        ContactStatus::Ok => "ok",
        ContactStatus::Blocked => "blocked",
    }
}

fn parse_status(s: &str) -> ContactStatus {
    match s {
        "ok" => ContactStatus::Ok,
        "blocked" => ContactStatus::Blocked,
        _ => ContactStatus::Request,
    }
}

fn delivery_name(d: Delivery) -> &'static str {
    match d {
        Delivery::Pending => "pending",
        Delivery::Sent => "sent",
        Delivery::Delivered => "delivered",
        Delivery::Read => "read",
    }
}

fn parse_delivery(s: &str) -> Option<Delivery> {
    match s {
        "pending" | "failed" => Some(Delivery::Pending),
        "sent" => Some(Delivery::Sent),
        "delivered" => Some(Delivery::Delivered),
        "read" => Some(Delivery::Read),
        _ => None,
    }
}

/// Rótulo de um prazo de mensagem temporária.
pub fn ttl_label(secs: u32) -> String {
    match secs {
        0 => "Desligadas".into(),
        s if s < 3600 => format!("{} min", s / 60),
        3600 => "1 hora".into(),
        s if s < 86_400 => format!("{} horas", s / 3600),
        86_400 => "1 dia".into(),
        s => format!("{} dias", s / 86_400),
    }
}

/// Apara espaços e corta em `max` caracteres.
fn clamp(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

/// Corta sem aparar (o texto da mensagem é mantido como foi escrito).
fn clamp_keep(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}

fn random_hex(bytes: usize) -> String {
    let mut b = vec![0u8; bytes];
    rand::Rng::fill(&mut rand::rng(), &mut b[..]);
    hex::encode(b)
}

/// Horário de uma conversa ainda vazia. Em segundos cheios, como o horário
/// das mensagens (que vem do rumor, em segundos): senão uma mensagem mandada
/// no mesmo segundo pareceria mais antiga que a conversa vazia e não
/// apareceria como a última.
fn placeholder_ts() -> i64 {
    now_secs() * 1000
}

/// Tocando a mesma música, o aviso é repetido de tempos em tempos (para
/// quem abriu o app depois).
pub const LISTENING_HEARTBEAT_MS: i64 = 4 * 60_000;
/// Sem aviso novo por este tempo, considera que parou.
pub const LISTENING_TTL_MS: i64 = 9 * 60_000;

fn listening_alive(l: &Listening, recebido: i64, agora: i64) -> bool {
    if agora - recebido > LISTENING_TTL_MS {
        return false;
    }
    match l.d {
        Some(d) if l.st > 0 => agora < l.st + d as i64 * 1000 + 90_000,
        _ => true,
    }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn now_secs() -> i64 {
    now_ms() / 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotulos_de_mensagem_temporaria() {
        assert_eq!(ttl_label(0), "Desligadas");
        assert_eq!(ttl_label(3600), "1 hora");
        assert_eq!(ttl_label(86_400), "1 dia");
        assert_eq!(ttl_label(604_800), "7 dias");
    }

    #[test]
    fn corpo_de_mensagem_vai_e_volta() {
        let corpos = vec![
            MessageBody::Text("oi".into()),
            MessageBody::System { sys: "join".into(), v: 0, legacy: None },
            MessageBody::Sound { id: "aplausos".into() },
            MessageBody::Deleted,
        ];
        for c in corpos {
            let volta = MessageBody::from_stored(c.kind_name(), &c.to_json());
            assert_eq!(volta, c);
        }
    }

    #[test]
    fn aviso_antigo_do_pc_continua_legivel() {
        let b = MessageBody::from_stored("sys", r#"{"text":"Chamada de voz perdida"}"#);
        assert_eq!(b, MessageBody::System { sys: String::new(), v: 0, legacy: Some("Chamada de voz perdida".into()) });
    }

    #[test]
    fn ajustes_antigos_ganham_os_campos_novos() {
        let antigo = r#"{"relays":["wss://nos.lol"],"read_receipts":false,"typing":true,"ttl_default":0}"#;
        let s: Settings = serde_json::from_str(antigo).unwrap();
        assert!(!s.read_receipts);
        assert!(s.notifications);
        assert_eq!(s.calls_from, CallsFrom::Contacts);
        assert_eq!(s.notif_preview, NotifPreview::Name);
    }
}
