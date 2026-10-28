"""Applique la modification Teyvin à une copie du code d'Irminsul.

Usage : python teyvin/apply.py <dossier irminsul> --version 0.2.2-teyvin.1 --repo propriétaire/teyvin-compagnon

La modification reste minuscule pour survivre aux mises à jour d'Irminsul : un fichier ajouté
(src/relay.rs) et quatre lignes. Chaque point d'ancrage doit apparaître exactement une fois ; sinon le
script s'arrête avec un message clair (le code d'Irminsul a changé : à corriger à la main).
"""

import argparse
import re
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MARK = "// Teyvin Compagnon"


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
    owner, name = args.repo.split("/")

    print("Teyvin Compagnon : application de la modification")
    shutil.copy(HERE / "relay.rs", src / "relay.rs")
    print("  ajouté : relay.rs")

    # 1. Déclarer le module
    edit(src / "main.rs", "mod player_data;\n", f"mod player_data;\nmod relay; {MARK}\n")

    # 2. Lancer le relais, avec le lien de l'historique de vœux qu'Irminsul trouve déjà
    anchor = "    let (wish_url_tx, wish_url_rx) = watch::channel(None);\n"
    edit(src / "app.rs", anchor, anchor + f"    crate::relay::start(wish_url_rx.clone()); {MARK}\n")

    # 3. Publier un export GOOD à chaque nouvelle donnée du jeu
    anchor = "        if has_new_data {\n"
    edit(
        src / "monitor.rs",
        anchor,
        anchor
        + "            crate::relay::publish(self.player_data.export_genshin_optimizer("
        + f"&crate::relay::export_settings())); {MARK}\n",
    )

    # 4. Mises à jour automatiques depuis les releases du compagnon (et non celles d'Irminsul)
    edit(src / "update.rs", '.repo_owner("konkers")', f'.repo_owner("{owner}")')
    edit(src / "update.rs", '.repo_name("irminsul")', f'.repo_name("{name}")')

    # 5. Version : celle de la release, pour que la mise à jour automatique sache qu'elle est à jour
    cargo = args.irminsul / "Cargo.toml"
    text = cargo.read_bytes().decode("utf-8")
    text, n = re.subn(r'(?m)^version = "[^"]+"', f'version = "{args.version}"', text, count=1)
    if n != 1:
        sys.exit("ERREUR : version introuvable dans Cargo.toml")
    cargo.write_bytes(text.encode("utf-8"))
    print(f"  version : {args.version}")

    print("OK")


if __name__ == "__main__":
    main()
