//! Teyvin Compagnon : écran du compagnon, aux couleurs de Teyvin.
//!
//! Remplace l'écran d'Irminsul (src/app.rs) en gardant son moteur : même démarrage (vérification des mises à
//! jour, lecture du lien de vœux, capture), mêmes messages vers le moniteur. L'écran ne montre que ce qui
//! sert : une bannière de nuit avec l'état général, quatre étapes (compagnon prêt, jeu connecté, compte lu,
//! Teyvin à jour), le démarrage avec Windows et un bouton vers Teyvin. En français si Windows l'est, en
//! anglais sinon.

use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use egui::{
    Align2, Button, Color32, Context, CornerRadius, FontId, Id, OpenUrl, PointerButton, Rect,
    RichText, Sense, Stroke, StrokeKind, Vec2, ViewportCommand, pos2, vec2,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};

use crate::monitor::Monitor;
use crate::player_data::ExportSettings;
use crate::update::check_for_app_update;
use crate::{AppState, Message, ReloadHandle, State, TracingLevel, capture, open_log_dir, wish};

/// Textes de l'écran, en français et en anglais (termes officiels du jeu en anglais).
/// `{v}` est remplacé par la version.
struct Texts {
    url: &'static str,
    brand: &'static str,
    step_ready: &'static str,
    step_launch: &'static str,
    step_game: &'static str,
    step_read: &'static str,
    step_teyvin: &'static str,
    starting_detail: &'static str,
    starting: &'static str,
    checking_updates: &'static str,
    loading_data: &'static str,
    update: &'static str,
    later: &'static str,
    update_available: &'static str,
    update_ready: &'static str,
    updating: &'static str,
    will_restart: &'static str,
    updated: &'static str,
    restarting: &'static str,
    capture_on: &'static str,
    capture_off: &'static str,
    waiting: &'static str,
    open_teyvin_detail: &'static str,
    teyvin_soon: &'static str,
    login_read: &'static str,
    with_wishes: &'static str,
    chars_inventory: &'static str,
    after_farm: &'static str,
    capture_stopped: &'static str,
    capture_stopped_hint: &'static str,
    start_capture: &'static str,
    waiting_game: &'static str,
    waiting_game_hint: &'static str,
    reading: &'static str,
    reading_hint: &'static str,
    read_hint: &'static str,
    all_done: &'static str,
    all_done_hint: &'static str,
    open_teyvin: &'static str,
    minimize: &'static str,
    logs: &'static str,
    credit: &'static str,
}

const FR: Texts = Texts {
    url: "https://teyvin.app/compte",
    brand: "TEYVIN COMPANION",
    step_ready: "Compagnon prêt",
    step_launch: "Lance le jeu",
    step_game: "Jeu connecté",
    step_read: "Compte lu",
    step_teyvin: "Teyvin à jour",
    starting_detail: "démarrage…",
    starting: "Démarrage…",
    checking_updates: "Vérification des mises à jour du compagnon.",
    loading_data: "Chargement des données du jeu.",
    update: "Mettre à jour",
    later: "Plus tard",
    update_available: "Mise à jour disponible",
    update_ready: "La version {v} du compagnon est prête.",
    updating: "Mise à jour en cours…",
    will_restart: "Le compagnon redémarrera tout seul.",
    updated: "Mis à jour",
    restarting: "Redémarrage du compagnon…",
    capture_on: "capture active",
    capture_off: "capture arrêtée",
    waiting: "en attente…",
    open_teyvin_detail: "ouvre Teyvin",
    teyvin_soon: "recharge sa page…",
    login_read: "connexion lue",
    with_wishes: "vœux compris",
    chars_inventory: "persos et inventaire",
    after_farm: "après un farm : reconnecte-toi",
    capture_stopped: "Capture arrêtée",
    capture_stopped_hint: "Relance la capture pour que le compagnon lise ton compte.",
    start_capture: "Démarrer la capture",
    waiting_game: "En attente du jeu",
    waiting_game_hint: "Lance Genshin et connecte-toi. Déjà en jeu ? Reviens à l'écran de connexion.",
    reading: "Lecture du compte…",
    reading_hint: "Encore un instant.",
    read_hint: "Teyvin se met à jour : recharge sa page, ou attends une minute.",
    all_done: "Tout est à jour",
    all_done_hint: "Le compte est lu à la connexion : après un farm, reconnecte-toi pour actualiser Teyvin.",
    open_teyvin: "Ouvrir Teyvin",
    minimize: "Réduire",
    logs: "Journaux",
    credit: "v{v} · basé sur Irminsul de Konkers",
};

