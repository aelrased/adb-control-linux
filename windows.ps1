# بناء وتشغيل ADB AppControl على ويندوز
# الاستخدام:
#   powershell -ExecutionPolicy Bypass -File .\windows.ps1
#   powershell -ExecutionPolicy Bypass -File .\windows.ps1 -Run
param(
    [switch]$Run
)

$ErrorActionPreference = "Stop"
Set-Location -Path $PSScriptRoot

Write-Host "==> التحقق من Rust" -ForegroundColor Cyan
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "Rust غير مثبّت. نزّله من https://rustup.rs ثم أعد المحاولة."
}

Write-Host "==> التحقق من adb المضمّن" -ForegroundColor Cyan
$bundled = Join-Path $PSScriptRoot "src-tauri\platform-tools\adb.exe"
if (Test-Path $bundled) {
    Write-Host "    adb مضمّن: $bundled (سيُحزم مع المثبّت)"
} else {
    Write-Host "    تحذير: src-tauri\platform-tools\adb.exe غير موجود." -ForegroundColor Yellow
    Write-Host "    نزّل Platform Tools: https://developer.android.com/tools/releases/platform-tools"
    Write-Host "    أو استخدم زر «تحديد adb» داخل البرنامج."
}

Write-Host "==> تجهيز Tauri CLI" -ForegroundColor Cyan
if (-not (Get-Command cargo-tauri -ErrorAction SilentlyContinue)) {
    cargo install tauri-cli --version "^2" --locked
}

Write-Host "==> بناء الإصدار (NSIS + MSI)" -ForegroundColor Cyan
Push-Location src-tauri
try {
    cargo tauri build
} finally {
    Pop-Location
}

$exe = Join-Path $PSScriptRoot "src-tauri\target\release\adb-appcontrol.exe"
$nsis = Join-Path $PSScriptRoot "src-tauri\target\release\bundle\nsis"
$msi = Join-Path $PSScriptRoot "src-tauri\target\release\bundle\msi"

Write-Host ""
Write-Host "تم البناء:" -ForegroundColor Green
Write-Host "  $exe"
if (Test-Path $nsis) { Get-ChildItem $nsis -Filter *.exe | ForEach-Object { Write-Host "  $($_.FullName)" } }
if (Test-Path $msi) { Get-ChildItem $msi -Filter *.msi | ForEach-Object { Write-Host "  $($_.FullName)" } }

if ($Run -and (Test-Path $exe)) {
    Write-Host "==> تشغيل البرنامج" -ForegroundColor Cyan
    Start-Process $exe
}
