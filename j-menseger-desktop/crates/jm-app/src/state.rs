//! Estado da tela principal e o que muda nele.

use crate::app::Msg;
use crate::calls::{CallInfo, CallManager, RoomInfo};
use crate::sounds::SoundBank;
use crate::theme::{self as t, Look};
use iced::widget::{image, text_editor};
use iced::{Size, Task};
use jm_core::engine::{ChatSummary, Contact, ContactStatus, Engine, ExportedKeys, Message as CoreMessage, Profile, Settings};
use jm_core::groups::{ChannelKind, Group, Template};
use jm_core::pin::PinLock;
use jm_core::relays::RelayStatus;
use jm_core::store::CallLog;
use jm_media::recorder::Recorder;
use jm_media::screen::{Quality, Source};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Onde a pessoa está na barra da esquerda.
#[derive(Debug, Clone, PartialEq)]
pub enum Section {
    Chats,
    Calls,
    Group(String),
    Settings,
}

/// Páginas de Ajustes (as mesmas do celular).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPage {
    Profile,
    MyCode,
    Recovery,
    Privacy,
    Blocked,
    Notifications,
    Appearance,
    Soundboard,
    Network,
    Storage,
    About,
    Audio,
    Gifs,
    Devices,
}

/// GIF aberto em quadros, pronto para animar na tela.
pub struct Anim {
    pub frames: Vec<(image::Handle, u32)>,
    pub total: u32,
}

impl Anim {
    pub fn decode(bytes: &[u8], max_side: u32) -> Option<std::sync::Arc<Anim>> {
        let q = jm_media::gif::frames(bytes, max_side).ok()?;
        let frames: Vec<(image::Handle, u32)> = q.into_iter().map(|f| (image::Handle::from_rgba(f.w, f.h, f.rgba), f.delay_ms)).collect();
        let total = frames.iter().map(|f| f.1).sum::<u32>().max(1);
        Some(std::sync::Arc::new(Anim { frames, total }))
    }

    pub fn at(&self, ms: u128) -> image::Handle {
        let mut t = (ms % self.total as u128) as u32;
        for (h, d) in &self.frames {
            if t < *d {
                return h.clone();
            }
            t -= d;
        }
        self.frames[0].0.clone()
    }
}

/// Janelinha de GIFs da conversa.
#[derive(Default)]
pub struct GifPicker {
    pub query: String,
    pub hits: Vec<crate::gifs::Hit>,
    pub loading: bool,
    pub error: Option<String>,
    pub seq: u64,
    /// GIF que está sendo baixado para mandar.
    pub sending: Option<String>,
    /// Serviço dos resultados mostrados.
    pub from: Option<crate::gifs::Provider>,
}

/// Ajustes de voz e áudio (gravados só neste computador).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudioSettings {
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default = "um")]
    pub input_gain: f32,
    #[serde(default = "um")]
    pub output_volume: f32,
}

fn um() -> f32 {
    1.0
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { input: None, output: None, input_gain: 1.0, output_volume: 1.0 }
    }
}

impl AudioSettings {
    pub fn apply(&self) {
        jm_media::audio::set_prefs(jm_media::audio::AudioPrefs {
            input: self.input.clone(),
            output: self.output.clone(),
            input_gain: self.input_gain,
            output_volume: self.output_volume,
        });
    }
}

/// Teste do microfone em andamento: nível ao vivo e, se pedido, a própria
/// voz no fone para conferir.
pub struct MicTest {
    pub cap: jm_media::audio::Capture,
    pub rx: tokio::sync::mpsc::UnboundedReceiver<Vec<i16>>,
    pub levels: Vec<f32>,
    pub eco: Option<jm_media::audio::Playback>,
}

/// Interruptores dos ajustes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    ReadReceipts,
    Typing,
    Notifications,
    Sounds,
    LockScreenshots,
    ForceRelay,
    Presence,
    LinkPreviews,
}

/// Escolhas de uma lista (como o `choose` do celular).
#[derive(Debug, Clone, PartialEq)]
pub enum ChooseKind {
    TtlDefault,
    Ttl(String),
    NotifPreview,
    CallsFrom,
    PinDelay,
}

pub struct ChoiceOption {
    pub value: u32,
    pub label: &'static str,
    pub sub: Option<&'static str>,
}

const fn op(value: u32, label: &'static str) -> ChoiceOption {
    ChoiceOption { value, label, sub: None }
}

