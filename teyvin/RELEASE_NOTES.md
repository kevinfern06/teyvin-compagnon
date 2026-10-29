> 🇬🇧 **English version below.**

## ✨ Teyvin se met à jour tout seul quand tu lances le jeu

Teyvin Companion lit ton compte Genshin quand tu passes la porte de l'écran de connexion (persos, armes, artefacts,
matériaux, vœux) et le transmet à [Teyvin](https://teyvin.app) **sur ton ordinateur**. Plus aucun fichier à
exporter ni de lien à copier-coller.

## 🚀 Installation (2 minutes)

1. **Télécharge** `TeyvinCompanion.exe` ci-dessous (dans « Assets ») ; l'autre `.exe`, identique, sert aux mises
   à jour automatiques.
2. **Lance-le.** Windows demande l'autorisation administrateur : clique « Oui ». Elle sert à lire le trafic réseau du
   jeu, avec l'outil intégré à Windows.
   - Si Windows affiche « Windows a protégé votre ordinateur » : clique **Informations complémentaires**, puis
     **Exécuter quand même** (le programme n'est simplement pas signé, comme Irminsul).
3. Dans la fenêtre du compagnon, coche **Démarrer avec Windows** : tu n'auras plus jamais à y penser.
4. Sur [Teyvin](https://teyvin.app/compte), ouvre **Mon compte → Mettre à jour mon compte** et coche
   **J'utilise le compagnon**. Si ton navigateur demande l'accès au réseau local, accepte.
5. **Lance Genshin et passe la porte** de l'écran de connexion. Quelques secondes plus tard, Teyvin affiche
   « ton compte mis à jour » (si sa page était déjà ouverte : recharge-la, ou attends une minute).

## 🎮 Au quotidien

- Le compagnon lit ton compte **quand tu passes la porte** de l'écran de connexion (pas en direct pendant que tu
  joues).
  Après une grosse session de farm : **menu Paimon → déconnexion → reconnexion**, puis **recharge la page de
  Teyvin** (ou attends une minute) : il est à jour.
- **Vœux** : ouvre une fois l'historique des vœux en jeu, puis Teyvin les relit tout seul toutes les 30 minutes
  (le jeu les affiche lui-même avec environ 1 h de retard).
- La fenêtre te dit où tu en es : compagnon prêt → jeu connecté → compte lu → Teyvin à jour.
- Il se met à jour tout seul : une nouvelle version s'installe d'un clic au lancement.

## 🔒 Sécurité et vie privée

- **Rien ne part sur Internet.** Ton compte reste sur ton PC : le relais n'écoute que sur `127.0.0.1`, et
  **seul le site Teyvin** a le droit de le lire (les autres sites sont refusés).
- **Il ne touche ni au jeu ni à Windows** : il lit le trafic réseau du jeu (seulement ses ports), son journal et le
  cache de la page des vœux, en lecture seule. Aucun fichier du jeu n'est modifié.
- **Le lien des vœux** ne sert qu'à **lire** ton historique (il expire en 24 h) : impossible de jouer ou de dépenser
  avec. Il n'est enregistré nulle part.
- **Code ouvert** : tout le code est dans ce dépôt, et chaque `.exe` est compilé ici même par GitHub, à partir du code
  public d'Irminsul. Les nouvelles versions d'Irminsul sont relues avant d'être publiées.

## 🛡️ Et le risque de ban ?

**Aucun ban connu** lié à ce type d'outil, et le risque est très faible :

- le compagnon **ne lit ni ne modifie la mémoire du jeu**, n'injecte rien, n'automatise rien : c'est ce que
  l'anti-triche surveille, et il n'y touche pas ;
- il **n'envoie rien aux serveurs du jeu** : pour HoYoverse, tu es un joueur parfaitement normal ;
- il fonctionne comme Irminsul (utilisé par de nombreux joueurs) et lit les vœux comme le script de Paimon.moe
  (utilisé par des millions de joueurs depuis 2021), sans ban connu.

Par honnêteté : les conditions d'utilisation de HoYoverse interdisent en principe les outils tiers et elles peuvent
évoluer, donc personne ne peut promettre un risque nul. Si tu préfères le risque zéro, l'import par UID de Teyvin
(vitrine publique) reste disponible.

## ❤️ Crédits

Basé sur [Irminsul](https://github.com/konkers/irminsul) de Konkers (licence MIT), version v__UPSTREAM__
(commit `__SHA__`), avec le relais et l'écran de Teyvin. Projet de fan non officiel, sans lien avec HoYoverse ni
avec les auteurs d'Irminsul : merci de signaler les problèmes [ici](https://github.com/kevinfern06/teyvin-compagnon/issues), pas chez eux.

---

# 🇬🇧 English

## ✨ Teyvin updates itself when you launch the game

Teyvin Companion reads your Genshin account when you go through the door on the login screen (characters, weapons,
artifacts, materials, wishes) and hands it to [Teyvin](https://teyvin.app/en) **on your computer**. No more
files to export or links to copy and paste.

## 🚀 Installation (2 minutes)

1. **Download** `TeyvinCompanion.exe` below (under "Assets"); the other, identical `.exe` is used for automatic
   updates.
2. **Run it.** Windows asks for administrator permission: click "Yes". It is used to read the game's network
   traffic, with the tool built into Windows.
   - If Windows shows "Windows protected your PC": click **More info**, then **Run anyway** (the program simply
     isn't signed, just like Irminsul).
3. In the companion window, tick **Start with Windows**: you'll never have to think about it again.
4. On [Teyvin](https://teyvin.app/en/account), open **My account → Update my account** and tick
   **I use the companion**. If your browser asks for local network access, allow it.
5. **Launch Genshin and go through the door** on the login screen. A few seconds later, Teyvin shows
   "your account updated" (if its page was already open: reload it, or wait a minute).

## 🎮 Day to day

- The companion reads your account **when you go through the door** on the login screen (not live while you
  play).
  After a big farming session: **Paimon Menu → log out → log back in**, then **reload the Teyvin page** (or wait
  a minute): it's up to date.
- **Wishes**: open the Wish History in game once, then Teyvin reads it again on its own every 30 minutes
  (the game itself shows wishes with about a 1-hour delay).
- The window tells you where you stand: companion ready → game connected → account read → Teyvin up to date.
- It updates itself: a new version installs in one click at launch.

## 🔒 Security and privacy

- **Nothing goes over the Internet.** Your account stays on your PC: the relay only listens on `127.0.0.1`, and
  **only the Teyvin website** is allowed to read it (other sites are refused).
- **It touches neither the game nor Windows**: it reads the game's network traffic (only its ports), its log and
  the cache of the wish history page, read-only. No game file is modified.
- **The wish link** is only used to **read** your history (it expires after 24 hours): it can't be used to play or
  spend anything. It isn't saved anywhere.
- **Open source**: all the code is in this repository, and every `.exe` is built right here by GitHub, from
  Irminsul's public code. New Irminsul versions are reviewed before being published.

## 🛡️ What about the ban risk?

**No known bans** linked to this kind of tool, and the risk is very low:

- the companion **neither reads nor modifies the game's memory**, injects nothing and automates nothing: that is
  what the anti-cheat watches, and it doesn't touch any of it;
- it **sends nothing to the game's servers**: to HoYoverse, you are a perfectly normal player;
- it works like Irminsul (used by many players) and reads wishes like the Paimon.moe script (used by millions of
  players since 2021), with no known bans.

To be honest: HoYoverse's terms of service forbid third-party tools in principle and they may change, so nobody can
promise zero risk. If you'd rather take no risk at all, Teyvin's UID import (public showcase) is still available.

## ❤️ Credits

Based on [Irminsul](https://github.com/konkers/irminsul) by Konkers (MIT license), version v__UPSTREAM__
(commit `__SHA__`), with Teyvin's relay and screen. Unofficial fan project, not affiliated with HoYoverse or with
the authors of Irminsul: please report issues [here](https://github.com/kevinfern06/teyvin-compagnon/issues), not
to them.
