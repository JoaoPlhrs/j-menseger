//! A aplicação: telas, mensagens da interface e a entrada na conta.
//!
//! A janela nunca fala com a rede nem com o banco diretamente: ela chama o
//! motor e escuta os avisos dele. Operação demorada vira uma `Task`, que roda
//! fora da thread da interface.

use crate::state::{ChooseKind, ConfirmKind, Main, Popup, Section, SettingsPage, Toggle};
use crate::theme::{self as t, Backdrop};
use iced::widget::text_editor;
use iced::{Element, Point, Size, Subscription, Task};
use jm_core::engine::{ChatSummary, Engine, EngineEvent, Settings};
use jm_core::groups::{ChannelKind, Group, Template};
use jm_core::store::Store;
use jm_media::screen::Quality;
use std::path::PathBuf;
use std::sync::Arc;

/// Em qual tela estamos.
pub enum Screen {
    /// Abrindo a conta que já existe neste computador.
    Loading,
    /// Primeira vez: criar conta ou entrar com as 12 palavras.
    Welcome,
    /// Criando conta (nome → cor → palavras → avisos).
    Create(u8),
    /// Entrando numa conta que já existe (palavras → nome).
    Restore(u8),
    /// Tela principal.
    Main(Box<Main>),
}

/// O que a pessoa digitou nas telas de entrada.
#[derive(Default)]
pub struct Onboard {
    pub name: String,
    pub note: String,
    pub tone: u8,
    pub words: String,
    pub input: String,
    pub notify: bool,
}

pub struct App {
    pub screen: Screen,
    pub onb: Onboard,
    pub error: Option<String>,
    pub busy: bool,
    db_path: PathBuf,
    relay_override: Option<Vec<String>>,
}

/// O que volta quando o motor sobe.
#[derive(Clone)]
pub struct StartedOk {
    pub engine: Arc<Engine>,
    pub chats: Vec<ChatSummary>,
}

impl std::fmt::Debug for StartedOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StartedOk")
    }
}

/// Uma tecla que a janela recebeu e nenhum campo de texto usou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Digit(char),
    Backspace,
    Escape,
    Enter,
    F11,
}

#[derive(Clone)]
pub enum Msg {
    /* entrada */
    OnbGo(Screenish),
    OnbName(String),
    OnbNote(String),
    OnbTone(u8),
    OnbInput(String),
    OnbNotify(bool),
    OnbCreate { words_later: bool },
    OnbRestore,
    Started(Result<StartedOk, String>),

    /* geral */
    Engine(Box<EngineEvent>),
    Tick,
    Focus(bool),
    WindowSize(Size),
    WindowId(Option<iced::window::Id>),
    FileHover(bool),
    FileDropped(PathBuf),
    Key(Key),
    Done(Result<(), String>),
    DoneToast(Result<String, String>),
    Toast(String),
    CloseOverlays,
    /// Só nas versões de desenvolvimento (F9): abre o arquivo de teste.
    TestHook,
    Noop,

    /* navegação */
    Go(Section),
    Page(SettingsPage),
    OpenChat(String),
    CloseChat,
    Search(String),
    ToggleMembers,
    ShowRequests,

    /* dados recarregados */
    ChatsLoaded(Vec<ChatSummary>),
    GroupsLoaded(Vec<Group>),
    MessagesLoaded { key: String, msgs: Vec<jm_core::engine::Message>, older: bool },
    LoadOlder,

    /* conversa */
    Edit(text_editor::Action),
    Send,
    InsertEmoji(String),
    SetPopup(Option<Popup>),
    Reply(String),
    CancelReply,
    PickPhoto,
    PhotoPicked(Result<Option<crate::state::PhotoDraft>, String>),
    SendPhoto,
    SendSound(String),
    RecStart,
    RecStop { send: bool },
    Hover(Option<String>),
    Unhover(String),
    MsgMenu(String),
    Menu(crate::state::CtxKind),
    MenuNow(crate::state::CtxKind),
    React(String, String),
    ReactMore(String),
    Copy(String),
    AskDelete(String),
    DeleteForMe(String),
    DeleteForAll(String),
    PlayVoice(String),
    VoiceReady(String, Arc<Vec<f32>>),
    SeekVoice(String, f32),
    PlaySound(String),
    PlayMediaSound(String),
    OpenViewer(String),
    SavePhoto(String),
    JoinInvite(String),
    OpenUrl(String),