impl ChooseKind {
    pub fn title(&self) -> &'static str {
        match self {
            ChooseKind::TtlDefault | ChooseKind::Ttl(_) => "Mensagens temporárias",
            ChooseKind::NotifPreview => "Prévia nas notificações",
            ChooseKind::CallsFrom => "Quem pode me ligar",
            ChooseKind::PinDelay => "Pedir o PIN",
        }
    }

    pub fn sub(&self) -> &'static str {
        match self {
            ChooseKind::TtlDefault => "Vale para conversas que você começar a partir de agora.",
            ChooseKind::Ttl(k) if k.starts_with("dm:") => "As mensagens novas somem para os dois depois do prazo. A outra pessoa recebe o aviso da mudança.",
            ChooseKind::Ttl(_) => "Vale para as mensagens que você mandar neste canal a partir de agora.",
            ChooseKind::NotifPreview => "O que aparece na notificação quando chega uma mensagem.",
            ChooseKind::CallsFrom => "",
            ChooseKind::PinDelay => "Depois de quanto tempo com a janela fora de foco.",
        }
    }

    pub fn options(&self) -> Vec<ChoiceOption> {
        match self {
            ChooseKind::TtlDefault | ChooseKind::Ttl(_) => {
                vec![op(0, "Desligadas"), op(3600, "1 hora"), op(86_400, "1 dia"), op(604_800, "7 dias")]
            }
            ChooseKind::NotifPreview => vec![
                op(0, "Nome e mensagem"),
                op(1, "Só o nome"),
                ChoiceOption { value: 2, label: "Nada", sub: Some("Mostra apenas \"Nova mensagem\"") },
            ],
            ChooseKind::CallsFrom => vec![
                op(0, "Meus contatos"),
                ChoiceOption { value: 1, label: "Ninguém", sub: Some("Chamadas recebidas são ignoradas") },
            ],
            ChooseKind::PinDelay => vec![
                op(0, "Imediatamente"),
                op(60, "Após 1 minuto fora do app"),
                op(300, "Após 5 minutos"),
                op(900, "Após 15 minutos"),
            ],
        }
    }
}

/// Ações que pedem confirmação.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmKind {
    DeleteChat(String),
    ClearMedia,
    ClearCalls,
    Wipe,
    DeleteSound(String),
    LeaveGroup(String),
    RemoveMember(String, String),
    RemoveChannel(String, String),
    Block(String),
}

/// Janelinhas que abrem em cima do campo de mensagem.
#[derive(Debug, Clone, PartialEq)]
pub enum Popup {
    Emoji,
    Sounds,
    Attach,
    CallSounds,
    Gif,
}

/// Para que o teclado de PIN está aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinPurpose {
    Create,
    ConfirmChange,
    Remove,
    Recovery,
    Wipe,
}

pub struct PinPad {
    pub purpose: PinPurpose,
    pub buf: String,
    pub first: Option<String>,
    pub msg: String,
    pub error: bool,
}

impl PinPad {
    pub fn new(purpose: PinPurpose) -> Self {
        let msg = match purpose {
            PinPurpose::Create => "Ele é pedido ao abrir o app. Se esquecer, só restaurando pelas 12 palavras.",
            PinPurpose::ConfirmChange => "Confirme o PIN atual",
            PinPurpose::Remove => "Digite o PIN atual para tirar a senha",
            PinPurpose::Recovery => "Confirme seu PIN para ver a chave",
            PinPurpose::Wipe => "Confirme o PIN para apagar",
        };
        Self { purpose, buf: String::new(), first: None, msg: msg.into(), error: false }
    }

    pub fn title(&self) -> &'static str {
        match (self.purpose, &self.first) {
            (PinPurpose::Create, None) => "Crie um PIN de 4 dígitos",
            (PinPurpose::Create, Some(_)) => "Confirme o PIN",
            _ => "Digite seu PIN",
        }
    }
}

/// Foto escolhida, já preparada, esperando a pessoa confirmar o envio.
#[derive(Clone)]
pub struct PhotoDraft {
    pub bytes: Arc<Vec<u8>>,
    pub mime: String,
    pub w: u32,
    pub h: u32,
    pub th: String,
    pub handle: image::Handle,
}

/// Para onde vai a foto recortada.
#[derive(Debug, Clone, PartialEq)]
pub enum CropTarget {
    Me,
    Group(String),
    /// Banner do seu perfil (retângulo 3:1).
    Banner,
}

impl CropTarget {
    /// Largura ÷ altura do recorte.
    pub fn ratio(&self) -> f32 {
        match self {
            CropTarget::Banner => 3.0,
            _ => 1.0,
        }
    }
}

/// Foto aberta para recortar (já com a rotação do EXIF aplicada).
#[derive(Clone)]
pub struct CropDraft {
    pub target: CropTarget,
    pub bytes: Arc<Vec<u8>>,
    pub w: u32,
    pub h: u32,
    pub handle: image::Handle,
}

