//! Mise à jour du compagnon en un clic. Rien ne se passe sans le bouton du joueur : le compagnon télécharge le
//! fichier de la dernière release, vérifie sa taille et son empreinte, prend la place de l'ancien fichier puis se
//! relance. L'ancien fichier est gardé jusqu'au prochain lancement, et sert à revenir en arrière si le nouveau ne
//! démarre pas. En cas d'échec, la page de téléchargement de teyvin.app s'ouvre.
//! Cela remplace la bibliothèque `self_update` d'Irminsul, qui embarque du code d'auto-suppression.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, watch};

use crate::{AppState, Message, State};

const LATEST_RELEASE: &str = "https://api.github.com/repos/__REPO__/releases/latest";
/// Fichier de la release à installer.
const ASSET: &str = "Teyvin.exe";

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: usize,
    /// « sha256:… », calculée par GitHub.
    digest: Option<String>,
}

#[derive(Deserialize)]
struct Latest {
    tag_name: String,
    assets: Vec<Asset>,
}

fn client(timeout: u64) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .user_agent("teyvin-companion")
        .build()?)
}

async fn latest_release() -> Result<Latest> {
    let body = client(10)?
        .get(LATEST_RELEASE)
        .send()
        .await
        .context("error fetching the latest release")?
        .error_for_status()?
        .text()
        .await?;
    Ok(serde_json::from_str(&body)?)
}

/// La version de `tag`, si elle est plus récente que `current` (semver : 0.2.2-teyvin.11 > 0.2.2-teyvin.10).
/// Une réponse en retard (cache de GitHub) ne propose donc jamais de retour en arrière.
fn newer_version(current: &str, tag: &str) -> Option<String> {
    let current = Version::parse(current).ok()?;
    let latest = Version::parse(tag.trim_start_matches('v')).ok()?;
    (latest > current).then(|| latest.to_string())
}

/// Refuse un fichier tronqué, abîmé ou qui n'est pas un programme Windows.
fn check_download(bytes: &[u8], size: usize, digest: Option<&str>) -> Result<()> {
    if bytes.len() != size {
        bail!("download size mismatch: {} instead of {size}", bytes.len());
    }
    if !bytes.starts_with(b"MZ") {
        bail!("the download is not a Windows program");
    }
    if let Some(expected) = digest.and_then(|d| d.strip_prefix("sha256:")) {
        let actual: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
        if !actual.eq_ignore_ascii_case(expected) {
            bail!("download checksum mismatch");
        }
    }
    Ok(())
}

/// Ancien fichier, gardé à côté du nouveau : Windows permet de renommer un programme en cours d'exécution.
fn old_path(exe: &Path) -> PathBuf {
    let stem = exe.file_stem().unwrap_or_default().to_string_lossy();
    exe.with_file_name(format!("{stem}.old.exe"))
}

/// Met `bytes` à la place de `exe`, en remettant tout en état si une étape échoue.
fn swap_exe(exe: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let old = old_path(exe);
    let new = exe.with_extension("new");
    fs::write(&new, bytes).context("error writing the new version")?;
    let result = (|| -> Result<()> {
        match fs::remove_file(&old) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
            _ => (),
        }
        fs::rename(exe, &old)?;
        if let Err(e) = fs::rename(&new, exe) {
            let _ = fs::rename(&old, exe);
            return Err(e.into());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&new);
    }
    result.context("error replacing the program")?;
    Ok(old)
}

/// Revient à l'ancien fichier, si le nouveau n'a pas pu démarrer.
fn restore_exe(exe: &Path, old: &Path) {
    let _ = fs::remove_file(exe);
    if let Err(e) = fs::rename(old, exe) {
        tracing::error!("unable to restore the previous version: {e}");
    }
}

/// Efface l'ancien fichier laissé par une mise à jour. false s'il est encore utilisé (le temps qu'il se ferme).
fn remove_old(exe: &Path) -> bool {
    match fs::remove_file(old_path(exe)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => false,
        _ => true,
    }
}

