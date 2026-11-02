//! Teyvin Compagnon : relais local entre Irminsul et le site Teyvin.
//!
//! Irminsul lit déjà le compte (persos, armes, artefacts, matériaux) et le lien de l'historique de vœux.
//! Ce module garde le dernier export GOOD et ce lien en mémoire et les sert sur 127.0.0.1, pour que le
//! site Teyvin se mette à jour tout seul. Rien ne sort de l'ordinateur : écoute en local uniquement,
//! lecture autorisée au seul site Teyvin (CORS), contrôle de l'en-tête Host contre le DNS rebinding.
//!
//! Points d'entrée (une ligne chacun dans le code d'Irminsul, voir apply.py) :
//! - `start` : lance le serveur (app.rs, à côté du canal du lien de vœux) ;
//! - `publish` : enregistre un nouvel export (monitor.rs, quand de nouvelles données arrivent).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::watch;

/// Port fixe, connu du site.
pub const PORT: u16 = 47317;

/// Sites autorisés à lire les données (production et développement local).
const ORIGINS: &[&str] = &[
    "https://teyvin.app",
    "https://www.teyvin.app",
    // Ancienne adresse, le temps que les joueurs passent sur teyvin.app
    "https://teyvin.vercel.app",
    "http://localhost:3000",
];

struct Snapshot {
    good: String,
    /// Millisecondes depuis l'époque Unix.
    updated_at: u128,
}

static LATEST: Mutex<Option<Snapshot>> = Mutex::new(None);
/// Dernière fois que le site Teyvin a lu l'export (millisecondes depuis l'époque Unix).
static SYNCED_AT: Mutex<Option<u128>> = Mutex::new(None);
static WISH: OnceLock<watch::Receiver<Option<String>>> = OnceLock::new();

/// Réglages d'export par défaut d'Irminsul, repris tels quels (ils suivent ses mises à jour).
pub fn export_settings() -> crate::player_data::ExportSettings {
    let saved = serde_json::to_value(crate::app::SavedAppState::default())
        .expect("SavedAppState se sérialise toujours");
    serde_json::from_value(saved["export_settings"].clone())
        .expect("export_settings fait partie de SavedAppState")
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Heure du dernier export (compte lu), pour l'écran du compagnon.
pub fn exported_at() -> Option<u128> {
    LATEST.lock().ok().and_then(|l| l.as_ref().map(|s| s.updated_at))
}

/// Heure de la dernière lecture par Teyvin.
pub fn synced_at() -> Option<u128> {
    SYNCED_AT.lock().ok().and_then(|s| *s)
}

/// Lien de l'historique de vœux trouvé ?
pub fn has_wish_url() -> bool {
    wish_url().is_some()
}

/// Enregistre le dernier export GOOD (appelé à chaque nouvelle donnée du jeu).
pub fn publish(good: anyhow::Result<String>) {
    match good {
        Ok(good) => {
            let updated_at = now_ms();
            if let Ok(mut latest) = LATEST.lock() {
                *latest = Some(Snapshot { good, updated_at });
            }
        }
        Err(e) => tracing::warn!("Teyvin : export impossible : {e}"),
    }
}

/// Lance le serveur local dans son propre fil (sans dépendre du runtime tokio d'Irminsul).
pub fn start(wish_url_rx: watch::Receiver<Option<String>>) {
    let _ = WISH.set(wish_url_rx);
    thread::spawn(|| {
        let listener = match TcpListener::bind(("127.0.0.1", PORT)) {
            Ok(listener) => listener,
            Err(e) => {
                tracing::warn!("Teyvin : relais indisponible sur le port {PORT} : {e}");
                return;
            }
        };
        tracing::info!("Teyvin : relais à l'écoute sur 127.0.0.1:{PORT}");
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                if let Err(e) = handle(stream) {
                    tracing::debug!("Teyvin : requête ignorée : {e}");
                }
            });
        }
    });
}

struct Request {
    method: String,
    path: String,
    host: String,
    origin: Option<String>,
}

fn read_request(stream: &TcpStream) -> anyhow::Result<Request> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    // Les requêtes du site sont minuscules : on borne la lecture
    let mut reader = BufReader::new(stream.take(16 * 1024));
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let (mut host, mut origin) = (String::new(), None);
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            match name.trim().to_ascii_lowercase().as_str() {
                "host" => host = value.trim().to_string(),
                "origin" => origin = Some(value.trim().to_string()),
                _ => {}
            }
        }
    }
    Ok(Request {
        method,
        path,
        host,
        origin,
    })
}

