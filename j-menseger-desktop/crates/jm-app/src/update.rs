//! O que cada ação da interface faz.

use crate::app::{cursor, Key, Msg};
use crate::calls::CallState;
use crate::platform;
use crate::state::*;
use crate::theme::{self as t, Look};
use iced::widget::{image, text_editor};
use iced::Task;
use jm_core::engine::{CallsFrom, ContactStatus, Engine, EngineEvent, MediaMeta, MessageBody, NotifPreview};
use jm_core::groups::{ChannelKind, GroupPatch};
use jm_core::messages::{MediaKind, SoundMeta};
use jm_media::screen::Fps;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const EDITOR_ID: &str = "composer";
pub const GIF_INPUT_ID: &str = "gif-busca";
pub const MESSAGES_ID: &str = "messages";

/// Limite de uma mensagem de voz (cabe com folga no tamanho máximo de mídia).
const VOICE_MAX_SECS: f64 = 600.0;
/// Limite da gravação de um som novo, antes do corte (como no celular).
const SOUND_REC_MAX: f64 = 10.0;

fn done(fut: impl std::future::Future<Output = Result<(), String>> + Send + 'static) -> Task<Msg> {
    Task::perform(fut, Msg::Done)
}

fn toast_done(fut: impl std::future::Future<Output = Result<String, String>> + Send + 'static) -> Task<Msg> {
    Task::perform(fut, Msg::DoneToast)
}

fn s<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

impl Main {
    fn eng(&self) -> Arc<Engine> {
        Arc::clone(&self.engine)
    }

    fn open_key(&self) -> Option<String> {
        self.chat.as_ref().map(|c| c.key.clone())
    }

    pub fn reload_chats(&mut self) -> Task<Msg> {
        if self.chats_loading {
            self.chats_dirty = true;
            return Task::none();
        }
        self.chats_loading = true;
        let e = self.eng();
        Task::perform(async move { e.chat_list().unwrap_or_default() }, Msg::ChatsLoaded)
    }

    fn reload_groups(&mut self) -> Task<Msg> {
        let e = self.eng();
        Task::perform(async move { e.groups() }, Msg::GroupsLoaded)
    }

    fn reload_contacts(&mut self) {
        self.contacts = self.engine.contacts().into_iter().map(|c| (c.pk.clone(), c)).collect();
    }

    fn poll_call(&self) -> Task<Msg> {
        let c = Arc::clone(&self.calls);
        Task::perform(async move { c.info().await }, Msg::CallUpdated)
    }

    fn poll_room(&self) -> Task<Msg> {
        let c = Arc::clone(&self.calls);
        Task::perform(async move { c.room_info().await }, Msg::RoomUpdated)
    }

    fn close_overlays(&mut self) {
        self.menu = None;
        self.popup = None;
    }

    fn apply_protection(&self) -> Task<Msg> {
        let Some(id) = self.window else { return Task::none() };
        let on = self.settings.lock_screenshots;
        iced::window::run(id, move |w| platform::protect_window(w, on)).map(move |ok| {
            if on && !ok && platform::can_protect_window() {
                Msg::Toast("Este Windows não permite esconder a janela das capturas.".into())
            } else {
                Msg::Noop
            }
        })
    }

    fn save_look(&mut self) {
        t::apply(&self.look);
        self.forms.hue = t::hue_of(t::accent());
        if let Err(e) = self.engine.set_ui_pref("look", &self.look) {
            tracing::warn!("aparência não salva: {e}");
        }
    }

    /// Manda uma foto ou GIF já preparado na conversa aberta.
    fn send_draft(&mut self, d: PhotoDraft) -> Task<Msg> {
        self.send_draft_opt(d, false)
    }

    fn send_draft_opt(&mut self, d: PhotoDraft, view_once: bool) -> Task<Msg> {
        let Some(key) = self.open_key() else { return Task::none() };
        let e = self.eng();
        let meta = MediaMeta {
            kind: Some(MediaKind::Image),
            mime: d.mime.clone(),
            w: Some(d.w),
            h: Some(d.h),
            th: Some(d.th.clone()),
            view_once,
            ..Default::default()
        };
        done(async move { e.send_media(&key, d.bytes.to_vec(), meta).await.map(|_| ()).map_err(s) })
    }

    /// Conta (ou para de contar) aos contatos o que está tocando.
    fn share_listening(&mut self) -> Task<Msg> {
        let l = self.now_playing.as_ref().filter(|_| self.listen_share).map(|(t, desde)| jm_core::messages::Listening {
            s: t.title.clone(),
            a: t.artist.clone(),
            al: t.album.clone(),
            id: t.id.clone(),
            st: *desde,
            d: t.secs,
        });
        let e = self.eng();
        Task::perform(async move { e.set_listening(l).await }, |_| Msg::Noop)
    }

    fn gif_changed(&mut self) {
        self.gif.hits.clear();
        self.gif.error = None;
        self.gif.from = None;
        if let Err(e) = self.engine.set_ui_pref("gifs", &self.gif_keys) {
            tracing::warn!("chaves de GIF não salvas: {e}");
        }
    }

    fn gif_search(&mut self) -> Task<Msg> {
        let Some((p, k)) = self.gif_keys.usable() else { return Task::none() };
        self.gif.seq += 1;
        let seq = self.gif.seq;
        self.gif.loading = true;
        let q = self.gif.query.clone();
        Task::perform(
            async move { tokio::task::spawn_blocking(move || crate::gifs::search(p, &k, &q)).await.map_err(|e| e.to_string()).and_then(|r| r) },
            move |r| Msg::GifResults(seq, p, r),
        )
    }

    fn anim_from_url(&mut self, url: String) -> Task<Msg> {
        if self.anims.contains_key(&url) || !self.anim_loading.insert(url.clone()) {
            return Task::none();
        }
        let k = url.clone();
        Task::perform(
            async move { tokio::task::spawn_blocking(move || crate::gifs::download_preview(&url).ok().and_then(|b| Anim::decode(&b, 240))).await.ok().flatten() },
            move |a| Msg::AnimReady(k.clone(), a),
        )
    }

    fn anim_from_media(&mut self, mid: String) -> Task<Msg> {
        if !self.anim_loading.insert(mid.clone()) {
            return Task::none();
        }
        let e = self.eng();
        let k = mid.clone();
        Task::perform(
            async move { tokio::task::spawn_blocking(move || e.media_bytes(&mid).and_then(|b| Anim::decode(&b, 640))).await.ok().flatten() },
            move |a| Msg::AnimReady(k.clone(), a),
        )
    }

    fn save_audio(&mut self) {
        self.audio.apply();
        if let Err(e) = self.engine.set_ui_pref("audio", &self.audio) {
            tracing::warn!("ajustes de áudio não salvos: {e}");
        }
    }

    fn refresh_devices(&mut self) {
        let mut i: Vec<String> = vec![PADRAO.to_string()];
        i.extend(jm_media::audio::input_devices().into_iter().map(|d| d.name));
        let mut o: Vec<String> = vec![PADRAO.to_string()];
        o.extend(jm_media::audio::output_devices().into_iter().map(|d| d.name));
        self.audio_inputs = i;
        self.audio_outputs = o;
    }

    fn start_mic_test(&mut self, eco: bool) {
        self.mic_test = None;
        if self.call.is_some() || self.room.is_some() {
            self.mic_error = Some("O microfone está na chamada agora. Teste depois de desligar.".into());
            return;
        }
        match jm_media::audio::Capture::start(None) {
            Ok((cap, rx)) => {
                self.mic_error = None;
                let eco = if eco {
                    match jm_media::audio::Playback::start(None) {
                        Ok(p) => Some(p),
                        Err(e) => {
                            self.mic_error = Some(format!("Não deu para tocar o retorno: {e}"));
                            None
                        }
                    }
                } else {
                    None
                };
                self.mic_test = Some(MicTest { cap, rx, levels: Vec::new(), eco });
            }
            Err(e) => self.mic_error = Some(e.to_string()),
        }
    }

    fn restart_mic_test(&mut self) {
        if let Some(t) = &self.mic_test {
            let eco = t.eco.is_some();
            self.start_mic_test(eco);
        }
    }

    fn lock_now(&mut self) {
        if self.pin.is_some() && self.lock.is_none() {
            self.lock = Some(LockState { msg: "Digite seu PIN".into(), ..Default::default() });
            self.modal = None;
            self.close_overlays();
            // Bloqueado, nada está à vista: o que chegar não vira "lido".
            let _ = self.engine.set_view(self.open_key().as_deref(), false);
        }
    }

    fn unlock(&mut self) {
        self.lock = None;
        let _ = self.engine.set_view(self.open_key().as_deref(), self.focused);
    }

