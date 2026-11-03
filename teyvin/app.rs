//! Teyvin Compagnon : écran du compagnon, aux couleurs de Teyvin.
//!
//! Remplace l'écran d'Irminsul (src/app.rs) en gardant son moteur : même démarrage (vérification des mises à
//! jour, lecture du lien de vœux, capture), mêmes messages vers le moniteur. L'écran ne montre que ce qui
//! sert, sur l'illustration du chat : à gauche l'état général, quatre étapes (compagnon prêt, jeu connecté, compte
//! lu, Teyvin à jour) et un bouton vers Teyvin ; à droite, l'illustration reste dégagée. En français si Windows
//! l'est, en anglais sinon.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use egui::{
    Align2, Button, Checkbox, Color32, Context, CornerRadius, FontData, FontFamily, FontId, Id, Painter,
    PointerButton, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, ViewportCommand, pos2, vec2,
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
    download_url: &'static str,
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
    auto_open: &'static str,
    credit: &'static str,
}

const FR: Texts = Texts {
    url: "https://teyvin.app/compte",
    download_url: "https://teyvin.app/compagnon",
    brand: "Compagnon",
    step_ready: "Compagnon prêt",
    step_launch: "Lance le jeu",
    step_game: "Jeu connecté",
    step_read: "Compte lu",
    step_teyvin: "Teyvin à jour",
    starting_detail: "démarrage…",
    starting: "Démarrage…",
    checking_updates: "Vérification des mises à jour du compagnon.",
    loading_data: "Chargement des données du jeu.",
    update: "Télécharger",
    later: "Plus tard",
    update_available: "Mise à jour disponible",
    update_ready: "La version {v} du compagnon est sortie. Le bouton ouvre sa page de téléchargement.",
    capture_on: "capture active",
    capture_off: "capture arrêtée",
    waiting: "en attente…",
    open_teyvin_detail: "ouvre Teyvin",
    teyvin_soon: "encore quelques secondes…",
    login_read: "connexion lue",
    with_wishes: "vœux compris",
    chars_inventory: "persos et inventaire",
    after_farm: "après un farm : reconnecte-toi",
    capture_stopped: "Capture arrêtée",
    capture_stopped_hint: "Relance la capture pour que le compagnon lise ton compte.",
    start_capture: "Démarrer la capture",
    waiting_game: "En attente du jeu",
    waiting_game_hint: "Lance Genshin et connecte-toi. Déjà en jeu ? Déconnecte-toi (menu Paimon, la porte) puis reconnecte-toi.",
    reading: "Lecture du compte…",
    reading_hint: "Encore un instant.",
    read_hint: "Teyvin se met à jour : sa page ouverte le fera dans quelques secondes.",
    all_done: "Tout est à jour",
    all_done_hint: "Le compte est lu à la connexion : après un farm, reconnecte-toi pour actualiser Teyvin.",
    open_teyvin: "Ouvrir Teyvin",
    minimize: "Réduire",
    logs: "Journaux",
    auto_open: "Ouvrir Teyvin tout seul",
    credit: "v{v} · basé sur Irminsul de Konkers",
};