pub struct CropState {
    pub target: CropTarget,
    pub bytes: Arc<Vec<u8>>,
    pub handle: image::Handle,
    pub w: u32,
    pub h: u32,
    /// Quadrado escolhido, em pixels da imagem.
    pub x: f32,
    pub y: f32,
    pub side: f32,
    pub busy: bool,
}

/// Editor de corte de um som novo (até 8 s), como no celular.
pub struct TrimState {
    pub pcm: Arc<Vec<f32>>,
    pub dur: f64,
    pub a: f64,
    pub b: f64,
    /// Começo da janela visível (arquivos com mais de 30 s).
    pub v0: f64,
    pub name: String,
    pub emoji: String,
    pub busy: bool,
    pub peaks: Vec<f32>,
    pub overview: Vec<f32>,
}

pub const TRIM_MAX: f64 = 8.0;
pub const TRIM_MIN: f64 = 0.3;
pub const TRIM_VIEW: f64 = 30.0;

impl TrimState {
    pub fn new(name: String, pcm: Arc<Vec<f32>>) -> Self {
        let dur = pcm.len() as f64 / 48_000.0;
        let (mut a, mut b) = jm_media::audiofile::auto_trim(&pcm);
        if b - a < TRIM_MIN {
            a = 0.0;
            b = dur.min(TRIM_MAX);
        }
        if b - a > TRIM_MAX {
            b = a + TRIM_MAX;
        }
        let overview = if dur > TRIM_VIEW { jm_media::audiofile::peaks(&pcm, 0.0, dur, 160) } else { Vec::new() };
        let mut s = Self { pcm, dur, a, b, v0: 0.0, name, emoji: "🔊".into(), busy: false, peaks: Vec::new(), overview };
        s.fit_view();
        s.refresh_peaks();
        s
    }

    pub fn vlen(&self) -> f64 {
        self.dur.min(TRIM_VIEW)
    }

    pub fn fit_view(&mut self) {
        let vlen = self.vlen();
        if self.a < self.v0 || self.b > self.v0 + vlen {
            self.v0 = (self.a - (vlen - (self.b - self.a)) / 2.0).clamp(0.0, (self.dur - vlen).max(0.0));
        }
    }

    pub fn refresh_peaks(&mut self) {
        self.peaks = jm_media::audiofile::peaks(&self.pcm, self.v0, self.v0 + self.vlen(), 180);
    }
}

pub struct GroupForm {
    pub lid: String,
    pub name: String,
    pub desc: String,
    pub ch_name: String,
    pub ch_kind: ChannelKind,
}

/// Janelas sobre a tela principal.
pub enum Modal {
    AddContact { code: String, nick: String, error: Option<String>, busy: bool },
    MyCode,
    NewChat,
    NewGroup { name: String, template: Template, busy: bool },
    Invite { lid: String },
    Profile { pk: String, nick: String },
    Choose(ChooseKind),
    Confirm(ConfirmKind),
    DeleteMsg { id: String, mine: bool },
    ReactPicker { id: String },
    Pin(PinPad),
    Forgot { input: String, error: Option<String> },
    Viewer { mid: String },
    PhotoPreview(PhotoDraft),
    GroupInfo(GroupForm),
    EditChannel { lid: String, id: String, name: String, topic: String },
    AddSound,
    RecordSound { rec: Option<Recorder>, levels: Vec<f32> },
    Trim(TrimState),
    EditSound { id: String, name: String, emoji: String },
    Share { sources: Vec<Source> },
    Crop(CropState),
    Search { all: Vec<CoreMessage>, query: String },
    Forward { id: String, query: String },
    PeerVolume { pk: String, v: f32 },
    SetStatus { e: String, t: String, dnd: bool, dur: u8 },
    NewEvent { title: String, date: String, time: String, desc: String },
    ViewOnce { handle: image::Handle },
    Pins { list: Vec<CoreMessage> },
    NewPoll { q: String, opts: Vec<String>, multi: bool },
    Voters { id: String },
    Roles { lid: String, form: Option<RoleForm> },
    MemberRoles { lid: String, pk: String },
}