    /* ------------------------------------------------------------------ */

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Frame(quando) => {
                self.motion.now = quando.max(self.motion.now);
                return Task::none();
            }
            Msg::RailHover(k) => {
                self.motion.hover(k);
                return Task::none();
            }
            _ => {}
        }
        let antes = self.snapshot();
        let r = self.update_inner(msg);
        let depois = self.snapshot();
        if antes != depois {
            self.motion.react(&antes, &depois);
        }
        r
    }

    fn update_inner(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Frame(_) | Msg::RailHover(_) => Task::none(),
            Msg::SetAnimations(on) => {
                self.motion.enabled = on;
                if let Err(e) = self.engine.set_ui_pref("animacoes", &on) {
                    tracing::warn!("opção de animações não salva: {e}");
                }
                Task::none()
            }
            Msg::Engine(ev) => self.on_engine(*ev),
            Msg::Tick => self.on_tick(),
            Msg::Noop => Task::none(),
            Msg::Toast(t) => {
                if !t.is_empty() {
                    self.toast(t);
                }
                Task::none()
            }
            Msg::Done(r) => {
                if let Err(e) = r {
                    if !e.is_empty() {
                        self.toast(e);
                    }
                }
                Task::none()
            }
            Msg::DoneToast(r) => {
                match r {
                    Ok(t) | Err(t) => {
                        if !t.is_empty() {
                            self.toast(t)
                        }
                    }
                }
                Task::none()
            }
            Msg::CloseOverlays => {
                self.close_overlays();
                Task::none()
            }
            Msg::TestHook => {
                // Só no modo de teste: enche a grade de GIFs com um GIF local
                // para conferir o desenho sem chave de API.
                if cfg!(debug_assertions) && std::env::var("J_MENSEGER_TESTE_ALVO").as_deref() == Ok("gifgrade") {
                    if let Some(b) = std::env::var("J_MENSEGER_TESTE_ARQUIVO").ok().and_then(|p| std::fs::read(p).ok()) {
                        if let Some(a) = Anim::decode(&b, 240) {
                            self.gif.hits = (0..8)
                                .map(|i| crate::gifs::Hit {
                                    id: format!("t{i}"),
                                    title: format!("Teste {i}"),
                                    preview: format!("teste:{i}"),
                                    send: String::new(),
                                    w: 240,
                                    h: if i % 3 == 0 { 240 } else { 160 },
                                })
                                .collect();
                            for i in 0..8 {
                                self.anims.insert(format!("teste:{i}"), Arc::clone(&a));
                            }
                            self.gif.error = None;
                            self.gif.from = Some(crate::gifs::Provider::Giphy);
                        }
                    }
                    return Task::none();
                }
                test_hook()
            }
            Msg::Focus(f) => {
                self.focused = f;
                if !f {
                    self.blurred_at = Some(Instant::now());
                    self.hover = None;
                } else if let (Some(p), Some(b)) = (&self.pin, self.blurred_at.take()) {
                    if b.elapsed() >= Duration::from_secs(p.delay as u64) {
                        self.lock_now();
                    }
                }
                if self.lock.is_none() {
                    let _ = self.engine.set_view(self.open_key().as_deref(), f);
                }
                Task::none()
            }
            Msg::WindowSize(sz) => {
                self.size = sz;
                Task::none()
            }
            Msg::WindowId(id) => {
                self.window = id;
                if self.settings.lock_screenshots {
                    return self.apply_protection();
                }
                Task::none()
            }
            Msg::FileHover(h) => {
                self.file_hover = h && self.lock.is_none();
                Task::none()
            }
            Msg::FileDropped(p) => {
                self.file_hover = false;
                if self.lock.is_some() {
                    return Task::none();
                }
                if platform::is_image(&p) {
                    if self.chat.is_none() {
                        self.toast("Abra uma conversa para mandar a foto.");
                        return Task::none();
                    }
                    return Task::perform(async move { platform::photo_from_path(p).await.map(Some) }, Msg::PhotoPicked);
                }
                if platform::is_audio(&p) {
                    return Task::perform(platform::audio_from_path(p), Msg::SbDecoded);
                }
                self.toast("Dá para soltar aqui fotos (para mandar) e áudios (para o soundboard).");
                Task::none()
            }
            Msg::Key(k) => self.on_key(k),

            /* ---------------------------------------------- navegação */
            Msg::Go(sec) => {
                self.close_overlays();
                self.show_requests = false;
                if let Section::Group(lid) = &sec {
                    let lid = lid.clone();
                    self.section = sec;
                    // Ao abrir um grupo, entra no primeiro canal de texto (ou
                    // no último aberto dele).
                    let ja = self.chat.as_ref().is_some_and(|c| c.key.starts_with(&format!("g:{lid}:")));
                    if ja {
                        return Task::none();
                    }
                    if let Some(ch) = self.group(&lid).and_then(|g| g.meta.text_channels().next()) {
                        return Task::done(Msg::OpenChat(format!("g:{lid}:{}", ch.id)));
                    }
                    return Task::none();
                }
                if sec == Section::Settings {
                    self.forms.prof_name = self.me.name.clone();
                    self.forms.prof_note = self.me.note.clone();
                    self.forms.prof_tone = self.me.tone;
                    self.storage = self.engine.storage_usage();
                }
                let fecha_grupo = self.chat.as_ref().is_some_and(|c| c.key.starts_with("g:"));
                self.section = sec;
                if fecha_grupo && self.section == Section::Chats {
                    return Task::done(Msg::CloseChat);
                }
                Task::none()
            }
            Msg::Page(p) => {
                if p == SettingsPage::Audio {
                    self.refresh_devices();
                } else {
                    self.mic_test = None;
                }
                self.page = p;
                self.section = Section::Settings;
                self.close_overlays();
                if p == SettingsPage::Storage {
                    self.storage = self.engine.storage_usage();
                }
                if p != SettingsPage::Recovery {
                    self.forms.keys = None;
                    self.forms.reveal_words = false;
                    self.forms.reveal_nsec = false;
                }
                Task::none()
            }
            Msg::ShowRequests => {
                self.show_requests = true;
                self.section = Section::Chats;
                Task::done(Msg::CloseChat)
            }
            Msg::OpenChat(key) => self.open_chat(key),
            Msg::CloseChat => {
                if let Some(c) = self.chat.take() {
                    let txt = c.editor.text();
                    if !txt.trim().is_empty() {
                        self.drafts.insert(c.key, txt);
                    }
                }
                self.close_overlays();
                let e = self.eng();
                done(async move { e.set_active(None).await.map_err(s) })
            }
            Msg::Search(q) => {
                self.search = q;
                Task::none()
            }
            Msg::ToggleMembers => {
                self.show_members = !self.show_members;
                Task::none()
            }

            /* ---------------------------------------------- dados */
            Msg::ChatsLoaded(c) => {
                self.chats = c;
                self.chats_loading = false;
                if std::mem::take(&mut self.chats_dirty) {
                    return self.reload_chats();
                }
                Task::none()
            }
            Msg::GroupsLoaded(g) => {
                self.groups = g;
                // O grupo aberto pode ter sumido (saiu ou foi removido).
                if let Section::Group(lid) = &self.section {
                    if !self.groups.iter().any(|g| &g.lid == lid) {
                        self.section = Section::Chats;
                        if self.chat.as_ref().is_some_and(|c| c.key.starts_with("g:")) {
                            self.chat = None;
                        }
                    }
                }
                Task::none()
            }
            Msg::MessagesLoaded { key, msgs, older } => {
                let Some(c) = self.chat.as_mut().filter(|c| c.key == key) else { return Task::none() };
                c.more = msgs.len() >= PAGE;
                if older {
                    let ja: std::collections::HashSet<String> = c.messages.iter().map(|m| m.id.clone()).collect();
                    let mut novas: Vec<_> = msgs.into_iter().filter(|m| !ja.contains(&m.id)).collect();
                    novas.append(&mut c.messages);
                    c.messages = novas;
                    Task::none()
                } else {
                    c.messages = msgs;
                    to_bottom()
                }
            }
            Msg::LoadOlder => {
                let Some(c) = &self.chat else { return Task::none() };
                let (key, antes) = (c.key.clone(), c.messages.first().map(|m| m.ts).unwrap_or(i64::MAX));
                let e = self.eng();
                Task::perform(async move { (key.clone(), e.messages(&key, antes, PAGE).unwrap_or_default()) }, |(key, msgs)| {
                    Msg::MessagesLoaded { key, msgs, older: true }
                })
            }

            /* ---------------------------------------------- conversa */
            Msg::Edit(action) => {
                let e = self.eng();
                let Some(c) = self.chat.as_mut() else { return Task::none() };
                let mudou = action.is_edit();
                c.editor.perform(action);
                if mudou && c.key.starts_with("dm:") {
                    let k = c.key.clone();
                    return Task::perform(async move { e.send_typing(&k).await }, |_| Msg::Noop);
                }
                Task::none()
            }
            Msg::Send => self.send_text(),
            Msg::InsertEmoji(em) => {
                if let Some(c) = self.chat.as_mut() {
                    c.editor.perform(text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(em))));
                }
                iced::widget::operation::focus(EDITOR_ID)
            }
            Msg::SetPopup(p) => {
                self.menu = None;
                self.popup = if self.popup == p { None } else { p };
                if self.popup == Some(Popup::Gif) {
                    let busca = if self.gif.hits.is_empty() && !self.gif.loading { self.gif_search() } else { Task::none() };
                    return Task::batch([busca, iced::widget::operation::focus(GIF_INPUT_ID)]);
                }
                Task::none()
            }
            Msg::Reply(id) => {
                self.close_overlays();
                self.modal = None;
                if let Some(c) = self.chat.as_mut() {
                    c.reply = Some(id);
                }
                iced::widget::operation::focus(EDITOR_ID)
            }
            Msg::CancelReply => {
                if let Some(c) = self.chat.as_mut() {
                    c.reply = None;
                }
                Task::none()
            }
            Msg::PickPhoto => {
                self.close_overlays();
                Task::perform(platform::pick_photo(), Msg::PhotoPicked)
            }
            Msg::PhotoPicked(r) => {
                match r {
                    Ok(Some(d)) if self.chat.is_some() => {
                        self.anims.remove(PREVIA_GIF);
                        let gif = is_gif_mime(&d.mime).then(|| Arc::clone(&d.bytes));
                        self.modal = Some(Modal::PhotoPreview(d));
                        if let Some(b) = gif {
                            return Task::perform(
                                async move { tokio::task::spawn_blocking(move || Anim::decode(&b, 640)).await.ok().flatten() },
                                |a| Msg::AnimReady(PREVIA_GIF.to_string(), a),
                            );
                        }
                    }
                    Ok(_) => {}
                    Err(e) => self.toast(e),
                }
                Task::none()
            }
            Msg::SendPhoto => {
                let Some(Modal::PhotoPreview(d)) = self.modal.take() else { return Task::none() };
                self.send_draft(d)
            }
            Msg::GifProviderSet(p) => {
                self.gif_keys.provider = p;
                self.gif_changed();
                Task::none()
            }
            Msg::NowPlaying(t) => {
                self.np_polling = false;
                self.now_playing = match (self.now_playing.take(), t) {
                    (Some((antes, desde)), Some(n)) if antes.title == n.title && antes.artist == n.artist => Some((n, desde)),
                    (_, Some(n)) => Some((n, jm_core::engine::now_ms())),
                    (_, None) => None,
                };
                self.share_listening()
            }
            Msg::ListenShare(on) => {
                self.listen_share = on;
                if let Err(e) = self.engine.set_ui_pref("spotify", &on) {
                    tracing::warn!("opção do Spotify não salva: {e}");
                }
                if on {
                    self.np_at = Instant::now() - Duration::from_secs(60);
                }
                self.share_listening()
            }
            Msg::GifQuery(q) => {
                self.gif.query = q;
                self.gif.seq += 1;
                let seq = self.gif.seq;
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(350)).await;
                        seq
                    },
                    Msg::GifSearchNow,
                )
            }
            Msg::GifSearchNow(seq) => {
                if seq != self.gif.seq {
                    return Task::none();
                }
                self.gif_search()
            }
            Msg::GifResults(seq, de, r) => {
                if seq != self.gif.seq {
                    return Task::none();
                }
                self.gif.loading = false;
                match r {
                    Ok(hits) => {
                        self.gif.error = None;
                        self.gif.from = Some(de);
                        let urls: Vec<String> = hits.iter().map(|h| h.preview.clone()).collect();
                        self.gif.hits = hits;
                        Task::batch(urls.into_iter().map(|u| self.anim_from_url(u)))
                    }
                    Err(e) => {
                        self.gif.error = Some(e);
                        Task::none()
                    }
                }
            }
            Msg::GifPick(i) => {
                let Some(h) = self.gif.hits.get(i).cloned() else { return Task::none() };
                if self.gif.sending.is_some() || self.chat.is_none() {
                    return Task::none();
                }
                self.gif.sending = Some(h.id.clone());
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let b = crate::gifs::download(&h.send, jm_media::gif::GIF_MAX)?;
                            platform::gif_draft(b)
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    Msg::GifDownloaded,
                )
            }
            Msg::GifDownloaded(r) => {
                self.gif.sending = None;
                match r {
                    Ok(d) => {
                        self.popup = None;
                        self.send_draft(d)
                    }
                    Err(e) => {
                        self.toast(e);
                        Task::none()
                    }
                }
            }
            Msg::GifFromFile => {
                self.close_overlays();
                Task::perform(platform::pick_gif(), Msg::PhotoPicked)
            }
            Msg::AnimReady(k, a) => {
                self.anim_loading.remove(&k);
                if let Some(a) = a {
                    // Guarda poucos: cada GIF aberto ocupa memória.
                    if self.anims.len() > 160 {
                        self.anims.clear();
                    }
                    self.anims.insert(k, a);
                }
                Task::none()
            }
            Msg::SendSound(id) => {
                self.popup = None;
                let Some(key) = self.open_key() else { return Task::none() };
                let Some(info) = self.sounds.info(&id) else { return Task::none() };
                self.sounds.play(&id);
                let e = self.eng();
                if info.builtin {
                    return done(async move { e.send_sound(&key, &id).await.map(|_| ()).map_err(s) });
                }
                let Some(bytes) = self.sounds.bytes(&id) else { return Task::none() };
                let mime = if bytes.starts_with(b"OggS") { "audio/ogg" } else { "audio/webm" };
                let meta = MediaMeta {
                    kind: Some(MediaKind::Audio),
                    mime: mime.into(),
                    dur: Some(info.dur as f64),
                    sb: Some(SoundMeta { n: info.name.clone(), e: info.emoji.clone() }),
                    ..Default::default()
                };
                done(async move { e.send_media(&key, bytes, meta).await.map(|_| ()).map_err(s) })
            }
            Msg::RecStart => {
                self.close_overlays();
                let Some(key) = self.open_key() else { return Task::none() };
                if self.call.is_some() || self.room.is_some() {
                    self.toast("O microfone está na chamada agora.");
                    return Task::none();
                }
                match jm_media::recorder::Recorder::start() {
                    Ok(rec) => {
                        if let Some(p) = self.sounds.player() {
                            p.stop_track();
                        }
                        self.rec = Some(VoiceRec { rec, chat: key, levels: Vec::new() });
                    }
                    Err(e) => self.toast(format!("Não deu para usar o microfone: {e}")),
                }
                Task::none()
            }
            Msg::RecStop { send } => {
                let Some(v) = self.rec.take() else { return Task::none() };
                let r = v.rec.stop();
                if !send {
                    return Task::none();
                }
                if r.dur < 0.8 || r.pcm.is_empty() {
                    self.toast("Áudio curto demais. Segure um pouco mais.");
                    return Task::none();
                }
                let e = self.eng();
                done(async move {
                    let wf = jm_media::audiofile::waveform(&r.levels, 48);
                    let dur = r.dur;
                    let bytes = tokio::task::spawn_blocking(move || jm_media::audiofile::encode_ogg_opus(&r.pcm, 24_000, false))
                        .await
                        .map_err(s)?
                        .map_err(|_| "Não deu para gravar o áudio.".to_string())?;
                    let meta = MediaMeta { kind: Some(MediaKind::Audio), mime: "audio/ogg".into(), dur: Some(dur), wf: Some(wf), ..Default::default() };
                    e.send_media(&v.chat, bytes, meta).await.map(|_| ()).map_err(s)
                })
            }
            Msg::Hover(h) => {
                self.hover = h;
                Task::none()
            }
            Msg::Unhover(id) => {
                // Só limpa se ainda for a mesma: ao passar de uma mensagem para
                // a de cima, a entrada na nova pode chegar antes da saída.
                if self.hover.as_deref() == Some(id.as_str()) {
                    self.hover = None;
                }
                Task::none()
            }
            Msg::MsgMenu(id) => open_menu(CtxKind::Message(id)),
            Msg::Menu(kind) => open_menu(kind),
            Msg::MenuNow(kind) => {
                self.popup = None;
                self.menu = Some(CtxMenu { at: cursor(), kind });
                Task::none()
            }
            Msg::React(id, em) => {
                self.close_overlays();
                self.modal = None;
                let Some(key) = self.open_key() else { return Task::none() };
                let e = self.eng();
                done(async move { e.send_reaction(&key, &id, &em).await.map_err(s) })
            }
            Msg::ReactMore(id) => {
                self.close_overlays();
                self.modal = Some(Modal::ReactPicker { id });
                Task::none()
            }
            Msg::Copy(txt) => {
                self.close_overlays();
                self.toast("Copiado");
                iced::clipboard::write(txt)
            }
            Msg::AskDelete(id) => {
                self.close_overlays();
                let mine = self
                    .chat
                    .as_ref()
                    .and_then(|c| c.messages.iter().find(|m| m.id == id))
                    .is_some_and(|m| (m.from_me || self.can_moderate(&m.chat)) && !matches!(m.body, MessageBody::Deleted | MessageBody::System { .. }));
                self.modal = Some(Modal::DeleteMsg { id, mine });
                Task::none()
            }
            Msg::DeleteForMe(id) => {
                self.modal = None;
                let Some(key) = self.open_key() else { return Task::none() };
                if let Err(e) = self.engine.delete_for_me(&key, &[id]) {
                    self.toast(e.to_string());
                }
                Task::none()
            }
            Msg::DeleteForAll(id) => {
                self.modal = None;
                let Some(key) = self.open_key() else { return Task::none() };
                let e = self.eng();
                done(async move { e.delete_for_all(&key, &[id]).await.map_err(s) })
            }
            Msg::PlayVoice(mid) => {
                let Some(p) = self.sounds.player() else {
                    self.toast("Nenhuma saída de som encontrada.");
                    return Task::none();
                };
                if let Some((id, pos, dur, _)) = p.track() {
                    if id == mid && pos + 0.05 < dur {
                        p.toggle_pause();
                        self.track = p.track();
                        return Task::none();
                    }
                }
                let Some(bytes) = self.engine.media_bytes(&mid) else { return Task::none() };
                Task::perform(
                    async move {
                        let pcm = tokio::task::spawn_blocking(move || jm_media::audiofile::decode(&bytes).map(|p| p.samples)).await;
                        (mid, pcm)
                    },
                    |(mid, r)| match r {
                        Ok(Ok(pcm)) => Msg::VoiceReady(mid, Arc::new(pcm)),
                        _ => Msg::Toast("Não deu para tocar esse áudio.".into()),
                    },
                )
            }
            Msg::VoiceReady(mid, pcm) => {
                if let Some(p) = self.sounds.player() {
                    p.play_track(&mid, pcm);
                    self.track = p.track();
                }
                Task::none()
            }
            Msg::SeekVoice(mid, frac) => {
                if let Some(p) = self.sounds.player() {
                    if p.track().is_some_and(|t| t.0 == mid) {
                        p.seek(frac);
                        self.track = p.track();
                    }
                }
                Task::none()
            }
            Msg::PlaySound(id) => {
                if self.sounds.play(&id) == 0.0 && self.sounds.player().is_none() {
                    self.toast("Nenhuma saída de som encontrada.");
                }
                Task::none()
            }
            Msg::PlayMediaSound(mid) => {
                if let Some(b) = self.engine.media_bytes(&mid) {
                    self.sounds.play_bytes(&format!("m:{mid}"), &b);
                }
                Task::none()
            }
            Msg::OpenViewer(mid) => {
                self.close_overlays();
                self.modal = Some(Modal::Viewer { mid });
                Task::none()
            }
            Msg::SavePhoto(mid) => {
                let Some(bytes) = self.engine.media_bytes(&mid) else { return Task::none() };
                let mime = self
                    .chat
                    .as_ref()
                    .and_then(|c| {
                        c.messages.iter().find_map(|m| match &m.body {
                            MessageBody::Media(x) if x.mid == mid => Some(x.mime.clone()),
                            _ => None,
                        })
                    })
                    .unwrap_or_else(|| "image/jpeg".into());
                toast_done(platform::save_photo(bytes, mime))
            }
            Msg::JoinInvite(id) => {
                let e = self.eng();
                Task::perform(async move { e.accept_invite(&id).await.map(|g| g.lid).map_err(s) }, Msg::GroupCreated)
            }
            Msg::OpenUrl(u) => {
                platform::open_url(&u);
                Task::none()
            }

            /* ---------------------------------------------- lista */
            Msg::RowMenu(key) => open_menu(CtxKind::Chat(key)),
            Msg::PinChat(key, on) => {
                self.menu = None;
                if let Err(e) = self.engine.set_chat_pinned(&key, on) {
                    self.toast(e.to_string());
                }
                self.reload_chats()
            }
            Msg::MuteChat(key, on) => {
                self.menu = None;
                if let Err(e) = self.engine.set_chat_muted(&key, on) {
                    self.toast(e.to_string());
                } else {
                    self.toast(if on { "Silenciada" } else { "Notificações ativadas" });
                }
                self.reload_chats()
            }
            Msg::MarkRead(key) => {
                self.menu = None;
                if let Err(e) = self.engine.mark_read(&key) {
                    self.toast(e.to_string());
                }
                self.reload_chats()
            }

            /* ---------------------------------------------- contatos */
            Msg::OpenAddContact(code) => {
                self.close_overlays();
                self.modal = Some(Modal::AddContact { code, nick: String::new(), error: None, busy: false });
                Task::none()
            }
            Msg::OpenMyCode => {
                self.close_overlays();
                self.modal = Some(Modal::MyCode);
                Task::none()
            }
            Msg::OpenNewChat => {
                self.close_overlays();
                self.modal = Some(Modal::NewChat);
                Task::none()
            }
            Msg::OpenProfile(pk) => {
                self.close_overlays();
                if pk == self.me_pk() {
                    return Task::done(Msg::Page(SettingsPage::Profile)).chain(Task::done(Msg::Go(Section::Settings)));
                }
                let nick = self.contacts.get(&pk).map(|c| c.nick.clone()).unwrap_or_default();
                self.modal = Some(Modal::Profile { pk, nick });
                Task::none()
            }
            Msg::AddCode(v) => {
                if let Some(Modal::AddContact { code, error, .. }) = &mut self.modal {
                    *code = v;
                    *error = None;
                }
                Task::none()
            }
            Msg::AddNick(v) => {
                if let Some(Modal::AddContact { nick, .. }) = &mut self.modal {
                    *nick = v.chars().take(40).collect();
                }
                Task::none()
            }
            Msg::AddPaste => iced::clipboard::read().map(Msg::AddPasted),
            Msg::AddPasted(v) => {
                if let (Some(v), Some(Modal::AddContact { code, error, .. })) = (v, &mut self.modal) {
                    *code = v.trim().to_string();
                    *error = None;
                }
                Task::none()
            }
            Msg::AddConfirm => {
                let (e, me) = (self.eng(), self.engine.pk_hex());
                let Some(Modal::AddContact { code, nick, error, busy }) = &mut self.modal else { return Task::none() };
                if *busy {
                    return Task::none();
                }
                let c = code.trim().to_string();
                match jm_core::identity::parse_pubkey(&c) {
                    None if jm_core::shortcode::parse(&c).is_some() => {
                        // Código curto: descobre a chave nos servidores e adiciona.
                        *busy = true;
                        let n = nick.clone();
                        return Task::perform(
                            async move {
                                let pk = e.resolve_short_code(&c).await.map_err(s)?;
                                if pk == me {
                                    return Err("Esse é o seu próprio código.".to_string());
                                }
                                e.add_contact_with(&pk, &n).await.map(|c| c.pk).map_err(s)
                            },
                            Msg::ContactAdded,
                        );
                    }
                    None => {
                        *error = Some("Código inválido. Use o código curto (ex.: K7QF-9X2M) ou o completo, que começa com npub1.".into());
                        return Task::none();
                    }
                    Some(pk) if pk.to_hex() == me => {
                        *error = Some("Esse é o seu próprio código.".into());
                        return Task::none();
                    }
                    _ => {}
                }
                *busy = true;
                let n = nick.clone();
                Task::perform(async move { e.add_contact_with(&c, &n).await.map(|c| c.pk).map_err(s) }, Msg::ContactAdded)
            }
            Msg::ContactAdded(r) => match r {
                Ok(pk) => {
                    self.modal = None;
                    self.reload_contacts();
                    self.toast("Contato adicionado");
                    self.section = Section::Chats;
                    Task::done(Msg::OpenChat(format!("dm:{pk}")))
                }
                Err(e) => {
                    if let Some(Modal::AddContact { error, busy, .. }) = &mut self.modal {
                        *error = Some(e);
                        *busy = false;
                    } else {
                        self.toast(e);
                    }
                    Task::none()
                }
            },
            Msg::ProfileNick(v) => {
                if let Some(Modal::Profile { nick, .. }) = &mut self.modal {
                    *nick = v.chars().take(40).collect();
                }
                Task::none()
            }
            Msg::ProfileSaveNick => {
                let Some(Modal::Profile { pk, nick }) = &self.modal else { return Task::none() };
                let (pk, nick) = (pk.clone(), nick.clone());
                match self.engine.update_contact(&pk, Some(nick.trim()), None) {
                    Ok(()) => {
                        self.reload_contacts();
                        self.toast(if nick.trim().is_empty() { "Apelido removido" } else { "Apelido salvo" });
                    }
                    Err(e) => self.toast(e.to_string()),
                }
                self.reload_chats()
            }
            Msg::ProfileVerify(on) => {
                let Some(Modal::Profile { pk, .. }) = &self.modal else { return Task::none() };
                let pk = pk.clone();
                if let Err(e) = self.engine.update_contact(&pk, None, Some(on)) {
                    self.toast(e.to_string());
                }
                self.reload_contacts();
                self.toast(if on { "Marcado como verificado" } else { "Verificação removida" });
                Task::none()
            }
            Msg::Accept(pk) => {
                let e = self.eng();
                Task::perform(
                    async move { e.accept_contact(&pk).await.map(|_| "Contato aceito".to_string()).map_err(s) },
                    Msg::DoneToast,
                )
                .chain(Task::done(Msg::Noop))
            }
            Msg::Block(pk) => {
                self.modal = None;
                if let Err(e) = self.engine.set_contact_status(&pk, ContactStatus::Blocked) {
                    self.toast(e.to_string());
                } else {
                    self.toast("Contato bloqueado");
                }
                self.reload_contacts();
                if self.chat.as_ref().is_some_and(|c| c.key == format!("dm:{pk}")) {
                    return Task::batch([Task::done(Msg::CloseChat), self.reload_chats()]);
                }
                self.reload_chats()
            }
            Msg::Unblock(pk) => {
                if let Err(e) = self.engine.set_contact_status(&pk, ContactStatus::Ok) {
                    self.toast(e.to_string());
                } else {
                    self.toast("Contato desbloqueado");
                }
                self.reload_contacts();
                self.reload_chats()
            }
            Msg::CopyMyCode => {
                self.toast("Código copiado");
                iced::clipboard::write(self.engine.npub())
            }
            Msg::ShortCodeCreate => {
                if self.short_code_busy {
                    return Task::none();
                }
                self.short_code_busy = true;
                let e = self.eng();
                Task::perform(async move { e.create_short_code().await.map_err(s) }, Msg::ShortCodeReady)
            }
            Msg::ShortCodeReady(r) => {
                self.short_code_busy = false;
                match r {
                    Ok(c) => self.short_code = Some(c),
                    Err(e) => self.toast(format!("Não deu para publicar o código: {e}")),
                }
                Task::none()
            }
            Msg::CopyShortCode => {
                let Some((c, _)) = self.short_code.clone() else { return Task::none() };
                self.toast("Código curto copiado");
                iced::clipboard::write(c)
            }
            Msg::CopyInvite => {
                self.toast("Convite copiado. Cole onde quiser mandar.");
                iced::clipboard::write(invite_text(&self.engine.npub()))
            }

            /* ---------------------------------------------- grupos */
            Msg::OpenNewGroup => {
                self.close_overlays();
                self.modal = Some(Modal::NewGroup { name: String::new(), template: jm_core::groups::Template::Amigos, busy: false });
                Task::none()
            }
            Msg::NewGroupName(v) => {
                if let Some(Modal::NewGroup { name, .. }) = &mut self.modal {
                    *name = v.chars().take(40).collect();
                }
                Task::none()
            }
            Msg::NewGroupTemplate(tpl) => {
                if let Some(Modal::NewGroup { template, .. }) = &mut self.modal {
                    *template = tpl;
                }
                Task::none()
            }
            Msg::NewGroupCreate => {
                let e = self.eng();
                let Some(Modal::NewGroup { name, template, busy }) = &mut self.modal else { return Task::none() };
                let n = name.trim().to_string();
                if n.chars().count() < 2 || *busy {
                    return Task::none();
                }
                *busy = true;
                let tpl = *template;
                Task::perform(async move { e.create_group(&n, tpl).await.map(|g| g.lid).map_err(s) }, Msg::GroupCreated)
            }
            Msg::GroupCreated(r) => match r {
                Ok(lid) => {
                    if matches!(self.modal, Some(Modal::NewGroup { .. })) {
                        self.modal = None;
                    }
                    self.groups = self.engine.groups();
                    Task::done(Msg::Go(Section::Group(lid)))
                }
                Err(e) => {
                    if let Some(Modal::NewGroup { busy, .. }) = &mut self.modal {
                        *busy = false;
                    }
                    self.toast(e);
                    Task::none()
                }
            },
            Msg::GroupMenu(lid) => open_menu(CtxKind::Group(lid)),
            Msg::OpenGroupInfo(lid) => {
                self.close_overlays();
                if let Some(g) = self.group(&lid) {
                    self.modal = Some(Modal::GroupInfo(GroupForm {
                        lid,
                        name: g.meta.name.clone(),
                        desc: g.meta.desc.clone(),
                        ch_name: String::new(),
                        ch_kind: ChannelKind::Text,
                    }));
                }
                Task::none()
            }
            Msg::GroupName(v) => {
                if let Some(Modal::GroupInfo(f)) = &mut self.modal {
                    f.name = v.chars().take(40).collect();
                }
                Task::none()
            }
            Msg::GroupDesc(v) => {
                if let Some(Modal::GroupInfo(f)) = &mut self.modal {
                    f.desc = v.chars().take(140).collect();
                }
                Task::none()
            }
            Msg::GroupSave => {
                let Some(Modal::GroupInfo(f)) = &self.modal else { return Task::none() };
                let Some(g) = self.group(&f.lid) else { return Task::none() };
                let mut patches = Vec::new();
                if f.name.trim() != g.meta.name && f.name.trim().chars().count() >= 2 {
                    patches.push(GroupPatch::Name(f.name.trim().into()));
                }
                if f.desc.trim() != g.meta.desc {
                    patches.push(GroupPatch::Desc(f.desc.trim().into()));
                }
                if patches.is_empty() {
                    return Task::none();
                }
                let (e, lid) = (self.eng(), f.lid.clone());
                toast_done(async move { e.edit_group(&lid, &patches).await.map(|_| "Grupo atualizado".to_string()).map_err(s) })
            }
            Msg::GroupPhoto(lid) => {
                self.close_overlays();
                Task::perform(platform::pick_for_crop(CropTarget::Group(lid)), Msg::PhotoForCrop)
            }
            Msg::GroupPhotoRemove(lid) => {
                let e = self.eng();
                toast_done(async move { e.set_group_pic(&lid, "").await.map(|_| "Foto do grupo removida".to_string()).map_err(s) })
            }
            Msg::MuteGroup(lid, on) => {
                self.menu = None;
                if let Err(e) = self.engine.set_group_muted(&lid, on) {
                    self.toast(e.to_string());
                } else {
                    self.toast(if on { "Grupo silenciado" } else { "Notificações do grupo ativadas" });
                }
                self.reload_groups()
            }
            Msg::ChannelDraftName(v) => {
                if let Some(Modal::GroupInfo(f)) = &mut self.modal {
                    f.ch_name = v.chars().take(24).collect();
                }
                Task::none()
            }
            Msg::ChannelDraftKind(k) => {
                if let Some(Modal::GroupInfo(f)) = &mut self.modal {
                    f.ch_kind = k;
                }
                Task::none()
            }
            Msg::ChannelAdd => {
                let e = self.eng();
                let Some(Modal::GroupInfo(f)) = &mut self.modal else { return Task::none() };
                let nome = f.ch_name.trim().to_string();
                if nome.is_empty() {
                    return Task::none();
                }
                f.ch_name.clear();
                let (lid, kind) = (f.lid.clone(), f.ch_kind);
                toast_done(async move {
                    e.edit_group(&lid, &[GroupPatch::AddChannel { name: nome, kind }]).await.map(|_| "Canal criado".to_string()).map_err(s)
                })
            }
            Msg::ChannelEdit(id) => {
                let lid = match &self.modal {
                    Some(Modal::GroupInfo(f)) => f.lid.clone(),
                    _ => match &self.section {
                        Section::Group(l) => l.clone(),
                        _ => return Task::none(),
                    },
                };
                let Some((nome, assunto)) = self
                    .group(&lid)
                    .and_then(|g| g.meta.channels.iter().find(|c| c.id == id))
                    .map(|ch| (ch.name.clone(), ch.topic.clone().unwrap_or_default()))
                else {
                    return Task::none();
                };
                self.close_overlays();
                self.modal = Some(Modal::EditChannel { lid, id, name: nome, topic: assunto });
                Task::none()
            }
            Msg::ChannelEditName(v) => {
                if let Some(Modal::EditChannel { name, .. }) = &mut self.modal {
                    *name = v.chars().take(24).collect();
                }
                Task::none()
            }
            Msg::ChannelEditTopic(v) => {
                if let Some(Modal::EditChannel { topic, .. }) = &mut self.modal {
                    *topic = v.chars().take(120).collect();
                }
                Task::none()
            }
            Msg::ChannelEditSave => {
                let Some(Modal::EditChannel { lid, id, name, topic }) = self.modal.take() else { return Task::none() };
                let e = self.eng();
                let l2 = lid.clone();
                let task = toast_done(async move {
                    e.edit_group(&l2, &[GroupPatch::EditChannel { id, name: Some(name.trim().into()), topic: Some(topic.trim().into()) }])
                        .await
                        .map(|_| "Canal atualizado".to_string())
                        .map_err(s)
                });
                Task::batch([task, Task::done(Msg::OpenGroupInfo(lid))])
            }
            Msg::ChannelMove(id, dir) => {
                let lid = match &self.modal {
                    Some(Modal::GroupInfo(f)) => f.lid.clone(),
                    _ => return Task::none(),
                };
                let e = self.eng();
                done(async move { e.edit_group(&lid, &[GroupPatch::MoveChannel { id, dir }]).await.map_err(s) })
            }
            Msg::EditStart(id) => {
                self.close_overlays();
                self.modal = None;
                let Some(c) = self.chat.as_mut() else { return Task::none() };
                let Some(MessageBody::Text(t)) = c.messages.iter().find(|m| m.id == id && m.from_me).map(|m| m.body.clone()) else {
                    return Task::none();
                };
                c.reply = None;
                c.edit = Some(id);
                c.editor = text_editor::Content::with_text(&t);
                c.editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
                iced::widget::operation::focus(EDITOR_ID)
            }
            Msg::EditCancel => {
                if let Some(c) = self.chat.as_mut() {
                    if c.edit.take().is_some() {
                        c.editor = text_editor::Content::new();
                    }
                }
                Task::none()
            }
            Msg::PinMsg(id, on) => {
                self.close_overlays();
                if matches!(self.modal, Some(Modal::Pins { .. })) {
                    self.modal = None;
                }
                let Some(key) = self.chat.as_ref().map(|c| c.key.clone()) else { return Task::none() };
                let e = self.eng();
                toast_done(async move {
                    e.pin_message(&key, &id, on).await.map(|_| if on { "Mensagem fixada".to_string() } else { "Mensagem desafixada".to_string() }).map_err(s)
                })
            }
            Msg::OpenPins => {
                self.close_overlays();
                let Some(c) = &self.chat else { return Task::none() };
                let list: Vec<jm_core::engine::Message> = c
                    .pins
                    .iter()
                    .filter_map(|id| self.engine.message(id))
                    .filter(|m| m.chat == c.key && !matches!(m.body, MessageBody::Deleted))
                    .collect();
                Task::done(Msg::PinsLoaded(list))
            }
            Msg::PinsLoaded(list) => {
                self.modal = Some(Modal::Pins { list });
                Task::none()
            }
            Msg::JumpTo(id) => {
                self.modal = None;
                self.close_overlays();
                let Some(c) = &self.chat else { return Task::none() };
                if c.messages.iter().any(|m| m.id == id) {
                    self.flash = Some((id.clone(), Instant::now()));
                    return self.scroll_to_msg(&id);
                }
                let e = self.eng();
                let key = c.key.clone();
                Task::perform(async move { (key.clone(), e.messages(&key, i64::MAX, 3000).unwrap_or_default()) }, move |(k, msgs)| {
                    Msg::JumpLoaded(k, id.clone(), msgs)
                })
            }
            Msg::JumpLoaded(key, id, msgs) => {
                let Some(c) = self.chat.as_mut().filter(|c| c.key == key) else { return Task::none() };
                if !msgs.iter().any(|m| m.id == id) {
                    return self.toast_t("Essa mensagem não está mais neste computador");
                }
                c.more = msgs.len() >= 3000;
                c.messages = msgs;
                self.flash = Some((id.clone(), Instant::now()));
                self.scroll_to_msg(&id)
            }
            Msg::OpenSearch => {
                self.close_overlays();
                let Some(c) = &self.chat else { return Task::none() };
                let e = self.eng();
                let key = c.key.clone();
                Task::perform(async move { e.messages(&key, i64::MAX, 5000).unwrap_or_default() }, Msg::SearchLoaded)
            }
            Msg::SearchLoaded(all) => {
                let all = all.into_iter().filter(|m| matches!(m.body, MessageBody::Text(_) | MessageBody::Poll { .. })).collect();
                self.modal = Some(Modal::Search { all, query: String::new() });
                iced::widget::operation::focus(crate::v_modal::SEARCH_ID)
            }
            Msg::SearchInput(v) => {
                if let Some(Modal::Search { query, .. }) = &mut self.modal {
                    *query = v.chars().take(80).collect();
                }
                Task::none()
            }
            Msg::OpenPoll => {
                self.close_overlays();
                self.modal = Some(Modal::NewPoll { q: String::new(), opts: vec![String::new(), String::new()], multi: false });
                Task::none()
            }
            Msg::PollQ(v) => {
                if let Some(Modal::NewPoll { q, .. }) = &mut self.modal {
                    *q = v.chars().take(200).collect();
                }
                Task::none()
            }
            Msg::PollOpt(i, v) => {
                if let Some(Modal::NewPoll { opts, .. }) = &mut self.modal {
                    if let Some(o) = opts.get_mut(i) {
                        *o = v.chars().take(80).collect();
                    }
                }
                Task::none()
            }
            Msg::PollAddOpt => {
                if let Some(Modal::NewPoll { opts, .. }) = &mut self.modal {
                    if opts.len() < 10 {
                        opts.push(String::new());
                    }
                }
                Task::none()
            }
            Msg::PollDelOpt(i) => {
                if let Some(Modal::NewPoll { opts, .. }) = &mut self.modal {
                    if opts.len() > 2 && i < opts.len() {
                        opts.remove(i);
                    }
                }
                Task::none()
            }
            Msg::PollMulti(on) => {
                if let Some(Modal::NewPoll { multi, .. }) = &mut self.modal {
                    *multi = on;
                }
                Task::none()
            }
            Msg::PollSend => {
                let Some(Modal::NewPoll { q, opts, multi }) = &self.modal else { return Task::none() };
                let opcoes: Vec<String> = opts.iter().map(|o| o.trim().to_string()).filter(|o| !o.is_empty()).collect();
                if q.trim().is_empty() || opcoes.len() < 2 {
                    return self.toast_t("Escreva a pergunta e pelo menos duas opções.");
                }
                let (q, multi) = (q.trim().to_string(), *multi);
                self.modal = None;
                let Some(key) = self.chat.as_ref().map(|c| c.key.clone()) else { return Task::none() };
                let e = self.eng();
                Task::batch([done(async move { e.send_poll(&key, &q, &opcoes, multi).await.map(|_| ()).map_err(s) }), to_bottom()])
            }
            Msg::Vote(id, i) => {
                let Some(c) = &self.chat else { return Task::none() };
                let Some(MessageBody::Poll { multi, votes, .. }) = c.messages.iter().find(|m| m.id == id).map(|m| &m.body) else {
                    return Task::none();
                };
                let atual = votes.get(&self.me_pk()).cloned().unwrap_or_default();
                let novo: Vec<u8> = if *multi {
                    if atual.contains(&i) {
                        atual.into_iter().filter(|x| *x != i).collect()
                    } else {
                        let mut v = atual;
                        v.push(i);
                        v.sort();
                        v
                    }
                } else if atual.contains(&i) {
                    Vec::new()
                } else {
                    vec![i]
                };
                let key = c.key.clone();
                let e = self.eng();
                done(async move { e.vote(&key, &id, &novo).await.map_err(s) })
            }
            Msg::OpenVoters(id) => {
                self.close_overlays();
                self.modal = Some(Modal::Voters { id });
                Task::none()
            }
            Msg::OpenRoles(lid) => {
                self.close_overlays();
                self.modal = Some(Modal::Roles { lid, form: None });
                Task::none()
            }
            Msg::RoleForm(id) => {
                if let Some(Modal::Roles { lid, form }) = &mut self.modal {
                    let g = self.engine.group(lid);
                    let r = id.as_ref().and_then(|id| g.as_ref().and_then(|g| g.meta.roles.iter().find(|r| &r.id == id).cloned()));
                    let n = g.map(|g| g.meta.roles.len()).unwrap_or(0);
                    *form = Some(match r {
                        Some(r) => RoleForm { id: Some(r.id), name: r.n, c: r.c, p: r.p },
                        None => RoleForm { id: None, name: String::new(), c: (n % 8) as u8, p: Vec::new() },
                    });
                    if id.is_none() && n >= jm_core::groups::MAX_ROLES {
                        *form = None;
                        return self.toast_t("No máximo 12 cargos por grupo");
                    }
                }
                iced::widget::operation::focus(crate::v_modal::ROLE_NAME_ID)
            }
            Msg::RoleName(v) => {
                if let Some(Modal::Roles { form: Some(f), .. }) = &mut self.modal {
                    f.name = v.chars().take(24).collect();
                }
                Task::none()
            }
            Msg::RolePerm(perm, on) => {
                if let Some(Modal::Roles { form: Some(f), .. }) = &mut self.modal {
                    f.p.retain(|x| x != &perm);
                    if on {
                        f.p.push(perm);
                    }
                }
                Task::none()
            }
            Msg::OpenEvent => {
                self.close_overlays();
                let amanha = chrono::Local::now() + chrono::Duration::days(1);
                self.modal = Some(Modal::NewEvent { title: String::new(), date: amanha.format("%d/%m/%Y").to_string(), time: "20:00".into(), desc: String::new() });
                Task::none()
            }
            Msg::EventField(k, v) => {
                if let Some(Modal::NewEvent { title, date, time, desc }) = &mut self.modal {
                    match k {
                        0 => *title = v.chars().take(120).collect(),
                        1 => *date = v.chars().take(10).collect(),
                        2 => *time = v.chars().take(5).collect(),
                        _ => *desc = v.chars().take(500).collect(),
                    }
                }
                Task::none()
            }
            Msg::EventSend => {
                let Some(Modal::NewEvent { title, date, time, desc }) = &self.modal else { return Task::none() };
                let quando = chrono::NaiveDateTime::parse_from_str(&format!("{} {}", date.trim(), time.trim()), "%d/%m/%Y %H:%M")
                    .ok()
                    .and_then(|d| d.and_local_timezone(chrono::Local).single());
                let Some(quando) = quando else { return self.toast_t("Data ou hora inválida. Use 23/11/2026 e 20:00.") };
                if title.trim().is_empty() {
                    return self.toast_t("Dê um nome ao evento.");
                }
                let (t2, d2, at) = (title.trim().to_string(), desc.trim().to_string(), quando.timestamp_millis());
                self.modal = None;
                let Some(key) = self.open_key() else { return Task::none() };
                let e = self.eng();
                Task::batch([done(async move { e.send_event(&key, &t2, at, &d2).await.map(|_| ()).map_err(s) }), to_bottom()])
            }
            Msg::Rsvp(id, r) => {
                let Some(key) = self.open_key() else { return Task::none() };
                let e = self.eng();
                done(async move { e.rsvp(&key, &id, &r).await.map_err(s) })
            }
            Msg::RoleColor(c) => {
                if let Some(Modal::Roles { form: Some(f), .. }) = &mut self.modal {
                    f.c = c % 8;
                }
                Task::none()
            }
            Msg::RoleSave => {
                let Some(Modal::Roles { lid, form }) = &mut self.modal else { return Task::none() };
                let Some(f) = form.take() else { return Task::none() };
                if f.name.trim().is_empty() {
                    *form = Some(f);
                    return Task::none();
                }
                let lid = lid.clone();
                let patches = match f.id {
                    Some(id) => vec![GroupPatch::EditRole { id, n: Some(f.name.trim().into()), c: Some(f.c), p: Some(f.p) }],
                    None => {
                        // Cargo novo com poderes: cria e, em seguida, liga os poderes.
                        let antes: Vec<String> = self.engine.group(&lid).map(|g| g.meta.roles.iter().map(|r| r.id.clone()).collect()).unwrap_or_default();
                        let e = self.eng();
                        let (l2, n, c, p) = (lid.clone(), f.name.trim().to_string(), f.c, f.p);
                        return done(async move {
                            e.edit_group(&l2, &[GroupPatch::AddRole { n, c }]).await.map_err(s)?;
                            if !p.is_empty() {
                                if let Some(novo) = e.group(&l2).and_then(|g| g.meta.roles.iter().find(|r| !antes.contains(&r.id)).map(|r| r.id.clone())) {
                                    e.edit_group(&l2, &[GroupPatch::EditRole { id: novo, n: None, c: None, p: Some(p) }]).await.map_err(s)?;
                                }
                            }
                            Ok(())
                        });
                    }
                };
                let e = self.eng();
                done(async move { e.edit_group(&lid, &patches).await.map_err(s) })
            }
            Msg::RoleDelete(id) => {
                let Some(Modal::Roles { lid, form }) = &mut self.modal else { return Task::none() };
                *form = None;
                let lid = lid.clone();
                let e = self.eng();
                toast_done(async move { e.edit_group(&lid, &[GroupPatch::RemoveRole(id)]).await.map(|_| "Cargo apagado".to_string()).map_err(s) })
            }
            Msg::OpenMemberRoles(lid, pk) => {
                self.close_overlays();
                self.modal = Some(Modal::MemberRoles { lid, pk });
                Task::none()
            }
            Msg::RoleToggle(lid, pk, id, on) => {
                if on && self.group(&lid).is_some_and(|g| g.meta.roles_of(&pk).len() >= jm_core::groups::MAX_ROLES_POR_MEMBRO) {
                    return self.toast_t("No máximo 4 cargos por pessoa");
                }
                let e = self.eng();
                done(async move { e.edit_group(&lid, &[GroupPatch::MemberRole { pk, id, on }]).await.map_err(s) })
            }
            Msg::ShareAudio(on) => {
                self.share_audio = on;
                let _ = self.engine.set_ui_pref("share_audio", &on);
                Task::none()
            }
            Msg::OpenPeerVolume(pk) => {
                self.close_overlays();
                let v = self.calls.peer_volume(&pk);
                self.modal = Some(Modal::PeerVolume { pk, v });
                Task::none()
            }
            Msg::PeerVolume(nv) => {
                let Some(Modal::PeerVolume { pk, v }) = &mut self.modal else { return Task::none() };
                *v = nv;
                let (pk, c) = (pk.clone(), Arc::clone(&self.calls));
                Task::perform(async move { c.set_peer_volume(&pk, nv).await }, |_| Msg::Noop)
            }
            Msg::OpenSetStatus => {
                self.close_overlays();
                let s = self.me.status.clone().filter(|x| x.clean(jm_core::engine::now_ms()).is_some()).unwrap_or_default();
                self.modal = Some(Modal::SetStatus { e: s.e, t: s.t, dnd: s.dnd, dur: 0 });
                Task::none()
            }
            Msg::StatusField(k, v) => {
                if let Some(Modal::SetStatus { e, t, .. }) = &mut self.modal {
                    match k {
                        0 => *e = v.chars().take(8).collect(),
                        1 => *t = v.chars().take(80).collect(),
                        _ => {
                            // Sugestão pronta: "emoji|texto".
                            if let Some((a, b)) = v.split_once('|') {
                                *e = a.into();
                                *t = b.into();
                            }
                        }
                    }
                }
                Task::none()
            }
            Msg::StatusDnd(on) => {
                if let Some(Modal::SetStatus { dnd, .. }) = &mut self.modal {
                    *dnd = on;
                }
                Task::none()
            }
            Msg::StatusDur(d) => {
                if let Some(Modal::SetStatus { dur, .. }) = &mut self.modal {
                    *dur = d;
                }
                Task::none()
            }
            Msg::StatusSave(limpar) => {
                let Some(Modal::SetStatus { e, t, dnd, dur }) = self.modal.take() else { return Task::none() };
                let agora = jm_core::engine::now_ms();
                let x = match dur {
                    1 => agora + 3_600_000,
                    2 => agora + 4 * 3_600_000,
                    3 => chrono::Local::now().date_naive().and_hms_opt(23, 59, 59).and_then(|d| d.and_local_timezone(chrono::Local).single()).map(|d| d.timestamp_millis()).unwrap_or(0),
                    _ => 0,
                };
                let st = (!limpar).then_some(jm_core::messages::CustomStatus { e, t, dnd, x });
                let e = self.eng();
                toast_done(async move { e.set_my_status(st).await.map(|_| "Status atualizado".to_string()).map_err(s) })
            }
            Msg::RemoveDevice(dev) => {
                let e = self.eng();
                toast_done(async move { e.remove_device(&dev).await.map(|_| "Pedido enviado: o aparelho sai assim que ficar on-line".to_string()).map_err(s) })
            }
            Msg::MentionPick(nome) => {
                let Some(c) = self.chat.as_mut() else { return Task::none() };
                let t = c.editor.text();
                let t = t.trim_end_matches('\n');
                let Some(i) = t.rfind('@') else { return Task::none() };
                let novo = format!("{}@{} ", &t[..i], nome);
                c.editor = text_editor::Content::with_text(&novo);
                c.editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
                iced::widget::operation::focus(EDITOR_ID)
            }
            Msg::SendTextReady(key, texto, reply, lp) => {
                let e = self.eng();
                Task::batch([done(async move { e.send_text_full(&key, &texto, reply.as_deref(), false, lp).await.map(|_| ()).map_err(s) }), to_bottom()])
            }
            Msg::PickFile => {
                self.close_overlays();
                Task::perform(platform::pick_file(), Msg::FilePicked)
            }
            Msg::FilePicked(r) => match r {
                Ok(Some((nome, mime, bytes))) => {
                    let Some(key) = self.open_key() else { return Task::none() };
                    let e = self.eng();
                    let meta = MediaMeta { kind: Some(MediaKind::File), mime, name: Some(nome), ..Default::default() };
                    Task::batch([done(async move { e.send_media(&key, bytes, meta).await.map(|_| ()).map_err(s) }), to_bottom()])
                }
                Ok(None) => Task::none(),
                Err(e) => self.toast_t(&e),
            },
            Msg::SaveFile(mid, nome) => {
                self.close_overlays();
                let Some(bytes) = self.engine.media_bytes(&mid) else { return self.toast_t("O arquivo ainda não chegou inteiro.") };
                toast_done(platform::save_file(bytes, nome))
            }
            Msg::OpenForward(id) => {
                self.close_overlays();
                self.modal = Some(Modal::Forward { id, query: String::new() });
                Task::none()
            }
            Msg::ForwardQuery(q) => {
                if let Some(Modal::Forward { query, .. }) = &mut self.modal {
                    *query = q.chars().take(40).collect();
                }
                Task::none()
            }
            Msg::ForwardTo(to) => {
                let Some(Modal::Forward { id, .. }) = self.modal.take() else { return Task::none() };
                let e = self.eng();
                let nome = self.chat_title(&to);
                toast_done(async move { e.forward(&id, &to).await.map(|_| format!("Encaminhada para {nome}")).map_err(s) })
            }
            Msg::OpenViewOnce(id) => {
                self.close_overlays();
                let Some(key) = self.open_key() else { return Task::none() };
                let e = self.eng();
                let i2 = id.clone();
                Task::perform(async move { e.open_view_once(&key, &i2).await.map_err(s) }, move |r| Msg::ViewOnceBytes(id.clone(), r))
            }
            Msg::ViewOnceBytes(id, r) => {
                let bytes = match r {
                    Ok(b) => b,
                    Err(e) => return self.toast_t(&e),
                };
                let audio = self.chat.as_ref().and_then(|c| c.messages.iter().find(|m| m.id == id)).is_some_and(|m| matches!(&m.body, MessageBody::Media(x) if x.kind == MediaKind::Audio));
                if audio {
                    let mid = format!("vo:{id}");
                    return Task::perform(
                        async move {
                            let pcm = tokio::task::spawn_blocking(move || jm_media::audiofile::decode(&bytes).map(|p| p.samples)).await;
                            (mid, pcm)
                        },
                        |(mid, r)| match r {
                            Ok(Ok(pcm)) => Msg::VoiceReady(mid, Arc::new(pcm)),
                            _ => Msg::Toast("Não deu para tocar esse áudio.".into()),
                        },
                    );
                }
                self.modal = Some(Modal::ViewOnce { handle: image::Handle::from_bytes(bytes) });
                Task::none()
            }
            Msg::SendPhotoOnce => {
                let Some(Modal::PhotoPreview(d)) = self.modal.take() else { return Task::none() };
                self.send_draft_opt(d, true)
            }
            Msg::BackupExport => {
                let e = self.eng();
                toast_done(async move {
                    let dados = tokio::task::spawn_blocking(move || e.export_backup()).await.map_err(|e| e.to_string())?.map_err(s)?;
                    platform::save_backup(dados).await
                })
            }
            Msg::BackupImport => {
                let e = self.eng();
                toast_done(async move {
                    let Some(f) = rfd::AsyncFileDialog::new()
                        .set_title("Restaurar backup do J-Menseger")
                        .add_filter("Backup do J-Menseger", &["jmbackup"])
                        .pick_file()
                        .await
                    else {
                        return Err(String::new());
                    };
                    let dados = tokio::fs::read(f.path()).await.map_err(|e| e.to_string())?;
                    let r = e.import_backup(&dados).await.map_err(s)?;
                    Ok(format!("Backup restaurado: {} mensagens, {} contatos e {} grupos novos", r.messages, r.contacts, r.groups))
                })
            }
            Msg::TrayClose(on) => {
                self.tray_close = on;
                let _ = self.engine.set_ui_pref("tray", &on);
                Task::none()
            }
            Msg::TrayAutostart(on) => match crate::tray::set_autostart(on) {
                Ok(()) => {
                    self.autostart = on;
                    Task::none()
                }
                Err(e) => self.toast_t(&e),
            },
            Msg::MemberMenu(lid, pk) => open_menu(CtxKind::Member(lid, pk)),
            Msg::SetAdmin(lid, pk, on) => {
                self.menu = None;
                let e = self.eng();
                toast_done(async move {
                    e.edit_group(&lid, &[GroupPatch::Admin { pk, on }])
                        .await
                        .map(|_| if on { "Agora é admin".to_string() } else { "Deixou de ser admin".to_string() })
                        .map_err(s)
                })
            }
            Msg::OpenInvite(lid) => {
                self.close_overlays();
                self.modal = Some(Modal::Invite { lid });
                Task::none()
            }
            Msg::InviteSend(lid, pk) => {
                let (e, nome) = (self.eng(), self.first_name(&pk));
                toast_done(async move { e.invite_to_group(&lid, &pk).await.map(|_| format!("Convite enviado para {nome}")).map_err(s) })
            }

            /* ---------------------------------------------- chamadas */
            Msg::UpdateCheck => {
                self.update_at = Instant::now();
                self.update_msg = Some("Procurando…".into());
                Task::perform(
                    async { tokio::task::spawn_blocking(crate::atualizar::procurar).await.map_err(|e| e.to_string()).and_then(|r| r) },
                    Msg::UpdateFound,
                )
            }
            Msg::UpdateFound(r) => {
                match r {
                    Ok(Some(p)) => {
                        self.update_msg = None;
                        self.update = Some(p);
                    }
                    Ok(None) => self.update_msg = Some(format!("Você está na versão mais nova ({}).", crate::atualizar::VERSAO)),
                    Err(e) => self.update_msg = Some(e),
                }
                Task::none()
            }
            Msg::UpdateStart => {
                let Some(p) = self.update.clone() else { return Task::none() };
                if self.update_prog.is_some() {
                    return Task::none();
                }
                let prog = Arc::new(std::sync::atomic::AtomicU64::new(0));
                self.update_prog = Some(Arc::clone(&prog));
                Task::perform(
                    async move { tokio::task::spawn_blocking(move || crate::atualizar::baixar(&p, prog)).await.map_err(|e| e.to_string()).and_then(|r| r) },
                    Msg::UpdateDownloaded,
                )
            }
            Msg::UpdateDownloaded(r) => {
                self.update_prog = None;
                match r.and_then(|caminho| crate::atualizar::instalar(&caminho)) {
                    // O instalador troca o programa e o abre de novo.
                    Ok(()) => iced::exit(),
                    Err(e) => {
                        self.toast(e.clone());
                        self.update_msg = Some(e);
                        Task::none()
                    }
                }
            }
            Msg::ToggleCamera => {
                self.camera_wanted = false;
                let c = Arc::clone(&self.calls);
                Task::perform(async move { c.toggle_camera().await }, Msg::CameraToggled)
            }
            Msg::CameraToggled(r) => {
                match r {
                    Ok(false) => self.local_preview = None,
                    Ok(true) => {}
                    Err(e) => self.toast(e),
                }
                self.after_media_change()
            }
            Msg::Call(pk, video) => {
                self.camera_wanted = video;
                self.close_overlays();
                if self.room.is_some() {
                    self.toast("Saia da sala de voz para ligar.");
                    return Task::none();
                }
                let c = Arc::clone(&self.calls);
                Task::perform(
                    async move {
                        match c.start(&pk, video).await {
                            Ok(()) => Msg::CallUpdated(c.info().await),
                            Err(e) => Msg::Toast(e),
                        }
                    },
                    |m| m,
                )
            }
            Msg::AcceptCall => {
                self.camera_wanted = self.call.as_ref().is_some_and(|c| matches!(c.state, CallState::Ringing { video: true }));
                let c = Arc::clone(&self.calls);
                Task::perform(
                    async move {
                        match c.accept().await {
                            Ok(()) => Msg::CallUpdated(c.info().await),
                            Err(e) => Msg::Toast(e),
                        }
                    },
                    |m| m,
                )
            }
            Msg::DeclineCall => {
                let c = Arc::clone(&self.calls);
                Task::perform(
                    async move {
                        c.decline().await;
                        c.info().await
                    },
                    Msg::CallUpdated,
                )
            }
            Msg::HangUp => {
                self.popup = None;
                let c = Arc::clone(&self.calls);
                Task::perform(
                    async move {
                        c.hang_up().await;
                        c.info().await
                    },
                    Msg::CallUpdated,
                )
            }
            Msg::ToggleMute => {
                let c = Arc::clone(&self.calls);
                let agora = self.call.as_ref().map(|c| c.muted).or(self.room.as_ref().map(|r| r.muted)).unwrap_or(false);
                let sala = self.room.is_some();
                Task::perform(
                    async move {
                        c.set_muted(!agora).await;
                        if sala {
                            Msg::RoomUpdated(c.room_info().await)
                        } else {
                            Msg::CallUpdated(c.info().await)
                        }
                    },
                    |m| m,
                )
            }
            Msg::CallUpdated(info) => self.on_call_info(info),
            Msg::RoomUpdated(info) => {
                if info.is_some() {
                    self.joining_room = false;
                }
                self.room = info;
                self.on_room_media()
            }
            Msg::RoomFrame(pk, v, h) => {
                self.room_frames.insert(pk, (v, h));
                Task::none()
            }
            Msg::FrameArrived(v, h) => {
                self.remote_frame = Some((v, h));
                Task::none()
            }
            Msg::OpenShare => {
                self.popup = None;
                self.modal = Some(Modal::Share { sources: self.calls.share_sources() });
                Task::none()
            }
            Msg::ShareQuality(q) => {
                self.share_quality = q;
                Task::none()
            }
            Msg::ShareSource(id) => {
                let Some(Modal::Share { sources }) = self.modal.take() else { return Task::none() };
                let Some(src) = sources.into_iter().find(|s| s.id == id) else { return Task::none() };
                let (c, q) = (Arc::clone(&self.calls), self.share_quality);
                let audio = self.share_audio && jm_media::sysaudio::supported() && src.kind != jm_media::screen::SourceKind::Camera;
                done(async move {
                    c.start_sharing(src, q, Fps::Fps30).await?;
                    if audio {
                        if let Err(e) = jm_media::sysaudio::start() {
                            tracing::warn!("áudio do PC: {e}");
                            return Err(format!("A tela foi, mas o áudio do PC não: {e}"));
                        }
                    }
                    Ok(())
                })
            }
            Msg::StopSharing => {
                let c = Arc::clone(&self.calls);
                let sala = self.room.is_some();
                Task::perform(
                    async move {
                        c.stop_sharing().await;
                        if sala {
                            Msg::RoomUpdated(c.room_info().await)
                        } else {
                            Msg::CallUpdated(c.info().await)
                        }
                    },
                    |m| m,
                )
            }
            Msg::CallSound(id) => {
                let c = Arc::clone(&self.calls);
                Task::perform(async move { c.call_sound(&id).await }, |_| Msg::Noop)
            }
            Msg::SaveBannerSound => {
                let r = self.calls.save_banner_sound();
                self.sounds.changed();
                match r {
                    Some(n) => self.toast(format!("\"{n}\" está no seu soundboard")),
                    None => self.toast("Não foi possível salvar o som."),
                }
                Task::none()
            }
            Msg::JoinRoom(lid, ch) => {
                self.close_overlays();
                if self.call.as_ref().is_some_and(|c| !matches!(c.state, CallState::Ended { .. })) {
                    self.toast("Termine a chamada atual primeiro.");
                    return Task::none();
                }
                if self.room.as_ref().is_some_and(|r| r.lid == lid && r.ch == ch) {
                    return Task::none();
                }
                if self.rec.is_some() {
                    self.rec = None;
                }
                self.joining_room = true;
                let c = Arc::clone(&self.calls);
                Task::perform(
                    async move {
                        if c.room_info().await.is_some() {
                            c.leave_room().await;
                        }
                        match c.join_room(&lid, &ch).await {
                            Ok(()) => Msg::RoomUpdated(c.room_info().await),
                            Err(e) => Msg::Toast(e),
                        }
                    },
                    |m| m,
                )
                .chain(Task::done(Msg::Noop))
            }
            Msg::LeaveRoom => {
                self.popup = None;
                self.joining_room = false;
                let c = Arc::clone(&self.calls);
                Task::perform(
                    async move {
                        c.leave_room().await;
                        c.room_info().await
                    },
                    Msg::RoomUpdated,
                )
            }

            /* ---------------------------------------------- ajustes */
            Msg::ProfName(v) => {
                self.forms.prof_name = v.chars().take(40).collect();
                Task::none()
            }
            Msg::ProfNote(v) => {
                self.forms.prof_note = v.chars().take(80).collect();
                Task::none()
            }
            Msg::ProfTone(v) => {
                self.forms.prof_tone = v % 6;
                Task::none()
            }
            Msg::ProfSave => {
                let nome = self.forms.prof_name.trim().to_string();
                if nome.chars().count() < 2 {
                    self.toast("Escolha um nome com pelo menos 2 letras.");
                    return Task::none();
                }
                let (e, nota, tom) = (self.eng(), self.forms.prof_note.trim().to_string(), self.forms.prof_tone);
                toast_done(async move {
                    e.update_profile(Some(&nome), Some(&nota), Some(tom), None).await.map(|_| "Perfil atualizado".to_string()).map_err(s)
                })
            }
            Msg::ProfPhoto => Task::perform(platform::pick_for_crop(CropTarget::Me), Msg::PhotoForCrop),
            Msg::CallFull(on) => {
                let on = on && self.in_voice();
                if on == self.call_full {
                    return Task::none();
                }
                self.call_full = on;
                self.menu = None;
                match self.window {
                    Some(id) => iced::window::set_mode(id, if on { iced::window::Mode::Fullscreen } else { iced::window::Mode::Windowed }),
                    None => Task::none(),
                }
            }
            Msg::AudioRefresh => {
                self.refresh_devices();
                Task::none()
            }
            Msg::AudioInput(v) => {
                self.audio.input = (v != PADRAO).then_some(v);
                self.save_audio();
                self.restart_mic_test();
                Task::none()
            }
            Msg::AudioOutput(v) => {
                self.audio.output = (v != PADRAO).then_some(v);
                self.save_audio();
                if let Err(e) = self.sounds.reopen_player() {
                    self.toast(format!("Não deu para abrir essa saída: {e}"));
                }
                self.restart_mic_test();
                Task::none()
            }
            Msg::AudioGain(v) => {
                self.audio.input_gain = v.clamp(0.0, 3.0);
                self.save_audio();
                Task::none()
            }
            Msg::AudioVolume(v) => {
                self.audio.output_volume = v.clamp(0.0, 2.0);
                self.save_audio();
                if let Some(t) = &self.mic_test {
                    if let Some(e) = &t.eco {
                        e.set_volume(self.audio.output_volume);
                    }
                }
                Task::none()
            }
            Msg::MicTestToggle => {
                if self.mic_test.take().is_none() {
                    self.start_mic_test(false);
                }
                Task::none()
            }
            Msg::MicEcho(on) => {
                self.start_mic_test(on);
                Task::none()
            }
            Msg::SpeakerTest => {
                if self.sounds.play("tcharam") == 0.0 {
                    self.toast("Nenhuma saída de som funcionando. Confira a saída escolhida.");
                }
                Task::none()
            }
            Msg::BannerPick => Task::perform(platform::pick_for_crop(CropTarget::Banner), Msg::PhotoForCrop),
            Msg::BannerRemove => {
                let e = self.eng();
                toast_done(async move { e.update_profile_extras(Some(""), None).await.map(|_| "Banner removido".to_string()).map_err(s) })
            }
            Msg::GameInput(v) => {
                self.forms.game = v.chars().take(30).collect();
                Task::none()
            }
            Msg::GameAdd => {
                let novo = self.forms.game.trim().to_string();
                if novo.is_empty() {
                    return Task::none();
                }
                if self.me.games.len() >= jm_core::media::MAX_GAMES {
                    self.toast("Cabem até 6 jogos. Tire um para pôr outro.");
                    return Task::none();
                }
                if self.me.games.iter().any(|g| g.to_lowercase() == novo.to_lowercase()) {
                    self.toast("Esse jogo já está na lista.");
                    return Task::none();
                }
                self.forms.game.clear();
                let mut lista = self.me.games.clone();
                lista.push(novo);
                self.me.games = lista.clone();
                let e = self.eng();
                done(async move { e.update_profile_extras(None, Some(&lista)).await.map_err(s) })
            }
            Msg::GameRemove(i) => {
                let mut lista = self.me.games.clone();
                if i < lista.len() {
                    lista.remove(i);
                }
                self.me.games = lista.clone();
                let e = self.eng();
                done(async move { e.update_profile_extras(None, Some(&lista)).await.map_err(s) })
            }
            Msg::ProfPhotoRemove => {
                let e = self.eng();
                toast_done(async move { e.update_profile(None, None, None, Some("")).await.map(|_| "Foto removida".to_string()).map_err(s) })
            }
            Msg::Toggle(k, on) => self.toggle(k, on),
            Msg::Choose(k) => {
                self.close_overlays();
                if matches!(k, ChooseKind::PinDelay) && self.pin.is_none() {
                    return Task::none();
                }
                self.modal = Some(Modal::Choose(k));
                Task::none()
            }
            Msg::Chosen(k, v) => {
                self.modal = None;
                let e = self.eng();
                match k {
                    ChooseKind::TtlDefault => done(async move { e.update_settings(|s| s.ttl_default = v).await.map(|_| ()).map_err(s) }),
                    ChooseKind::Ttl(key) => done(async move { e.set_ttl(&key, v).await.map_err(s) }),
                    ChooseKind::NotifPreview => {
                        let p = match v {
                            0 => NotifPreview::Full,
                            1 => NotifPreview::Name,
                            _ => NotifPreview::None,
                        };
                        done(async move { e.update_settings(|s| s.notif_preview = p).await.map(|_| ()).map_err(s) })
                    }
                    ChooseKind::CallsFrom => {
                        let c = if v == 0 { CallsFrom::Contacts } else { CallsFrom::Nobody };
                        done(async move { e.update_settings(|s| s.calls_from = c).await.map(|_| ()).map_err(s) })
                    }
                    ChooseKind::PinDelay => {
                        if let Err(err) = self.engine.set_pin_delay(v) {
                            self.toast(err.to_string());
                        }
                        self.pin = self.engine.pin_lock();
                        Task::none()
                    }
                }
            }
            Msg::Confirm(k) => {
                self.close_overlays();
                self.modal = Some(Modal::Confirm(k));
                Task::none()
            }
            Msg::ConfirmYes => {
                let Some(Modal::Confirm(k)) = self.modal.take() else { return Task::none() };
                self.confirmed(k)
            }
            Msg::PinToggle => {
                self.modal = Some(Modal::Pin(PinPad::new(if self.pin.is_some() { PinPurpose::Remove } else { PinPurpose::Create })));
                Task::none()
            }
            Msg::PinChange => {
                self.modal = Some(Modal::Pin(PinPad::new(PinPurpose::ConfirmChange)));
                Task::none()
            }
            Msg::PinKey(c) => self.pin_key(c),
            Msg::PinBack => {
                if let Some(Modal::Pin(p)) = &mut self.modal {
                    p.buf.pop();
                    p.error = false;
                }
                Task::none()
            }
            Msg::PinResult(ok) => self.pin_result(ok),
            Msg::PinCreated(r) => {
                match r {
                    Ok(lock) => {
                        self.pin = Some(lock);
                        self.modal = None;
                        self.toast("PIN ativado");
                    }
                    Err(e) => {
                        self.modal = None;
                        self.toast(e);
                    }
                }
                Task::none()
            }
            Msg::PinForgot => {
                self.modal = Some(Modal::Forgot { input: String::new(), error: None });
                Task::none()
            }
            Msg::ForgotInput(v) => {
                if let Some(l) = &mut self.lock {
                    if let Some(f) = &mut l.forgot {
                        f.0 = v;
                        f.1 = None;
                        return Task::none();
                    }
                }
                if let Some(Modal::Forgot { input, error }) = &mut self.modal {
                    *input = v;
                    *error = None;
                }
                Task::none()
            }
            Msg::ForgotConfirm => self.forgot_confirm(),
            Msg::OpenRecovery => {
                self.close_overlays();
                if self.pin.is_some() {
                    self.modal = Some(Modal::Pin(PinPad::new(PinPurpose::Recovery)));
                    return Task::none();
                }
                self.open_recovery();
                Task::none()
            }
            Msg::RevealWords => {
                self.forms.reveal_words = !self.forms.reveal_words;
                Task::none()
            }
            Msg::RevealNsec => {
                self.forms.reveal_nsec = !self.forms.reveal_nsec;
                Task::none()
            }
            Msg::CopyNsec => {
                let Some(nsec) = self.forms.keys.as_ref().map(|k| k.nsec.clone()) else { return Task::none() };
                self.toast("Chave copiada. Cuidado com onde você vai colar.");
                iced::clipboard::write(nsec)
            }
            Msg::WordsDone => {
                let _ = self.engine.set_words_pending(false);
                self.words_pending = false;
                self.toast("Ótimo. Guarde em lugar seguro.");
                Task::done(Msg::Page(SettingsPage::Profile))
            }
            Msg::Accent(h) => {
                self.look.accent = h;
                self.save_look();
                Task::none()
            }
            Msg::AccentHue(h) => {
                self.look.accent = t::hex(t::hsl(h, 0.68, 0.52));
                t::apply(&self.look);
                self.forms.hue = h;
                let _ = self.engine.set_ui_pref("look", &self.look);
                Task::none()
            }
            Msg::SetBackdrop(b) => {
                self.look.backdrop = b;
                self.save_look();
                Task::none()
            }
            Msg::LookReset => {
                self.look = Look::default();
                self.save_look();
                self.toast("Cores de fábrica");
                Task::none()
            }
            Msg::Volume(v) => {
                self.sounds.set_volume(v);
                Task::none()
            }
            Msg::SbPlay(id) => {
                self.sounds.play(&id);
                Task::none()
            }
            Msg::SbHide(id) => {
                self.sounds.changed();
                match self.engine.delete_sound(&id) {
                    Ok(()) => self.toast("Som removido da lista. Dá para restaurar depois."),
                    Err(e) => self.toast(e.to_string()),
                }
                Task::none()
            }
            Msg::SbRestore => {
                self.sounds.changed();
                match self.engine.restore_builtins() {
                    Ok(()) => self.toast("Sons de fábrica restaurados"),
                    Err(e) => self.toast(e.to_string()),
                }
                Task::none()
            }
            Msg::SbEdit(id) => {
                if let Some(s) = self.sounds.info(&id).filter(|s| !s.builtin) {
                    self.modal = Some(Modal::EditSound { id, name: s.name, emoji: s.emoji });
                }
                Task::none()
            }
            Msg::SbEditName(v) => {
                if let Some(Modal::EditSound { name, .. }) = &mut self.modal {
                    *name = v.chars().take(24).collect();
                }
                Task::none()
            }
            Msg::SbEditEmoji(v) => {
                if let Some(Modal::EditSound { emoji, .. }) = &mut self.modal {
                    *emoji = v;
                }
                Task::none()
            }
            Msg::SbEditSave => {
                let Some(Modal::EditSound { id, name, emoji }) = self.modal.take() else { return Task::none() };
                let nome = if name.trim().is_empty() { None } else { Some(name.trim().to_string()) };
                self.sounds.changed();
                match self.engine.edit_custom_sound(&id, nome.as_deref(), Some(&emoji)) {
                    Ok(()) => self.toast("Som atualizado"),
                    Err(e) => self.toast(e.to_string()),
                }
                Task::none()
            }
            Msg::SbAdd => {
                self.close_overlays();
                self.modal = Some(Modal::AddSound);
                Task::none()
            }
            Msg::SbRecord => {
                self.modal = Some(Modal::RecordSound { rec: None, levels: Vec::new() });
                Task::none()
            }
            Msg::SbRecStart => {
                if self.call.is_some() || self.room.is_some() {
                    self.toast("O microfone está na chamada agora.");
                    return Task::none();
                }
                if let Some(Modal::RecordSound { rec, levels }) = &mut self.modal {
                    match jm_media::recorder::Recorder::start() {
                        Ok(r) => {
                            *rec = Some(r);
                            levels.clear();
                        }
                        Err(e) => {
                            let m = format!("Não deu para usar o microfone: {e}");
                            self.toast(m);
                        }
                    }
                }
                Task::none()
            }
            Msg::SbRecStop => {
                let Some(Modal::RecordSound { rec, .. }) = &mut self.modal else { return Task::none() };
                let Some(r) = rec.take() else { return Task::none() };
                let g = r.stop();
                if g.dur < TRIM_MIN || g.pcm.is_empty() {
                    self.modal = None;
                    self.toast("Gravação curta demais.");
                    return Task::none();
                }
                self.modal = Some(Modal::Trim(TrimState::new("Meu som".into(), Arc::new(g.pcm))));
                Task::none()
            }
            Msg::SbImport => {
                self.modal = None;
                Task::perform(platform::pick_audio(), |r| match r {
                    Ok(Some(x)) => Msg::SbDecoded(Ok(x)),
                    Ok(None) => Msg::Noop,
                    Err(e) => Msg::SbDecoded(Err(e)),
                })
            }
            Msg::SbDecoded(r) => {
                match r {
                    Ok((nome, pcm)) => {
                        if (pcm.len() as f64) / 48_000.0 < TRIM_MIN {
                            self.toast("Esse áudio é curto demais.");
                        } else {
                            self.modal = Some(Modal::Trim(TrimState::new(nome, pcm)));
                        }
                    }
                    Err(e) => self.toast(e),
                }
                Task::none()
            }
            Msg::TrimSet(a, b) => {
                if let Some(Modal::Trim(st)) = &mut self.modal {
                    st.a = a;
                    st.b = b;
                    if let Some(p) = self.sounds.player() {
                        if p.track().is_some_and(|t| t.0 == "trim") {
                            p.stop_track();
                        }
                    }
                }
                Task::none()
            }
            Msg::TrimView(v0) => {
                if let Some(Modal::Trim(st)) = &mut self.modal {
                    let vlen = st.vlen();
                    let len = st.b - st.a;
                    st.v0 = v0.clamp(0.0, (st.dur - vlen).max(0.0));
                    st.a = st.a.clamp(st.v0, (st.v0 + vlen - len).max(st.v0));
                    st.b = st.a + len;
                    st.refresh_peaks();
                }
                Task::none()
            }
            Msg::TrimPlay => {
                let Some(Modal::Trim(st)) = &self.modal else { return Task::none() };
                let Some(p) = self.sounds.player() else { return Task::none() };
                if p.track().is_some_and(|t| t.0 == "trim" && !t.3 && t.1 + 0.02 < t.2) {
                    p.stop_track();
                } else {
                    p.play_track("trim", Arc::new(jm_media::audiofile::clip(&st.pcm, st.a, st.b)));
                }
                self.track = p.track();
                Task::none()
            }
            Msg::TrimName(v) => {
                if let Some(Modal::Trim(st)) = &mut self.modal {
                    st.name = v.chars().take(24).collect();
                }
                Task::none()
            }
            Msg::TrimEmoji(v) => {
                if let Some(Modal::Trim(st)) = &mut self.modal {
                    st.emoji = v;
                }
                Task::none()
            }
            Msg::TrimSave => {
                let e = self.eng();
                if let Some(p) = self.sounds.player() {
                    p.stop_track();
                }
                let Some(Modal::Trim(st)) = &mut self.modal else { return Task::none() };
                if st.busy {
                    return Task::none();
                }
                st.busy = true;
                let (pcm, a, b) = (Arc::clone(&st.pcm), st.a, st.b);
                let nome = if st.name.trim().is_empty() { "Meu som".to_string() } else { st.name.trim().to_string() };
                let emoji = st.emoji.clone();
                Task::perform(
                    async move {
                        let bytes = tokio::task::spawn_blocking(move || {
                            let trecho = jm_media::audiofile::clip(&pcm, a, b);
                            jm_media::audiofile::encode_ogg_opus(&trecho, 64_000, true)
                        })
                        .await
                        .map_err(s)?
                        .map_err(s)?;
                        if bytes.len() > jm_core::media::MAX_SOUND_BYTES {
                            return Err("O som ficou grande demais. Escolha um trecho menor.".to_string());
                        }
                        let r = e.save_custom_sound(&bytes, "audio/ogg", &nome, &emoji, b - a).map_err(s)?;
                        Ok(r.id)
                    },
                    Msg::TrimSaved,
                )
            }
            Msg::TrimSaved(r) => {
                self.sounds.changed();
                match r {
                    Ok(id) => {
                        self.modal = None;
                        let nome = self.sounds.info(&id).map(|s| s.name).unwrap_or_default();
                        self.toast(format!("\"{nome}\" está no seu soundboard"));
                        self.sounds.play(&id);
                    }
                    Err(e) => {
                        if let Some(Modal::Trim(st)) = &mut self.modal {
                            st.busy = false;
                        }
                        self.toast(if e.is_empty() { "Não foi possível salvar o som.".to_string() } else { e });
                    }
                }
                Task::none()
            }
            Msg::RelayInput(v) => {
                self.forms.relay = v;
                Task::none()
            }
            Msg::RelayAdd => {
                let mut u = self.forms.relay.trim().to_string();
                if u.is_empty() {
                    return Task::none();
                }
                if !u.contains("://") {
                    u = format!("wss://{u}");
                }
                if !(u.starts_with("wss://") || u.starts_with("ws://")) || u.len() < 9 || u.contains(' ') {
                    self.toast("Endereço inválido. Use o formato wss://servidor.com");
                    return Task::none();
                }
                if self.settings.relays.iter().any(|r| r.trim_end_matches('/') == u.trim_end_matches('/')) {
                    self.toast("Esse servidor já está na lista.");
                    return Task::none();
                }
                self.forms.relay.clear();
                let e = self.eng();
                toast_done(async move {
                    e.update_settings(|s| s.relays.push(u)).await.map(|_| "Servidor adicionado".to_string()).map_err(s)
                })
            }
            Msg::RelayRemove(u) => {
                if self.settings.relays.len() <= 1 {
                    self.toast("Deixe pelo menos um servidor.");
                    return Task::none();
                }
                let e = self.eng();
                done(async move { e.update_settings(|s| s.relays.retain(|r| r != &u)).await.map(|_| ()).map_err(s) })
            }
            Msg::RelaysReset => {
                let e = self.eng();
                toast_done(async move {
                    e.update_settings(|s| s.relays = jm_core::engine::Settings::default().relays)
                        .await
                        .map(|_| "Servidores de fábrica".to_string())
                        .map_err(s)
                })
            }
            Msg::TurnUrl(v) => {
                self.forms.turn_url = v;
                Task::none()
            }
            Msg::TurnUser(v) => {
                self.forms.turn_user = v;
                Task::none()
            }
            Msg::TurnPass(v) => {
                self.forms.turn_pass = v;
                Task::none()
            }
            Msg::TurnSave => {
                let url = self.forms.turn_url.trim().to_string();
                if !url.is_empty() && !url.split([',', ' ']).filter(|x| !x.is_empty()).all(|x| x.starts_with("turn:") || x.starts_with("turns:")) {
                    self.toast("O endereço precisa começar com turn: ou turns:");
                    return Task::none();
                }
                let (user, pass) = (self.forms.turn_user.trim().to_string(), self.forms.turn_pass.clone());
                let e = self.eng();
                toast_done(async move {
                    e.update_settings(|s| {
                        s.turn = jm_core::engine::TurnServer { url: url.clone(), user, pass };
                        if url.is_empty() {
                            s.force_relay = false;
                        }
                    })
                    .await
                    .map(|_| "Servidor TURN salvo".to_string())
                    .map_err(s)
                })
            }

            /* ---------------------------------------------- recorte */
            Msg::PhotoForCrop(r) => {
                match r {
                    Ok(Some(d)) => {
                        let r = d.target.ratio();
                        let lado = (d.w as f32).min(d.h as f32 * r) * 0.9;
                        self.modal = Some(Modal::Crop(CropState {
                            x: (d.w as f32 - lado) / 2.0,
                            y: (d.h as f32 - lado / r) / 2.0,
                            side: lado,
                            target: d.target,
                            bytes: d.bytes,
                            handle: d.handle,
                            w: d.w,
                            h: d.h,
                            busy: false,
                        }));
                    }
                    Ok(None) => {}
                    Err(e) => self.toast(e),
                }
                Task::none()
            }
            Msg::CropSet(x, y, side) => {
                if let Some(Modal::Crop(c)) = &mut self.modal {
                    let r = c.target.ratio();
                    c.side = side.clamp(16.0, (c.w as f32).min(c.h as f32 * r));
                    c.x = x.clamp(0.0, c.w as f32 - c.side);
                    c.y = y.clamp(0.0, c.h as f32 - c.side / r);
                }
                Task::none()
            }
            Msg::CropSave => {
                let Some(Modal::Crop(c)) = &mut self.modal else { return Task::none() };
                if c.busy {
                    return Task::none();
                }
                c.busy = true;
                let (bytes, alvo) = (Arc::clone(&c.bytes), c.target.clone());
                let (x, y, lado, r) = (c.x as u32, c.y as u32, c.side as u32, c.target.ratio());
                let e = self.eng();
                Task::perform(
                    async move {
                        let banner = alvo == CropTarget::Banner;
                        let url = tokio::task::spawn_blocking(move || {
                            if banner {
                                jm_media::photo::banner(&bytes, Some((x, y, lado, (lado as f32 / r) as u32)))
                            } else {
                                jm_media::photo::avatar(&bytes, Some((x, y, lado)))
                            }
                        })
                        .await
                        .map_err(s)?
                        .map_err(|_| "Não deu para usar essa foto.".to_string())?;
                        match alvo {
                            CropTarget::Banner => {
                                e.update_profile_extras(Some(&url), None).await.map_err(s)?;
                                Ok("Banner atualizado. Seus contatos recebem o novo banner.".to_string())
                            }
                            CropTarget::Me => {
                                e.update_profile(None, None, None, Some(&url)).await.map_err(s)?;
                                Ok("Foto atualizada. Seus contatos recebem a nova foto.".to_string())
                            }
                            CropTarget::Group(lid) => {
                                e.set_group_pic(&lid, &url).await.map_err(s)?;
                                Ok("Foto do grupo atualizada".to_string())
                            }
                        }
                    },
                    Msg::CropDone,
                )
            }
            Msg::CropDone(r) => {
                match r {
                    Ok(t) => {
                        self.modal = None;
                        self.toast(t);
                    }
                    Err(e) => {
                        if let Some(Modal::Crop(c)) = &mut self.modal {
                            c.busy = false;
                        }
                        self.toast(e);
                    }
                }
                Task::none()
            }

            /* ---------------------------------------------- bloqueio */
            Msg::LockKey(c) => self.lock_key(c),
            Msg::LockBack => {
                if let Some(l) = &mut self.lock {
                    l.buf.pop();
                    l.error = false;
                }
                Task::none()
            }
            Msg::LockResult(ok) => {
                let Some(l) = &mut self.lock else { return Task::none() };
                if ok {
                    self.unlock();
                    return Task::none();
                }
                l.tries += 1;
                l.buf.clear();
                l.error = true;
                if l.tries >= 5 {
                    l.wait_until = Some(Instant::now() + Duration::from_secs(30));
                    l.msg = "Muitas tentativas. Espere 30 segundos.".into();
                } else {
                    l.msg = "PIN incorreto".into();
                }
                Task::none()
            }
            Msg::LockForgot => {
                if let Some(l) = &mut self.lock {
                    l.forgot = if l.forgot.is_some() { None } else { Some((String::new(), None)) };
                }
                Task::none()
            }

            // As mensagens das telas de entrada não chegam aqui.
            _ => Task::none(),
        }
    }

    /* ------------------------------------------------------------------ */

    fn open_chat(&mut self, key: String) -> Task<Msg> {
        self.close_overlays();
        self.modal = None;
        self.show_requests = false;
        if let Some(c) = self.chat.take() {
            let txt = c.editor.text();
            if txt.trim().is_empty() {
                self.drafts.remove(&c.key);
            } else {
                self.drafts.insert(c.key, txt);
            }
        }
        if let Err(e) = self.engine.ensure_chat(&key) {
            tracing::debug!("conversa nova: {e}");
        }
        if let Some(rest) = key.strip_prefix("g:") {
            let lid = rest.split(':').next().unwrap_or("").to_string();
            self.section = Section::Group(lid);
        } else if matches!(self.section, Section::Group(_) | Section::Settings) {
            self.section = Section::Chats;
        }
        let rascunho = self.drafts.get(&key).cloned().unwrap_or_default();
        self.chat = Some(OpenChat {
            key: key.clone(),
            messages: Vec::new(),
            more: false,
            editor: text_editor::Content::with_text(rascunho.trim_end_matches('\n')),
            reply: None,
            progress: HashMap::new(),
            edit: None,
            pins: self.engine.pins(&key),
        });
        let e = self.eng();
        let k = key.clone();
        let carregar = Task::perform(async move { (k.clone(), e.messages(&k, i64::MAX, PAGE).unwrap_or_default()) }, |(key, msgs)| {
            Msg::MessagesLoaded { key, msgs, older: false }
        });
        let _ = self.engine.set_view(Some(&key), self.focused && self.lock.is_none());
        Task::batch([carregar, iced::widget::operation::focus(EDITOR_ID), self.reload_chats()])
    }

    fn send_text(&mut self) -> Task<Msg> {
        let e = self.eng();
        let bloqueado = self
            .chat
            .as_ref()
            .and_then(|c| Main::peer_of(&c.key))
            .is_some_and(|pk| self.contacts.get(&pk).is_some_and(|x| x.status != ContactStatus::Ok));
        if bloqueado {
            return Task::none();
        }
        let Some(c) = self.chat.as_mut() else { return Task::none() };
        let texto = c.editor.text();
        let texto = texto.trim().to_string();
        if let Some(id) = c.edit.take() {
            c.editor = text_editor::Content::new();
            if texto.is_empty() {
                return Task::none();
            }
            let key = c.key.clone();
            return done(async move { e.edit_message(&key, &id, &texto).await.map_err(s) });
        }
        if texto.is_empty() {
            return Task::none();
        }
        let reply = c.reply.take();
        c.editor = text_editor::Content::new();
        let key = c.key.clone();
        self.drafts.remove(&key);
        // Link no texto: primeiro a prévia (até ~6 s), depois a mensagem.
        if let Some(url) = crate::linkprev::first_link(&texto).filter(|_| self.settings.link_previews) {
            return Task::perform(
                async move { tokio::task::spawn_blocking(move || crate::linkprev::fetch(&url)).await.ok().flatten() },
                move |lp| Msg::SendTextReady(key.clone(), texto.clone(), reply.clone(), lp),
            );
        }
        Task::batch([done(async move { e.send_text_with(&key, &texto, reply.as_deref()).await.map(|_| ()).map_err(s) }), to_bottom()])
    }

    fn on_key(&mut self, k: Key) -> Task<Msg> {
        if let Some(l) = &self.lock {
            if l.forgot.is_some() {
                return Task::none();
            }
            return match k {
                Key::Digit(c) => self.lock_key(c),
                Key::Backspace => Task::done(Msg::LockBack),
                _ => Task::none(),
            };
        }
        if let Some(Modal::Pin(_)) = &self.modal {
            return match k {
                Key::Digit(c) => self.pin_key(c),
                Key::Backspace => Task::done(Msg::PinBack),
                Key::Escape => {
                    self.modal = None;
                    Task::none()
                }
                _ => Task::none(),
            };
        }
        if k == Key::F11 {
            return if self.in_voice() { Task::done(Msg::CallFull(!self.call_full)) } else { Task::none() };
        }
        if k == Key::Escape {
            if self.call_full && self.menu.is_none() && self.popup.is_none() && self.modal.is_none() {
                return Task::done(Msg::CallFull(false));
            }
            if self.menu.is_none() && self.popup.is_none() && self.modal.is_none() && self.chat.as_ref().is_some_and(|c| c.edit.is_some()) {
                return Task::done(Msg::EditCancel);
            }
            if self.menu.is_some() || self.popup.is_some() {
                self.close_overlays();
            } else if let Some(m) = &self.modal {
                // Não fecha no meio de um envio ou de uma gravação.
                let ocupado = matches!(m, Modal::RecordSound { rec: Some(_), .. })
                    || matches!(m, Modal::Trim(TrimState { busy: true, .. }))
                    || matches!(m, Modal::Crop(CropState { busy: true, .. }));
                if !ocupado {
                    if matches!(m, Modal::Trim(_)) {
                        if let Some(p) = self.sounds.player() {
                            p.stop_track();
                        }
                    }
                    self.modal = None;
                }
            } else if self.rec.is_some() {
                return Task::done(Msg::RecStop { send: false });
            } else if let Some(c) = self.chat.as_mut() {
                c.reply = None;
            }
        }
        Task::none()
    }

    fn lock_key(&mut self, c: char) -> Task<Msg> {
        let Some(l) = &mut self.lock else { return Task::none() };
        if l.wait_until.is_some() || l.buf.len() >= 4 {
            return Task::none();
        }
        l.error = false;
        l.buf.push(c);
        if l.buf.len() < 4 {
            return Task::none();
        }
        let (pin, lock) = (l.buf.clone(), self.pin.clone());
        Task::perform(
            async move { tokio::task::spawn_blocking(move || lock.is_some_and(|x| x.check(&pin))).await.unwrap_or(false) },
            Msg::LockResult,
        )
    }

    fn pin_key(&mut self, c: char) -> Task<Msg> {
        let Some(Modal::Pin(p)) = &mut self.modal else { return Task::none() };
        if p.buf.len() >= 4 {
            return Task::none();
        }
        p.error = false;
        p.buf.push(c);
        if p.buf.len() < 4 {
            return Task::none();
        }
        let v = p.buf.clone();
        if p.purpose == PinPurpose::Create {
            match p.first.take() {
                None => {
                    p.first = Some(v);
                    p.buf.clear();
                    p.msg = "Digite de novo para confirmar".into();
                }
                Some(primeiro) if primeiro == v => {
                    let e = self.eng();
                    return Task::perform(
                        async move { tokio::task::spawn_blocking(move || e.set_pin(&v).map_err(s)).await.map_err(s).and_then(|r| r) },
                        Msg::PinCreated,
                    );
                }
                Some(_) => {
                    p.buf.clear();
                    p.error = true;
                    p.msg = "Os PINs não batem. Comece de novo.".into();
                }
            }
            return Task::none();
        }
        let lock = self.pin.clone();
        Task::perform(
            async move { tokio::task::spawn_blocking(move || lock.is_some_and(|x| x.check(&v))).await.unwrap_or(false) },
            Msg::PinResult,
        )
    }

    fn pin_result(&mut self, ok: bool) -> Task<Msg> {
        let Some(Modal::Pin(p)) = &mut self.modal else { return Task::none() };
        if !ok {
            p.buf.clear();
            p.error = true;
            p.msg = "PIN incorreto. Tente de novo.".into();
            return Task::none();
        }
        match p.purpose {
            PinPurpose::ConfirmChange => {
                self.modal = Some(Modal::Pin(PinPad::new(PinPurpose::Create)));
                Task::none()
            }
            PinPurpose::Remove => {
                self.modal = None;
                match self.engine.remove_pin() {
                    Ok(()) => {
                        self.pin = None;
                        self.toast("PIN desligado. O app abre sem senha.");
                    }
                    Err(e) => self.toast(e.to_string()),
                }
                Task::none()
            }
            PinPurpose::Recovery => {
                self.modal = None;
                self.open_recovery();
                Task::none()
            }
            PinPurpose::Wipe => {
                self.modal = None;
                self.wipe()
            }
            PinPurpose::Create => Task::none(),
        }
    }

    fn forgot_confirm(&mut self) -> Task<Msg> {
        let entrada = match (&self.lock, &self.modal) {
            (Some(LockState { forgot: Some((i, _)), .. }), _) => i.clone(),
            (_, Some(Modal::Forgot { input, .. })) => input.clone(),
            _ => return Task::none(),
        };
        if !self.engine.verify_recovery(entrada.trim()) {
            let erro = Some("Essas palavras não são desta conta. Confira a ordem e a grafia.".to_string());
            if let Some(LockState { forgot: Some(f), .. }) = &mut self.lock {
                f.1 = erro;
            } else if let Some(Modal::Forgot { error, .. }) = &mut self.modal {
                *error = erro;
            }
            return Task::none();
        }
        match self.engine.remove_pin() {
            Ok(()) => {
                self.pin = None;
                self.modal = None;
                if self.lock.is_some() {
                    self.unlock();
                }
                self.toast("PIN removido. Crie outro em Privacidade se quiser.");
            }
            Err(e) => self.toast(e.to_string()),
        }
        Task::none()
    }

    fn open_recovery(&mut self) {
        self.forms.keys = Some(self.engine.export_keys());
        self.forms.reveal_words = false;
        self.forms.reveal_nsec = false;
        self.page = SettingsPage::Recovery;
        self.section = Section::Settings;
    }

    fn wipe(&mut self) -> Task<Msg> {
        let (e, c) = (self.eng(), Arc::clone(&self.calls));
        if let Some(p) = self.sounds.player() {
            p.stop_track();
            p.stop_loop();
        }
        self.rec = None;
        Task::perform(
            async move {
                c.leave_room().await;
                c.hang_up().await;
                e.wipe().await.map_err(s)
            },
            Msg::Wiped,
        )
    }

    fn toggle(&mut self, k: Toggle, on: bool) -> Task<Msg> {
        let e = self.eng();
        if k == Toggle::ForceRelay && on && self.settings.turn.url.trim().is_empty() {
            self.toast("Configure um servidor TURN em Rede e servidores primeiro.");
            return Task::none();
        }
        if k == Toggle::LockScreenshots && !platform::can_protect_window() {
            self.toast("Esconder das capturas só funciona no Windows.");
            return Task::none();
        }
        let t = done(async move {
            e.update_settings(|s| match k {
                Toggle::ReadReceipts => s.read_receipts = on,
                Toggle::Typing => s.typing = on,
                Toggle::Notifications => s.notifications = on,
                Toggle::Sounds => s.sounds = on,
                Toggle::LockScreenshots => s.lock_screenshots = on,
                Toggle::Presence => s.presence = on,
                Toggle::LinkPreviews => s.link_previews = on,
                Toggle::ForceRelay => s.force_relay = on,
            })
            .await
            .map(|_| ())
            .map_err(s)
        });
        if k == Toggle::LockScreenshots {
            self.settings.lock_screenshots = on;
            return Task::batch([t, self.apply_protection()]);
        }
        t
    }

    fn confirmed(&mut self, k: ConfirmKind) -> Task<Msg> {
        let e = self.eng();
        match k {
            ConfirmKind::DeleteChat(key) => {
                if let Err(err) = self.engine.delete_chat(&key) {
                    self.toast(err.to_string());
                } else {
                    self.toast("Conversa apagada");
                }
                self.drafts.remove(&key);
                if self.chat.as_ref().is_some_and(|c| c.key == key) {
                    self.chat = None;
                }
                self.reload_chats()
            }
            ConfirmKind::ClearMedia => {
                match self.engine.clear_media() {
                    Ok(n) => self.toast(format!("{n} arquivos apagados")),
                    Err(err) => self.toast(err.to_string()),
                }
                self.forget_media();
                self.storage = self.engine.storage_usage();
                if let Some(k) = self.open_key() {
                    return self.open_chat(k);
                }
                Task::none()
            }
            ConfirmKind::ClearCalls => {
                if let Err(err) = self.engine.clear_calls() {
                    self.toast(err.to_string());
                }
                self.call_log = self.engine.calls(200);
                Task::none()
            }
            ConfirmKind::Wipe => {
                if self.pin.is_some() {
                    self.modal = Some(Modal::Pin(PinPad::new(PinPurpose::Wipe)));
                    return Task::none();
                }
                self.wipe()
            }
            ConfirmKind::DeleteSound(id) => {
                self.sounds.changed();
                match self.engine.delete_sound(&id) {
                    Ok(()) => self.toast("Som excluído"),
                    Err(err) => self.toast(err.to_string()),
                }
                self.sounds.forget(&id);
                Task::none()
            }
            ConfirmKind::LeaveGroup(lid) => {
                let sair_da_sala = self.room.as_ref().is_some_and(|r| r.lid == lid);
                if self.chat.as_ref().is_some_and(|c| c.key.starts_with(&format!("g:{lid}:"))) {
                    self.chat = None;
                }
                self.section = Section::Chats;
                let c = Arc::clone(&self.calls);
                toast_done(async move {
                    if sair_da_sala {
                        c.leave_room().await;
                    }
                    e.leave_group(&lid).await.map(|_| "Você saiu do grupo".to_string()).map_err(s)
                })
            }
            ConfirmKind::RemoveMember(lid, pk) => {
                toast_done(async move { e.remove_member(&lid, &pk).await.map(|_| "Removido do grupo".to_string()).map_err(s) })
            }
            ConfirmKind::RemoveChannel(lid, id) => {
                let l2 = lid.clone();
                Task::batch([
                    toast_done(async move {
                        e.edit_group(&l2, &[GroupPatch::RemoveChannel(id)]).await.map(|_| "Canal apagado".to_string()).map_err(s)
                    }),
                    Task::done(Msg::OpenGroupInfo(lid)),
                ])
            }
            ConfirmKind::Block(pk) => Task::done(Msg::Block(pk)),
        }
    }

    /// Depois de ligar/desligar câmera ou tela: atualiza a sala ou a chamada.
    fn after_media_change(&self) -> Task<Msg> {
        let c = Arc::clone(&self.calls);
        if self.room.is_some() {
            Task::perform(async move { c.room_info().await }, Msg::RoomUpdated)
        } else {
            Task::perform(async move { c.info().await }, Msg::CallUpdated)
        }
    }

    /// Sala de voz: busca as imagens novas de quem está mandando vídeo e a
    /// prévia da sua câmera.
    fn on_room_media(&mut self) -> Task<Msg> {
        let Some(r) = &self.room else {
            self.room_frames.clear();
            if self.call.is_none() {
                self.local_preview = None;
            }
            return Task::none();
        };
        let camera = r.sharing.as_deref().is_some_and(|s| s.starts_with("Câmera"));
        if camera {
            let desde = self.local_preview.as_ref().map(|(v, _)| *v).unwrap_or(0);
            if let Some((v, w, h, px)) = self.calls.camera_preview(desde) {
                self.local_preview = Some((v, image::Handle::from_rgba(w, h, px)));
            }
        } else {
            self.local_preview = None;
        }
        let com_video: Vec<(String, u64)> = r.peers.iter().filter(|p| p.video).map(|p| (p.pk.clone(), p.frame_version)).collect();
        self.room_frames.retain(|pk, _| com_video.iter().any(|(p, _)| p == pk));
        let mut tarefas = Vec::new();
        for (pk, versao) in com_video {
            let visto = self.room_frames.get(&pk).map(|(v, _)| *v).unwrap_or(0);
            if versao > visto {
                let c = Arc::clone(&self.calls);
                tarefas.push(Task::perform(
                    async move {
                        c.take_room_frame(&pk, visto)
                            .await
                            .map(|(v, img)| (pk, v, iced::widget::image::Handle::from_rgba(img.width, img.height, img.rgba)))
                    },
                    |r| match r {
                        Some((pk, v, h)) => Msg::RoomFrame(pk, v, h),
                        None => Msg::Noop,
                    },
                ));
            }
        }
        Task::batch(tarefas)
    }

    fn on_call_info(&mut self, info: Option<crate::calls::CallInfo>) -> Task<Msg> {
        let antes_tocando = self.call.as_ref().is_some_and(|c| matches!(c.state, CallState::Ringing { .. }));
        let chegou = info.as_ref().map(|i| i.frame_version).unwrap_or(0);
        let visto = self.remote_frame.as_ref().map(|(v, _)| *v).unwrap_or(0);
        if let Some(i) = &info {
            if let CallState::Ringing { video } = i.state {
                if !antes_tocando {
                    // Chamada chegando: avisa mesmo com a janela minimizada.
                    if !self.focused && self.settings.notifications {
                        let quem = self.name_of(&i.peer);
                        platform::notify(quem, if video { "Chamada de vídeo chegando".into() } else { "Chamada de voz chegando".into() });
                    }
                    if let Some(id) = self.window {
                        self.call = info;
                        return iced::window::request_user_attention(id, Some(iced::window::UserAttention::Critical));
                    }
                }
            }
        }
        self.call = info;
        // Sua câmera: prévia pequena, e ligar sozinha na chamada de vídeo.
        let camera_ligada = self.call.as_ref().is_some_and(|c| c.sharing.as_deref().is_some_and(|s| s.starts_with("Câmera")));
        if camera_ligada {
            let desde = self.local_preview.as_ref().map(|(v, _)| *v).unwrap_or(0);
            if let Some((v, w, h, px)) = self.calls.camera_preview(desde) {
                self.local_preview = Some((v, image::Handle::from_rgba(w, h, px)));
            }
        } else {
            self.local_preview = None;
        }
        if self.camera_wanted && self.call.as_ref().is_some_and(|c| matches!(c.state, CallState::Active { .. })) {
            self.camera_wanted = false;
            return Task::done(Msg::ToggleCamera);
        }
        if self.call.is_none() {
            self.camera_wanted = false;
            self.remote_frame = None;
            if self.popup == Some(Popup::CallSounds) {
                self.popup = None;
            }
            return Task::none();
        }
        if chegou > visto {
            let c = Arc::clone(&self.calls);
            return Task::perform(
                async move {
                    c.take_frame(visto)
                        .await
                        .map(|(v, img)| (v, iced::widget::image::Handle::from_rgba(img.width, img.height, img.rgba)))
                },
                |r| match r {
                    Some((v, h)) => Msg::FrameArrived(v, h),
                    None => Msg::Noop,
                },
            );
        }
        Task::none()
    }

    fn on_tick(&mut self) -> Task<Msg> {
        let agora = jm_core::engine::now_ms();
        self.typing.retain(|_, (_, ate)| *ate > agora);
        if self.toast.as_ref().is_some_and(|(_, ate)| *ate < Instant::now()) {
            self.toast = None;
        }
        if let Some(l) = &mut self.lock {
            if l.wait_until.is_some_and(|w| w < Instant::now()) {
                l.wait_until = None;
                l.tries = 3;
                l.msg = "Digite seu PIN".into();
            }
        }
        self.track = self.sounds.player().and_then(|p| p.track());
        let mut tarefas = Vec::new();
        if let Some(r) = &mut self.rec {
            r.levels.push(r.rec.level());
            if r.levels.len() > 120 {
                r.levels.remove(0);
            }
        }
        if self.rec.as_ref().is_some_and(|r| r.rec.elapsed() >= VOICE_MAX_SECS) {
            tarefas.push(Task::done(Msg::RecStop { send: true }));
        }
        if let Some(Modal::RecordSound { rec: Some(r), levels }) = &mut self.modal {
            levels.push(r.level());
            if levels.len() > 120 {
                levels.remove(0);
            }
            if r.elapsed() >= SOUND_REC_MAX {
                tarefas.push(Task::done(Msg::SbRecStop));
            }
        }
        if self.call_full && !self.in_voice() {
            // A chamada acabou: volta para a janela normal.
            tarefas.push(Task::done(Msg::CallFull(false)));
        }
        if self.focused {
            let faltando: Vec<String> = self
                .chat
                .as_ref()
                .map(|c| {
                    c.messages
                        .iter()
                        .rev()
                        .take(40)
                        .filter_map(|m| match &m.body {
                            MessageBody::Media(x) if x.ready && is_gif_mime(&x.mime) && !self.anims.contains_key(&x.mid) && !self.anim_loading.contains(&x.mid) => Some(x.mid.clone()),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            for mid in faltando {
                tarefas.push(self.anim_from_media(mid));
            }
        }
        if self.update.is_none() && self.update_prog.is_none() && self.update_at.elapsed() >= Duration::from_secs(6 * 3600) {
            self.update_at = Instant::now();
            tarefas.push(Task::perform(
                async { tokio::task::spawn_blocking(crate::atualizar::procurar).await.map_err(|e| e.to_string()).and_then(|r| r) },
                |r| Msg::UpdateFound(r.map_err(|e| {
                    tracing::info!("atualização: {e}");
                    e
                })),
            ));
        }
        if crate::tray::SUPPORTED {
            crate::tray::ensure();
            for ev in crate::tray::poll() {
                match ev {
                    crate::tray::Ev::Show => {
                        if let Some(id) = self.window {
                            tarefas.push(iced::window::set_mode(id, iced::window::Mode::Windowed));
                            tarefas.push(iced::window::gain_focus(id));
                        }
                    }
                    crate::tray::Ev::Quit => tarefas.push(iced::exit()),
                }
            }
        }
        for a in crate::notif::poll() {
            match a {
                crate::notif::Action::Reply(chat, texto) => {
                    let e = self.eng();
                    let _ = self.engine.mark_read(&chat);
                    tarefas.push(done(async move { e.send_text_with(&chat, &texto, None).await.map(|_| ()).map_err(s) }));
                    tarefas.push(self.reload_chats());
                }
                crate::notif::Action::Read(chat) => {
                    let _ = self.engine.mark_read(&chat);
                    tarefas.push(self.reload_chats());
                }
                crate::notif::Action::Open(chat) => {
                    if let Some(id) = self.window {
                        tarefas.push(iced::window::set_mode(id, iced::window::Mode::Windowed));
                        tarefas.push(iced::window::gain_focus(id));
                    }
                    tarefas.push(Task::done(Msg::OpenChat(chat)));
                }
            }
        }
        if self.evt_at.elapsed() >= Duration::from_secs(30) {
            self.evt_at = Instant::now();
            for (chat, titulo, at) in self.engine.due_event_reminders(15 * 60_000) {
                let min = ((at - jm_core::engine::now_ms()) / 60_000).max(0);
                let quando = if min <= 1 { "agora".to_string() } else { format!("em {min} minutos") };
                crate::notif::message(format!("📅 {titulo}"), format!("Começa {quando}."), Some(chat), false);
            }
        }
        if self.dev_at.is_none_or(|t| t.elapsed() >= Duration::from_secs(6 * 3600)) {
            self.dev_at = Some(Instant::now());
            let e = self.eng();
            let (nome, sistema) = crate::platform::device_name();
            tarefas.push(Task::perform(async move { e.announce_device(&nome, &sistema).await }, |_| Msg::Noop));
        }
        if self.sync_at.elapsed() >= Duration::from_secs(60) {
            self.sync_at = Instant::now();
            let e = self.eng();
            tarefas.push(Task::perform(
                async move {
                    e.sync_request().await;
                    e.sync_push(false).await;
                },
                |_| Msg::Noop,
            ));
        }
        let ausente = !self.focused || self.lock.is_some();
        if self.pres_sent.is_none_or(|(a, t)| a != ausente || t.elapsed() >= Duration::from_secs(60)) {
            let forca = self.pres_sent.is_none_or(|(a, _)| a != ausente);
            self.pres_sent = Some((ausente, Instant::now()));
            let e = self.eng();
            tarefas.push(Task::perform(async move { e.set_presence(ausente, forca).await }, |_| Msg::Noop));
        }
        let vendo_perfil = self.section == Section::Settings && self.page == SettingsPage::Profile;
        if (self.listen_share || vendo_perfil) && !self.np_polling && self.np_at.elapsed() >= Duration::from_secs(5) {
            self.np_polling = true;
            self.np_at = Instant::now();
            tarefas.push(Task::perform(
                async { tokio::task::spawn_blocking(crate::nowplaying::current).await.ok().flatten() },
                Msg::NowPlaying,
            ));
        }
        if let Some(t) = &mut self.mic_test {
            while let Ok(q) = t.rx.try_recv() {
                if let Some(e) = &t.eco {
                    e.push(&q);
                }
            }
            t.levels.push(t.cap.level());
            if t.levels.len() > 120 {
                t.levels.remove(0);
            }
        }
        if self.call.is_some() {
            tarefas.push(self.poll_call());
        }
        if self.room.is_some() || self.joining_room {
            tarefas.push(self.poll_room());
        }
        Task::batch(tarefas)
    }

    fn on_engine(&mut self, ev: EngineEvent) -> Task<Msg> {
        match ev {
            EngineEvent::Pins { chat } => {
                if let Some(c) = self.chat.as_mut().filter(|c| c.key == chat) {
                    c.pins = self.engine.pins(&chat);
                }
                Task::none()
            }
            EngineEvent::Presence { .. } => Task::none(),
            EngineEvent::Devices => Task::none(),
            EngineEvent::RemoteLogout => {
                self.toast("Outro aparelho desta conta desconectou este computador.");
                self.wipe()
            }
            EngineEvent::Synced { contacts, groups } => {
                self.toast(format!("Seu outro aparelho mandou {contacts} contatos e {groups} grupos"));
                self.reload_contacts();
                Task::batch([self.reload_groups(), self.reload_chats()])
            }
            EngineEvent::Message { chat, message } => {
                if let Some(c) = self.chat.as_mut().filter(|c| c.key == chat) {
                    if !c.messages.iter().any(|x| x.id == message.id) {
                        let fim = c.messages.last().is_none_or(|l| l.ts <= message.ts);
                        c.messages.push(*message);
                        if !fim {
                            c.messages.sort_by_key(|x| x.ts);
                        }
                    }
                }
                self.reload_chats()
            }
            EngineEvent::MessageChanged { chat, message } => {
                if let Some(c) = self.chat.as_mut().filter(|c| c.key == chat) {
                    if let Some(x) = c.messages.iter_mut().find(|x| x.id == message.id) {
                        *x = *message;
                    }
                }
                Task::none()
            }
            EngineEvent::MessageUpdated { chat, id, status } => {
                if let Some(c) = self.chat.as_mut().filter(|c| c.key == chat) {
                    if let Some(x) = c.messages.iter_mut().find(|x| x.id == id) {
                        x.status = Some(status);
                    }
                }
                Task::none()
            }
            EngineEvent::MessageDeleted { chat, id } => {
                if let Some(c) = self.chat.as_mut().filter(|c| c.key == chat) {
                    c.messages.retain(|x| x.id != id);
                    if c.reply.as_deref() == Some(id.as_str()) {
                        c.reply = None;
                    }
                }
                self.reload_chats()
            }
            EngineEvent::Incoming { chat, message, visible } => {
                self.typing.remove(&chat);
                self.on_incoming(&chat, &message, visible);
                Task::none()
            }
            EngineEvent::Progress { chat, id, done, total } => {
                if let Some(c) = self.chat.as_mut().filter(|c| c.key == chat) {
                    if done >= total {
                        c.progress.remove(&id);
                    } else {
                        c.progress.insert(id, (done, total));
                    }
                }
                Task::none()
            }
            EngineEvent::ChatsChanged => self.reload_chats(),
            EngineEvent::ChatGone { chat } => {
                if self.chat.as_ref().is_some_and(|c| c.key == chat) {
                    self.chat = None;
                }
                self.reload_chats()
            }
            EngineEvent::ContactChanged { .. } => {
                self.reload_contacts();
                self.reload_chats()
            }
            EngineEvent::Typing { chat, who } => {
                self.typing.insert(chat, (who, jm_core::engine::now_ms() + 6000));
                Task::none()
            }
            EngineEvent::Relays(st) => {
                self.relays = st;
                Task::none()
            }
            EngineEvent::GroupsChanged => Task::batch([self.reload_groups(), self.reload_chats()]),
            EngineEvent::CallSignal { from, signal } => {
                let c = Arc::clone(&self.calls);
                let sala = signal.is_room();
                Task::perform(
                    async move {
                        c.on_signal(&from, signal).await;
                        if sala {
                            Msg::RoomUpdated(c.room_info().await)
                        } else {
                            Msg::CallUpdated(c.info().await)
                        }
                    },
                    |m| m,
                )
            }
            EngineEvent::MissedCall { from, video, .. } => {
                if !self.focused && self.settings.notifications {
                    let quem = self.name_of(&from);
                    platform::notify(quem, if video { "Chamada de vídeo perdida".into() } else { "Chamada de voz perdida".into() });
                }
                self.reload_chats()
            }
            EngineEvent::Room { lid, ch, members } => {
                self.rooms.insert((lid.clone(), ch.clone()), members.clone());
                let c = Arc::clone(&self.calls);
                Task::perform(
                    async move {
                        c.on_room_update(&lid, &ch, &members).await;
                        c.room_info().await
                    },
                    Msg::RoomUpdated,
                )
            }
            EngineEvent::CallsChanged => {
                self.call_log = self.engine.calls(200);
                Task::none()
            }
            EngineEvent::Listening { .. } => Task::none(),
            EngineEvent::ProfileChanged => {
                self.me = self.engine.me();
                self.refresh_qr();
                Task::none()
            }
            EngineEvent::SettingsChanged => {
                self.settings = self.engine.settings();
                Task::none()
            }
        }
    }

    /// Mensagem de outra pessoa: som, toque do soundboard e notificação.
    fn on_incoming(&mut self, chat: &str, m: &jm_core::engine::Message, visible: bool) {
        let mencao = matches!(&m.body, MessageBody::Text(t) if self.mentions_me(chat, t, &m.author));
        let silenciada = !mencao && self.summary(chat).is_some_and(|c| c.muted)
            || chat
                .strip_prefix("g:")
                .and_then(|r| r.split(':').next())
                .and_then(|lid| self.group(lid))
                .is_some_and(|g| g.muted && !mencao);
        // Som do soundboard chegando na conversa aberta: toca na hora, como no celular.
        if visible {
            match &m.body {
                MessageBody::Sound { id } => {
                    self.sounds.play(id);
                }
                MessageBody::Media(x) if x.sb.is_some() && x.ready => {
                    if let Some(b) = self.engine.media_bytes(&x.mid) {
                        self.sounds.play_bytes(&format!("m:{}", x.mid), &b);
                    }
                }
                _ => {}
            }
            return;
        }
        if silenciada || m.body.is_system() {
            return;
        }
        // Não perturbe: nada de som nem aviso (as mensagens chegam normalmente).
        if self.me.status.as_ref().and_then(|x| x.clean(jm_core::engine::now_ms())).is_some_and(|x| x.dnd) {
            return;
        }
        if self.settings.sounds {
            if let Some(p) = self.sounds.player() {
                p.beep(jm_media::player::Tone::Blip);
            }
        }
        if self.settings.notifications && (!self.focused || self.lock.is_some()) {
            let quem = match jm_core::messages::ChatKey::parse(chat) {
                Some(jm_core::messages::ChatKey::Channel { lid, .. }) => {
                    let g = self.group(&lid).map(|g| g.meta.name.clone()).unwrap_or_default();
                    format!("{} · {g}", self.first_name(&m.author))
                }
                _ => self.name_of(&m.author),
            };
            let (titulo, corpo) = match self.settings.notif_preview {
                NotifPreview::Full if self.lock.is_none() => (quem, self.engine.preview(m)),
                NotifPreview::None => ("J-Menseger".to_string(), "Nova mensagem".to_string()),
                _ => (quem, "Nova mensagem".to_string()),
            };
            // Com PIN ligado, nada de responder pela notificação (seria um atalho por cima do bloqueio).
            let livre = self.pin.is_none();
            crate::notif::message(titulo, corpo, livre.then(|| chat.to_string()), livre && self.can_write(chat));
        }
    }
}

/// Abre um menu de contexto na posição do ponteiro. A posição chega por
/// outro caminho (a escuta de eventos da janela), que pode vir um instante
/// depois do clique: espera um quadro antes de ler.
fn open_menu(kind: CtxKind) -> Task<Msg> {
    Task::perform(tokio::time::sleep(Duration::from_millis(30)), move |_| Msg::MenuNow(kind.clone()))
}

/// Nome mostrado para "usar o dispositivo padrão do sistema".
pub const PADRAO: &str = "Padrão do sistema";

/// Leva a lista de mensagens para a mais nova. A lista é ancorada embaixo,
/// então o deslocamento conta a partir do fim: zero é a última mensagem.
impl Main {
    fn toast_t(&mut self, s: &str) -> Task<Msg> {
        self.toast(s);
        Task::none()
    }

    /// Rola a lista até perto da mensagem (a altura de cada uma varia, então
    /// é uma estimativa pela posição; o destaque mostra qual é).
    fn scroll_to_msg(&self, id: &str) -> Task<Msg> {
        let Some(c) = &self.chat else { return Task::none() };
        let Some(i) = c.messages.iter().position(|m| m.id == id) else { return Task::none() };
        let n = c.messages.len().max(2) - 1;
        let y = 1.0 - (i as f32 / n as f32);
        iced::widget::operation::snap_to(MESSAGES_ID, iced::widget::operation::RelativeOffset { x: 0.0, y: y.clamp(0.0, 1.0) })
    }
}

fn to_bottom() -> Task<Msg> {
    iced::widget::operation::snap_to(MESSAGES_ID, iced::widget::operation::RelativeOffset::START)
}

/// Testes automáticos da interface: as janelas de escolher arquivo do sistema
/// não existem no ambiente de teste, então F9 abre o arquivo indicado em
/// `J_MENSEGER_TESTE_ARQUIVO` como se a pessoa o tivesse escolhido. Não existe
/// na versão final.
#[cfg(debug_assertions)]
fn test_hook() -> Task<Msg> {
    let (Ok(alvo), Ok(p)) = (std::env::var("J_MENSEGER_TESTE_ALVO"), std::env::var("J_MENSEGER_TESTE_ARQUIVO")) else {
        return Task::none();
    };
    let p = std::path::PathBuf::from(p);
    match alvo.as_str() {
        "som" => Task::perform(platform::audio_from_path(p), Msg::SbDecoded),
        "perfil" => Task::perform(platform::crop_from_path(CropTarget::Me, p), Msg::PhotoForCrop),
        "banner" => Task::perform(platform::crop_from_path(CropTarget::Banner, p), Msg::PhotoForCrop),
        "foto" => Task::perform(async move { platform::photo_from_path(p).await.map(Some) }, Msg::PhotoPicked),
        _ => Task::none(),
    }
}

#[cfg(not(debug_assertions))]
fn test_hook() -> Task<Msg> {
    Task::none()
}

/// O texto que vai no "Convidar amigos" (o mesmo do celular).
pub fn invite_text(npub: &str) -> String {
    format!("Fale comigo no J-Menseger (mensagens cifradas de ponta a ponta).\nMeu código: {npub}\n\nNo app: Adicionar contato → cole este código.")
}