const EN: Texts = Texts {
    url: "https://teyvin.app/en/account",
    download_url: "https://teyvin.app/en/companion",
    brand: "Companion",
    step_ready: "Companion ready",
    step_launch: "Launch the game",
    step_game: "Game connected",
    step_read: "Account read",
    step_teyvin: "Teyvin up to date",
    starting_detail: "starting…",
    starting: "Starting…",
    checking_updates: "Checking for companion updates.",
    loading_data: "Loading game data.",
    update: "Download",
    later: "Later",
    update_available: "Update available",
    update_ready: "Companion version {v} is out. The button opens its download page.",
    capture_on: "capture on",
    capture_off: "capture stopped",
    waiting: "waiting…",
    open_teyvin_detail: "open Teyvin",
    teyvin_soon: "a few more seconds…",
    login_read: "login read",
    with_wishes: "wishes included",
    chars_inventory: "characters and inventory",
    after_farm: "after farming: log in again",
    capture_stopped: "Capture stopped",
    capture_stopped_hint: "Start the capture again so the companion can read your account.",
    start_capture: "Start capture",
    waiting_game: "Waiting for the game",
    waiting_game_hint: "Launch Genshin and log in. Already in game? Log out (Paimon menu, the door), then log back in.",
    reading: "Reading account…",
    reading_hint: "Just a moment.",
    read_hint: "Teyvin is updating: its open page will do so within seconds.",
    all_done: "All up to date",
    all_done_hint: "Your account is read when you log in: after farming, log in again to refresh Teyvin.",
    open_teyvin: "Open Teyvin",
    minimize: "Minimize",
    logs: "Logs",
    auto_open: "Open Teyvin on its own",
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

// Couleurs de Teyvin (DA « Grimoire », tokens de app/globals.css)
const BG: Color32 = Color32::from_rgb(5, 7, 8);
const POPOVER: Color32 = Color32::from_rgb(14, 13, 26);
const INK: Color32 = Color32::from_rgb(233, 239, 241);
const MUTED: Color32 = Color32::from_rgb(160, 170, 178);
const ACCENT: Color32 = Color32::from_rgb(253, 225, 120);
const ON_ACCENT: Color32 = Color32::from_rgb(58, 44, 0);
const ORNAMENT: Color32 = Color32::from_rgb(205, 191, 154);
const OK: Color32 = Color32::from_rgb(74, 222, 128);
const WARN: Color32 = Color32::from_rgb(251, 154, 108);

const MARGIN: f32 = 22.0;
/// Colonne de gauche : l'orbe et le chat restent à sa droite.
const COLUMN: f32 = 372.0;
const ROW: f32 = 38.0;
const STEPS_TOP: f32 = 170.0;
const STEPS_HEIGHT: f32 = 4.0 * ROW + 12.0;
const BOLD: &str = "rubik-gras";
const WORDMARK_HEIGHT: f32 = 24.0;
const WORDMARK_RATIO: f32 = 2.928;

fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

/// Police du texte du site (Rubik, licence OFL) à la place de celle d'egui ; les icônes restent en secours.
fn install_fonts(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    let icons = egui_material_icons::font_insert();
    fonts.font_data.insert(icons.name.clone(), icons.data.into());
    for (name, data) in [
        ("rubik", &include_bytes!("../assets/teyvin-rubik-regular.ttf")[..]),
        (BOLD, &include_bytes!("../assets/teyvin-rubik-gras.ttf")[..]),
    ] {
        fonts.font_data.insert(name.into(), FontData::from_static(data).into());
    }
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "rubik".into());
    proportional.push(icons.name);
    let mut heavy = vec![BOLD.to_string()];
    heavy.extend(proportional.iter().cloned());
    fonts.families.insert(FontFamily::Name(BOLD.into()), heavy);
    ctx.set_fonts(fonts);
}

/// Réglages gardés entre deux lancements. Les réglages d'export restent ceux d'Irminsul par défaut :
/// le relais les reprend pour produire l'export lu par Teyvin.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedAppState {
    export_settings: ExportSettings,
    #[serde(default)]
    tracing_level: TracingLevel,
    #[serde(default = "auto_open_default")]
    auto_open: bool,
}

const fn auto_open_default() -> bool {
    true
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
            auto_open: auto_open_default(),
        }
    }
}

pub struct IrminsulApp {
    ui_message_tx: mpsc::UnboundedSender<Message>,
    state_rx: watch::Receiver<AppState>,
    saved_state: SavedAppState,
    download_acknowledged: bool,
}

/// Réglage du joueur, lu par `open_teyvin_with_game`.
static AUTO_OPEN: AtomicBool = AtomicBool::new(true);