async fn remove_previous_version() {
    let Ok(exe) = env::current_exe() else { return };
    for _ in 0..10 {
        if remove_old(&exe) {
            return;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Télécharge et installe la nouvelle version, puis la lance à la place de celle-ci. Ne revient qu'en cas d'échec.
async fn install(asset: Option<&Asset>) -> Result<()> {
    let asset = asset.context("the release has no program to install")?;
    let exe = env::current_exe()?;
    let bytes = client(300)?
        .get(&asset.browser_download_url)
        .send()
        .await
        .context("error downloading the update")?
        .error_for_status()?
        .bytes()
        .await?;
    check_download(&bytes, asset.size, asset.digest.as_deref())?;
    let old = swap_exe(&exe, &bytes)?;
    // Mêmes options, mêmes droits : le compagnon reprend comme s'il venait d'être lancé
    match Command::new(&exe).args(env::args_os().skip(1)).spawn() {
        Ok(_) => {
            tracing::info!("update installed, restarting");
            std::process::exit(0);
        }
        Err(e) => {
            restore_exe(&exe, &old);
            Err(e).context("error starting the new version")
        }
    }
}

/// Montre la nouvelle version s'il y en a une, puis attend la réponse du joueur avant de continuer.
pub async fn check_for_app_update(
    state_tx: &watch::Sender<AppState>,
    ui_message_rx: &mut mpsc::UnboundedReceiver<Message>,
) -> Result<()> {
    tokio::spawn(remove_previous_version());

    let mut app_state = state_tx.borrow().clone();
    app_state.state = State::CheckingForUpdate;
    state_tx.send(app_state.clone()).unwrap();

    let latest = latest_release().await?;
    let current = env!("CARGO_PKG_VERSION");
    let Some(version) = newer_version(current, &latest.tag_name) else {
        tracing::info!("{current} is current, continuing with app startup");
        return Ok(());
    };
    tracing::info!("Found update {current} -> {version}");

    app_state.state = State::WaitingForUpdateConfirmation(version);
    state_tx.send(app_state.clone()).unwrap();

    loop {
        match ui_message_rx.recv().await {
            Some(Message::UpdateAcknowledged) => break,
            Some(Message::UpdateCanceled) | None => return Ok(()),
            _ => (),
        }
    }

    app_state.state = State::Updating;
    state_tx.send(app_state).unwrap();
    let asset = latest.assets.iter().find(|a| a.name == ASSET);
    if let Err(e) = install(asset).await {
        // Ne bloque pas le joueur : la page de téléchargement prend le relais, et ce compagnon continue
        tracing::error!("update failed: {e:#}");
        crate::relay::open_site(crate::app::download_page());
    }
    Ok(())
}

#[cfg(test)]
mod teyvin_tests {
    use super::*;

    /// Dossier temporaire avec un faux programme ; retourne son chemin.
    fn fake_exe(test: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("teyvin-update-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("Teyvin.exe");
        fs::write(&exe, b"ancien").unwrap();
        exe
    }

    #[test]
    fn offers_only_newer_versions() {
        assert_eq!(newer_version("0.2.2-teyvin.10", "v0.2.2-teyvin.11").unwrap(), "0.2.2-teyvin.11");
        assert_eq!(newer_version("0.2.2-teyvin.9", "v0.2.2-teyvin.11").unwrap(), "0.2.2-teyvin.11");
        assert_eq!(newer_version("0.2.2-teyvin.11", "v0.2.3-teyvin.1").unwrap(), "0.2.3-teyvin.1");
        assert!(newer_version("0.2.2-teyvin.11", "v0.2.2-teyvin.11").is_none());
        // Réponse en retard : jamais de retour en arrière
        assert!(newer_version("0.2.2-teyvin.10", "v0.2.2-teyvin.9").is_none());
        assert!(newer_version("0.2.2-teyvin.10", "pas une version").is_none());
    }

    #[test]
    fn refuses_a_bad_download() {
        let wrong = "sha256:80ba8f5d7ac4d7f4cc3ed6a4cfb1ba3f1fb9b13a8ab6e4a0a1ae1f2fdb2d7f0a";
        let actual: String = Sha256::digest(b"MZabc").iter().map(|b| format!("{b:02x}")).collect();
        let right = format!("sha256:{actual}");
        assert!(check_download(b"MZabc", 5, Some(&right)).is_ok());
        assert!(check_download(b"MZabc", 5, None).is_ok());
        assert!(check_download(b"MZabc", 5, Some(wrong)).is_err(), "mauvaise empreinte");
        assert!(check_download(b"MZab", 5, None).is_err(), "fichier tronqué");
        assert!(check_download(b"<html", 5, None).is_err(), "pas un programme");
    }

    #[test]
    fn swaps_the_program_and_keeps_the_old_one() {
        let exe = fake_exe("swap");
        let old = swap_exe(&exe, b"nouveau").unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"nouveau");
        assert_eq!(fs::read(&old).unwrap(), b"ancien");
        assert_eq!(old.file_name().unwrap(), "Teyvin.old.exe");
        assert!(!exe.with_extension("new").exists());

        // Une mise à jour suivante remplace l'ancien fichier resté là
        swap_exe(&exe, b"encore").unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"encore");
        assert_eq!(fs::read(&old).unwrap(), b"nouveau");
    }

    #[test]
    fn restores_the_old_program() {
        let exe = fake_exe("restore");
        let old = swap_exe(&exe, b"nouveau").unwrap();
        restore_exe(&exe, &old);
        assert_eq!(fs::read(&exe).unwrap(), b"ancien");
        assert!(!old.exists());
    }

    #[test]
    fn leaves_everything_in_place_when_the_swap_fails() {
        let exe = fake_exe("fail");
        // Un dossier à la place de l'ancien fichier : impossible à effacer comme un fichier
        fs::create_dir(old_path(&exe)).unwrap();
        assert!(swap_exe(&exe, b"nouveau").is_err());
        assert_eq!(fs::read(&exe).unwrap(), b"ancien");
        assert!(!exe.with_extension("new").exists());
    }

    #[test]
    fn removes_only_the_old_program() {
        let exe = fake_exe("clean");
        let old = swap_exe(&exe, b"nouveau").unwrap();
        assert!(remove_old(&exe));
        assert!(!old.exists());
        assert!(exe.exists());
        // Rien à effacer : ce n'est pas une erreur
        assert!(remove_old(&exe));
    }
}
