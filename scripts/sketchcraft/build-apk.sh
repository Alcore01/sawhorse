#!/usr/bin/env bash
# sketchcraft E-35 (H-20261117-12) : construire l'application Android de VolumeCraft (APK).
#
# Ce que fait ce script, sur la machine de JC (Linux) :
#   1. trouve un Java 21 à 24 et vérifie le kit Android (SDK) ;
#   2. construit le noyau (npm run wasm) puis l'application web (npm run build) ;
#   3. la copie dans le projet Android (mobile/, Capacitor) et construit l'APK (Gradle) ;
#   4. range l'APK dans ~/sketchcraft/apk/ sous un nom daté (rien n'est écrasé).
#
# Usage :  scripts/sketchcraft/build-apk.sh            (tout)
#          SKIP_WASM=1 scripts/sketchcraft/build-apk.sh (noyau déjà construit)
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${APK_OUT:-$HOME/sketchcraft/apk}"

die() { echo "ERREUR : $*" >&2; exit 1; }

# 1. Java 21 à 24 (Gradle 8.14 du projet Android ne tourne pas sous Java 25 : « Unsupported
#    class file major version 69 », vu chez JC le 17 nov. 2026) --------------------------------
jmajor() { "$1/bin/java" -version 2>&1 | sed -n 's/.*version "\([0-9]*\).*/\1/p' | head -1; }
JDK=""
for c in "${JAVA_HOME:-}" /usr/lib/jvm/java-21-openjdk-amd64 /usr/lib/jvm/java-21-openjdk /usr/lib/jvm/*21* /usr/lib/jvm/*-2[2-4]* "$HOME/android-studio/jbr" "$(dirname "$(dirname "$(readlink -f "$(command -v java 2>/dev/null || echo /nonexistent)")")")"; do
  [[ -n "$c" && -x "$c/bin/java" ]] || continue
  m="$(jmajor "$c")"
  if [[ -n "$m" && "$m" -ge 21 && "$m" -le 24 ]]; then JDK="$c"; break; fi
done
[[ -n "$JDK" ]] || die "il faut un Java 21 (Gradle refuse Java 25). Ubuntu/Debian : sudo apt install openjdk-21-jdk, puis relancer ce script (il le trouvera seul)."
export JAVA_HOME="$JDK"
export PATH="$JAVA_HOME/bin:$PATH"
echo "Java pour Gradle : $JAVA_HOME (version $(jmajor "$JAVA_HOME"))"

# 2. Kit Android ------------------------------------------------------------------------
SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
if [[ ! -d "$SDK/platforms" ]]; then
  cat >&2 <<MSG
ERREUR : le kit Android (SDK) est introuvable (cherché : $SDK).
Le plus simple : installer Android Studio (https://developer.android.com/studio),
l'ouvrir une fois : il installe le kit dans ~/Android/Sdk. Puis relancer ce script.
Sans Android Studio : les « command-line tools » de la même page, puis
  sdkmanager "platform-tools" "platforms;android-36" "build-tools;36.0.0"
et ANDROID_HOME=<dossier du kit> avant ce script.
MSG
  exit 1
fi
export ANDROID_HOME="$SDK"

# 3. Application web --------------------------------------------------------------------
cd "$REPO"
[[ -d node_modules ]] || npm install
if [[ "${SKIP_WASM:-}" != "1" ]]; then npm run wasm; fi
npm run build

# 4. Projet Android ---------------------------------------------------------------------
cd "$REPO/mobile"
[[ -d node_modules ]] || npm install
npx cap sync android
cd android
chmod +x gradlew
./gradlew --no-daemon assembleDebug

APK="$REPO/mobile/android/app/build/outputs/apk/debug/app-debug.apk"
[[ -f "$APK" ]] || die "l'APK n'a pas été produit ($APK)."
mkdir -p "$OUT"
DEST="$OUT/VolumeCraft-$(date +%Y%m%d-%H%M%S).apk"
cp "$APK" "$DEST"
echo
echo "APK prêt : $DEST"
echo "Sur la tablette : copiez-le (câble USB, clé, cloud), ouvrez-le et acceptez l'installation"
echo "d'applications de cette source. Ou, tablette branchée en mode débogage USB :"
echo "  $SDK/platform-tools/adb install -r \"$DEST\""