/// Ouvre Teyvin une fois, dès que Genshin est lancé : les joueurs s'en servent pendant qu'ils jouent, et la page se
/// met à jour toute seule une fois le compte lu. Dans son propre fil : l'écran ne se redessine plus quand la fenêtre
/// est réduite.
fn open_teyvin_with_game() {
    // Une page déjà ouverte se signale au relais toutes les 15 s : on lui laisse le temps de le faire
    const SITE_CAN_ANSWER: Duration = Duration::from_secs(20);
    let started = Instant::now();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(2));
            if !AUTO_OPEN.load(Ordering::Relaxed)
                || started.elapsed() < SITE_CAN_ANSWER
                || crate::relay::game_started_ms().is_none()
            {
                continue;
            }
            if !crate::relay::site_open() {
                crate::relay::open_site(tx().url);
            }
            return;
        }
    });
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
    open_teyvin_with_game();
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
        install_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_theme(egui::Theme::Dark);

        let saved_state: SavedAppState = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        tracing_reload_handle.set_filter(saved_state.tracing_level.get_filter());
        AUTO_OPEN.store(saved_state.auto_open, Ordering::Relaxed);

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
        }
    }

    fn send(&self, msg: Message) {
        if let Err(e) = self.ui_message_tx.send(msg) {
            tracing::error!("Unable to send UI message: {e}");
        }
    }
}

/// Une étape : faite, en cours (jaune) ou à venir (grisée).
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
                (t.waiting_game, t.waiting_game_hint, false, None)
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
                // Jusqu'à ce que Teyvin ait récupéré le compte (il interroge le relais toutes les 15 s)
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
            style.visuals.widgets.inactive.fg_stroke.color = ACCENT;
            style.visuals.widgets.hovered.fg_stroke.color = ACCENT;
            style.visuals.widgets.active.fg_stroke.color = ACCENT;
        });

        let state = self.state_rx.borrow_and_update().clone();
        match state.state {
            State::WaitingForDownloadConfirmation(_) if !self.download_acknowledged => {
                self.download_acknowledged = true;
                self.send(Message::DownloadAcknowledged);
            }
            _ => {}
        }
        let v = view(&state);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                self.backdrop(ui, full);
                self.title_bar(ui, full);
                self.header(ui, full, &v);
                self.steps(ui, full, &v);
                self.footer(ui, full, &v);
            });
    }
}

impl IrminsulApp {
    /// Illustration du chat sur toute la fenêtre, assombrie à gauche pour la lisibilité.
    fn backdrop(&self, ui: &mut egui::Ui, full: Rect) {
        egui::Image::new(egui::include_image!("../assets/teyvin-fond.webp")).paint_at(ui, full);
        let painter = ui.painter();
        let split = full.left() + MARGIN + COLUMN;
        let fade = split + 70.0;
        shade(painter, Rect::from_x_y_ranges(full.left()..=split, full.y_range()), [0.74, 0.6]);
        shade(painter, Rect::from_x_y_ranges(split..=fade, full.y_range()), [0.6, 0.0]);
    }

    /// Logo et état général, en haut de la colonne.
    fn header(&self, ui: &mut egui::Ui, full: Rect, v: &View) {
        let x = full.left() + MARGIN;
        let top = full.top() + 16.0;
        egui::Image::new(egui::include_image!("../assets/teyvin-logo.webp"))
            .paint_at(ui, Rect::from_min_size(pos2(x, top), Vec2::splat(36.0)));
        let wordmark = Rect::from_min_size(pos2(x + 44.0, top + 7.0), vec2(WORDMARK_HEIGHT * WORDMARK_RATIO, WORDMARK_HEIGHT));
        egui::Image::new(egui::include_image!("../assets/teyvin-wordmark.webp")).paint_at(ui, wordmark);

        let painter = ui.painter();
        painter.text(
            pos2(wordmark.right() + 10.0, wordmark.bottom() - 2.0),
            Align2::LEFT_BOTTOM,
            tx().brand,
            FontId::proportional(13.0),
            ORNAMENT,
        );
        let title = painter.text(pos2(x, top + 52.0), Align2::LEFT_TOP, &v.title, bold(27.0), INK);
        if v.check {
            painter.text(
                pos2(title.right() + 10.0, title.center().y),
                Align2::LEFT_CENTER,
                egui_material_icons::icons::ICON_CHECK_CIRCLE,
                FontId::proportional(23.0),
                OK,
            );
        }
        let subtitle = painter.layout(
            v.subtitle.clone(),
            FontId::proportional(13.5),
            if v.warn { WARN } else { INK.gamma_multiply(0.88) },
            COLUMN,
        );
        painter.galley(pos2(x, title.bottom() + 5.0), subtitle, INK);
        if v.spinner {
            let r = Rect::from_center_size(pos2(x + COLUMN - 10.0, title.center().y), Vec2::splat(20.0));
            ui.put(r, egui::Spinner::new().color(ACCENT));
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
        // Sur fond sombre : le haut de l'illustration est très clair par endroits
        let icon = |text: &str| {
            Button::new(RichText::new(text).size(20.0).color(INK))
                .fill(Color32::from_rgba_unmultiplied(POPOVER.r(), POPOVER.g(), POPOVER.b(), 150))
                .stroke(Stroke::NONE)
                .corner_radius(10)
        };
        if ui.put(minimize, icon(egui_material_icons::icons::ICON_REMOVE)).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
        }
        if ui.put(close, icon(egui_material_icons::icons::ICON_CLOSE)).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    }