/// Menu de contexto (botão direito), na posição do ponteiro.
pub struct CtxMenu {
    pub at: iced::Point,
    pub kind: CtxKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CtxKind {
    Message(String),
    Chat(String),
    Group(String),
    Member(String, String),
    New,
}

/// A conversa aberta.
pub struct OpenChat {
    pub key: String,
    pub messages: Vec<CoreMessage>,
    pub more: bool,
    pub editor: text_editor::Content,
    pub reply: Option<String>,
    pub progress: HashMap<String, (u32, u32)>,
    /// Mensagem sendo editada (o texto dela vai para o campo).
    pub edit: Option<String>,
    /// Mensagens fixadas (a mais recente primeiro).
    pub pins: Vec<String>,
}

/// Formulário de cargo (novo quando `id` é `None`).
#[derive(Debug, Clone, Default)]
pub struct RoleForm {
    pub id: Option<String>,
    pub name: String,
    pub c: u8,
    /// Poderes marcados.
    pub p: Vec<String>,
}

pub const PAGE: usize = 60;

/// Gravação de mensagem de voz em andamento.
pub struct VoiceRec {
    pub rec: Recorder,
    pub chat: String,
    pub levels: Vec<f32>,
}

/// Tela de bloqueio do PIN.
#[derive(Default)]
pub struct LockState {
    pub buf: String,
    pub tries: u32,
    pub wait_until: Option<Instant>,
    pub msg: String,
    pub error: bool,
    pub forgot: Option<(String, Option<String>)>,
}

/// Campos de texto das páginas de ajustes.
#[derive(Default)]
pub struct Forms {
    pub prof_name: String,
    pub prof_note: String,
    pub prof_tone: u8,
    pub relay: String,
    pub turn_url: String,
    pub turn_user: String,
    pub turn_pass: String,
    pub game: String,
    pub keys: Option<ExportedKeys>,
    pub reveal_words: bool,
    pub reveal_nsec: bool,
    pub hue: f32,
}

pub struct Main {
    pub engine: Arc<Engine>,
    pub calls: Arc<CallManager>,
    pub sounds: Arc<SoundBank>,
    pub me: Profile,
    pub settings: Settings,
    pub look: Look,
    pub pin: Option<PinLock>,
    pub words_pending: bool,

    pub section: Section,
    pub page: SettingsPage,
    pub show_requests: bool,
    pub chats: Vec<ChatSummary>,
    pub groups: Vec<Group>,
    pub contacts: HashMap<String, Contact>,
    pub call_log: Vec<CallLog>,
    pub relays: Vec<RelayStatus>,
    pub rooms: HashMap<(String, String), Vec<String>>,
    pub chat: Option<OpenChat>,
    pub drafts: HashMap<String, String>,
    pub typing: HashMap<String, (String, i64)>,
    pub search: String,
    pub show_members: bool,

    pub call: Option<CallInfo>,
    pub room: Option<RoomInfo>,
    pub joining_room: bool,
    pub remote_frame: Option<(u64, image::Handle)>,
    /// Última imagem de cada pessoa da sala que está mandando vídeo.
    pub room_frames: HashMap<String, (u64, image::Handle)>,
    /// Sua câmera, pequena (versão, imagem).
    pub local_preview: Option<(u64, image::Handle)>,
    /// Ligar a câmera assim que a chamada de vídeo conectar.
    pub camera_wanted: bool,
    /// Atualização encontrada no site.
    pub update: Option<crate::atualizar::Pacote>,
    /// Progresso do download (milésimos), enquanto baixa.
    pub update_prog: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
    pub update_msg: Option<String>,
    pub update_at: Instant,
    /// Último aviso de online/ausente (ausente?, quando).
    pub pres_sent: Option<(bool, Instant)>,
    /// Fechar a janela deixa o app na bandeja (Windows).
    pub tray_close: bool,
    /// Já avisou que o app continua na bandeja.
    pub tray_told: bool,
    /// Abre junto com o Windows.
    pub autostart: bool,
    /// Última conferência da sincronia com os outros aparelhos da conta.
    pub sync_at: Instant,
    /// Último aviso "este aparelho existe" para os outros aparelhos da conta.
    pub dev_at: Option<Instant>,
    /// Última conferência dos lembretes de eventos.
    pub evt_at: Instant,
    /// Mensagem destacada depois de pular até ela.
    pub flash: Option<(String, Instant)>,
    pub share_quality: Quality,
    /// Mandar também o áudio do PC ao transmitir (Windows).
    pub share_audio: bool,

    pub modal: Option<Modal>,
    pub popup: Option<Popup>,
    pub menu: Option<CtxMenu>,
    pub toast: Option<(String, Instant)>,
    pub hover: Option<String>,
    pub file_hover: bool,

    pub forms: Forms,
    pub rec: Option<VoiceRec>,
    pub track: Option<(String, f32, f32, bool)>,
    pub storage: (i64, i64),
    pub lock: Option<LockState>,
    pub focused: bool,
    pub blurred_at: Option<Instant>,
    pub window: Option<iced::window::Id>,
    pub size: Size,

