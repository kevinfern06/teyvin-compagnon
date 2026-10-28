## ✨ Teyvin se met à jour tout seul quand tu joues

Teyvin Compagnon lit ton compte Genshin à chaque connexion au jeu (persos, armes, artefacts, matériaux, vœux) et le
transmet à [Teyvin](https://teyvin.vercel.app) **sur ton ordinateur**. Plus aucun fichier à exporter ni de lien à
copier-coller.

## 🚀 Installation (2 minutes)

1. **Télécharge** `teyvin-compagnon-windows-x64.exe` ci-dessous (dans « Assets »).
2. **Lance-le.** Windows demande l'autorisation administrateur : clique « Oui ». Elle sert à lire le trafic réseau du
   jeu, avec l'outil intégré à Windows.
   - Si Windows affiche « Windows a protégé votre ordinateur » : clique **Informations complémentaires**, puis
     **Exécuter quand même** (le programme n'est simplement pas signé, comme Irminsul).
3. Dans la fenêtre du compagnon, coche **Démarrer avec Windows** : tu n'auras plus jamais à y penser.
4. Sur [Teyvin](https://teyvin.vercel.app/compte), ouvre **Mon compte → Mettre à jour mon compte** et coche
   **J'utilise le compagnon**. Si ton navigateur demande l'accès au réseau local, accepte.
5. **Lance Genshin et connecte-toi.** Quelques secondes plus tard, Teyvin affiche « ton compte mis à jour ».

## 🎮 Au quotidien

- Le compagnon lit ton compte **à la connexion au jeu** (pas en direct pendant que tu joues).
  Après une grosse session de farm : **menu Paimon → déconnexion → reconnexion**, et Teyvin est à jour.
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