    /// Les quatre étapes, l'une sous l'autre et reliées par un fil, dans un panneau.
    fn steps(&self, ui: &mut egui::Ui, full: Rect, v: &View) {
        let panel = Rect::from_min_size(pos2(full.left() + MARGIN, full.top() + STEPS_TOP), vec2(COLUMN, STEPS_HEIGHT));
        let painter = ui.painter();
        painter.rect_filled(panel, CornerRadius::same(14), POPOVER.gamma_multiply(0.82));
        painter.rect_stroke(panel, CornerRadius::same(14), Stroke::new(1.0_f32, ORNAMENT.gamma_multiply(0.22)), StrokeKind::Inside);

        let center = |i: usize| pos2(panel.left() + 26.0, panel.top() + 6.0 + ROW * (i as f32 + 0.5));
        for i in 0..v.steps.len() - 1 {
            let (from, to) = (center(i), center(i + 1));
            let color = if v.steps[i].0 == Step::Done { OK.gamma_multiply(0.45) } else { ORNAMENT.gamma_multiply(0.22) };
            painter.line_segment([pos2(from.x, from.y + 11.0), pos2(to.x, to.y - 11.0)], Stroke::new(1.0_f32, color));
        }
        for (i, (step, label, detail)) in v.steps.iter().enumerate() {
            let center = center(i);
            match step {
                Step::Done => {
                    painter.circle_filled(center, 10.0, OK);
                    painter.text(center, Align2::CENTER_CENTER, egui_material_icons::icons::ICON_CHECK, FontId::proportional(14.0), BG);
                }
                Step::Current => {
                    painter.circle_stroke(center, 9.5, Stroke::new(2.0_f32, ACCENT));
                    painter.text(center, Align2::CENTER_CENTER, (i + 1).to_string(), bold(11.5), ACCENT);
                }
                Step::Todo => {
                    painter.circle_stroke(center, 9.5, Stroke::new(1.0_f32, MUTED.gamma_multiply(0.6)));
                    painter.text(center, Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(11.5), MUTED);
                }
            }
            let (font, color) = match step {
                Step::Done => (bold(13.5), INK),
                Step::Current => (bold(13.5), ACCENT),
                Step::Todo => (FontId::proportional(13.5), MUTED),
            };
            painter.text(pos2(center.x + 22.0, center.y), Align2::LEFT_CENTER, *label, font, color);
            painter.text(
                pos2(panel.right() - 14.0, center.y),
                Align2::RIGHT_CENTER,
                detail,
                FontId::proportional(11.5),
                if *step == Step::Todo { MUTED.gamma_multiply(0.7) } else { MUTED },
            );
        }
    }

