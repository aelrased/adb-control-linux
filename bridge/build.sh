#!/usr/bin/env bash
# Build the Android bridge APK without Gradle.
# Requires: JDK 17+, Android SDK build-tools;34.0.0 + platforms;android-34
set -euo pipefail
cd "$(dirname "$0")"

SDK="${ANDROID_HOME:-$HOME/Android/Sdk}"
BT="$SDK/build-tools/34.0.0"
PLATFORM="$SDK/platforms/android-34/android.jar"

[ -x "$BT/aapt2" ] || { echo "aapt2 not found in $BT"; exit 1; }
[ -f "$PLATFORM" ] || { echo "android.jar not found in $PLATFORM"; exit 1; }

OUT=build
rm -rf "$OUT"
mkdir -p "$OUT/classes" "$OUT/dex"

echo "==> javac"
javac -source 8 -target 8 -Xlint:-options \
  -bootclasspath "$PLATFORM:$BT/core-lambda-stubs.jar" \
  -d "$OUT/classes" $(find src -name '*.java')

echo "==> d8"
"$BT/d8" --lib "$PLATFORM" --min-api 24 --output "$OUT/dex" \
  $(find "$OUT/classes" -name '*.class')

echo "==> aapt2 link"
"$BT/aapt2" link -o "$OUT/base.apk" -I "$PLATFORM" \
  --manifest AndroidManifest.xml \
  --min-sdk-version 24 --target-sdk-version 28

echo "==> add dex"
cp "$OUT/dex/classes.dex" "$OUT/classes.dex"
(cd "$OUT" && zip -q base.apk classes.dex)

echo "==> zipalign"
"$BT/zipalign" -f 4 "$OUT/base.apk" "$OUT/aligned.apk"

echo "==> sign"
KS="keystore/bridge.keystore"
mkdir -p keystore
if [ ! -f "$KS" ]; then
  keytool -genkeypair -keystore "$KS" -alias bridge \
    -keyalg RSA -keysize 2048 -validity 10000 \
    -storepass android -keypass android \
    -dname "CN=ADB AppControl Bridge, O=OpenSource, C=US" >/dev/null 2>&1
fi
"$BT/apksigner" sign --ks "$KS" \
  --ks-pass pass:android --key-pass pass:android \
  --out "$OUT/bridge.apk" "$OUT/aligned.apk"

mkdir -p ../src-tauri/assets
cp "$OUT/bridge.apk" ../src-tauri/assets/bridge.apk
ls -lh "$OUT/bridge.apk" ../src-tauri/assets/bridge.apk
echo "OK"