fn handle(mut stream: TcpStream) -> anyhow::Result<()> {
    let req = read_request(&stream)?;

    // Une page piégée qui ferait pointer son domaine sur 127.0.0.1 (DNS rebinding) n'a pas ce Host
    let host_ok = req.host == format!("127.0.0.1:{PORT}") || req.host == format!("localhost:{PORT}");
    // Navigateur : seulement le site Teyvin. Sans en-tête Origin (outil local), on répond aussi.
    let allowed = req.origin.as_deref().filter(|o| ORIGINS.contains(o));
    if !host_ok || (req.origin.is_some() && allowed.is_none()) {
        return respond(&mut stream, 403, None, "text/plain", "Interdit");
    }

    if req.method == "OPTIONS" {
        return respond(&mut stream, 204, allowed, "text/plain", "");
    }
    if req.method != "GET" {
        return respond(&mut stream, 405, allowed, "text/plain", "Méthode non prise en charge");
    }

    let path = req.path.split('?').next().unwrap_or_default();
    match path {
        "/status" => {
            // Teyvin passe ici chaque minute : s'il a déjà l'export le plus récent, il est à jour
            if allowed.is_some() {
                if let (Ok(mut s), Some(exp)) = (SYNCED_AT.lock(), exported_at()) {
                    if s.is_some_and(|t| t >= exp) {
                        *s = Some(now_ms());
                    }
                }
            }
            let (good, wish) = (
                LATEST.lock().ok().and_then(|l| l.as_ref().map(|s| s.updated_at)),
                wish_url().is_some(),
            );
            let body = serde_json::json!({
                "app": "teyvin-compagnon",
                "version": env!("CARGO_PKG_VERSION"),
                "goodUpdatedAt": good,
                "wish": wish,
            });
            respond(&mut stream, 200, allowed, "application/json", &body.to_string())
        }
        "/good" => {
            let good = LATEST.lock().ok().and_then(|l| l.as_ref().map(|s| s.good.clone()));
            match good {
                Some(good) => {
                    if allowed.is_some() {
                        if let Ok(mut s) = SYNCED_AT.lock() {
                            *s = Some(now_ms());
                        }
                    }
                    respond(&mut stream, 200, allowed, "application/json", &good)
                }
                None => respond(&mut stream, 404, allowed, "text/plain", "Pas encore de données"),
            }
        }
        "/wish" => match wish_url() {
            Some(url) => {
                let body = serde_json::json!({ "url": url });
                respond(&mut stream, 200, allowed, "application/json", &body.to_string())
            }
            None => respond(&mut stream, 404, allowed, "text/plain", "Pas encore de lien"),
        },
        _ => respond(&mut stream, 404, allowed, "text/plain", "Inconnu"),
    }
}

fn wish_url() -> Option<String> {
    WISH.get().and_then(|rx| rx.borrow().clone())
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    origin: Option<&str>,
    content_type: &str,
    body: &str,
) -> anyhow::Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "OK",
    };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(origin) = origin {
        head.push_str(&format!(
            "Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\nAccess-Control-Allow-Private-Network: true\r\nAccess-Control-Max-Age: 600\r\n"
        ));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()?;
    Ok(())
}

/// Heure de lancement du jeu (ms depuis 1970), si le jeu tourne déjà. On demande seulement à Windows l'heure de création
/// du processus, sans lire le jeu. Le moteur en a besoin pour une reconnexion : voir teyvin/seed_search.rs.
#[cfg(windows)]
pub fn game_started_ms() -> Option<u64> {
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    // Le processus existe mais refuse de donner son heure : on cherche alors sur les dernières heures
    const FALLBACK_MS: u64 = 6 * 3_600_000;
    // FILETIME : intervalles de 100 ns depuis 1601
    const FILETIME_TO_UNIX_MS: u64 = 11_644_473_600_000;

    let pid = unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut pid = None;
        let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
        while more {
            let len = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if name.eq_ignore_ascii_case("GenshinImpact.exe") || name.eq_ignore_ascii_case("YuanShen.exe") {
                pid = Some(entry.th32ProcessID);
                break;
            }
            more = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
        pid?
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as u64;
    let created = unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok().and_then(|process| {
            let (mut created, mut a, mut b, mut c) =
                (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
            let times = GetProcessTimes(process, &mut created, &mut a, &mut b, &mut c);
            let _ = CloseHandle(process);
            times.ok().map(|_| created)
        })
    };
    Some(created.map_or(now.saturating_sub(FALLBACK_MS), |t| {
        (((t.dwHighDateTime as u64) << 32 | t.dwLowDateTime as u64) / 10_000).saturating_sub(FILETIME_TO_UNIX_MS)
    }))
}

#[cfg(not(windows))]
pub fn game_started_ms() -> Option<u64> {
    None
}