    /// Boutons sous les étapes (un seul en jaune : l'action attendue), version et journaux tout en bas.
    fn footer(&mut self, ui: &mut egui::Ui, full: Rect, v: &View) {
        let t = tx();
        let x = full.left() + MARGIN;
        let y = full.top() + STEPS_TOP + STEPS_HEIGHT + 14.0;
        let h = 36.0;
        let gap = 10.0;

        let primary = |text: String| {
            Button::new(RichText::new(text).color(ON_ACCENT).family(FontFamily::Name(BOLD.into())).size(14.0))
                .fill(ACCENT)
                .corner_radius(10)
        };
        let outline = |text: String| {
            Button::new(RichText::new(text).color(INK).size(14.0))
                .fill(POPOVER.gamma_multiply(0.7))
                .stroke(Stroke::new(1.0_f32, ORNAMENT.gamma_multiply(0.35)))
                .corner_radius(10)
        };

        // Trois boutons quand une mise à jour attend une réponse
        let (open_width, second_width) = if v.secondary.is_some() { (152.0, 120.0) } else { (190.0, COLUMN - 190.0 - gap) };
        let open = Rect::from_min_size(pos2(x, y), vec2(open_width, h));
        let second = Rect::from_min_size(pos2(open.right() + gap, y), vec2(second_width, h));
        let third = Rect::from_min_max(pos2(second.right() + gap, y), pos2(x + COLUMN, y + h));

        let open_label = format!("{}  {}", t.open_teyvin, egui_material_icons::icons::ICON_OPEN_IN_NEW);
        let open_button = if v.action.is_some() { outline(open_label) } else { primary(open_label) };
        if ui.put(open, open_button).clicked() {
            crate::relay::open_site(t.url);
        }

        // Action du moment (mettre à jour, démarrer la capture), sinon « Réduire »
        match (&v.action, &v.secondary) {
            (Some((label, msg)), secondary) => {
                let msg = clone_message(msg);
                if ui.put(second, primary((*label).to_string())).clicked() {
                    if matches!(msg, Message::UpdateAcknowledged) {
                        crate::relay::open_site(t.download_url);
                    }
                    self.send(msg);
                }
                if let Some((label, msg)) = secondary {
                    let msg = clone_message(msg);
                    if ui.put(third, outline((*label).to_string())).clicked() {
                        self.send(msg);
                    }
                }
            }
            (None, _) => {
                if ui.put(second, outline(t.minimize.to_string())).clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
            }
        }

        // Réglage de l'ouverture automatique, sauf si une action prend la place
        if v.secondary.is_none() {
            let room = Rect::from_min_max(pos2(x, y + h + 10.0), pos2(x + COLUMN, y + h + 32.0));
            let label = RichText::new(t.auto_open).size(12.5).color(INK.gamma_multiply(0.85));
            ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
                if ui.add(Checkbox::new(&mut self.saved_state.auto_open, label)).changed() {
                    AUTO_OPEN.store(self.saved_state.auto_open, Ordering::Relaxed);
                }
            });
        }

        // Pied : version, crédit, journaux
        let painter = ui.painter();
        let bottom = full.bottom() - 12.0;
        painter.text(
            pos2(x, bottom),
            Align2::LEFT_BOTTOM,
            t.credit.replace("{v}", env!("CARGO_PKG_VERSION")),
            FontId::proportional(11.0),
            MUTED,
        );
        let logs = Rect::from_min_size(pos2(x + COLUMN - 70.0, bottom - 17.0), vec2(70.0, 18.0));
        let layout = egui::Layout::right_to_left(egui::Align::Center);
        ui.scope_builder(egui::UiBuilder::new().max_rect(logs).layout(layout), |ui| {
            ui.spacing_mut().button_padding = Vec2::ZERO;
            if ui.add(Button::new(RichText::new(t.logs).size(11.0).color(MUTED)).frame(false)).clicked() {
                if let Err(e) = open_log_dir() {
                    tracing::error!("unable to open log dir: {e}");
                }
            }
        });
    }
}

/// Voile sombre de gauche à droite (opacité de chaque bord).
fn shade(painter: &Painter, rect: Rect, alpha: [f32; 2]) {
    let color = |a: f32| Color32::from_rgba_unmultiplied(BG.r(), BG.g(), BG.b(), (a * 255.0) as u8);
    let mut veil = egui::Mesh::default();
    veil.colored_vertex(rect.left_top(), color(alpha[0]));
    veil.colored_vertex(rect.right_top(), color(alpha[1]));
    veil.colored_vertex(rect.right_bottom(), color(alpha[1]));
    veil.colored_vertex(rect.left_bottom(), color(alpha[0]));
    veil.add_triangle(0, 1, 2);
    veil.add_triangle(0, 2, 3);
    painter.add(veil);
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