const EN: Texts = Texts {
    url: "https://teyvin.app/en/account",
    brand: "TEYVIN COMPANION",
    step_ready: "Companion ready",
    step_launch: "Launch the game",
    step_game: "Game connected",
    step_read: "Account read",
    step_teyvin: "Teyvin up to date",
    starting_detail: "starting…",
    starting: "Starting…",
    checking_updates: "Checking for companion updates.",
    loading_data: "Loading game data.",
    update: "Update",
    later: "Later",
    update_available: "Update available",
    update_ready: "Companion version {v} is ready.",
    updating: "Updating…",
    will_restart: "The companion will restart on its own.",
    updated: "Updated",
    restarting: "Restarting the companion…",
    capture_on: "capture on",
    capture_off: "capture stopped",
    waiting: "waiting…",
    open_teyvin_detail: "open Teyvin",
    teyvin_soon: "reload its page…",
    login_read: "login read",
    with_wishes: "wishes included",
    chars_inventory: "characters and inventory",
    after_farm: "after farming: log in again",
    capture_stopped: "Capture stopped",
    capture_stopped_hint: "Start the capture again so the companion can read your account.",
    start_capture: "Start capture",
    waiting_game: "Waiting for the game",
    waiting_game_hint: "Launch Genshin and log in. Already in game? Go back to the login screen.",
    reading: "Reading account…",
    reading_hint: "Just a moment.",
    read_hint: "Teyvin is updating: reload its page, or wait a minute.",
    all_done: "All up to date",
    all_done_hint: "Your account is read when you log in: after farming, log in again to refresh Teyvin.",
    open_teyvin: "Open Teyvin",
    minimize: "Minimize",
    logs: "Logs",
    credit: "v{v} · based on Irminsul by Konkers",
};

/// Textes dans la langue de Windows : français si elle l'est, anglais sinon.
/// La variable d'environnement `TEYVIN_LANG` (fr, en) force la langue, pour vérifier les deux.
fn tx() -> &'static Texts {
    static FRENCH: OnceLock<bool> = OnceLock::new();
    let french = *FRENCH.get_or_init(|| match std::env::var("TEYVIN_LANG") {
        Ok(lang) => lang.to_ascii_lowercase().starts_with("fr"),
        Err(_) => windows_is_french(),
    });
    if french { &FR } else { &EN }
}

/// Langue de l'interface de Windows (pas celle des formats de date) : sa langue principale, LANG_FRENCH = 0x0C.
#[cfg(windows)]
fn windows_is_french() -> bool {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    // SAFETY : fonction Windows sans argument, toujours disponible
    (unsafe { GetUserDefaultUILanguage() } & 0x3FF) == 0x0C
}

#[cfg(not(windows))]
fn windows_is_french() -> bool {
    std::env::var("LANG").is_ok_and(|lang| lang.starts_with("fr"))
}

// Couleurs de Teyvin (DA « Irminsul nuit »)
const BG: Color32 = Color32::from_rgb(6, 9, 11);
const INK: Color32 = Color32::from_rgb(233, 239, 241);
const MUTED: Color32 = Color32::from_rgb(141, 154, 159);
const TEAL: Color32 = Color32::from_rgb(45, 212, 191);
const TEAL_LIGHT: Color32 = Color32::from_rgb(153, 246, 228);
const ON_TEAL: Color32 = Color32::from_rgb(3, 37, 33);
const WARN: Color32 = Color32::from_rgb(251, 154, 108);