    pub chats_loading: bool,
    pub chats_dirty: bool,
    /// QR do convite (refeito quando o nome muda).
    pub qr: Option<iced::widget::qr_code::Data>,
    pub motion: crate::motion::Motion,
    /// Mostrar aos contatos a música do Spotify.
    pub listen_share: bool,
    /// Música vista agora no Spotify e quando começou (ms).
    pub now_playing: Option<(crate::nowplaying::Track, i64)>,
    pub np_polling: bool,
    pub np_at: Instant,
    pub gif_keys: crate::gifs::GifKeys,
    pub gif: GifPicker,
    /// GIFs já abertos em quadros (por mídia ou endereço da prévia).
    pub anims: HashMap<String, std::sync::Arc<Anim>>,
    pub anim_loading: std::collections::HashSet<String>,
    pub anim_clock: Instant,
    /// Chamada ou sala em tela cheia (palco no estilo Discord).
    pub call_full: bool,
    pub audio: AudioSettings,
    pub audio_inputs: Vec<String>,
    pub audio_outputs: Vec<String>,
    pub mic_test: Option<MicTest>,
    pub mic_error: Option<String>,
    /// Código de convite curto e quando vence (segundos).
    pub short_code: Option<(String, i64)>,
    pub short_code_busy: bool,
    pics: RefCell<HashMap<u64, Option<image::Handle>>>,
    media: RefCell<HashMap<String, Option<image::Handle>>>,
}

impl Main {
    pub fn new(engine: Arc<Engine>, chats: Vec<ChatSummary>) -> (Self, Task<Msg>) {
        // As preferências de áudio valem antes de abrir a saída de som.
        engine.ui_pref::<AudioSettings>("audio").unwrap_or_default().apply();
        let player = match jm_media::player::Player::new() {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!("sem saída de som: {e}");
                None
            }
        };
        let sounds = SoundBank::new(Arc::clone(&engine), player);
        let calls = CallManager::new(Arc::clone(&engine), Arc::clone(&sounds));
        let look: Look = engine.ui_pref("look").unwrap_or_default();
        t::apply(&look);
        let me = engine.me();
        let settings = engine.settings();
        let pin = engine.pin_lock();
        let contacts = engine.contacts().into_iter().map(|c| (c.pk.clone(), c)).collect();
        let forms = Forms {
            prof_name: me.name.clone(),
            prof_note: me.note.clone(),
            prof_tone: me.tone,
            turn_url: settings.turn.url.clone(),
            turn_user: settings.turn.user.clone(),
            turn_pass: settings.turn.pass.clone(),
            hue: t::hue_of(t::accent()),
            ..Default::default()
        };
        let m = Self {
            groups: engine.groups(),
            call_log: engine.calls(200),
            words_pending: engine.words_pending(),
            storage: engine.storage_usage(),
            lock: pin.as_ref().map(|_| LockState { msg: "Digite seu PIN".into(), ..Default::default() }),
            calls,
            sounds,
            me,
            settings,
            look,
            pin,
            section: Section::Chats,
            page: SettingsPage::Profile,
            show_requests: false,
            chats,
            contacts,
            relays: Vec::new(),
            rooms: HashMap::new(),
            chat: None,
            drafts: HashMap::new(),
            typing: HashMap::new(),
            search: String::new(),
            show_members: true,
            call: None,
            room: None,
            joining_room: false,
            remote_frame: None,
            room_frames: HashMap::new(),
            local_preview: None,
            camera_wanted: false,
            update: None,
            update_prog: None,
            update_msg: None,
            update_at: Instant::now() - Duration::from_secs(6 * 3600 - 8),
            pres_sent: None,
            tray_close: engine.ui_pref::<bool>("tray").unwrap_or(true),
            tray_told: false,
            autostart: crate::tray::autostart(),
            sync_at: Instant::now() - Duration::from_secs(50),
            evt_at: Instant::now(),
            dev_at: None,
            flash: None,
            share_audio: engine.ui_pref::<bool>("share_audio").unwrap_or(true),
            share_quality: Quality::P1080,
            modal: None,
            popup: None,
            menu: None,
            toast: None,
            hover: None,
            file_hover: false,
            forms,
            rec: None,
            track: None,
            focused: true,
            blurred_at: None,
            window: None,
            size: Size::new(1180.0, 760.0),
            chats_loading: false,
            chats_dirty: false,
            qr: None,
            call_full: false,
            motion: Default::default(),
            listen_share: false,
            now_playing: None,
            np_polling: false,
            np_at: Instant::now() - Duration::from_secs(60),
            gif_keys: Default::default(),
            gif: GifPicker::default(),
            anims: HashMap::new(),
            anim_loading: Default::default(),
            anim_clock: Instant::now(),
            audio: AudioSettings::default(),
            audio_inputs: Vec::new(),
            audio_outputs: Vec::new(),
            mic_test: None,
            mic_error: None,
            short_code: None,
            short_code_busy: false,
            pics: RefCell::new(HashMap::new()),
            media: RefCell::new(HashMap::new()),
            engine,
        };
        let mut m = m;
        m.refresh_qr();
        m.short_code = m.engine.short_code();
        m.audio = m.engine.ui_pref("audio").unwrap_or_default();
        m.gif_keys = m.engine.ui_pref("gifs").unwrap_or_default();
        m.listen_share = m.engine.ui_pref("spotify").unwrap_or(false);
        m.motion.enabled = m.engine.ui_pref("animacoes").unwrap_or(true);
        let e = Arc::clone(&m.engine);
        let task = Task::batch([
            iced::window::oldest().map(Msg::WindowId),
            Task::perform(async move { e.relay_status().await }, |st| {
                Msg::Engine(Box::new(jm_core::engine::EngineEvent::Relays(st)))
            }),
        ]);
        (m, task)
    }
}

