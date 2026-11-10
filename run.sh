#!/usr/bin/env bash
# تشغيل ADB AppControl — Linux مباشرة من المصدر
set -euo pipefail
cd "$(dirname "$0")/src-tauri"
exec cargo run "$@"