const BANNER_HEIGHT: f32 = 250.0;
const MARGIN: f32 = 22.0;

/// Réglages gardés entre deux lancements. Les réglages d'export restent ceux d'Irminsul par défaut :
/// le relais les reprend pour produire l'export lu par Teyvin.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedAppState {
    export_settings: ExportSettings,
    #[serde(default)]
    tracing_level: TracingLevel,
}

impl Default for SavedAppState {
    fn default() -> Self {
        Self {
            export_settings: ExportSettings {
                include_characters: true,
                include_artifacts: true,
                include_weapons: true,
                include_materials: true,
                fake_initialize_4th_line: false,
                min_character_level: 1,
                min_character_ascension: 0,
                min_character_constellation: 0,
                min_artifact_level: 0,
                min_artifact_rarity: 5,
                min_weapon_level: 1,
                min_weapon_refinement: 0,
                min_weapon_ascension: 0,
                min_weapon_rarity: 3,
            },
            tracing_level: Default::default(),
        }
    }
}

pub struct IrminsulApp {
    ui_message_tx: mpsc::UnboundedSender<Message>,
    state_rx: watch::Receiver<AppState>,
    saved_state: SavedAppState,
    download_acknowledged: bool,
    restarting: bool,
}

/// Même démarrage qu'Irminsul : mises à jour, lien des vœux, moniteur de capture (dans un runtime tokio).
fn start_async_runtime(
    egui_ctx: Context,
    capture_backend: capture::BackendType,
) -> (mpsc::UnboundedSender<Message>, watch::Receiver<AppState>) {
    let (ui_message_tx, mut ui_message_rx) = mpsc::unbounded_channel::<Message>();
    let (state_tx, state_rx) = watch::channel(AppState::new());
    let (wish_url_tx, wish_url_rx) = watch::channel(None);
    crate::relay::start(wish_url_rx);
    // Pas de journal des paquets bruts (outil de débogage d'Irminsul)
    let (_log_packets_tx, log_packets_rx) = watch::channel(false);
    let mut updater_state_rx = state_rx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = check_for_app_update(&state_tx, &mut ui_message_rx).await {
                tracing::error!("error checking for update: {e}");
            }

            tokio::spawn(async move {
                let Ok(mut wish) = wish::Wish::new(wish_url_tx).await else {
                    tracing::error!("Failed to create new wish monitor");
                    return;
                };
                if let Err(e) = wish.monitor().await {
                    tracing::error!("Error monitoring for wishes: {e}");
                }
            });

            tokio::spawn(async move {
                loop {
                    let _ = updater_state_rx.changed().await;
                    egui_ctx.request_repaint();
                }
            });

            let monitor = match Monitor::new(state_tx, ui_message_rx, log_packets_rx, capture_backend).await {
                Ok(monitor) => monitor,
                Err(e) => {
                    tracing::error!("error loading monitor task: {e}");
                    return;
                }
            };
            monitor.run().await;
        });
    });
    (ui_message_tx, state_rx)
}

impl IrminsulApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        mut tracing_reload_handle: ReloadHandle,
        capture_backend: capture::BackendType,
    ) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        egui_material_icons::initialize(&cc.egui_ctx);

        let saved_state: SavedAppState = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        tracing_reload_handle.set_filter(saved_state.tracing_level.get_filter());

        let (ui_message_tx, state_rx) = start_async_runtime(cc.egui_ctx.clone(), capture_backend);
        // La capture démarre toujours seule : c'est tout l'intérêt du compagnon
        if let Err(e) = ui_message_tx.send(Message::StartCapture) {
            tracing::error!("Failed to send auto start message: {e}");
        }

        Self {
            ui_message_tx,
            state_rx,
            saved_state,
            download_acknowledged: false,
            restarting: false,
        }
    }

    fn send(&self, msg: Message) {
        if let Err(e) = self.ui_message_tx.send(msg) {
            tracing::error!("Unable to send UI message: {e}");
        }
    }

    /// Relance le compagnon (après une mise à jour) et ferme cette instance.
    fn restart(&mut self, ctx: &Context) {
        if self.restarting {
            return;
        }
        match std::env::current_exe() {
            Ok(exe) => {
                let _ = std::process::Command::new(exe).spawn();
            }
            Err(e) => tracing::error!("unable to find the executable to restart it: {e}"),
        }
        ctx.send_viewport_cmd(ViewportCommand::Close);
        self.restarting = true;
    }
}