    /* lista de conversas */
    RowMenu(String),
    PinChat(String, bool),
    MuteChat(String, bool),
    MarkRead(String),

    /* contatos */
    OpenAddContact(String),
    OpenMyCode,
    OpenNewChat,
    OpenProfile(String),
    AddCode(String),
    AddNick(String),
    AddPaste,
    AddPasted(Option<String>),
    AddConfirm,
    ContactAdded(Result<String, String>),
    ProfileNick(String),
    ProfileSaveNick,
    ProfileVerify(bool),
    Accept(String),
    Block(String),
    Unblock(String),
    CopyMyCode,
    ShortCodeCreate,
    ShortCodeReady(Result<(String, i64), String>),
    CopyShortCode,
    CopyInvite,

    /* grupos */
    OpenNewGroup,
    NewGroupName(String),
    NewGroupTemplate(Template),
    NewGroupCreate,
    GroupCreated(Result<String, String>),
    GroupMenu(String),
    OpenGroupInfo(String),
    GroupName(String),
    GroupDesc(String),
    GroupSave,
    GroupPhoto(String),
    GroupPhotoRemove(String),
    MuteGroup(String, bool),
    ChannelDraftName(String),
    ChannelDraftKind(ChannelKind),
    ChannelAdd,
    ChannelEdit(String),
    ChannelEditName(String),
    ChannelEditTopic(String),
    ChannelEditSave,
    ChannelMove(String, i8),
    MemberMenu(String, String),
    SetAdmin(String, String, bool),
    OpenInvite(String),
    InviteSend(String, String),

    /* chamadas */
    Call(String, bool),
    AcceptCall,
    DeclineCall,
    HangUp,
    ToggleMute,
    CallUpdated(Option<crate::calls::CallInfo>),
    RoomUpdated(Option<crate::calls::RoomInfo>),
    FrameArrived(u64, iced::widget::image::Handle),
    OpenShare,
    ShareQuality(Quality),
    ShareSource(u32),
    StopSharing,
    CallSound(String),
    SaveBannerSound,
    JoinRoom(String, String),
    LeaveRoom,

    /* ajustes */
    ProfName(String),
    ProfNote(String),
    ProfTone(u8),
    ProfSave,
    ProfPhoto,
    ProfPhotoRemove,
    GifProviderSet(crate::gifs::Provider),
    GifQuery(String),
    GifSearchNow(u64),
    GifResults(u64, crate::gifs::Provider, Result<Vec<crate::gifs::Hit>, String>),
    GifPick(usize),
    GifDownloaded(Result<crate::state::PhotoDraft, String>),
    GifFromFile,
    AnimReady(String, Option<std::sync::Arc<crate::state::Anim>>),
    NowPlaying(Option<crate::nowplaying::Track>),
    /// Um quadro novo da tela (enquanto algo anima).
    Frame(std::time::Instant),
    RailHover(Option<String>),
    SetAnimations(bool),
    ListenShare(bool),
    /// Liga ou desliga a chamada em tela cheia.
    CallFull(bool),
    AudioInput(String),
    AudioOutput(String),
    AudioGain(f32),
    AudioVolume(f32),
    AudioRefresh,
    MicTestToggle,
    MicEcho(bool),
    SpeakerTest,
    BannerPick,
    BannerRemove,
    GameInput(String),
    GameAdd,
    GameRemove(usize),
    Toggle(Toggle, bool),
    Choose(ChooseKind),
    Chosen(ChooseKind, u32),
    Confirm(ConfirmKind),
    ConfirmYes,
    PinToggle,
    PinChange,
    PinKey(char),
    PinBack,
    PinResult(bool),
    PinCreated(Result<jm_core::pin::PinLock, String>),
    PinForgot,
    ForgotInput(String),
    ForgotConfirm,
    OpenRecovery,
    RevealWords,
    RevealNsec,
    CopyNsec,
    WordsDone,
    Accent(String),
    AccentHue(f32),
    SetBackdrop(Backdrop),
    LookReset,
    Volume(f32),
    SbPlay(String),
    SbHide(String),
    SbRestore,
    SbEdit(String),
    SbEditName(String),
    SbEditEmoji(String),
    SbEditSave,
    SbAdd,
    SbRecord,
    SbRecStart,
    SbRecStop,
    SbImport,
    SbDecoded(Result<(String, Arc<Vec<f32>>), String>),
    TrimSet(f64, f64),
    TrimView(f64),
    TrimPlay,
    TrimName(String),
    TrimEmoji(String),
    TrimSave,
    TrimSaved(Result<String, String>),
    RelayInput(String),
    RelayAdd,
    RelayRemove(String),
    RelaysReset,
    TurnUrl(String),
    TurnUser(String),
    TurnPass(String),
    TurnSave,
    Wiped(Result<(), String>),

