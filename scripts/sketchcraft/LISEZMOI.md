# Installer sketchcraft (E-24)

sketchcraft tourne dans le navigateur : un petit serveur local (`serve.mjs`, Node seul, à
l'écoute de cet ordinateur uniquement, 127.0.0.1) sert l'application construite (`dist/`), et
le lanceur l'ouvre dans le navigateur par défaut. Fermer la fenêtre du serveur arrête
sketchcraft.

## Linux (la machine où se trouve le code)

```bash
cd ~/sketchcraft/code
bash scripts/sketchcraft/install-linux.sh
```

- Vérifie Node (22 ou plus), Rust, la cible wasm32 et wasm-pack (propose d'installer ce qui
  manque, sans sudo ; dit quand il faut `sudo apt install build-essential`).
- Construit le noyau et l'application.
- Crée le lanceur `~/.local/bin/sketchcraft`, l'entrée « sketchcraft » du menu des
  applications et l'icône du bureau.
- Après un `git pull`, le lanceur reconstruit tout seul avant d'ouvrir l'application.
- Retirer le lanceur et les icônes : `bash scripts/sketchcraft/install-linux.sh --retirer`.

## Windows

Le noyau se compile sous Linux (sous Windows il faudrait Visual Studio Build Tools) ; le
fichier obtenu est le même pour les deux systèmes. Donc :

1. Sous Linux : `bash scripts/sketchcraft/package-windows.sh` crée
   `~/sketchcraft/sketchcraft-windows-<version>.zip`.
2. Copiez ce .zip sur le PC Windows, décompressez-le, double-cliquez sur
   `sketchcraft\installer.cmd`.
   - Node manquant : proposé à l'installation par winget.
   - L'application va dans `%LOCALAPPDATA%\sketchcraft` ; chaque fichier copié est vérifié par
     son empreinte (`EMPREINTES.txt`) ; la version précédente est gardée dans `sauvegardes\`.
   - Raccourcis « sketchcraft » sur le Bureau et dans le menu Démarrer.
3. Retirer les raccourcis : `installer.cmd -Retirer`.

## Tablette Android (APK)

L'application Android emporte VolumeCraft entier : tout tourne sur la tablette, sans réseau
ni hébergement. Elle se construit sur la machine Linux, une fois installés :

- Java 21 : `sudo apt install openjdk-21-jdk` (le Java 25 fourni avec Android Studio est trop récent pour Gradle) ;
- le kit Android : installer Android Studio (developer.android.com/studio) et l'ouvrir une
  fois (il met le kit dans `~/Android/Sdk`).

Puis, dans le dossier du code :

    scripts/sketchcraft/build-apk.sh

L'APK est rangé dans `~/sketchcraft/apk/VolumeCraft-<date>.apk`. Le copier sur la tablette,
l'ouvrir et autoriser l'installation depuis cette source. Au stylet, les touches qui manquent
(Échap, Entrée, axes, Maj, Annuler) sont des boutons au-dessus de la vue.