/// Une étape : faite, en cours (orange) ou à venir (grisée).
#[derive(Clone, Copy, PartialEq)]
enum Step {
    Done,
    Current,
    Todo,
}

struct View {
    title: String,
    subtitle: String,
    warn: bool,
    /// Coche à côté du titre (compte lu, tout est à jour).
    check: bool,
    steps: [(Step, &'static str, String); 4],
    /// Bouton d'action à la place de « Réduire » (démarrer la capture, mettre à jour…).
    action: Option<(&'static str, Message)>,
    secondary: Option<(&'static str, Message)>,
    spinner: bool,
}

fn view(state: &AppState) -> View {
    let t = tx();
    let wait = |title: &str, subtitle: &str| View {
        title: title.to_string(),
        subtitle: subtitle.to_string(),
        warn: false,
        check: false,
        steps: [
            (Step::Current, t.step_ready, t.starting_detail.into()),
            (Step::Todo, t.step_launch, "—".into()),
            (Step::Todo, t.step_read, "—".into()),
            (Step::Todo, t.step_teyvin, "—".into()),
        ],
        action: None,
        secondary: None,
        spinner: true,
    };

    match &state.state {
        State::Starting | State::CheckingForUpdate | State::CheckingForData | State::Downloading => {
            wait(t.starting, t.checking_updates)
        }
        State::WaitingForDownloadConfirmation(_) => wait(t.starting, t.loading_data),
        State::WaitingForUpdateConfirmation(version) => View {
            spinner: false,
            action: Some((t.update, Message::UpdateAcknowledged)),
            secondary: Some((t.later, Message::UpdateCanceled)),
            ..wait(t.update_available, &t.update_ready.replace("{v}", &version.to_string()))
        },
        State::Updating => wait(t.updating, t.will_restart),
        State::Updated { .. } => wait(t.updated, t.restarting),
        State::Main => {
            let u = &state.updated;
            let game = u.items_updated.or(u.characters_updated);
            let read = u.items_updated.is_some() && u.characters_updated.is_some();
            let exported = crate::relay::exported_at();
            let synced = crate::relay::synced_at();
            let teyvin = matches!((exported, synced), (Some(e), Some(s)) if s >= e);

            let capture = if state.capturing { Step::Done } else { Step::Current };
            let mut steps = [
                (capture, t.step_ready, if state.capturing { t.capture_on.into() } else { t.capture_off.into() }),
                (Step::Todo, t.step_launch, "—".into()),
                (Step::Todo, t.step_read, "—".into()),
                (Step::Todo, t.step_teyvin, t.open_teyvin_detail.into()),
            ];
            if state.capturing {
                steps[1] = match game {
                    Some(_) => (Step::Done, t.step_game, t.login_read.into()),
                    None => (Step::Current, t.step_launch, t.waiting.into()),
                };
            }
            if game.is_some() {
                let detail = if crate::relay::has_wish_url() { t.with_wishes } else { t.chars_inventory };
                steps[2] = (if read { Step::Done } else { Step::Current }, t.step_read, detail.into());
            }
            if read {
                // Rappel utile plutôt qu'une heure : le compte n'est lu qu'à la connexion au jeu
                steps[3] = if teyvin {
                    (Step::Done, t.step_teyvin, t.after_farm.into())
                } else {
                    (Step::Current, t.step_teyvin, t.teyvin_soon.into())
                };
            }

            let (title, subtitle, warn, action) = if !state.capturing {
                (t.capture_stopped, t.capture_stopped_hint, true, Some((t.start_capture, Message::StartCapture)))
            } else if game.is_none() {
                (t.waiting_game, t.waiting_game_hint, true, None)
            } else if !read {
                (t.reading, t.reading_hint, false, None)
            } else if !teyvin {
                (t.step_read, t.read_hint, false, None)
            } else {
                (t.all_done, t.all_done_hint, false, None)
            };
            View {
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                warn,
                check: read,
                steps,
                action,
                secondary: None,
                // Jusqu'à ce que Teyvin ait récupéré le compte (il interroge le relais chaque minute)
                spinner: game.is_some() && !teyvin,
            }
        }
    }
}

impl eframe::App for IrminsulApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.saved_state);
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // L'état de Teyvin (lu par le relais) ne réveille pas l'écran : on le redessine chaque seconde
        ctx.request_repaint_after(Duration::from_secs(1));
        ctx.style_mut(|style| {
            style.interaction.selectable_labels = false;
            style.visuals.override_text_color = Some(INK);
        });

        let state = self.state_rx.borrow_and_update().clone();
        match state.state {
            State::WaitingForDownloadConfirmation(_) if !self.download_acknowledged => {
                self.download_acknowledged = true;
                self.send(Message::DownloadAcknowledged);
            }
            State::Updated { .. } => self.restart(ctx),
            _ => {}
        }
        let v = view(&state);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                self.banner(ui, full, &v);
                self.title_bar(ui, full);
                self.steps(ui, full, &v);
                self.footer(ui, full, &v);
            });
    }
}

