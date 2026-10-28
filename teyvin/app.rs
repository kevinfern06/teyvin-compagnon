//! Teyvin Compagnon : écran du compagnon, aux couleurs de Teyvin.
//!
//! Remplace l'écran d'Irminsul (src/app.rs) en gardant son moteur : même démarrage (vérification des mises à
//! jour, lecture du lien de vœux, capture), mêmes messages vers le moniteur. L'écran ne montre que ce qui
//! sert : une bannière de nuit avec l'état général, quatre étapes (compagnon prêt, jeu connecté, compte lu,
//! Teyvin à jour), le démarrage avec Windows et un bouton vers Teyvin.

use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::Local;
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

const TEYVIN_URL: &str = "https://teyvin.vercel.app/compte";

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
    /// Tâche planifiée « Démarrer avec Windows » présente ?
    autostart: bool,
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
            autostart: autostart::enabled(),
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
    steps: [(Step, &'static str, String); 4],
    /// Bouton d'action à la place de « Réduire » (démarrer la capture, mettre à jour…).
    action: Option<(&'static str, Message)>,
    secondary: Option<(&'static str, Message)>,
    spinner: bool,
}

fn clock(ms: u128) -> String {
    let secs = (ms / 1000) as i64;
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|d| d.with_timezone(&Local).format("%H h %M").to_string())
        .unwrap_or_default()
}

fn instant_clock(t: Instant) -> String {
    let ago = chrono::Duration::from_std(t.elapsed()).unwrap_or_default();
    (Local::now() - ago).format("%H h %M").to_string()
}

fn ago(ms: u128) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(ms);
    let minutes = (now.saturating_sub(ms) / 60_000) as u64;
    match minutes {
        0 => "à l'instant".to_string(),
        1 => "il y a 1 min".to_string(),
        m if m < 60 => format!("il y a {m} min"),
        m => format!("il y a {} h", m / 60),
    }
}

fn view(state: &AppState) -> View {
    let wait = |title: &str, subtitle: &str| View {
        title: title.to_string(),
        subtitle: subtitle.to_string(),
        warn: false,
        steps: [
            (Step::Current, "Compagnon prêt", "démarrage…".into()),
            (Step::Todo, "Jeu connecté", "—".into()),
            (Step::Todo, "Compte lu", "—".into()),
            (Step::Todo, "Teyvin à jour", "—".into()),
        ],
        action: None,
        secondary: None,
        spinner: true,
    };

    match &state.state {
        State::Starting | State::CheckingForUpdate | State::CheckingForData | State::Downloading => {
            wait("Démarrage…", "Vérification des mises à jour du compagnon.")
        }
        State::WaitingForDownloadConfirmation(_) => wait("Démarrage…", "Chargement des données du jeu."),
        State::WaitingForUpdateConfirmation(version) => View {
            spinner: false,
            action: Some(("Mettre à jour", Message::UpdateAcknowledged)),
            secondary: Some(("Plus tard", Message::UpdateCanceled)),
            ..wait("Mise à jour disponible", &format!("La version {version} du compagnon est prête."))
        },
        State::Updating => wait("Mise à jour en cours…", "Le compagnon redémarrera tout seul."),
        State::Updated { .. } => wait("Mis à jour", "Redémarrage du compagnon…"),
        State::Main => {
            let u = &state.updated;
            let game = u.items_updated.or(u.characters_updated);
            let read = u.items_updated.is_some() && u.characters_updated.is_some();
            let exported = crate::relay::exported_at();
            let synced = crate::relay::synced_at();
            let teyvin = matches!((exported, synced), (Some(e), Some(s)) if s >= e);

            let capture = if state.capturing { Step::Done } else { Step::Current };
            let mut steps = [
                (capture, "Compagnon prêt", if state.capturing { "capture active".into() } else { "capture arrêtée".into() }),
                (Step::Todo, "Jeu connecté", "en attente…".into()),
                (Step::Todo, "Compte lu", "—".into()),
                (Step::Todo, "Teyvin à jour", "ouvre Teyvin".into()),
            ];
            if state.capturing {
                steps[1] = match game {
                    Some(t) => (Step::Done, "Jeu connecté", format!("à {}", instant_clock(t))),
                    None => (Step::Current, "Jeu connecté", "en attente…".into()),
                };
            }
            if game.is_some() {
                let detail = if crate::relay::has_wish_url() { "vœux compris" } else { "persos et inventaire" };
                steps[2] = (if read { Step::Done } else { Step::Current }, "Compte lu", detail.into());
            }
            if read {
                steps[3] = match synced.filter(|_| teyvin) {
                    Some(s) => (Step::Done, "Teyvin à jour", format!("à {}", clock(s))),
                    None => (Step::Current, "Teyvin à jour", "ouvre Teyvin".into()),
                };
            }

            let (title, subtitle, warn, action) = if !state.capturing {
                (
                    "Capture arrêtée",
                    "Relance la capture pour que le compagnon lise ton compte.".to_string(),
                    true,
                    Some(("Démarrer la capture", Message::StartCapture)),
                )
            } else if game.is_none() {
                (
                    "En attente du jeu",
                    "Lance Genshin et connecte-toi. Déjà en jeu ? Reviens à l'écran de connexion.".to_string(),
                    true,
                    None,
                )
            } else if !read {
                ("Lecture du compte…", "Encore un instant.".to_string(), false, None)
            } else if !teyvin {
                ("Compte lu", "Ouvre Teyvin pour le mettre à jour.".to_string(), true, None)
            } else {
                (
                    "Tout est à jour",
                    format!("Teyvin synchronisé {}", synced.map(ago).unwrap_or_default()),
                    false,
                    None,
                )
            };
            View {
                title: title.to_string(),
                subtitle,
                warn,
                steps,
                action,
                secondary: None,
                spinner: !read && game.is_some(),
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

        let x = rect.left() + MARGIN;
        let y = rect.bottom() - MARGIN;
        painter.text(
            pos2(x, y - 62.0),
            Align2::LEFT_BOTTOM,
            "TEYVIN COMPAGNON",
            FontId::proportional(11.0),
            TEAL_LIGHT,
        );
        let title = painter.text(pos2(x, y - 26.0), Align2::LEFT_BOTTOM, &v.title, FontId::proportional(30.0), INK);
        if v.title == "Tout est à jour" {
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
            painter.rect_stroke(rect, CornerRadius::same(12), Stroke::new(1.0, border), StrokeKind::Inside);

            let dim = *step == Step::Todo;
            let center = pos2(rect.left() + 24.0, rect.top() + 24.0);
            match step {
                Step::Done => {
                    painter.circle_filled(center, 11.0, TEAL);
                    painter.text(center, Align2::CENTER_CENTER, egui_material_icons::icons::ICON_CHECK, FontId::proportional(15.0), ON_TEAL);
                }
                Step::Current => {
                    painter.circle_stroke(center, 10.0, Stroke::new(2.0, WARN));
                    painter.text(center, Align2::CENTER_CENTER, (i + 1).to_string(), FontId::proportional(12.0), WARN);
                }
                Step::Todo => {
                    painter.circle_stroke(center, 10.0, Stroke::new(1.0, MUTED.gamma_multiply(0.6)));
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

    /// Démarrer avec Windows, action du moment, ouvrir Teyvin ; version et journaux tout en bas.
    fn footer(&mut self, ui: &mut egui::Ui, full: Rect, v: &View) {
        let y = full.top() + BANNER_HEIGHT + 4.0 + 92.0 + 22.0;
        let h = 34.0;

        let mut autostart = self.autostart;
        let toggle = Rect::from_min_size(pos2(full.left() + MARGIN, y), vec2(240.0, h));
        if ui
            .put(toggle, egui::Checkbox::new(&mut autostart, RichText::new("Démarrer avec Windows").color(INK).size(14.0)))
            .changed()
        {
            match autostart::set(autostart) {
                Ok(()) => self.autostart = autostart,
                Err(e) => tracing::error!("Teyvin : démarrage avec Windows impossible : {e}"),
            }
        }

        let open = Rect::from_min_size(pos2(full.right() - MARGIN - 170.0, y), vec2(170.0, h));
        let open_label = format!("Ouvrir Teyvin  {}", egui_material_icons::icons::ICON_OPEN_IN_NEW);
        let open_button = Button::new(RichText::new(open_label).color(ON_TEAL).strong().size(14.0))
            .fill(TEAL)
            .corner_radius(8);
        if ui.put(open, open_button).clicked() {
            ui.ctx().open_url(OpenUrl::new_tab(TEYVIN_URL));
        }

        // Action du moment (mettre à jour, démarrer la capture), sinon « Réduire »
        let second = Rect::from_min_size(pos2(open.left() - 10.0 - 170.0, y), vec2(170.0, h));
        let third = Rect::from_min_size(pos2(second.left() - 10.0 - 120.0, y), vec2(120.0, h));
        let outline = |text: &str| {
            Button::new(RichText::new(text).color(INK).size(14.0))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 40)))
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
                if ui.put(second, outline("Réduire")).clicked() {
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
            format!("v{} · basé sur Irminsul de Konkers", env!("CARGO_PKG_VERSION")),
            FontId::proportional(11.0),
            MUTED.gamma_multiply(0.8),
        );
        let logs = Rect::from_min_size(pos2(full.right() - MARGIN - 70.0, bottom - 16.0), vec2(70.0, 18.0));
        if ui
            .put(logs, Button::new(RichText::new("Journaux").size(11.0).color(MUTED)).frame(false))
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

/// « Démarrer avec Windows » : tâche planifiée à l'ouverture de session, avec les droits administrateur
/// (le compagnon en a besoin pour lire le jeu ; une tâche « plus haut niveau » évite la question de Windows).
mod autostart {
    const TASK: &str = "Teyvin Compagnon";

    #[cfg(windows)]
    fn schtasks(args: &str) -> std::io::Result<std::process::ExitStatus> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("schtasks")
            .raw_arg(args)
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    }

    #[cfg(windows)]
    pub fn enabled() -> bool {
        schtasks(&format!("/Query /TN \"{TASK}\"")).is_ok_and(|s| s.success())
    }

    #[cfg(windows)]
    pub fn set(on: bool) -> anyhow::Result<()> {
        let status = if on {
            let exe = std::env::current_exe()?;
            schtasks(&format!(
                "/Create /TN \"{TASK}\" /TR \"\\\"{}\\\"\" /SC ONLOGON /RL HIGHEST /F",
                exe.display()
            ))?
        } else {
            schtasks(&format!("/Delete /TN \"{TASK}\" /F"))?
        };
        anyhow::ensure!(status.success(), "schtasks a échoué ({status})");
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn enabled() -> bool {
        false
    }

    #[cfg(not(windows))]
    pub fn set(_on: bool) -> anyhow::Result<()> {
        anyhow::bail!("seulement sous Windows")
    }
}