/* ======================================================================
 *  Consultas usadas pelas telas
 * ====================================================================== */

impl Main {
    pub fn title(&self) -> String {
        let unread: u32 = self.chats.iter().filter(|c| !c.muted).map(|c| c.unread).sum();
        let base = match &self.chat {
            Some(c) => format!("{} · J-Menseger", self.chat_title(&c.key)),
            None => "J-Menseger".into(),
        };
        if unread > 0 {
            format!("({unread}) {base}")
        } else {
            base
        }
    }

    /// Ritmo do relógio: devagar parado, rápido quando algo se mexe na tela.
    pub fn tick_period(&self) -> Duration {
        let video = self.call.as_ref().is_some_and(|c| c.receiving_video);
        if video {
            Duration::from_millis(33)
        } else if self.call.is_some()
            || self.room.is_some()
            || self.joining_room
            || self.rec.is_some()
            || self.track.as_ref().is_some_and(|t| !t.3)
            || matches!(self.modal, Some(Modal::RecordSound { .. }) | Some(Modal::Trim(_)))
            || self.mic_test.is_some()
        {
            Duration::from_millis(80)
        } else if self.animating() {
            Duration::from_millis(60)
        } else if self.toast.is_some() || self.lock.as_ref().is_some_and(|l| l.wait_until.is_some()) {
            Duration::from_millis(500)
        } else {
            Duration::from_secs(2)
        }
    }

    /// Link de convite, o mesmo do QR do celular.
    pub fn invite_link(&self) -> String {
        let nome: String = self.me.name.chars().filter(|c| c.is_alphanumeric() || *c == ' ').collect::<String>().trim().replace(' ', "+");
        format!("jmenseger://add?k={}&n={}", self.engine.npub(), nome)
    }

    pub fn refresh_qr(&mut self) {
        self.qr = iced::widget::qr_code::Data::new(self.invite_link()).ok();
    }

    /// Há GIF animando na tela (só com a janela em foco).
    pub fn animating(&self) -> bool {
        if !self.focused {
            return false;
        }
        match &self.modal {
            None | Some(Modal::Viewer { .. }) => {}
            Some(Modal::PhotoPreview(d)) => return is_gif_mime(&d.mime) && self.anims.contains_key(PREVIA_GIF),
            Some(_) => return false,
        }
        if self.popup == Some(Popup::Gif) && !self.gif.hits.is_empty() {
            return true;
        }
        self.chat.as_ref().is_some_and(|c| {
            c.messages.iter().rev().take(40).any(|m| matches!(&m.body, jm_core::engine::MessageBody::Media(x) if is_gif_mime(&x.mime) && self.anims.contains_key(&x.mid)))
        })
    }

    /// Chave do item selecionado na barra da esquerda.
    pub fn rail_key(&self) -> String {
        match &self.section {
            Section::Chats => "chats".into(),
            Section::Calls => "calls".into(),
            Section::Group(lid) => lid.clone(),
            Section::Settings => "settings".into(),
        }
    }