    /* recorte de foto */
    PhotoForCrop(Result<Option<crate::state::CropDraft>, String>),
    CropSet(f32, f32, f32),
    CropSave,
    CropDone(Result<String, String>),

    /* bloqueio */
    LockKey(char),
    LockBack,
    LockResult(bool),
    LockForgot,
}

/// Destinos das telas de entrada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screenish {
    Welcome,
    Create(u8),
    Restore(u8),
}

impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nada de conteúdo de mensagem, chave ou PIN nos registros.
        f.write_str("Msg")
    }
}

/// Posição do ponteiro, guardada sem passar pela fila de mensagens (mexer o
/// mouse não deve redesenhar a janela inteira).
pub static CURSOR: std::sync::Mutex<Point> = std::sync::Mutex::new(Point::ORIGIN);

pub fn cursor() -> Point {
    *CURSOR.lock().unwrap()
}

impl App {
    pub fn new(db_path: PathBuf, relay_override: Option<Vec<String>>) -> (Self, Task<Msg>) {
        // Se já existe uma conta neste computador, entra direto.
        let existing = Store::open(&db_path).ok().and_then(|s| s.get_kv::<String>("account_sk").ok().flatten());
        let app = Self {
            screen: if existing.is_some() { Screen::Loading } else { Screen::Welcome },
            onb: Onboard { notify: true, tone: rand_tone(), ..Default::default() },
            error: None,
            busy: existing.is_some(),
            db_path: db_path.clone(),
            relay_override: relay_override.clone(),
        };
        let task = match existing {
            Some(sk) => Task::perform(start_engine(db_path, sk, relay_override), Msg::Started),
            None => Task::none(),
        };
        (app, task)
    }

    pub fn title(&self) -> String {
        match &self.screen {
            Screen::Main(m) => m.title(),
            _ => "J-Menseger".into(),
        }
    }