impl IrminsulApp {
    /// Paysage de nuit de Teyvin, fondu vers le fond, avec l'état général en grand.
    fn banner(&self, ui: &mut egui::Ui, full: Rect, v: &View) {
        let rect = Rect::from_min_size(full.min, vec2(full.width(), BANNER_HEIGHT));
        // Image 16:9 recadrée en bandeau (on garde la lune et le château)
        let visible = (rect.height() / rect.width()) / (810.0 / 1440.0);
        egui::Image::new(egui::include_image!("../assets/teyvin-nuit.webp"))
            .uv(Rect::from_min_max(pos2(0.0, 0.1), pos2(1.0, 0.1 + visible)))
            .tint(if v.warn { Color32::from_gray(150) } else { Color32::WHITE })
            .paint_at(ui, rect);

        // Fondu vers le fond, par bandes
        let painter = ui.painter();
        let bands = 40;
        for i in 0..bands {
            let t = i as f32 / bands as f32;
            let y0 = rect.top() + rect.height() * (0.35 + 0.65 * t);
            let y1 = rect.top() + rect.height() * (0.35 + 0.65 * (t + 1.0 / bands as f32));
            let alpha = (t * t * 255.0) as u8;
            painter.rect_filled(
                Rect::from_x_y_ranges(rect.x_range(), y0..=y1 + 1.0),
                0.0,
                Color32::from_rgba_unmultiplied(BG.r(), BG.g(), BG.b(), alpha),
            );
        }

        // Voile sombre derrière le texte (en bas à gauche), pour la lisibilité : dégradé de gauche à droite et du
        // haut vers le bas, le paysage reste visible ailleurs
        let xs = [(rect.left(), 1.0), (rect.left() + rect.width() * 0.35, 0.85), (rect.left() + rect.width() * 0.7, 0.0)];
        let ys = [(rect.top() + rect.height() * 0.38, 0.0), (rect.top() + rect.height() * 0.62, 1.0), (rect.bottom(), 1.0)];
        let mut veil = egui::Mesh::default();
        for (y, ay) in ys {
            for (x, ax) in xs {
                let alpha = (0.8 * ax * ay * 255.0) as u8;
                veil.colored_vertex(pos2(x, y), Color32::from_rgba_unmultiplied(BG.r(), BG.g(), BG.b(), alpha));
            }
        }
        for row in 0..2 {
            for col in 0..2 {
                let i = row * 3 + col;
                veil.add_triangle(i, i + 1, i + 3);
                veil.add_triangle(i + 1, i + 4, i + 3);
            }
        }
        painter.add(veil);

        let x = rect.left() + MARGIN;
        let y = rect.bottom() - MARGIN;
        painter.text(
            pos2(x, y - 62.0),
            Align2::LEFT_BOTTOM,
            tx().brand,
            FontId::proportional(11.0),
            TEAL_LIGHT,
        );
        let title = painter.text(pos2(x, y - 26.0), Align2::LEFT_BOTTOM, &v.title, FontId::proportional(30.0), INK);
        if v.check {
            painter.text(
                pos2(title.right() + 10.0, title.center().y),
                Align2::LEFT_CENTER,
                egui_material_icons::icons::ICON_CHECK_CIRCLE,
                FontId::proportional(26.0),
                TEAL,
            );
        }
        painter.text(
            pos2(x, y),
            Align2::LEFT_BOTTOM,
            &v.subtitle,
            FontId::proportional(14.0),
            if v.warn { WARN } else { MUTED },
        );
        if v.spinner {
            let r = Rect::from_center_size(pos2(rect.right() - MARGIN - 10.0, y - 10.0), Vec2::splat(20.0));
            ui.put(r, egui::Spinner::new().color(TEAL));
        }
    }