    /// Como a tela está, para saber o que acabou de aparecer.
    pub fn snapshot(&self) -> crate::motion::Snapshot {
        crate::motion::Snapshot {
            modal: self.modal.as_ref().map(std::mem::discriminant),
            modal_key: match &self.modal {
                Some(Modal::Profile { pk, .. }) => pk.clone(),
                Some(Modal::Viewer { mid }) => mid.clone(),
                _ => String::new(),
            },
            popup: self.popup.clone(),
            menu: self.menu.is_some(),
            toast: self.toast.as_ref().map(|t| t.0.clone()),
            page: if self.section == Section::Settings { format!("{:?}", self.page) } else { String::new() },
            rail_sel: self.rail_key(),
            ringing: self.call.as_ref().is_some_and(|c| matches!(c.state, crate::calls::CallState::Ringing { .. })),
        }
    }

    /// Algo na tela anima o tempo todo (pontinhos de "digitando", aviso de
    /// chamada, carregando).
    pub fn needs_frames(&self) -> bool {
        if !self.focused || !self.motion.enabled {
            return false;
        }
        let agora = jm_core::engine::now_ms();
        self.typing.values().any(|(_, ate)| *ate > agora)
            || self.call.as_ref().is_some_and(|c| matches!(c.state, crate::calls::CallState::Ringing { .. }))
            || (self.popup == Some(Popup::Gif) && (self.gif.loading || self.gif.sending.is_some()))
    }

    /// O que alguém está ouvindo (você incluído).
    pub fn listening(&self, pk: &str) -> Option<jm_core::messages::Listening> {
        if pk == self.me_pk() {
            self.engine.my_listening()
        } else {
            self.engine.listening_of(pk)
        }
    }

    /// Quadro atual de um GIF já aberto.
    pub fn anim_frame(&self, key: &str) -> Option<image::Handle> {
        self.anims.get(key).map(|a| a.at(self.anim_clock.elapsed().as_millis()))
    }

    /// Está numa chamada atendida ou numa sala de voz.
    pub fn in_voice(&self) -> bool {
        self.room.is_some()
            || self.call.as_ref().is_some_and(|c| !matches!(c.state, crate::calls::CallState::Ended { .. } | crate::calls::CallState::Ringing { .. }))
    }

    pub fn me_pk(&self) -> String {
        self.engine.pk_hex()
    }

    /// Nome de alguém como aparece na tela.
    pub fn name_of(&self, pk: &str) -> String {
        if pk == self.me_pk() {
            return self.me.name.clone();
        }
        if let Some(c) = self.contacts.get(pk) {
            if !c.nick.is_empty() || !c.name.is_empty() {
                return c.display_name();
            }
        }
        for g in &self.groups {
            if let Some(n) = g.meta.name_of(pk) {
                if !n.is_empty() {
                    return n.to_string();
                }
            }
        }
        match self.contacts.get(pk) {
            Some(c) => c.display_name(),
            None => jm_core::identity::short_hex(pk),
        }
    }

    pub fn first_name(&self, pk: &str) -> String {
        self.name_of(pk).split_whitespace().next().unwrap_or("").to_string()
    }

    pub fn tone_of(&self, pk: &str) -> u8 {
        if pk == self.me_pk() {
            return self.me.tone;
        }
        self.contacts.get(pk).map(|c| c.tone).unwrap_or_else(|| (pk.bytes().map(|b| b as u32).sum::<u32>() % 6) as u8)
    }

    /// Foto de alguém (sua, de um contato ou a que veio pelo grupo).
    pub fn pic_url(&self, pk: &str) -> Option<String> {
        if pk == self.me_pk() {
            return self.me.pic.clone();
        }
        if let Some(p) = self.contacts.get(pk).and_then(|c| c.pic.clone()) {
            return Some(p);
        }
        self.groups.iter().find_map(|g| g.pics.get(pk).cloned())
    }

    /// Imagem pronta para desenhar a partir de uma data URL (guardada).
    pub fn pic(&self, url: Option<&str>) -> Option<image::Handle> {
        let url = url?;
        let chave = hash(url);
        if let Some(h) = self.pics.borrow().get(&chave) {
            return h.clone();
        }
        let h = jm_media::photo::from_data_url(url).map(image::Handle::from_bytes);
        self.pics.borrow_mut().insert(chave, h.clone());
        h
    }

    pub fn pic_of(&self, pk: &str) -> Option<image::Handle> {
        self.pic(self.pic_url(pk).as_deref())
    }

    /// Foto de uma mensagem, lida do banco uma vez e guardada.
    pub fn media_image(&self, mid: &str) -> Option<image::Handle> {
        if let Some(h) = self.media.borrow().get(mid) {
            return h.clone();
        }
        let h = self.engine.media_bytes(mid).map(image::Handle::from_bytes);
        if h.is_some() {
            self.media.borrow_mut().insert(mid.to_string(), h.clone());
        }
        h
    }