    pub fn theme(&self) -> iced::Theme {
        iced::Theme::custom(
            "J-Menseger",
            iced::theme::Palette {
                background: t::main_bg(),
                text: t::FG,
                primary: t::accent(),
                success: t::accent(),
                warning: t::WARN,
                danger: t::DANGER,
            },
        )
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        let periodo = match &self.screen {
            Screen::Main(m) => m.tick_period(),
            _ => std::time::Duration::from_secs(2),
        };
        let quadros = match &self.screen {
            Screen::Main(m) if m.motion.running() || m.needs_frames() => iced::window::frames().map(Msg::Frame),
            _ => Subscription::none(),
        };
        Subscription::batch([iced::time::every(periodo).map(|_| Msg::Tick), iced::event::listen_with(on_event), quadros])
    }

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::OnbGo(s) => {
                self.error = None;
                self.screen = match s {
                    Screenish::Welcome => Screen::Welcome,
                    Screenish::Create(n) => {
                        if n == 0 && self.onb.words.is_empty() {
                            match jm_core::identity::new_words() {
                                Ok(w) => self.onb.words = w,
                                Err(e) => {
                                    self.error = Some(e.to_string());
                                    return Task::none();
                                }
                            }
                        }
                        if n >= 1 && self.onb.name.trim().chars().count() < 2 {
                            self.error = Some("Escolha um nome com pelo menos 2 letras.".into());
                            return Task::none();
                        }
                        Screen::Create(n)
                    }
                    Screenish::Restore(n) => {
                        if n >= 1 && jm_core::identity::keys_from_any(self.onb.input.trim()).is_err() {
                            self.error = Some("Essas palavras (ou essa chave) não formam uma conta válida. Confira a ordem e a grafia.".into());
                            return Task::none();
                        }
                        Screen::Restore(n)
                    }
                };
                Task::none()
            }
            Msg::OnbName(v) => {
                self.onb.name = v.chars().take(40).collect();
                Task::none()
            }
            Msg::OnbNote(v) => {
                self.onb.note = v.chars().take(80).collect();
                Task::none()
            }
            Msg::OnbTone(v) => {
                self.onb.tone = v % 6;
                Task::none()
            }
            Msg::OnbInput(v) => {
                self.onb.input = v;
                Task::none()
            }
            Msg::OnbNotify(v) => {
                self.onb.notify = v;
                Task::none()
            }
            Msg::OnbCreate { words_later } => {
                let (words, name, note, tone, notify) = (
                    self.onb.words.clone(),
                    self.onb.name.trim().to_string(),
                    self.onb.note.trim().to_string(),
                    self.onb.tone,
                    self.onb.notify,
                );
                let (db, relays) = (self.db_path.clone(), self.relay_override.clone());
                self.busy = true;
                self.error = None;
                Task::perform(
                    async move {
                        let store = Store::open(&db).map_err(|e| e.to_string())?;
                        let keys = Engine::create_identity_full(&store, &words, &name, &note, Some(tone)).map_err(|e| e.to_string())?;
                        store.set_kv("words_pending", &words_later).map_err(|e| e.to_string())?;
                        let mut s: Settings = store.get_kv("settings").ok().flatten().unwrap_or_default();
                        s.notifications = notify;
                        store.set_kv("settings", &s).map_err(|e| e.to_string())?;
                        let sk = keys.secret_key().to_secret_hex();
                        drop(store);
                        start_engine(db, sk, relays).await
                    },
                    Msg::Started,
                )
            }
            Msg::OnbRestore => {
                let entrada = self.onb.input.trim().to_string();
                let nome = self.onb.name.trim().to_string();
                if nome.chars().count() < 2 {
                    self.error = Some("Escolha um nome com pelo menos 2 letras.".into());
                    return Task::none();
                }
                let (db, relays, notify) = (self.db_path.clone(), self.relay_override.clone(), self.onb.notify);
                self.busy = true;
                self.error = None;
                Task::perform(
                    async move {
                        let store = Store::open(&db).map_err(|e| e.to_string())?;
                        let keys = Engine::restore_identity(&store, &entrada, &nome).map_err(|e| e.to_string())?;
                        let mut s: Settings = store.get_kv("settings").ok().flatten().unwrap_or_default();
                        s.notifications = notify;
                        store.set_kv("settings", &s).map_err(|e| e.to_string())?;
                        let sk = keys.secret_key().to_secret_hex();
                        drop(store);
                        start_engine(db, sk, relays).await
                    },
                    Msg::Started,
                )
            }
            Msg::Started(Ok(ok)) => {
                self.busy = false;
                self.onb = Onboard { notify: true, tone: rand_tone(), ..Default::default() };
                let (main, task) = Main::new(ok.engine, ok.chats);
                self.screen = Screen::Main(Box::new(main));
                task
            }
            Msg::Started(Err(e)) => {
                self.busy = false;
                self.error = Some(e);
                if matches!(self.screen, Screen::Loading) {
                    self.screen = Screen::Welcome;
                }
                Task::none()
            }
            Msg::Wiped(r) => {
                if let Err(e) = r {
                    tracing::warn!("apagar a conta: {e}");
                }
                t::apply(&t::Look::default());
                self.screen = Screen::Welcome;
                self.onb = Onboard { notify: true, tone: rand_tone(), ..Default::default() };
                Task::none()
            }
            outra => match &mut self.screen {
                Screen::Main(m) => m.update(outra),
                _ => Task::none(),
            },
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        match &self.screen {
            Screen::Main(m) => m.view(),
            _ => crate::v_onboard::view(self),
        }
    }
}