    /// Barre de titre invisible (déplacer la fenêtre) avec réduire et fermer.
    fn title_bar(&self, ui: &mut egui::Ui, full: Rect) {
        let size = 30.0;
        let close = Rect::from_min_size(pos2(full.right() - size - 8.0, full.top() + 6.0), Vec2::splat(size));
        let minimize = close.translate(vec2(-size - 4.0, 0.0));
        let drag = Rect::from_min_max(full.min, pos2(minimize.left(), full.top() + 40.0));
        let response = ui.interact(drag, Id::new("title_bar"), Sense::click_and_drag());
        if response.drag_started_by(PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        let icon = |text: &str| Button::new(RichText::new(text).size(20.0).color(INK)).frame(false);
        if ui.put(minimize, icon(egui_material_icons::icons::ICON_REMOVE)).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
        }
        if ui.put(close, icon(egui_material_icons::icons::ICON_CLOSE)).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    }

    /// Les quatre étapes, en cartes côte à côte.
    fn steps(&self, ui: &mut egui::Ui, full: Rect, v: &View) {
        let gap = 10.0;
        let top = full.top() + BANNER_HEIGHT + 4.0;
        let width = (full.width() - 2.0 * MARGIN - 3.0 * gap) / 4.0;
        let painter = ui.painter();
        for (i, (step, label, detail)) in v.steps.iter().enumerate() {
            let rect = Rect::from_min_size(pos2(full.left() + MARGIN + i as f32 * (width + gap), top), vec2(width, 92.0));
            let (fill, border) = match step {
                Step::Done if i == 3 => (Color32::from_rgba_unmultiplied(45, 212, 191, 20), TEAL.gamma_multiply(0.5)),
                Step::Current => (Color32::from_rgba_unmultiplied(255, 255, 255, 10), WARN.gamma_multiply(0.5)),
                _ => (Color32::from_rgba_unmultiplied(255, 255, 255, 10), Color32::from_rgba_unmultiplied(255, 255, 255, 22)),
            };
            painter.rect_filled(rect, CornerRadius::same(12), fill);
            painter.rect_stroke(rect, CornerRadius::same(12), Stroke::new(1.0_f32, border), StrokeKind::Inside);

            let dim = *step == Step::Todo;
            let center = pos2(rect.left() + 24.0, rect.top() + 24.0);
            match step {
                Step::Done => {
                    painter.circle_filled(center, 11.0, TEAL);
                    painter.text(center, Align2::CENTER_CENTER, egui_material_icons::icons::ICON_CHECK, FontId::proportional(15.0), ON_TEAL);
                }
                Step::Current => {
                    painter.circle_stroke(center, 10.0, Stroke::new(2.0_f32, WARN));
                    painter.text(center, Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(12.0), WARN);
                }
                Step::Todo => {
                    painter.circle_stroke(center, 10.0, Stroke::new(1.0_f32, MUTED.gamma_multiply(0.6)));
                    painter.text(center, Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(12.0), MUTED);
                }
            }
            let title_color = match step {
                Step::Done if i == 3 => TEAL,
                Step::Current => WARN,
                Step::Todo => MUTED,
                Step::Done => INK,
            };
            painter.text(pos2(rect.left() + 14.0, rect.top() + 50.0), Align2::LEFT_TOP, *label, FontId::proportional(14.0), title_color);
            painter.text(
                pos2(rect.left() + 14.0, rect.top() + 70.0),
                Align2::LEFT_TOP,
                detail,
                FontId::proportional(12.0),
                if dim { MUTED.gamma_multiply(0.6) } else { MUTED },
            );
        }
    }

