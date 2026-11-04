"""Applique la modification Teyvin à une copie du code d'Irminsul.

Usage : python teyvin/apply.py <dossier irminsul> --version 0.2.2-teyvin.1 --repo propriétaire/teyvin-compagnon

Le moteur d'Irminsul (lecture du jeu) reste intact, sauf la recherche de clé pour un jeu déjà lancé
(teyvin/seed_search.rs) et la reconnaissance des paquets du compte à chaque version (teyvin/packet_match.rs), dans une
copie du moteur de capture. On ajoute le relais (src/relay.rs), on
remplace l'écran (src/app.rs, aux couleurs de Teyvin) et les mises à jour (src/update.rs), et on modifie quelques lignes ailleurs. Chaque point
d'ancrage doit apparaître exactement une fois ; sinon le script s'arrête avec un message clair (le code
d'Irminsul a changé : à corriger à la main).
"""

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MARK = "// Teyvin Compagnon"
NAME = "Teyvin Companion"
FILE = "Teyvin.exe"


def edit(path: Path, anchor: str, replacement: str) -> None:
    raw = path.read_bytes().decode("utf-8")
    crlf = "\r\n" in raw
    text = raw.replace("\r\n", "\n")
    if replacement in text:
        print(f"  déjà fait : {path.name}")
        return
    count = text.count(anchor)
    if count != 1:
        sys.exit(f"ERREUR : ancre trouvée {count} fois dans {path} (attendu : 1)\n  {anchor!r}")
    text = text.replace(anchor, replacement)
    if crlf:
        text = text.replace("\n", "\r\n")
    path.write_bytes(text.encode("utf-8"))
    print(f"  modifié : {path.name}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("irminsul", type=Path)
    parser.add_argument("--version", required=True, help="version du compagnon, ex. 0.2.2-teyvin.1")
    parser.add_argument("--repo", required=True, help="dépôt des releases, ex. kevinfern06/teyvin-compagnon")
    args = parser.parse_args()

    src = args.irminsul / "src"

    print("Teyvin Compagnon : application de la modification")
    shutil.copy(HERE / "relay.rs", src / "relay.rs")
    print("  ajouté : relay.rs")
    # Écran du compagnon, aux couleurs de Teyvin (il lance aussi le relais avec le lien des vœux)
    shutil.copy(HERE / "app.rs", src / "app.rs")
    for asset in ("fond.webp", "logo.webp", "wordmark.webp", "rubik-regular.ttf", "rubik-gras.ttf"):
        shutil.copy(HERE / asset, args.irminsul / "assets" / f"teyvin-{asset}")
    print("  remplacé : app.rs (écran Teyvin)")
    # Logo de Teyvin : icône du .exe (build.rs) et de la fenêtre (main.rs), aux mêmes chemins qu'Irminsul
    shutil.copy(HERE / "icon.ico", args.irminsul / "assets" / "icon.ico")
    shutil.copy(HERE / "icon-256.png", args.irminsul / "assets" / "icon-256.png")
    print("  remplacé : icônes (logo Teyvin)")

    # 1. Déclarer le module
    edit(src / "main.rs", "mod player_data;\n", f"mod player_data;\nmod relay; {MARK}\n")

    # 2. Publier un export GOOD à chaque nouvelle donnée du jeu
    anchor = "        if has_new_data {\n"
    edit(
        src / "monitor.rs",
        anchor,
        anchor
        + "            crate::relay::publish(self.player_data.export_genshin_optimizer("
        + f"&crate::relay::export_settings()), self.player_data.first_character_time()); {MARK}\n",
    )
    # Date d'obtention du premier perso (le Voyageur) : à peu près la création du compte, pour son âge sur Teyvin
    anchor = "    pub fn export_genshin_optimizer(&self, settings: &ExportSettings) -> Result<String> {\n"
    edit(
        src / "player_data.rs",
        anchor,
        f"""    {MARK}
    pub fn first_character_time(&self) -> Option<u32> {{
        self.characters
            .iter()
            .filter(|c| c.avatar_type == 1 && c.born_time > 0)
            .map(|c| c.born_time)
            .min()
    }}

"""
        + anchor,
    )

    # 3. Mises à jour : le compagnon signale une nouvelle version sans jamais se remplacer lui-même (un .exe qui se
    # télécharge, se renomme et se supprime ressemble à un virus pour les antivirus)
    shutil.copy(HERE / "update.rs", src / "update.rs")
    edit(src / "update.rs", "__REPO__", args.repo)
    print("  remplacé : update.rs (signale seulement)")
    # self_update embarque son code d'auto-remplacement dès qu'elle est liée, même sans être appelée
    edit(args.irminsul / "Cargo.toml", 'self_update = "0.42.0"\n', f'semver = "1.0" {MARK.replace("//", "#")}\n')
    edit(
        src / "main.rs",
        """    Updating,
    /// `needs_caps` is set when the update dropped the packet capture
    /// permissions the user had granted, and they need to be re-granted.
    Updated {
        needs_caps: bool,
    },
""",
        f"    {MARK} : plus d'état « mise à jour en cours », voir teyvin/update.rs\n",
    )

    # 3 ter. Nom « Teyvin Companion » : titre de la fenêtre (barre des tâches) et détails du .exe (gestionnaire des
    # tâches). Le nom interne « Irminsul » reste : il fixe le dossier des réglages et des journaux
    edit(
        src / "main.rs",
        "            .with_decorations(false)\n",
        f'            .with_decorations(false)\n            .with_title("{NAME}") {MARK}\n',
    )
    edit(
        args.irminsul / "build.rs",
        '            .set_icon("assets/icon.ico")\n',
        '            .set_icon("assets/icon.ico")\n'
        f'            .set("ProductName", "{NAME}") {MARK}\n'
        f'            .set("FileDescription", "{NAME}")\n'
        f'            .set("OriginalFilename", "{FILE}")\n',
    )

    # 3 quater. Échelle de Windows : le compagnon suit celle de l'écran principal (« system » dans le manifeste) au
    # lieu de changer d'échelle en passant d'un écran à l'autre. Sa fenêtre sans bordure, glissée entre un écran à
    # 100 % et un écran à 125 %, accrochait à la limite et changeait brusquement de taille ; Windows l'agrandit
    # désormais lui-même sur l'autre écran (un peu moins net), comme les applications classiques
    edit(
        args.irminsul / "build.rs",
        "            .compile()?;\n",
        f"            .set_manifest(TEYVIN_MANIFEST) {MARK}\n            .compile()?;\n",
    )
    edit(
        args.irminsul / "build.rs",
        "#[tokio::main]\n",
        f"""{MARK} : échelle de Windows (voir teyvin/apply.py)
const TEYVIN_MANIFEST: &str = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">system</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>"#;

#[tokio::main]\n""",
    )

    # 3 quinquies. Jeu déjà lancé : le moteur ne retrouve pas la clé de session si le compagnon a manqué la première
    # connexion du jeu (une reconnexion échouait donc). On corrige une copie du moteur de capture, épinglée au même
    # commit, pour chercher depuis l'heure de lancement du jeu : voir teyvin/seed_search.rs
    cargo = args.irminsul / "Cargo.toml"
    manifest = cargo.read_bytes().decode("utf-8")
    found = re.search(r'auto-artifactarium = \{ git = "([^"]+)", rev = "([0-9a-f]+)" \}', manifest)
    if not found:
        sys.exit("ERREUR : auto-artifactarium introuvable dans Cargo.toml")
    engine = args.irminsul / "vendor" / "auto-artifactarium"
    if not engine.exists():
        subprocess.run(["git", "clone", "--quiet", found[1], str(engine)], check=True)
        subprocess.run(["git", "-C", str(engine), "checkout", "--quiet", found[2]], check=True)
    shutil.copy(HERE / "seed_search.rs", engine / "src" / "seed_search.rs")
    shutil.copy(HERE / "packet_match.rs", engine / "src" / "packet_match.rs")
    lib = engine / "src" / "lib.rs"
    edit(lib, "mod unk_util;\n", f"mod unk_util;\nmod packet_match; {MARK}\nmod seed_search; {MARK}\n")
    # 3 sexies. Nouvelle version du jeu : le moteur a en dur les numéros des paquets du compte, qui changent à chaque
    # version. On les reconnaît à leur forme : voir teyvin/packet_match.rs
    edit(
        lib,
        "    if !game_command.is_player_store_notify() {\n        return None;\n    }\n\n"
        "    return matches_items_all_data_notify(&game_command.proto_data);\n",
        f"    packet_match::items(game_command.command_id, &game_command.proto_data) {MARK}\n",
    )
    edit(
        lib,
        "    if !game_command.is_avatar_data_notify() {\n        return None;\n    }\n\n"
        "    return matches_avatars_all_data_notify(&game_command.proto_data);\n",
        f"    packet_match::avatars(game_command.command_id, &game_command.proto_data) {MARK}\n",
    )
    edit(
        lib,
        "    possible_seeds: Vec<u64>,\n}\n",
        f"    possible_seeds: Vec<u64>,\n    game_started_ms: Option<u64>, {MARK}\n    seed_searched: bool,\n}}\n",
    )
    edit(
        lib,
        "    #[instrument(skip_all, fields(len = bytes.len()))]\n    pub fn receive_packet(",
        f"""    /// Heure de lancement du jeu, s'il tournait déjà au démarrage de la capture. {MARK}
    pub fn set_game_started_ms(&mut self, started: Option<u64>) {{
        self.game_started_ms = started;
    }}

    /// Dernier recours quand le moteur n'a pas vu la première connexion du jeu ; une seule fois par connexion. {MARK}
    fn search_client_seed(&mut self, data: &[u8]) -> bool {{
        let (Some(started), None, false) = (self.game_started_ms, self.client_seed, self.seed_searched) else {{
            return false;
        }};
        self.seed_searched = true;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let from = (started as i64 - 5_000).max(now - 24 * 3_600_000);
        let Some((client_seed, key)) = seed_search::search(from, now, &self.possible_seeds, data) else {{
            warn!("Client seed not found since game start");
            return false;
        }};
        info!("Client seed found since game start");
        self.client_seed = Some(client_seed);
        self.key = Some(Key::Session(key));
        true
    }}

    #[instrument(skip_all, fields(len = bytes.len()))]
    pub fn receive_packet(""",
    )
    edit(
        lib,
        "                self.key = None;\n                Some(GamePacket::Connection(packet))\n",
        f"                self.key = None;\n                self.seed_searched = false; {MARK}\n                Some(GamePacket::Connection(packet))\n",
    )
    edit(
        lib,
        "                    match discovered_key {\n",
        f"""                    if discovered_key.is_none() {{ {MARK}
                        discovered_key = None; // libère l'emprunt de self.key pour la recherche
                        if self.search_client_seed(&data) {{
                            discovered_key = self.key.as_ref();
                        }}
                    }}

                    match discovered_key {{
""",
    )
    edit(
        src / "monitor.rs",
        "                let cancel_token = CancellationToken::new();\n",
        f"                self.sniffer.set_game_started_ms(crate::relay::game_started_ms()); {MARK}\n"
        "                let cancel_token = CancellationToken::new();\n",
    )
    edit(
        cargo,
        '\t"Win32_System_Console",\n',
        '\t"Win32_System_Console",\n\t"Win32_System_Diagnostics_ToolHelp",\n\t"Win32_System_Threading",\n',
    )
    edit(
        cargo,
        '# [patch."https://github.com/konkers/auto-artifactarium"]\n# auto-artifactarium = { path = "../auto-artifactarium" }\n',
        '[patch."https://github.com/konkers/auto-artifactarium"]\nauto-artifactarium = { path = "vendor/auto-artifactarium" }\n',
    )

    # 4. Version : celle de la release, pour que la mise à jour automatique sache qu'elle est à jour
    text = cargo.read_bytes().decode("utf-8")
    text, n = re.subn(r'(?m)^version = "[^"]+"', f'version = "{args.version}"', text, count=1)
    if n != 1:
        sys.exit("ERREUR : version introuvable dans Cargo.toml")
    cargo.write_bytes(text.encode("utf-8"))
    print(f"  version : {args.version}")

    print("OK")


if __name__ == "__main__":
    main()
