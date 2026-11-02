//! Signale une version plus récente du compagnon. Rien n'est téléchargé ni remplacé ici : le joueur passe par la
//! page de téléchargement de teyvin.app, ouverte par le bouton de l'écran (teyvin/app.rs). La bibliothèque
//! `self_update` d'Irminsul est retirée : elle embarque son code d'auto-remplacement dès qu'elle est liée.

use std::time::Duration;

use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use tokio::sync::{mpsc, watch};

use crate::{AppState, Message, State};

const LATEST_RELEASE: &str = "https://api.github.com/repos/__REPO__/releases/latest";

async fn latest_tag() -> Result<String> {
    #[derive(Deserialize)]
    struct Latest {
        tag_name: String,
    }

    let body = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("teyvin-companion")
        .build()?
        .get(LATEST_RELEASE)
        .send()
        .await
        .context("error fetching the latest release")?
        .error_for_status()?
        .text()
        .await?;
    Ok(serde_json::from_str::<Latest>(&body)?.tag_name)
}

/// La version de `tag`, si elle est plus récente que `current` (semver : 0.2.2-teyvin.11 > 0.2.2-teyvin.10).
/// Une réponse en retard (cache de GitHub) ne propose donc jamais de retour en arrière.
fn newer_version(current: &str, tag: &str) -> Option<String> {
    let current = Version::parse(current).ok()?;
    let latest = Version::parse(tag.trim_start_matches('v')).ok()?;
    (latest > current).then(|| latest.to_string())
}

/// Montre la nouvelle version s'il y en a une, puis attend la réponse du joueur avant de continuer.
pub async fn check_for_app_update(
    state_tx: &watch::Sender<AppState>,
    ui_message_rx: &mut mpsc::UnboundedReceiver<Message>,
) -> Result<()> {
    let mut app_state = state_tx.borrow().clone();
    app_state.state = State::CheckingForUpdate;
    state_tx.send(app_state.clone()).unwrap();

    let tag = latest_tag().await?;
    let current = env!("CARGO_PKG_VERSION");
    let Some(version) = newer_version(current, &tag) else {
        tracing::info!("{current} is current, continuing with app startup");
        return Ok(());
    };
    tracing::info!("Found update {current} -> {version}");

    app_state.state = State::WaitingForUpdateConfirmation(version);
    state_tx.send(app_state).unwrap();

    loop {
        match ui_message_rx.recv().await {
            Some(Message::UpdateAcknowledged | Message::UpdateCanceled) | None => return Ok(()),
            _ => (),
        }
    }
}

#[cfg(test)]
mod teyvin_tests {
    use super::*;

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
}