    /// Action du moment, ouvrir Teyvin ; version et journaux tout en bas.
    fn footer(&mut self, ui: &mut egui::Ui, full: Rect, v: &View) {
        let t = tx();
        let y =full.top() + BANNER_HEIGHT + 4.0 + 92.0 + 22.0;
        let h = 34.0;

        let open = Rect::from_min_size(pos2(full.right() - MARGIN - 170.0, y), vec2(170.0, h));
        let open_label = format!("{}  {}", t.open_teyvin, egui_material_icons::icons::ICON_OPEN_IN_NEW);
        let open_button = Button::new(RichText::new(open_label).color(ON_TEAL).strong().size(14.0))
            .fill(TEAL)
            .corner_radius(8);
        if ui.put(open, open_button).clicked() {
            ui.ctx().open_url(OpenUrl::new_tab(t.url));
        }

        // Action du moment (mettre à jour, démarrer la capture), sinon « Réduire »
        let second = Rect::from_min_size(pos2(open.left() - 10.0 - 170.0, y), vec2(170.0, h));
        let third = Rect::from_min_size(pos2(second.left() - 10.0 - 120.0, y), vec2(120.0, h));
        let outline = |text: &str| {
            Button::new(RichText::new(text).color(INK).size(14.0))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 40)))
                .corner_radius(8)
        };
        match (&v.action, &v.secondary) {
            (Some((label, msg)), secondary) => {
                let msg = clone_message(msg);
                if ui.put(second, outline(label)).clicked() {
                    self.send(msg);
                }
                if let Some((label, msg)) = secondary {
                    let msg = clone_message(msg);
                    if ui.put(third, outline(label)).clicked() {
                        self.send(msg);
                    }
                }
            }
            (None, _) => {
                if ui.put(second, outline(t.minimize)).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
            }
        }

        // Pied : version, journaux, crédit
        let painter = ui.painter();
        let bottom = full.bottom() - 14.0;
        painter.text(
            pos2(full.left() + MARGIN, bottom),
            Align2::LEFT_BOTTOM,
            t.credit.replace("{v}", env!("CARGO_PKG_VERSION")),
            FontId::proportional(11.0),
            MUTED.gamma_multiply(0.8),
        );
        let logs = Rect::from_min_size(pos2(full.right() - MARGIN - 70.0, bottom - 16.0), vec2(70.0, 18.0));
        if ui
            .put(logs, Button::new(RichText::new(t.logs).size(11.0).color(MUTED)).frame(false))
            .clicked()
        {
            if let Err(e) = open_log_dir() {
                tracing::error!("unable to open log dir: {e}");
            }
        }
    }
}

/// Les messages d'Irminsul ne sont pas clonables : on recrée ceux que l'écran envoie.
fn clone_message(msg: &Message) -> Message {
    match msg {
        Message::UpdateAcknowledged => Message::UpdateAcknowledged,
        Message::UpdateCanceled => Message::UpdateCanceled,
        Message::DownloadAcknowledged => Message::DownloadAcknowledged,
        Message::StopCapture => Message::StopCapture,
        _ => Message::StartCapture,
    }
}