    pub fn forget_media(&self) {
        self.media.borrow_mut().clear();
    }

    pub fn group(&self, lid: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.lid == lid)
    }

    pub fn is_admin(&self, lid: &str) -> bool {
        let me = self.me_pk();
        self.group(lid).is_some_and(|g| g.meta.is_admin(&me))
    }

    /// Título de uma conversa: nome da pessoa ou `# canal`.
    pub fn chat_title(&self, key: &str) -> String {
        match jm_core::messages::ChatKey::parse(key) {
            Some(jm_core::messages::ChatKey::Direct { peer }) => self.name_of(&peer),
            Some(jm_core::messages::ChatKey::Channel { lid, channel }) => {
                let ch = self
                    .group(&lid)
                    .and_then(|g| g.meta.channels.iter().find(|c| c.id == channel))
                    .map(|c| c.name.clone())
                    .unwrap_or(channel);
                format!("# {ch}")
            }
            None => key.to_string(),
        }
    }

    pub fn summary(&self, key: &str) -> Option<&ChatSummary> {
        self.chats.iter().find(|c| c.key == key)
    }

    pub fn peer_of(key: &str) -> Option<String> {
        key.strip_prefix("dm:").map(|s| s.to_string())
    }

    pub fn contact_status(&self, pk: &str) -> Option<ContactStatus> {
        self.contacts.get(pk).map(|c| c.status)
    }

    pub fn requests(&self) -> Vec<&Contact> {
        let mut v: Vec<&Contact> = self.contacts.values().filter(|c| c.status == ContactStatus::Request).collect();
        v.sort_by_key(|c| std::cmp::Reverse(c.added_at));
        v
    }

    pub fn blocked(&self) -> Vec<&Contact> {
        let mut v: Vec<&Contact> = self.contacts.values().filter(|c| c.status == ContactStatus::Blocked).collect();
        v.sort_by_key(|c| c.display_name().to_lowercase());
        v
    }

    pub fn friends(&self) -> Vec<&Contact> {
        let mut v: Vec<&Contact> = self.contacts.values().filter(|c| c.status == ContactStatus::Ok).collect();
        v.sort_by_key(|c| c.display_name().to_lowercase());
        v
    }

    pub fn typing_in(&self, key: &str) -> Option<&str> {
        self.typing.get(key).map(|(w, _)| w.as_str())
    }

    pub fn open_relays(&self) -> usize {
        self.relays.iter().filter(|r| r.connected).count()
    }

    /// O texto menciona a pessoa (`@nome` ou `@todos`) num grupo?
    /// `@todos` só vale de quem tem o poder (admin ou cargo com "todos").
    pub fn mentions_me(&self, chat: &str, texto: &str, autor: &str) -> bool {
        let Some(lid) = chat.strip_prefix("g:").and_then(|r| r.split(':').next()) else { return false };
        let t = texto.to_lowercase();
        if (t.contains("@todos") || t.contains("@everyone")) && self.group(lid).is_some_and(|g| g.meta.can(autor, "todos")) {
            return true;
        }
        self.my_names(lid).iter().any(|n| t.contains(&format!("@{n}")))
    }

    /// Nomes pelos quais a pessoa pode ser mencionada num grupo (minúsculos).
    pub fn my_names(&self, lid: &str) -> Vec<String> {
        let me = self.me_pk();
        let mut v = vec![self.me.name.clone()];
        if let Some(n) = self.group(lid).and_then(|g| g.meta.names.get(&me).cloned()) {
            v.push(n);
        }
        v.into_iter().map(|n| n.trim().to_lowercase()).filter(|n| !n.is_empty()).collect()
    }

    pub fn toast(&mut self, s: impl Into<String>) {
        self.toast = Some((s.into(), Instant::now() + Duration::from_millis(2800)));
    }

    pub fn group_unread(&self, lid: &str) -> u32 {
        let prefixo = format!("g:{lid}:");
        self.chats.iter().filter(|c| c.key.starts_with(&prefixo)).map(|c| c.unread).sum()
    }

    pub fn dm_unread(&self) -> u32 {
        self.chats
            .iter()
            .filter(|c| !c.is_group && !c.muted)
            .filter(|c| c.peer.as_deref().and_then(|p| self.contact_status(p)) == Some(ContactStatus::Ok))
            .map(|c| c.unread)
            .sum()
    }
}

fn hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Chave do GIF aberto na prévia antes de mandar.
pub const PREVIA_GIF: &str = "__previa_gif__";

pub fn is_gif_mime(m: &str) -> bool {
    m.to_ascii_lowercase().starts_with("image/gif")
}