fn rand_tone() -> u8 {
    (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0) % 6) as u8
}

/// Filtra os eventos da janela. Fica fora da fila o que não precisa redesenhar.
fn on_event(ev: iced::Event, status: iced::event::Status, id: iced::window::Id) -> Option<Msg> {
    use iced::event::Status;
    use iced::keyboard::{key::Named, Event as K, Key as IKey};
    use iced::window::Event as W;
    match ev {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            *CURSOR.lock().unwrap() = position;
            None
        }
        iced::Event::Window(W::Focused) => Some(Msg::Focus(true)),
        iced::Event::Window(W::Unfocused) => Some(Msg::Focus(false)),
        iced::Event::Window(W::Resized(s)) => Some(Msg::WindowSize(s)),
        iced::Event::Window(W::Opened { size, .. }) => {
            let _ = id;
            Some(Msg::WindowSize(size))
        }
        iced::Event::Window(W::FileHovered(_)) => Some(Msg::FileHover(true)),
        iced::Event::Window(W::FilesHoveredLeft) => Some(Msg::FileHover(false)),
        iced::Event::Window(W::FileDropped(p)) => Some(Msg::FileDropped(p)),
        iced::Event::Keyboard(K::KeyPressed { key, text, .. }) => {
            if status == Status::Captured {
                // Um campo de texto já usou a tecla; Esc ainda fecha janelas.
                return matches!(key, IKey::Named(Named::Escape)).then_some(Msg::Key(Key::Escape));
            }
            match key {
                IKey::Named(Named::Escape) => Some(Msg::Key(Key::Escape)),
                IKey::Named(Named::Backspace) => Some(Msg::Key(Key::Backspace)),
                IKey::Named(Named::Enter) => Some(Msg::Key(Key::Enter)),
                IKey::Named(Named::F11) => Some(Msg::Key(Key::F11)),
                IKey::Named(Named::F9) if cfg!(debug_assertions) => Some(Msg::TestHook),
                _ => {
                    let c = text.as_deref().and_then(|s| s.chars().next())?;
                    c.is_ascii_digit().then_some(Msg::Key(Key::Digit(c)))
                }
            }
        }
        _ => None,
    }
}

/// Abre o motor com a chave guardada e entrega os avisos para a interface.
async fn start_engine(db: PathBuf, sk_hex: String, relays: Option<Vec<String>>) -> Result<StartedOk, String> {
    let store = Store::open(&db).map_err(|e| e.to_string())?;
    if let Some(r) = relays {
        let mut s: Settings = store.get_kv("settings").ok().flatten().unwrap_or_default();
        s.relays = r;
        store.set_kv("settings", &s).map_err(|e| e.to_string())?;
    }
    let keys = jm_core::identity::keys_from_any(&sk_hex).map_err(|e| e.to_string())?;
    let (engine, rx) = Engine::open(keys, store).map_err(|e| e.to_string())?;
    engine.start().await.map_err(|e| e.to_string())?;
    let chats = engine.chat_list().unwrap_or_default();
    crate::bridge::install(rx);
    Ok(StartedOk { engine, chats })
}
