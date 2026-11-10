#!/usr/bin/env bash
# تثبيت ADB AppControl في قائمة تطبيقات لينكس (بدون root)
set -euo pipefail
cd "$(dirname "$0")"

APP=adb-appcontrol
BIN="$PWD/src-tauri/target/release/$APP"
ICON_SRC="$PWD/src-tauri/icons/512x512.png"

if [ ! -x "$BIN" ]; then
  echo "الملف التنفيذي غير موجود. ابنِ أولاً:"
  echo "  cd src-tauri && cargo build --release"
  exit 1
fi

mkdir -p "$HOME/.local/bin" \
         "$HOME/.local/share/applications" \
         "$HOME/.local/share/icons/hicolor/512x512/apps"

ln -sf "$BIN" "$HOME/.local/bin/$APP"
cp "$ICON_SRC" "$HOME/.local/share/icons/hicolor/512x512/apps/$APP.png"

cat > "$HOME/.local/share/applications/$APP.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=ADB Control Linux
Name[ar]=ADB Control Linux
GenericName=Android Device Manager
Comment=ADB Control for Linux - manage Android devices
Comment[ar]=إدارة أجهزة أندرويد والتلفزيون عبر ADB
Exec=$BIN
Icon=$APP
Terminal=false
Categories=Utility;Development;System;
Keywords=adb;android;tv;debloat;apk;
StartupWMClass=$APP
EOF

command -v update-desktop-database >/dev/null && update-desktop-database "$HOME/.local/share/applications" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" >/dev/null 2>&1 || true

echo "تم التثبيت:"
echo "  $HOME/.local/share/applications/$APP.desktop"
echo "  $HOME/.local/share/icons/hicolor/512x512/apps/$APP.png"
echo "  $HOME/.local/bin/$APP"
