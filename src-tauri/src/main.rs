// ADB AppControl — Linux (open-source reimplementation)
// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::Engine as _;
use serde::Serialize;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Manager;

static TMP_COUNTER: AtomicUsize = AtomicUsize::new(0);
static RESOURCE_DIR: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Companion APK that reads real launcher icons and labels through PackageManager.
/// Built from ../bridge (see bridge/build.sh) and embedded at compile time.
const BRIDGE_APK: &[u8] = include_bytes!("../assets/bridge.apk");
const BRIDGE_PKG: &str = "com.adbappcontrol.bridge";

/// Universal Android Debloater NG list (bundled snapshot, same as the Android app).
const UAD_JSON: &str = include_str!("../assets/uad_lists.json");

#[derive(serde::Deserialize)]
struct UadEntry {
    #[serde(default)]
    removal: String,
}

static UAD_MAP: OnceLock<std::collections::HashMap<String, UadEntry>> = OnceLock::new();

fn uad_map() -> &'static std::collections::HashMap<String, UadEntry> {
    UAD_MAP.get_or_init(|| {
        serde_json::from_str::<std::collections::HashMap<String, UadEntry>>(UAD_JSON)
            .unwrap_or_default()
    })
}

/// UDADB-NG safety class: Recommended / Advanced / Expert / Unsafe.
fn uad_removal(package: &str) -> Option<String> {
    uad_map()
        .get(package)
        .map(|e| e.removal.trim().to_string())
        .filter(|r| !r.is_empty())
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Config {
    adb_path: Option<String>,
}

fn config_file() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("adb-appcontrol")
        .join("config.json")
}

fn read_config() -> Config {
    std::fs::read_to_string(config_file())
        .ok()
        .and_then(|raw| serde_json::from_str::<Config>(&raw).ok())
        .unwrap_or_default()
}

fn write_config(config: &Config) -> Result<(), String> {
    let path = config_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, raw).map_err(|e| e.to_string())
}

fn adb_file_name() -> &'static str {
    if cfg!(windows) {
        "adb.exe"
    } else {
        "adb"
    }
}

fn adb_candidates() -> Vec<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();

    if let Ok(path) = std::env::var("ADB_PATH") {
        candidates.push(path.into());
    }
    if let Some(path) = read_config().adb_path {
        candidates.push(path.into());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for rel in [
                adb_file_name().to_string(),
                format!("platform-tools/{}", adb_file_name()),
                format!("platform-tools\\{}", adb_file_name()),
                format!("adb/{}", adb_file_name()),
                format!("adb\\{}", adb_file_name()),
            ] {
                candidates.push(dir.join(rel));
            }
        }
    }
    if let Some(dir) = RESOURCE_DIR.get() {
        candidates.push(dir.join("platform-tools").join(adb_file_name()));
        candidates.push(dir.join("resources").join("platform-tools").join(adb_file_name()));
    }
    if cfg!(debug_assertions) {
        candidates.push(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("platform-tools")
                .join(adb_file_name()),
        );
    }
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(
                std::path::PathBuf::from(local)
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join("adb.exe"),
            );
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            candidates.push(
                std::path::PathBuf::from(profile)
                    .join("AppData")
                    .join("Local")
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join("adb.exe"),
            );
        }
        candidates.push(std::path::PathBuf::from("C:\\platform-tools\\adb.exe"));
    }
    #[cfg(not(windows))]
    {
        candidates.push("/usr/bin/adb".into());
        candidates.push("/usr/local/bin/adb".into());
        candidates.push("/opt/android-sdk/platform-tools/adb".into());
    }
    candidates
}

fn adb_path() -> std::path::PathBuf {
    for candidate in adb_candidates() {
        if candidate.is_file() {
            return candidate;
        }
    }
    std::path::PathBuf::from(adb_file_name())
}

fn adb_bin() -> String {
    adb_path().to_string_lossy().to_string()
}

fn hidden_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn debug_on() -> bool {
    std::env::var("ADBAC_DEBUG").map(|v| v == "1").unwrap_or(false)
}

fn run_adb(args: &[String]) -> Result<String, String> {
    if debug_on() {
        eprintln!("[adb] {} {}", adb_bin(), args.join(" "));
    }
    let out = hidden_command(&adb_bin())
        .args(args)
        .output()
        .map_err(|e| format!("تعذّر تشغيل adb: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        let msg = if stderr.trim().is_empty() { stdout } else { stderr };
        return Err(msg.trim().to_string());
    }
    Ok(if stdout.trim().is_empty() { stderr } else { stdout })
}

async fn adb(args: Vec<String>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || run_adb(&args))
        .await
        .map_err(|e| format!("خطأ داخلي: {e}"))?
}

fn s(v: &str) -> String {
    v.to_string()
}

fn valid_package(pkg: &str) -> bool {
    !pkg.is_empty()
        && pkg.len() < 200
        && pkg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
}

fn home_dir() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn download_dir() -> std::path::PathBuf {
    dirs::download_dir().unwrap_or_else(|| home_dir().join("Downloads"))
}

fn pictures_dir() -> std::path::PathBuf {
    dirs::picture_dir().unwrap_or_else(|| home_dir().join("Pictures"))
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// UTC civil date from a Unix day count (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}

/// File-friendly UTC stamp, e.g. `20261112-220815`.
fn utc_stamp() -> String {
    let secs = timestamp() as i64;
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let rem = secs.rem_euclid(86_400);
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        y,
        m,
        d,
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    )
}

fn cache_dir() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| home_dir().join(".cache"))
        .join("adb-appcontrol")
}

fn safe_serial(serial: &str) -> String {
    serial
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn icons_dir(serial: &str) -> std::path::PathBuf {
    cache_dir().join("icons").join(safe_serial(serial))
}

fn bridge_json_path(serial: &str) -> std::path::PathBuf {
    cache_dir().join(format!("apps-{}.json", safe_serial(serial)))
}

fn bridge_remote_dir() -> String {
    "/sdcard/Android/data/com.adbappcontrol.bridge/files".to_string()
}

#[derive(serde::Deserialize)]
struct BridgeApp {
    package: String,
    label: Option<String>,
    system: Option<bool>,
    version: Option<String>,
}

#[derive(Serialize)]
struct Device {
    serial: String,
    state: String,
    model: Option<String>,
    device: Option<String>,
    product: Option<String>,
    transport: Option<String>,
}

#[derive(Serialize)]
struct AppInfo {
    package: String,
    label: String,
    version: Option<String>,
    apk_path: Option<String>,
    system: bool,
    disabled: bool,
    #[serde(default)]
    removed: bool,
    uad: Option<String>,
}

fn pretty_label(package: &str) -> String {
    let last = package.rsplit('.').next().unwrap_or(package);
    let mut out = String::new();
    let mut new_word = true;
    for ch in last.chars() {
        if ch == '_' || ch == '-' {
            out.push(' ');
            new_word = true;
        } else if new_word {
            out.extend(ch.to_uppercase());
            new_word = false;
        } else {
            out.push(ch);
        }
    }
    if out.trim().is_empty() {
        package.to_string()
    } else {
        out
    }
}

#[tauri::command]
async fn check_adb() -> Result<String, String> {
    let raw = adb(vec![s("version")]).await?;
    Ok(raw.lines().next().unwrap_or("adb").trim().to_string())
}

#[tauri::command]
async fn adb_status() -> Result<serde_json::Value, String> {
    let path = adb_path();
    let version = adb(vec![s("version")])
        .await
        .ok()
        .and_then(|out| out.lines().next().map(|l| l.trim().to_string()));
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "exists": path.is_file(),
        "version": version,
    }))
}

#[tauri::command]
async fn set_adb_path(path: String) -> Result<String, String> {
    let candidate = std::path::PathBuf::from(&path);
    if !candidate.is_file() {
        return Err("الملف غير موجود".into());
    }
    let out = hidden_command(&path)
        .arg("version")
        .output()
        .map_err(|e| format!("تعذّر تشغيل الملف: {e}"))?;
    if !out.status.success() {
        return Err("هذا الملف ليس adb صالحاً".into());
    }
    let version = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or("adb")
        .trim()
        .to_string();
    let mut config = read_config();
    config.adb_path = Some(path);
    write_config(&config)?;
    Ok(version)
}

#[tauri::command]
async fn list_devices() -> Result<Vec<Device>, String> {
    let out = adb(vec![s("devices"), s("-l")]).await?;
    let mut devices = Vec::new();
    for line in out.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('*') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let mut dev = Device {
            serial: parts[0].to_string(),
            state: parts[1].to_string(),
            model: None,
            device: None,
            product: None,
            transport: None,
        };
        for p in &parts[2..] {
            if let Some((k, v)) = p.split_once(':') {
                match k {
                    "model" => dev.model = Some(v.to_string()),
                    "device" => dev.device = Some(v.to_string()),
                    "product" => dev.product = Some(v.to_string()),
                    "transport_id" => dev.transport = Some(v.to_string()),
                    _ => {}
                }
            }
        }
        devices.push(dev);
    }
    Ok(devices)
}

#[tauri::command]
async fn connect_device(addr: String) -> Result<String, String> {
    let addr = addr.trim().to_string();
    if addr.is_empty() || addr.contains(char::is_whitespace) {
        return Err("عنوان غير صالح".into());
    }
    let target = if addr.contains(':') { addr } else { format!("{addr}:5555") };
    adb(vec![s("connect"), target]).await
}

#[tauri::command]
async fn disconnect_device(addr: String) -> Result<String, String> {
    adb(vec![s("disconnect"), addr]).await
}

#[tauri::command]
async fn device_info(serial: String) -> Result<serde_json::Value, String> {
    let props = [
        "ro.product.manufacturer",
        "ro.product.model",
        "ro.product.device",
        "ro.build.version.release",
        "ro.build.version.sdk",
        "ro.product.cpu.abi",
        "ro.serialno",
        "ro.build.fingerprint",
    ];
    let mut map = serde_json::Map::new();
    for prop in props {
        let out = adb(vec![s("-s"), serial.clone(), s("shell"), s("getprop"), s(prop)]).await;
        map.insert(
            prop.to_string(),
            serde_json::Value::String(out.unwrap_or_default().trim().to_string()),
        );
    }
    let battery = adb(vec![
        s("-s"),
        serial.clone(),
        s("shell"),
        s("dumpsys"),
        s("battery"),
    ])
    .await
    .unwrap_or_default();
    let mut level = String::new();
    for line in battery.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("level:") {
            level = v.trim().to_string();
        }
    }
    map.insert("battery".into(), serde_json::Value::String(level));
    Ok(serde_json::Value::Object(map))
}

#[derive(Serialize)]
struct ProcessInfo {
    pid: u32,
    name: String,
    rss_mb: f64,
}

/// RAM usage from /proc/meminfo, in decimal MB (same numbers as ADB AppControl).
#[tauri::command]
async fn device_memory(serial: String) -> Result<serde_json::Value, String> {
    let raw = adb(vec![
        s("-s"),
        serial,
        s("shell"),
        s("cat"),
        s("/proc/meminfo"),
    ])
    .await?;
    let mut total_kb: u64 = 0;
    let mut available_kb: u64 = 0;
    let mut free_kb: u64 = 0;
    let mut cached_kb: u64 = 0;
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next().unwrap_or("");
        let value: u64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        match key {
            "MemTotal:" => total_kb = value,
            "MemAvailable:" => available_kb = value,
            "MemFree:" => free_kb = value,
            "Cached:" => cached_kb = value,
            _ => {}
        }
    }
    if total_kb == 0 {
        return Err("تعذّر قراءة الذاكرة من الجهاز".into());
    }
    // Older kernels lack MemAvailable.
    let available = if available_kb > 0 {
        available_kb
    } else {
        free_kb + cached_kb
    };
    let used_kb = total_kb.saturating_sub(available);
    Ok(serde_json::json!({
        "total_mb": total_kb / 1000,
        "used_mb": used_kb / 1000,
        "free_mb": available / 1000,
        "percent": if total_kb > 0 { ((used_kb as f64 / total_kb as f64) * 1000.0).round() / 10.0 } else { 0.0 },
    }))
}

/// Running processes with resident memory, filled by `ps` (no root needed).
#[tauri::command]
async fn list_processes(serial: String) -> Result<Vec<ProcessInfo>, String> {
    let raw = match adb(vec![
        s("-s"),
        serial.clone(),
        s("shell"),
        s("ps"),
        s("-A"),
        s("-o"),
        s("PID,RSS,NAME"),
    ])
    .await
    {
        Ok(out) => out,
        Err(_) => {
            adb(vec![
                s("-s"),
                serial,
                s("shell"),
                s("ps"),
                s("-A"),
            ])
            .await?
        }
    };

    let mut procs: Vec<ProcessInfo> = Vec::new();
    for line in raw.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let parsed = if let Ok(pid) = parts[0].parse::<u32>() {
            // `ps -o PID,RSS,NAME`
            parts[1]
                .parse::<u64>()
                .ok()
                .map(|rss| (pid, rss, parts[2..].join(" ")))
        } else if parts.len() >= 8 {
            // Fallback: `ps -A` → USER PID PPID VSZ RSS ... NAME
            parts[1]
                .parse::<u32>()
                .ok()
                .zip(parts[4].parse::<u64>().ok())
                .map(|(pid, rss)| (pid, rss, parts[parts.len() - 1].to_string()))
        } else {
            None
        };
        let Some((pid, rss_kb, name)) = parsed else {
            continue;
        };
        if rss_kb == 0 || name.starts_with('[') || name.is_empty() || name == "NAME" {
            continue;
        }
        procs.push(ProcessInfo {
            pid,
            name,
            rss_mb: (rss_kb as f64 / 1024.0 * 10.0).round() / 10.0,
        });
    }
    procs.sort_by(|a, b| b.rss_mb.partial_cmp(&a.rss_mb).unwrap_or(std::cmp::Ordering::Equal));
    procs.truncate(300);
    Ok(procs)
}

/// Force-stop an app process (`am force-stop`). System daemons need root and
/// are rejected by the package-name validation below.
#[tauri::command]
async fn kill_process(serial: String, name: String) -> Result<String, String> {
    let pkg = name.split(':').next().unwrap_or(&name).to_string();
    if !valid_package(&pkg) {
        return Err("هذا الإجراء متاح لتطبيقات أندرويد فقط (اسم حزمة صالح)".into());
    }
    let out = adb(vec![
        s("-s"),
        serial,
        s("shell"),
        s("am"),
        s("force-stop"),
        s(&pkg),
    ])
    .await?;
    let text = out.trim();
    if text.contains("Error") || text.contains("Exception") {
        return Err(text.to_string());
    }
    Ok(format!("تم إيقاف {pkg}"))
}

#[tauri::command]
async fn list_apps(serial: String) -> Result<Vec<AppInfo>, String> {
    let base = vec![s("-s"), serial.clone()];

    let mut cmd = base.clone();
    cmd.extend([s("shell"), s("pm"), s("list"), s("packages"), s("-f")]);
    let packages_raw = adb(cmd).await?;

    let mut dcmd = base.clone();
    dcmd.extend([s("shell"), s("pm"), s("list"), s("packages"), s("-d")]);
    let disabled_raw = adb(dcmd).await.unwrap_or_default();

    let mut ucmd = base.clone();
    ucmd.extend([s("shell"), s("pm"), s("list"), s("packages"), s("-3")]);
    let user_raw = adb(ucmd).await.unwrap_or_default();

    let disabled: std::collections::HashSet<String> = disabled_raw
        .lines()
        .filter_map(|l| l.trim().strip_prefix("package:"))
        .map(|p| p.trim().to_string())
        .collect();
    let user: std::collections::HashSet<String> = user_raw
        .lines()
        .filter_map(|l| l.trim().strip_prefix("package:"))
        .map(|p| p.trim().to_string())
        .collect();

    let mut versions: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut vcmd = base.clone();
    vcmd.extend([
        s("shell"),
        s(
            "dumpsys package packages | grep -E \"Package \\[|versionName=|versionCode=\"",
        ),
    ]);
    if let Ok(vraw) = adb(vcmd).await {
        let mut current: Option<String> = None;
        for line in vraw.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("Package [") {
                if let Some(end) = rest.find(']') {
                    current = Some(rest[..end].to_string());
                }
            } else if let Some(v) = line.strip_prefix("versionName=") {
                if let Some(pkg) = &current {
                    versions.insert(pkg.clone(), v.trim().to_string());
                }
            }
        }
    }

    let mut apps = Vec::new();
    for line in packages_raw.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("package:") else { continue };
        let Some((path, pkg)) = rest.rsplit_once('=') else { continue };
        let pkg = pkg.trim().to_string();
        let path = path.trim().to_string();
        if pkg.is_empty() {
            continue;
        }
        let system = !user.contains(&pkg);
        apps.push(AppInfo {
            label: pretty_label(&pkg),
            version: versions.get(&pkg).cloned(),
            apk_path: Some(path),
            system,
            disabled: disabled.contains(&pkg),
            removed: false,
            uad: uad_removal(&pkg),
            package: pkg,
        });
    }
    // Merge real labels / system flags / versions exported by the bridge app.
    if let Ok(raw) = std::fs::read_to_string(bridge_json_path(&serial)) {
        if let Ok(list) = serde_json::from_str::<Vec<BridgeApp>>(&raw) {
            let map: std::collections::HashMap<String, BridgeApp> =
                list.into_iter().map(|b| (b.package.clone(), b)).collect();
            for app in apps.iter_mut() {
                if let Some(b) = map.get(&app.package) {
                    if let Some(label) = &b.label {
                        if !label.trim().is_empty() && label != "null" {
                            app.label = label.clone();
                        }
                    }
                    if let Some(system) = b.system {
                        app.system = system;
                    }
                    if let Some(version) = &b.version {
                        if !version.trim().is_empty() && version != "null" {
                            app.version = Some(version.clone());
                        }
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    if debug_on() {
        eprintln!("[apps] {} packages ({} system)", apps.len(), apps.iter().filter(|a| a.system).count());
    }
    Ok(apps)
}

/// Packages uninstalled for user 0 but still present on the device (restorable).
/// Mirrors the Android app: `pm list packages -u --user 0` minus installed ones.
#[tauri::command]
async fn list_removed_apps(serial: String) -> Result<Vec<AppInfo>, String> {
    let all = adb(vec![
        s("-s"),
        serial.clone(),
        s("shell"),
        s("pm"),
        s("list"),
        s("packages"),
        s("-u"),
        s("--user"),
        s("0"),
    ])
    .await?;
    let installed = adb(vec![
        s("-s"),
        serial.clone(),
        s("shell"),
        s("pm"),
        s("list"),
        s("packages"),
        s("--user"),
        s("0"),
    ])
    .await
    .unwrap_or_default();

    let installed: std::collections::HashSet<String> = installed
        .lines()
        .filter_map(|l| l.trim().strip_prefix("package:"))
        .map(|p| p.trim().to_string())
        .collect();

    let labels: std::collections::HashMap<String, BridgeApp> =
        std::fs::read_to_string(bridge_json_path(&serial))
            .ok()
            .and_then(|raw| serde_json::from_str::<Vec<BridgeApp>>(&raw).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|b| (b.package.clone(), b))
            .collect();

    let mut apps = Vec::new();
    for line in all.lines() {
        let Some(rest) = line.trim().strip_prefix("package:") else {
            continue;
        };
        let pkg = rest.trim().to_string();
        if pkg.is_empty() || installed.contains(&pkg) {
            continue;
        }
        let mut app = AppInfo {
            label: pretty_label(&pkg),
            version: None,
            apk_path: None,
            system: false,
            disabled: false,
            removed: true,
            uad: uad_removal(&pkg),
            package: pkg.clone(),
        };
        if let Some(b) = labels.get(&pkg) {
            if let Some(label) = &b.label {
                if !label.trim().is_empty() && label != "null" {
                    app.label = label.clone();
                }
            }
            if let Some(version) = &b.version {
                if !version.trim().is_empty() && version != "null" {
                    app.version = Some(version.clone());
                }
            }
        }
        apps.push(app);
    }
    apps.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    Ok(apps)
}

/// One permission entry shown by the permissions manager.
#[derive(serde::Serialize)]
struct PermissionInfo {
    name: String,
    granted: bool,
    /// runtime / install / requested
    kind: String,
    dangerous: bool,
    group: Option<String>,
    #[serde(default)]
    flags: Vec<String>,
}

struct PermState {
    granted: bool,
    kind: String,
    flags: Vec<String>,
}

fn valid_permission(perm: &str) -> bool {
    !perm.is_empty()
        && perm.len() < 200
        && perm.contains('.')
        && perm
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
}

/// Parse `pm list permissions -g -d` into permission -> group.
fn parse_dangerous_permissions(raw: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let mut group: Option<String> = None;
    for line in raw.lines() {
        let t = line.trim();
        if let Some(g) = t.strip_prefix("group:") {
            group = Some(g.trim().to_string());
        } else if let Some(p) = t.strip_prefix("permission:") {
            let name = p.trim().to_string();
            if valid_permission(&name) {
                if let Some(g) = &group {
                    map.insert(name, g.clone());
                }
            }
        }
    }
    map
}

/// Parse `dumpsys package <pkg>` for requested / install / runtime permissions.
/// Runtime permissions are read from the `User 0:` block only.
fn parse_package_permissions(
    dump: &str,
    dangerous: &std::collections::HashMap<String, String>,
) -> Vec<PermissionInfo> {
    let mut requested: Vec<String> = Vec::new();
    let mut states: std::collections::HashMap<String, PermState> = std::collections::HashMap::new();
    let mut section = String::new();
    let mut user0 = true;
    let mut pending: Option<(String, bool, String, Vec<String>)> = None;

    fn finish(
        states: &mut std::collections::HashMap<String, PermState>,
        pending: &mut Option<(String, bool, String, Vec<String>)>,
    ) {
        if let Some((name, granted, kind, flags)) = pending.take() {
            let entry = states.entry(name).or_insert_with(|| PermState {
                granted,
                kind: kind.clone(),
                flags: Vec::new(),
            });
            if granted {
                entry.granted = true;
            }
            if entry.kind != "runtime" {
                entry.kind = kind;
            }
            for f in flags {
                if !entry.flags.contains(&f) {
                    entry.flags.push(f);
                }
            }
        }
    }

    for raw in dump.lines() {
        let t = raw.trim();
        if pending.is_some() {
            let (name, granted, kind, mut flags) = pending.take().unwrap();
            let cleaned = t.trim_end_matches(']').trim();
            if !cleaned.is_empty() {
                for f in cleaned.split('|').map(str::trim).filter(|f| !f.is_empty()) {
                    if !flags.iter().any(|x| x == f) {
                        flags.push(f.to_string());
                    }
                }
            }
            pending = Some((name, granted, kind, flags));
            if t.ends_with(']') {
                finish(&mut states, &mut pending);
            }
            continue;
        }

        if let Some(rest) = t.strip_prefix("User ") {
            if let Some(idx) = rest.find(':') {
                user0 = rest[..idx].trim() == "0";
                section.clear();
                continue;
            }
        }
        if t == "requested permissions:" {
            section = "requested".into();
            continue;
        }
        if t == "install permissions:" {
            section = "install".into();
            continue;
        }
        if t == "runtime permissions:" {
            section = if user0 { "runtime".into() } else { "skip".into() };
            continue;
        }
        if t.ends_with(':')
            && !t.starts_with("android.")
            && !t.starts_with("com.")
            && !t.starts_with("org.")
        {
            section.clear();
            continue;
        }

        if section == "requested" {
            if valid_permission(t) && !requested.iter().any(|p| p == t) {
                requested.push(t.to_string());
            }
            continue;
        }
        if section != "install" && section != "runtime" {
            continue;
        }
        let Some((name, rest)) = t.split_once(": granted=") else { continue };
        if !valid_permission(name) {
            continue;
        }
        let granted = rest.trim_start().starts_with("true");
        let kind = if section == "runtime" { "runtime" } else { "install" }.to_string();
        let flags_idx = rest.find("flags=[");
        let mut collected: Vec<String> = Vec::new();
        let is_open = if let Some(idx) = flags_idx {
            let after = &rest[idx + "flags=[".len()..];
            let body = after.trim_end();
            let open = !body.contains(']');
            for f in body.trim_end_matches(']').split('|').map(str::trim) {
                if !f.is_empty() {
                    collected.push(f.to_string());
                }
            }
            open
        } else {
            false
        };
        if is_open {
            pending = Some((name.to_string(), granted, kind, collected));
        } else {
            let entry = states.entry(name.to_string()).or_insert_with(|| PermState {
                granted,
                kind: kind.clone(),
                flags: Vec::new(),
            });
            if granted {
                entry.granted = true;
            }
            if entry.kind != "runtime" {
                entry.kind = kind;
            }
            for f in collected {
                if !entry.flags.contains(&f) {
                    entry.flags.push(f);
                }
            }
        }
    }
    finish(&mut states, &mut pending);

    let mut names = requested;
    for name in states.keys() {
        if !names.iter().any(|p| p == name) {
            names.push(name.clone());
        }
    }

    let mut out = Vec::new();
    for name in names {
        let state = states.remove(&name);
        let (granted, kind, flags) = match state {
            Some(s) => (s.granted, s.kind, s.flags),
            None => (false, "requested".to_string(), Vec::new()),
        };
        let group = dangerous.get(&name).cloned();
        out.push(PermissionInfo {
            dangerous: group.is_some(),
            group,
            name,
            granted,
            kind,
            flags,
        });
    }
    out.sort_by(|a, b| {
        let rank = |k: &str| match k {
            "runtime" => 0,
            "install" => 1,
            _ => 2,
        };
        rank(&a.kind)
            .cmp(&rank(&b.kind))
            .then_with(|| b.dangerous.cmp(&a.dangerous))
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

fn list_permissions_blocking(serial: &str, package: &str) -> Result<Vec<PermissionInfo>, String> {
    if !valid_package(package) {
        return Err("اسم حزمة غير صالح".into());
    }
    let installed = run_adb(&[
        s("-s"),
        s(serial),
        s("shell"),
        s("pm"),
        s("list"),
        s("packages"),
        s("--user"),
        s("0"),
    ])?;
    if !installed
        .lines()
        .any(|l| l.trim() == format!("package:{package}"))
    {
        return Err(format!("التطبيق {package} غير مثبّت للمستخدم 0"));
    }
    let dump = run_adb(&[
        s("-s"),
        s(serial),
        s("shell"),
        s("dumpsys"),
        s("package"),
        s(package),
    ])?;
    if !dump.contains("requested permissions:") && !dump.contains("runtime permissions:") {
        return Err(format!("لم يتم العثور على صلاحيات للحزمة {package}"));
    }
    let dangerous_raw = run_adb(&[
        s("-s"),
        s(serial),
        s("shell"),
        s("pm"),
        s("list"),
        s("permissions"),
        s("-g"),
        s("-d"),
    ])
    .unwrap_or_default();
    let dangerous = parse_dangerous_permissions(&dangerous_raw);
    Ok(parse_package_permissions(&dump, &dangerous))
}

#[tauri::command]
async fn list_permissions(serial: String, package: String) -> Result<Vec<PermissionInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || list_permissions_blocking(&serial, &package))
        .await
        .map_err(|e| format!("خطأ داخلي: {e}"))?
}

fn set_permissions_blocking(
    serial: &str,
    package: &str,
    permissions: &[String],
    granted: bool,
) -> Result<serde_json::Value, String> {
    if !valid_package(package) {
        return Err("اسم حزمة غير صالح".into());
    }
    let verb = if granted { "grant" } else { "revoke" };
    let mut ok = 0u32;
    let mut failed = Vec::new();
    for perm in permissions {
        if !valid_permission(perm) {
            failed.push(serde_json::json!({
                "permission": perm,
                "error": "اسم صلاحية غير صالح"
            }));
            continue;
        }
        let res = run_adb(&[
            s("-s"),
            s(serial),
            s("shell"),
            s("pm"),
            s(verb),
            s(package),
            s(perm),
        ]);
        match res {
            Ok(out) => {
                let text = out.trim();
                if text.contains("Exception")
                    || text.contains("Error")
                    || text.contains("Failure")
                    || text.contains("not a changeable")
                {
                    failed.push(serde_json::json!({
                        "permission": perm,
                        "error": text
                    }));
                } else {
                    ok += 1;
                }
            }
            Err(e) => failed.push(serde_json::json!({
                "permission": perm,
                "error": e
            })),
        }
    }
    Ok(serde_json::json!({ "ok": ok, "failed": failed }))
}

#[tauri::command]
async fn set_permissions(
    serial: String,
    package: String,
    permissions: Vec<String>,
    granted: bool,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        set_permissions_blocking(&serial, &package, &permissions, granted)
    })
    .await
    .map_err(|e| format!("خطأ داخلي: {e}"))?
}

#[tauri::command]
async fn app_action(serial: String, package: String, action: String) -> Result<String, String> {
    if !valid_package(&package) {
        return Err("اسم حزمة غير صالح".into());
    }
    if action == "uninstall_deep" {
        // Best-effort factory removal, identical to the Android version:
        // drop system updates, stop the app, then uninstall for user 0.
        let _ = adb(vec![
            s("-s"),
            serial.clone(),
            s("shell"),
            s("pm"),
            s("uninstall-system-updates"),
            s(&package),
        ])
        .await;
        let _ = adb(vec![
            s("-s"),
            serial.clone(),
            s("shell"),
            s("am"),
            s("force-stop"),
            s(&package),
        ])
        .await;
        let out = adb(vec![
            s("-s"),
            serial.clone(),
            s("shell"),
            s("pm"),
            s("uninstall"),
            s("--user"),
            s("0"),
            s(&package),
        ])
        .await
        .unwrap_or_default();

        // `-u` keeps listing system packages whose /system copy remains.
        let left = adb(vec![
            s("-s"),
            serial,
            s("shell"),
            s("pm"),
            s("list"),
            s("packages"),
            s("-u"),
            s("--user"),
            s("0"),
        ])
        .await
        .unwrap_or_default();
        let still_present = left
            .lines()
            .any(|l| l.trim() == format!("package:{package}"));
        if !still_present {
            return Ok(format!("تم الحذف النهائي: {package}"));
        }
        if out.contains("Failure") || out.contains("Error") || out.contains("Exception") {
            return Err(format!(
                "أُزيلت تحديثات النظام، لكن تعذّر الحذف الكامل — نسخة /system تتطلب روت.\n{}",
                out.trim()
            ));
        }
        return Ok(format!(
            "أُزيل {package} للمستخدم 0 — نسخة المصنع تبقى في النظام (إزالتها الكاملة تتطلب روت)"
        ));
    }
    if action == "clear_cache" {
        let script = format!(
            "sdk=$(getprop ro.build.version.sdk); if [ \"$sdk\" -ge 30 ]; then pm clear --cache-only --user 0 {package}; else pm trim-caches 999999999999; echo TRIM_ALL; fi"
        );
        let out = adb(vec![s("-s"), serial, s("shell"), script]).await?;
        let text = out.trim();
        if text.contains("Failure") || text.contains("Failed") || text.contains("Exception") {
            return Err(text.to_string());
        }
        if text.contains("TRIM_ALL") {
            return Ok("تم تنظيف كاش جميع التطبيقات (مسح كاش تطبيق واحد متاح من Android 11+)".into());
        }
        return Ok(text.to_string());
    }

    let mut args = vec![
        s("-s"),
        serial,
        s("shell"),
        s("pm"),
    ];
    match action.as_str() {
        "disable" => args.extend([s("disable-user"), s("--user"), s("0"), package]),
        "enable" => args.extend([s("enable"), package]),
        "uninstall" => args.extend([s("uninstall"), s("--user"), s("0"), package]),
        "uninstall_keep" => args.extend([
            s("uninstall"),
            s("-k"),
            s("--user"),
            s("0"),
            package,
        ]),
        "clear_data" => args.extend([s("clear"), s("--user"), s("0"), package]),
        "clear_cache" => args.extend([s("clear"), s("--cache-only"), s("--user"), s("0"), package]),
        "restore" => args.extend([s("install-existing"), s("--user"), s("0"), package]),
        _ => return Err("إجراء غير معروف".into()),
    }
    let out = adb(args).await?;
    let t = out.trim();
    if t.contains("Failure") || t.contains("Exception") || t.contains("Error") {
        return Err(t.to_string());
    }
    Ok(if t.is_empty() { "Success".into() } else { t.to_string() })
}

fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .collect();
    if cleaned.is_empty() {
        "part.apk".to_string()
    } else {
        cleaned
    }
}

/// File base name used when saving an app, matching ADB AppControl:
/// `<App label> (<package>) [v.<version>]`.
fn apk_base_name(label: Option<&str>, package: &str, version: Option<&str>) -> String {
    let mut cleaned = String::new();
    for ch in label.unwrap_or("").chars() {
        if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            cleaned.push(' ');
        } else {
            cleaned.push(ch);
        }
    }
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut base = if cleaned.is_empty() {
        package.to_string()
    } else {
        format!("{cleaned} ({package})")
    };
    let version = version.unwrap_or("").trim();
    if !version.is_empty() {
        base = format!("{base} [v.{version}]");
    }
    if base.chars().count() > 150 {
        base = base.chars().take(150).collect();
    }
    let trimmed = base.trim_end_matches([' ', '.']).to_string();
    if trimmed.is_empty() {
        package.to_string()
    } else {
        trimmed
    }
}

fn adb_pull(serial: &str, remote: &str, dest: &std::path::Path) -> Result<(), String> {
    let out = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("pull"),
        s(remote),
        dest.to_string_lossy().to_string(),
    ])?;
    let lower = out.to_lowercase();
    if lower.contains("file pulled")
        || lower.contains("files pulled")
        || lower.contains("skipped")
    {
        Ok(())
    } else {
        Err(out.trim().to_string())
    }
}

/// Android 11+ often denies pulling `/data/app/...` directly. Copy the whole
/// install directory to shared storage, pull that, then remove the copy.
fn pull_app_dir_via_storage(
    serial: &str,
    remote_dir: &str,
    tmp: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let dir_name = remote_dir
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("app");
    let staging = format!("/storage/emulated/0/{dir_name}");
    let _ = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("rm"),
        s("-R"),
        s(&staging),
    ]);
    run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("cp"),
        s("-r"),
        s(remote_dir),
        s("/storage/emulated/0"),
    ])?;
    let local = tmp.join(dir_name);
    let pulled = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("pull"),
        s(&staging),
        s(&local.to_string_lossy()),
    ]);
    let _ = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("rm"),
        s("-R"),
        s(&staging),
    ]);
    if let Err(e) = pulled {
        return Err(e);
    }
    if !local.is_dir() {
        return Err("تعذر نسخ ملفات APK من الجهاز".into());
    }
    Ok(local)
}

fn pull_package_parts(
    serial: &str,
    package: &str,
    tmp: &std::path::Path,
) -> Result<Vec<(String, std::path::PathBuf)>, String> {
    let paths = run_adb(&[
        s("-s"),
        s(serial),
        s("shell"),
        s("pm"),
        s("path"),
        s(package),
    ])?;

    let remotes: Vec<String> = paths
        .lines()
        .filter_map(|line| line.trim().strip_prefix("package:"))
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    if remotes.is_empty() {
        return Err("لم يتم العثور على مسار APK".into());
    }

    // Pull every original APK, keeping its original file name.
    let mut parts: Vec<(String, std::path::PathBuf)> = Vec::new();
    for remote in &remotes {
        let name = remote.rsplit('/').next().unwrap_or("part.apk").to_string();
        let dest = tmp.join(&name);
        if adb_pull(serial, remote, &dest).is_ok() {
            parts.push((name, dest));
        } else {
            parts.clear();
            break;
        }
    }

    if parts.len() != remotes.len() {
        // Direct pull failed: stage the install directory on shared storage.
        let remote_dir = remotes[0]
            .rsplit_once('/')
            .map(|(d, _)| d.to_string())
            .unwrap_or_default();
        if remote_dir.is_empty() {
            return Err("تعذر قراءة مجلد التطبيق على الجهاز".into());
        }
        let staged = pull_app_dir_via_storage(serial, &remote_dir, tmp)?;
        parts.clear();
        for remote in &remotes {
            let name = remote.rsplit('/').next().unwrap_or("part.apk").to_string();
            let src = staged.join(&name);
            if !src.is_file() {
                return Err(format!("لم يتم العثور على الملف {name} على الجهاز"));
            }
            parts.push((name, src));
        }
    }
    Ok(parts)
}

fn extract_apk_blocking(
    serial: &str,
    package: &str,
    label: Option<&str>,
    version: Option<&str>,
) -> Result<String, String> {
    if !valid_package(package) {
        return Err("اسم حزمة غير صالح".into());
    }

    let dir = download_dir().join("adb-appcontrol-apks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let counter = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = cache_dir().join("tmp").join(format!(
        "{}-{}-{counter}",
        safe_serial(package),
        timestamp()
    ));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    let base = apk_base_name(label, package, version);

    let result = (|| -> Result<String, String> {
        let parts = pull_package_parts(serial, package, &tmp)?;

        if parts.len() == 1 {
            let dest = dir.join(format!("{base}.apk"));
            let _ = std::fs::remove_file(&dest);
            std::fs::copy(&parts[0].1, &dest).map_err(|e| e.to_string())?;
            return Ok(dest.to_string_lossy().to_string());
        }

        // Split package (App Bundle): archive the untouched original APK files,
        // keeping their original names, exactly like ADB AppControl.
        let dest = dir.join(format!("{base}.apks"));
        let file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .large_file(true);
        for (name, path) in &parts {
            writer
                .start_file(name.clone(), options)
                .map_err(|e| e.to_string())?;
            let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut writer).map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
        Ok(format!(
            "{}  ({} ملفات APK أصلية)",
            dest.display(),
            parts.len()
        ))
    })();

    let _ = std::fs::remove_dir_all(&tmp);
    result
}

#[tauri::command]
async fn extract_apk(
    serial: String,
    package: String,
    label: Option<String>,
    version: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        extract_apk_blocking(
            &serial,
            &package,
            label.as_deref(),
            version.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("خطأ داخلي: {e}"))?
}

#[derive(serde::Deserialize)]
struct BackupTarget {
    package: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

/// Create `backup-<stamp>.zip` containing every selected app's untouched APKs
/// under a per-app folder, plus a `backup.json` manifest.
fn backup_apps_blocking(
    serial: &str,
    targets: Vec<BackupTarget>,
) -> Result<serde_json::Value, String> {
    if targets.is_empty() {
        return Err("لم يتم تحديد تطبيقات للنسخ الاحتياطي".into());
    }

    let stamp = utc_stamp();
    let dir = download_dir().join("adb-appcontrol-backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let zip_path = dir.join(format!("backup-{stamp}.zip"));

    let counter = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = cache_dir()
        .join("backup")
        .join(format!("{stamp}-{counter}"));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    let result = (|| -> Result<serde_json::Value, String> {
        use std::io::Write as _;

        let file = std::fs::File::create(&zip_path).map_err(|e| e.to_string())?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .large_file(true);

        let mut apps_meta: Vec<serde_json::Value> = Vec::new();
        let mut failures: Vec<String> = Vec::new();
        let mut used_folders: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut ok = 0u32;

        for (index, target) in targets.iter().enumerate() {
            if !valid_package(&target.package) {
                failures.push(format!("{}: اسم حزمة غير صالح", target.package));
                continue;
            }
            let app_tmp = tmp.join(format!("app-{index}"));
            if let Err(e) = std::fs::create_dir_all(&app_tmp) {
                failures.push(format!("{}: {e}", target.package));
                continue;
            }

            let pulled = pull_package_parts(serial, &target.package, &app_tmp);
            let parts = match pulled {
                Ok(parts) => parts,
                Err(e) => {
                    failures.push(format!("{}: {e}", target.package));
                    let _ = std::fs::remove_dir_all(&app_tmp);
                    continue;
                }
            };

            let base = apk_base_name(
                target.label.as_deref(),
                &target.package,
                target.version.as_deref(),
            );
            let mut folder = base.clone();
            let mut n = 2;
            while !used_folders.insert(folder.clone()) {
                folder = format!("{base} ({n})");
                n += 1;
            }

            let mut files: Vec<String> = Vec::new();
            for (name, path) in &parts {
                writer
                    .start_file(format!("{folder}/{name}"), options)
                    .map_err(|e| e.to_string())?;
                let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
                std::io::copy(&mut f, &mut writer).map_err(|e| e.to_string())?;
                files.push(name.clone());
            }

            apps_meta.push(serde_json::json!({
                "package": target.package,
                "label": target.label,
                "version": target.version,
                "folder": folder,
                "files": files
            }));
            ok += 1;
            let _ = std::fs::remove_dir_all(&app_tmp);
        }

        if ok == 0 {
            let detail = if failures.is_empty() {
                String::new()
            } else {
                format!("\n{}", failures.join("\n"))
            };
            return Err(format!("تعذّر سحب أي تطبيق للنسخة الاحتياطية{detail}"));
        }

        let meta = serde_json::json!({
            "format": 1,
            "app": "ADB Control",
            "created": stamp,
            "created_unix": timestamp(),
            "device": serial,
            "apps": apps_meta
        });
        let raw = serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?;
        writer
            .start_file("backup.json", options)
            .map_err(|e| e.to_string())?;
        writer.write_all(&raw).map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "path": zip_path.to_string_lossy().to_string(),
            "ok": ok,
            "failed": failures
        }))
    })();

    let _ = std::fs::remove_dir_all(&tmp);
    if result.is_err() {
        let _ = std::fs::remove_file(&zip_path);
    }
    result
}

#[tauri::command]
async fn backup_apps(
    serial: String,
    apps: Vec<BackupTarget>,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || backup_apps_blocking(&serial, apps))
        .await
        .map_err(|e| format!("خطأ داخلي: {e}"))?
}

#[tauri::command]
async fn presets_dir() -> Result<String, String> {
    let dir = download_dir().join("adb-appcontrol-presets");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.to_string_lossy().to_string())
}

#[derive(serde::Deserialize)]
struct PresetEntry {
    package: String,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    removed: bool,
}

fn save_preset_blocking(
    path: &str,
    apps: Vec<PresetEntry>,
    stamp: Option<&str>,
    comment: Option<&str>,
) -> Result<String, String> {
    if path.trim().is_empty() {
        return Err("مسار ملف غير صالح".into());
    }
    let mut seen = std::collections::HashSet::new();
    let mut enabled: Vec<String> = Vec::new();
    let mut disabled: Vec<String> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    for app in apps {
        let package = app.package.trim().to_string();
        if !valid_package(&package) || !seen.insert(package.clone()) {
            continue;
        }
        if app.removed {
            removed.push(package);
        } else if app.disabled {
            disabled.push(package);
        } else {
            enabled.push(package);
        }
    }
    let total = enabled.len() + disabled.len() + removed.len();
    if total == 0 {
        return Err("لا توجد حزم صالحة لحفظها في Preset".into());
    }
    for list in [&mut enabled, &mut disabled, &mut removed] {
        list.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    }

    let mut text = String::new();
    // Same header as ADB AppControl 1.8.6, so presets are interchangeable.
    text.push_str("# Preset created in ADB Control\n");
    text.push_str(&format!("# Total applications: {total}\n"));
    let stamp = stamp
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(utc_stamp);
    text.push_str(&format!("# {stamp}\n"));
    if let Some(comment) = comment.map(str::trim).filter(|c| !c.is_empty()) {
        text.push_str(&format!("# {comment}\n"));
    }

    // When the list mixes enabled/disabled/uninstalled apps (the "All" tab),
    // record each state explicitly. Lines starting with '#' are comments, so
    // older/stock preset readers still see a plain package list.
    let has_state_sections = !disabled.is_empty() || !removed.is_empty();
    if has_state_sections {
        text.push_str(&format!("# Enabled applications: {}\n", enabled.len()));
        text.push_str(&format!("# Disabled applications: {}\n", disabled.len()));
        if !removed.is_empty() {
            text.push_str(&format!("# Uninstalled applications: {}\n", removed.len()));
        }
        text.push('\n');

        let mut section = |title: &str, list: &[String]| {
            if list.is_empty() {
                return;
            }
            text.push_str(&format!("# {title}\n"));
            for pkg in list {
                text.push_str(pkg);
                text.push('\n');
            }
            text.push('\n');
        };
        section("Enabled applications", &enabled);
        section("Disabled applications", &disabled);
        section("Uninstalled applications", &removed);
    } else {
        for pkg in &enabled {
            text.push_str(pkg);
            text.push('\n');
        }
    }

    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(path, text).map_err(|e| e.to_string())?;
    Ok(format!("تم حفظ Preset ({total}) في {path}"))
}

#[tauri::command]
async fn save_preset(
    path: String,
    apps: Vec<PresetEntry>,
    stamp: Option<String>,
    comment: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        save_preset_blocking(&path, apps, stamp.as_deref(), comment.as_deref())
    })
    .await
    .map_err(|e| format!("خطأ داخلي: {e}"))?
}

fn load_preset_blocking(path: &str) -> Result<Vec<String>, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("تعذّر قراءة الملف: {e}"))?;
    let mut seen = std::collections::HashSet::new();
    let mut packages = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Tolerate comma/space-separated lists and trailing comments.
        for token in line.split(|c: char| c == ',' || c.is_whitespace()) {
            let pkg = token.trim();
            if valid_package(pkg) && seen.insert(pkg.to_string()) {
                packages.push(pkg.to_string());
            }
        }
    }
    if packages.is_empty() {
        return Err("الملف لا يحتوي على أسماء حزم صالحة".into());
    }
    Ok(packages)
}

#[tauri::command]
async fn load_preset(path: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || load_preset_blocking(&path))
        .await
        .map_err(|e| format!("خطأ داخلي: {e}"))?
}

fn extract_apk_groups(
    zip_path: &str,
    dest: &std::path::Path,
) -> Result<Vec<(String, Vec<String>)>, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("ملف حزمة غير صالح: {e}"))?;
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if !name.to_lowercase().ends_with(".apk") {
            continue;
        }
        let group = if name.contains('/') {
            name.split('/').next().unwrap_or("app").to_string()
        } else {
            "app".to_string()
        };
        let file_name = sanitize_file_name(name.rsplit('/').next().unwrap_or("part.apk"));
        let target = dest.join(format!("{}-{}", sanitize_file_name(&group), file_name));
        let mut output = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
        groups
            .entry(group)
            .or_default()
            .push(target.to_string_lossy().to_string());
    }
    Ok(groups.into_iter().collect())
}

fn install_error_hint(message: &str) -> String {
    let lower = message.to_lowercase();
    if lower.contains("install_failed_update_incompatible")
        || lower.contains("signatures do not match")
        || lower.contains("signature")
    {
        return "توقيع النسخة المثبتة مختلف (نسخة معدّلة) — احذف التطبيق القديم ثم أعد التثبيت".into();
    }
    if lower.contains("install_failed_version_downgrade") {
        return "النسخة المثبتة أحدث من الملف — احذف التطبيق أو استخدم ملفاً أحدث".into();
    }
    if lower.contains("install_failed_user_restricted") {
        return "الجهاز يمنع التثبيت عبر USB — فعّل «Install via USB» في خيارات المطور".into();
    }
    if lower.contains("install_failed_insufficient_storage") {
        return "لا توجد مساحة كافية على الجهاز".into();
    }
    if lower.contains("install_failed_invalid_apk")
        || lower.contains("install_parse_failed")
    {
        return "ملف APK غير صالح (تالف أو ناقص)".into();
    }
    if lower.contains("install_failed_already_exists")
        || lower.contains("install_failed_already") 
    {
        return "التطبيق مثبّت بالفعل — فعّل «إعادة التثبيت»".into();
    }
    if lower.contains("install_failed_blocked") || lower.contains("install_failed_aborted") {
        return "تم رفض التثبيت من النظام (Play Protect أو سياسة الجهاز)".into();
    }
    if lower.contains("no such file") {
        return "ملف APK غير موجود".into();
    }
    message.trim().to_string()
}

fn install_apks_blocking(
    serial: &str,
    paths: Vec<String>,
    reinstall: bool,
) -> Result<String, String> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    let mut temp_dirs: Vec<std::path::PathBuf> = Vec::new();

    for path in paths {
        let lower = path.to_lowercase();
        if !std::path::Path::new(&path).exists() {
            return Err(format!("الملف غير موجود: {path}"));
        }
        if lower.ends_with(".apk") {
            let name = std::path::Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "app.apk".to_string());
            groups.push((name, vec![path]));
        } else if lower.ends_with(".apks") || lower.ends_with(".xapk") || lower.ends_with(".zip") {
            let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let tmp = cache_dir().join("install").join(format!("{}-{n}", timestamp()));
            std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
            let mut extracted = extract_apk_groups(&path, &tmp)?;
            groups.append(&mut extracted);
            temp_dirs.push(tmp);
        } else {
            return Err(format!("صيغة غير مدعومة: {path}"));
        }
    }

    if groups.is_empty() {
        for dir in &temp_dirs {
            let _ = std::fs::remove_dir_all(dir);
        }
        return Err("لم يتم العثور على ملفات APK للتثبيت".into());
    }

    let mut ok = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for (group_name, files) in groups.iter() {
        // Install straight from the host: `adb install <device path>` does not
        // work because adb always reads the APK from the host machine.
        let mut args = vec![
            s("-s"),
            serial.to_string(),
            if files.len() == 1 { s("install") } else { s("install-multiple") },
        ];
        if reinstall {
            args.push(s("-r"));
        }
        args.extend(files.iter().cloned());

        match run_adb(&args) {
            Ok(out) if out.contains("Success") => ok += 1,
            Ok(out) => failures.push(format!("{group_name}: {}", install_error_hint(out.trim()))),
            Err(e) => failures.push(format!("{group_name}: {}", install_error_hint(&e))),
        }
    }

    for dir in &temp_dirs {
        let _ = std::fs::remove_dir_all(dir);
    }

    let mut summary = format!("تم تثبيت {ok}/{} حزمة", groups.len());
    if let Some(first) = failures.first() {
        summary.push_str("\n");
        summary.push_str(first);
    }
    Ok(summary)
}

#[tauri::command]
async fn install_apks(
    serial: String,
    paths: Vec<String>,
    reinstall: bool,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || install_apks_blocking(&serial, paths, reinstall))
        .await
        .map_err(|e| format!("خطأ داخلي: {e}"))?
}

#[tauri::command]
async fn screenshot(serial: String) -> Result<String, String> {
    let remote = format!("/sdcard/ac_shot_{}.png", timestamp());
    adb(vec![
        s("-s"),
        serial.clone(),
        s("shell"),
        s("screencap"),
        s("-p"),
        remote.clone(),
    ])
    .await?;
    let dir = pictures_dir().join("ADB AppControl");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(format!("screenshot-{}.png", timestamp()));
    let out = adb(vec![
        s("-s"),
        serial.clone(),
        s("pull"),
        remote.clone(),
        dest.to_string_lossy().to_string(),
    ])
    .await?;
    if out.contains("error") {
        return Err(out);
    }
    let _ = adb(vec![s("-s"), serial, s("shell"), s("rm"), remote]).await;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
async fn logcat(serial: String, lines: Option<u32>) -> Result<String, String> {
    let n = lines.unwrap_or(400).to_string();
    adb(vec![
        s("-s"),
        serial,
        s("logcat"),
        s("-d"),
        s("-t"),
        n,
    ])
    .await
}

#[tauri::command]
async fn reboot(serial: String, mode: String) -> Result<String, String> {
    let mut args = vec![s("-s"), serial, s("reboot")];
    match mode.as_str() {
        "system" => {}
        "recovery" => args.push(s("recovery")),
        "bootloader" => args.push(s("bootloader")),
        "poweroff" => args.push(s("-p")),
        _ => return Err("وضع إعادة تشغيل غير معروف".into()),
    }
    adb(args).await
}

fn sync_bridge_blocking(serial: &str) -> Result<String, String> {
    let cache = cache_dir();
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;

    let apk_path = cache.join("bridge.apk");
    std::fs::write(&apk_path, BRIDGE_APK).map_err(|e| format!("تعذّر كتابة bridge.apk: {e}"))?;

    let installed = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("pm"),
        s("path"),
        s(BRIDGE_PKG),
    ])
    .map(|o| o.contains("package:"))
    .unwrap_or(false);

    if !installed {
        let install_args = &[
            s("-s"),
            serial.to_string(),
            s("install"),
            s("-r"),
            apk_path.to_string_lossy().to_string(),
        ];
        let out = run_adb(install_args)?;
        if out.contains("Failure") {
            // Older bridge builds may be signed with a different key.
            let _ = run_adb(&[s("-s"), serial.to_string(), s("uninstall"), s(BRIDGE_PKG)]);
            let retry = run_adb(install_args)?;
            if retry.contains("Failure") {
                return Err(retry);
            }
        }
    }

    let _ = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("pm"),
        s("grant"),
        s(BRIDGE_PKG),
        s("android.permission.WRITE_EXTERNAL_STORAGE"),
    ]);
    let sdk_out = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("getprop"),
        s("ro.build.version.sdk"),
    ])
    .unwrap_or_default();
    let sdk: i32 = sdk_out.trim().parse().unwrap_or(0);
    let remote = if sdk < 29 {
        "/sdcard/Download/ADBControlBridge".to_string()
    } else {
        bridge_remote_dir()
    };
    let ready = format!("{remote}/ready");
    let _ = run_adb(&[s("-s"), serial.to_string(), s("shell"), s("rm"), s("-f"), ready.clone()]);
    let _ = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("am"),
        s("force-stop"),
        s(BRIDGE_PKG),
    ]);
    let trigger = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("content"),
        s("call"),
        s("--uri"),
        s("content://com.adbappcontrol.bridge.export"),
        s("--method"),
        s("export"),
    ]);
    if trigger.is_err() {
        let _ = run_adb(&[
            s("-s"),
            serial.to_string(),
            s("shell"),
            s("am"),
            s("start"),
            s("-n"),
            s("com.adbappcontrol.bridge/.MainActivity"),
        ]);
    }

    let mut finished = false;
    for _ in 0..360 {
        std::thread::sleep(Duration::from_millis(500));
        if let Ok(out) = run_adb(&[
            s("-s"),
            serial.to_string(),
            s("shell"),
            s("cat"),
            ready.clone(),
        ]) {
            if out.contains("\"apps\"") {
                finished = true;
                break;
            }
        }
    }
    if !finished {
        return Err("انتهت المهلة قبل أن ينهي التطبيق المساعد قراءة الأيقونات".into());
    }

    let icons_target = icons_dir(serial);
    let _ = std::fs::remove_dir_all(&icons_target);
    if let Some(parent) = icons_target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    run_adb(&[
        s("-s"),
        serial.to_string(),
        s("pull"),
        format!("{remote}/icons"),
        icons_target.to_string_lossy().to_string(),
    ])?;

    let json_target = bridge_json_path(serial);
    run_adb(&[
        s("-s"),
        serial.to_string(),
        s("pull"),
        format!("{remote}/apps.json"),
        json_target.to_string_lossy().to_string(),
    ])?;

    let count = std::fs::read_dir(&icons_target)
        .map(|it| it.filter_map(|e| e.ok()).count())
        .unwrap_or(0);

    Ok(format!("تم استيراد {count} أيقونة وبيانات {} تطبيقاً", bridge_count(serial)))
}

fn bridge_count(serial: &str) -> usize {
    std::fs::read_to_string(bridge_json_path(serial))
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<BridgeApp>>(&raw).ok())
        .map(|v| v.len())
        .unwrap_or(0)
}

#[tauri::command]
async fn sync_bridge(serial: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || sync_bridge_blocking(&serial))
        .await
        .map_err(|e| format!("خطأ داخلي: {e}"))?
}

#[tauri::command]
async fn bridge_status(serial: String) -> Result<serde_json::Value, String> {
    let dir = icons_dir(&serial);
    let icons = std::fs::read_dir(&dir)
        .map(|it| it.filter_map(|e| e.ok()).count())
        .unwrap_or(0);
    let installed = run_adb(&[
        s("-s"),
        serial.clone(),
        s("shell"),
        s("pm"),
        s("path"),
        s(BRIDGE_PKG),
    ])
    .map(|o| o.contains("package:"))
    .unwrap_or(false);
    Ok(serde_json::json!({
        "installed": installed,
        "icons": icons,
        "apps": bridge_count(&serial),
    }))
}

#[tauri::command]
async fn get_icon(serial: String, package: String) -> Result<Option<String>, String> {
    if !valid_package(&package) {
        return Ok(None);
    }
    let path = icons_dir(&serial).join(format!("{package}.png"));
    if debug_on() {
        eprintln!("[icon] {package} {}", if path.exists() { "hit" } else { "miss" });
    }
    match std::fs::read(path) {
        Ok(bytes) => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            Ok(Some(format!("data:image/png;base64,{encoded}")))
        }
        Err(_) => Ok(None),
    }
}

#[tauri::command]
async fn discover_devices() -> Result<Vec<String>, String> {
    let raw = adb(vec![s("mdns"), s("services")]).await.unwrap_or_default();
    let mut endpoints: Vec<String> = Vec::new();
    for line in raw.lines() {
        if !line.contains("_adb") {
            continue;
        }
        if let Some(token) = line.split_whitespace().last() {
            let looks_like_endpoint = token.contains(':')
                && token.chars().any(|c| c.is_ascii_digit())
                && !token.contains('_');
            if looks_like_endpoint && !endpoints.iter().any(|e| e == token) {
                endpoints.push(token.to_string());
            }
        }
    }
    for endpoint in &endpoints {
        let _ = adb(vec![s("connect"), endpoint.clone()]).await;
    }
    Ok(endpoints)
}

#[tauri::command]
async fn scan_lan() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        use std::net::{TcpStream, UdpSocket};
        let probe = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        probe.connect("8.8.8.8:80").map_err(|e| e.to_string())?;
        let local = probe.local_addr().map_err(|e| e.to_string())?;
        let std::net::IpAddr::V4(v4) = local.ip() else {
            return Ok(Vec::new());
        };
        let o = v4.octets();
        let mut handles = Vec::new();
        for host in 1..=254u8 {
            let addr = std::net::SocketAddr::from(([o[0], o[1], o[2], host], 5555));
            handles.push(std::thread::spawn(move || {
                let timeout = std::time::Duration::from_millis(180);
                if TcpStream::connect_timeout(&addr, timeout).is_ok() {
                    Some(format!("{}:5555", addr.ip()))
                } else {
                    None
                }
            }));
        }
        let mut found = Vec::new();
        for handle in handles {
            if let Ok(Some(endpoint)) = handle.join() {
                let _ = run_adb(&[s("connect"), endpoint.clone()]);
                found.push(endpoint);
            }
        }
        found.sort();
        Ok(found)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Clone, Serialize, serde::Deserialize)]
struct CytProfileData {
    id: String,
    title: String,
    #[serde(default)]
    os: String,
    #[serde(default)]
    brand: String,
    #[serde(default)]
    header: String,
    #[serde(default)]
    packages: Vec<String>,
}

#[derive(Serialize)]
struct CytProfile {
    id: String,
    title: String,
    os: String,
    brand: String,
}

static EMBEDDED_PROFILES: &str = include_str!("../cleanyourtv/profiles.json");

fn cyt_data() -> Vec<CytProfileData> {
    let cache = cache_dir().join("cleanyourtv").join("profiles.json");
    if let Ok(raw) = std::fs::read_to_string(cache) {
        if let Ok(list) = serde_json::from_str::<Vec<CytProfileData>>(&raw) {
            if !list.is_empty() {
                return list;
            }
        }
    }
    serde_json::from_str(EMBEDDED_PROFILES).unwrap_or_default()
}

fn valid_pkg(pkg: &str) -> bool {
    pkg.contains('.')
        && pkg.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
}

fn parse_profile_text(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let cleaned = line
            .trim_start_matches('\u{feff}')
            .split('#')
            .next()
            .unwrap_or("")
            .trim();
        if cleaned.is_empty() {
            continue;
        }
        let pkg = cleaned.split_whitespace().next().unwrap_or("");
        if valid_pkg(pkg) && !out.iter().any(|p| p == pkg) {
            out.push(pkg.to_string());
        }
    }
    out
}

#[tauri::command]
async fn debloat_fetch() -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(|| {
        use std::io::Read;

        let dir = cache_dir().join("cleanyourtv");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let zip_path = dir.join("cleanyourtv.zip");
        let zip_arg = zip_path.to_string_lossy().to_string();
        let status = Command::new("curl")
            .args([
                "-L",
                "--fail",
                "-o",
                zip_arg.as_str(),
                "https://codeload.github.com/Gathin23/cleanyourtv/zip/refs/heads/main",
            ])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("فشل تنزيل قوائم CleanYourTV — تحقق من الاتصال".into());
        }

        let file = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        let mut map: std::collections::BTreeMap<String, CytProfileData> =
            std::collections::BTreeMap::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().to_string();
            let Some(rest) = name.split_once("/examples/").map(|(_, r)| r) else {
                continue;
            };
            let parts: Vec<&str> = rest.split('/').collect();
            if parts.len() != 2 {
                continue;
            }
            let id = parts[0].to_string();
            let file_name = parts[1];

            if file_name == "tv.json" {
                let mut text = String::new();
                let _ = entry.read_to_string(&mut text);
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                    let profile = map.entry(id.clone()).or_insert_with(|| CytProfileData {
                        id: id.clone(),
                        title: id.clone(),
                        os: String::new(),
                        brand: String::new(),
                        header: String::new(),
                        packages: Vec::new(),
                    });
                    profile.title = v.get("tv").and_then(|x| x.as_str()).unwrap_or(&id).to_string();
                    profile.os = v.get("os").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    profile.brand = v.get("brand").and_then(|x| x.as_str()).unwrap_or("").to_string();
                }
            } else if file_name.starts_with("debloat") && file_name.ends_with(".txt") {
                let mut text = String::new();
                let _ = entry.read_to_string(&mut text);
                let header: String = text
                    .lines()
                    .take(35)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(1600)
                    .collect();
                let profile = map.entry(id.clone()).or_insert_with(|| CytProfileData {
                    id: id.clone(),
                    title: id.clone(),
                    os: String::new(),
                    brand: String::new(),
                    header: String::new(),
                    packages: Vec::new(),
                });
                profile.header = header;
                profile.packages = parse_profile_text(&text);
            }
        }
        let _ = std::fs::remove_file(&zip_path);

        let list: Vec<CytProfileData> = map
            .into_values()
            .filter(|p| !p.packages.is_empty())
            .collect();
        let json = serde_json::to_string(&list).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("profiles.json"), json).map_err(|e| e.to_string())?;
        Ok(list.len())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn debloat_profiles() -> Result<Vec<CytProfile>, String> {
    let mut profiles: Vec<CytProfile> = cyt_data()
        .into_iter()
        .map(|p| CytProfile {
            id: p.id,
            title: p.title,
            os: p.os,
            brand: p.brand,
        })
        .collect();
    profiles.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(profiles)
}

#[tauri::command]
async fn debloat_match(serial: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let get = |prop: &str| -> String {
            run_adb(&[
                s("-s"),
                serial.clone(),
                s("shell"),
                s("getprop"),
                s(prop),
            ])
            .unwrap_or_default()
            .trim()
            .to_string()
        };
        let model = get("ro.product.model").to_lowercase();
        let device = get("ro.product.device").to_lowercase();
        let brand = get("ro.product.brand").to_lowercase();
        if model.is_empty() && device.is_empty() {
            return Ok(None);
        }

        let mut best: Option<(i32, String)> = None;
        for profile in cyt_data() {
            let haystack = format!("{} {} {}", profile.title, profile.header, profile.id).to_lowercase();
            let mut score = 0i32;
            if !model.is_empty() {
                score += 8 * haystack.matches(model.as_str()).count() as i32;
            }
            if !device.is_empty() {
                score += 8 * haystack.matches(device.as_str()).count() as i32;
            }
            if !brand.is_empty() {
                score += 2 * haystack.matches(brand.as_str()).count() as i32;
            }
            if !model.is_empty() && profile.id.to_lowercase().contains(&model) {
                score += 10;
            }
            if score > 0 && best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
                best = Some((score, profile.id));
            }
        }
        Ok(best.map(|(_, id)| id))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn debloat_packages(profile: String) -> Result<Vec<String>, String> {
    cyt_data()
        .into_iter()
        .find(|p| p.id == profile)
        .map(|p| p.packages)
        .ok_or_else(|| "ملف تعريف غير موجود".to_string())
}

#[tauri::command]
async fn debloat_apply(
    serial: String,
    profile: String,
    enable: bool,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let packages = cyt_data()
            .into_iter()
            .find(|p| p.id == profile)
            .map(|p| p.packages)
            .ok_or_else(|| "ملف تعريف غير موجود".to_string())?;
        if packages.is_empty() {
            return Err("القائمة لا تحتوي على حزم".into());
        }
        let mut done = 0usize;
        for pkg in &packages {
            let args = if enable {
                vec![
                    s("-s"),
                    serial.clone(),
                    s("shell"),
                    s("pm"),
                    s("enable"),
                    pkg.clone(),
                ]
            } else {
                vec![
                    s("-s"),
                    serial.clone(),
                    s("shell"),
                    s("pm"),
                    s("disable-user"),
                    s("--user"),
                    s("0"),
                    pkg.clone(),
                ]
            };
            if let Ok(out) = run_adb(&args) {
                if !out.contains("Failure") && !out.contains("Exception") {
                    done += 1;
                }
            }
        }
        let verb = if enable { "استعادة" } else { "تعطيل" };
        Ok(format!("تم {verb} {done}/{} حزمة", packages.len()))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn device_ip(serial: &str) -> Option<String> {
    if let Ok(routes) = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("ip"),
        s("route"),
    ]) {
        for line in routes.lines() {
            if !(line.contains(" wlan0 ") || line.contains(" eth0 ")) {
                continue;
            }
            if let Some(pos) = line.find(" src ") {
                if let Some(ip) = line[pos + 5..].split_whitespace().next() {
                    if ip.starts_with("192.")
                        || ip.starts_with("10.")
                        || ip.starts_with("172.")
                    {
                        return Some(ip.to_string());
                    }
                }
            }
        }
    }
    for iface in ["wlan0", "eth0"] {
        if let Ok(out) = run_adb(&[
            s("-s"),
            serial.to_string(),
            s("shell"),
            s("ip"),
            s("addr"),
            s("show"),
            s(iface),
        ]) {
            for line in out.lines() {
                if let Some(pos) = line.find("inet ") {
                    if let Some(ip) = line[pos + 5..].split('/').next() {
                        let ip = ip.trim();
                        if !ip.is_empty() {
                            return Some(ip.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

#[tauri::command]
async fn enable_wireless(serial: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if serial.contains(':') {
            return Ok(serial);
        }
        let ip = device_ip(&serial).ok_or_else(|| {
            "لم أجد عنوان IP للجهاز. تأكد من اتصاله بالواي فاي أو Ethernet".to_string()
        })?;
        let out = run_adb(&[
            s("-s"),
            serial.clone(),
            s("tcpip"),
            s("5555"),
        ])?;
        if out.to_lowercase().contains("error") {
            return Err(out.trim().to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(1800));
        let endpoint = format!("{ip}:5555");
        let connected = run_adb(&[s("connect"), endpoint.clone()])?;
        if connected.to_lowercase().contains("failed")
            || connected.to_lowercase().contains("cannot")
        {
            return Err(connected.trim().to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(800));
        run_adb(&[
            s("-s"),
            endpoint.clone(),
            s("shell"),
            s("echo"),
            s("ok"),
        ])?;
        Ok(endpoint)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn disable_wireless(serial: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if !serial.contains(':') {
            return Ok("الجهاز متصل عبر USB بالفعل".into());
        }
        let _ = run_adb(&[s("-s"), serial.clone(), s("usb")]);
        let _ = run_adb(&[s("disconnect"), serial]);
        Ok("تم إيقاف ADB اللاسلكي (عاد إلى USB)".into())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_command(serial: String, command: String) -> Result<String, String> {
    if command.trim().is_empty() {
        return Err("أمر فارغ".into());
    }
    // Split on whitespace; quotes are not interpreted (safe, no shell involved).
    let parts: Vec<String> = command
        .split_whitespace()
        .map(|p| p.to_string())
        .collect();
    let mut args = vec![s("-s"), serial, s("shell")];
    args.extend(parts);
    adb(args).await
}

fn main() {
    // Headless diagnostics: `adb-appcontrol-linux --selftest <serial>`
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--install") {
        match (args.get(2), args.get(3)) {
            (Some(serial), Some(path)) => {
                match install_apks_blocking(serial, vec![path.clone()], true) {
                    Ok(message) => println!("{message}"),
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                eprintln!("usage: --install <serial> <file.apk|file.apks|backup.zip>");
                std::process::exit(2);
            }
        }
        return;
    }

    if args.iter().any(|a| a == "--extract") {
        match (args.get(2), args.get(3)) {
            (Some(serial), Some(package)) => match extract_apk_blocking(serial, package, None, None) {
                Ok(path) => println!("{path}"),
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            },
            _ => {
                eprintln!("usage: --extract <serial> <package>");
                std::process::exit(2);
            }
        }
        return;
    }

    if args.iter().any(|a| a == "--permissions") {
        match (args.get(2), args.get(3)) {
            (Some(serial), Some(package)) => {
                match list_permissions_blocking(serial, package) {
                    Ok(list) => match serde_json::to_string_pretty(&list) {
                        Ok(raw) => println!("{raw}"),
                        Err(e) => {
                            eprintln!("error: {e}");
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                eprintln!("usage: --permissions <serial> <package>");
                std::process::exit(2);
            }
        }
        return;
    }

    if args.iter().any(|a| a == "--setperm") {
        match (args.get(2), args.get(3), args.get(4), args.get(5)) {
            (Some(serial), Some(package), Some(mode), Some(perm)) => {
                let granted = match mode.as_str() {
                    "grant" => true,
                    "revoke" => false,
                    _ => {
                        eprintln!("mode must be grant or revoke");
                        std::process::exit(2);
                    }
                };
                match set_permissions_blocking(serial, package, &[perm.clone()], granted) {
                    Ok(value) => println!("{value}"),
                    Err(e) => {
                        eprintln!("error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                eprintln!("usage: --setperm <serial> <package> <grant|revoke> <permission>");
                std::process::exit(2);
            }
        }
        return;
    }

    if args.iter().any(|a| a == "--selftest") {
        let serial = args.get(2).cloned().unwrap_or_default();
        if serial.is_empty() {
            eprintln!("usage: --selftest <serial>");
            std::process::exit(2);
        }
        match sync_bridge_blocking(&serial) {
            Ok(message) => println!("{message}"),
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    tauri::Builder::default()
        .setup(|app| {
            if let Ok(dir) = app.path().resource_dir() {
                let _ = RESOURCE_DIR.set(dir);
            }
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            check_adb,
            adb_status,
            set_adb_path,
            list_devices,
            connect_device,
            discover_devices,
            scan_lan,
            debloat_fetch,
            debloat_profiles,
            debloat_match,
            debloat_packages,
            debloat_apply,
            enable_wireless,
            disable_wireless,
            disconnect_device,
            device_info,
            device_memory,
            list_processes,
            kill_process,
            list_apps,
            list_removed_apps,
            list_permissions,
            set_permissions,
            app_action,
            extract_apk,
            backup_apps,
            presets_dir,
            save_preset,
            load_preset,
            install_apks,
            screenshot,
            logcat,
            reboot,
            run_command,
            sync_bridge,
            bridge_status,
            get_icon
        ])
        .run(tauri::generate_context!())
        .expect("فشل تشغيل التطبيق");
}

#[cfg(test)]
mod backup_tests {
    use super::*;

    #[test]
    fn utc_date_conversion_starts_at_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(utc_stamp().len(), 15);
    }

    #[test]
    fn preset_roundtrip_dedupes_and_skips_invalid_packages() {
        let path = std::env::temp_dir().join(format!(
            "adbac-preset-{}-{}.txt",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let path_str = path.to_string_lossy().to_string();
        save_preset_blocking(
            &path_str,
            vec![
                PresetEntry {
                    package: "com.example.one".into(),
                    disabled: false,
                    removed: false,
                },
                PresetEntry {
                    package: "com.example.one".into(),
                    disabled: true,
                    removed: false,
                },
                PresetEntry {
                    package: "com.example.off".into(),
                    disabled: true,
                    removed: false,
                },
                PresetEntry {
                    package: "com.example.gone".into(),
                    disabled: false,
                    removed: true,
                },
                PresetEntry {
                    package: "bad package".into(),
                    disabled: false,
                    removed: false,
                },
            ],
            Some("2026-11-12_22:22:03"),
            Some("Description"),
        )
        .unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("# Preset created in ADB Control"));
        assert!(raw.contains("# Total applications: 3"));
        assert!(raw.contains("# 2026-11-12_22:22:03"));
        assert!(raw.contains("# Description"));
        assert!(raw.contains("# Enabled applications: 1"));
        assert!(raw.contains("# Disabled applications: 1"));
        assert!(raw.contains("# Uninstalled applications: 1"));
        let enabled_pos = raw.find("# Enabled applications\n").unwrap();
        let disabled_pos = raw.find("# Disabled applications\n").unwrap();
        assert!(enabled_pos < disabled_pos);
        assert!(raw[enabled_pos..disabled_pos].contains("com.example.one"));
        assert!(raw[disabled_pos..].contains("com.example.off"));
        let list = load_preset_blocking(&path_str).unwrap();
        assert_eq!(
            list,
            vec![
                "com.example.one".to_string(),
                "com.example.off".to_string(),
                "com.example.gone".to_string()
            ]
        );
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod permission_tests {
    use super::*;

    #[test]
    fn parses_requested_install_and_user0_runtime_permissions() {
        let dump = r#"
    requested permissions:
      android.permission.CAMERA
      android.permission.POST_NOTIFICATIONS
      android.permission.WAKE_LOCK
    install permissions:
      android.permission.WAKE_LOCK: granted=true
    User 0: ceDataInode=1 installed=true
      runtime permissions:
        android.permission.CAMERA: granted=false, flags=[ USER_SET]
        android.permission.POST_NOTIFICATIONS: granted=true, flags=[ 
USER_SENSITIVE_WHEN_GRANTED|USER_SENSITIVE_WHEN_DENIED]
    User 10: installed=false
      runtime permissions:
        android.permission.CAMERA: granted=true
"#;
        let dangerous = parse_dangerous_permissions(
            "Dangerous Permissions:\n\ngroup:android.permission-group.CAMERA\n  permission:android.permission.CAMERA\n\ngroup:android.permission-group.NOTIFICATIONS\n  permission:android.permission.POST_NOTIFICATIONS\n",
        );
        let list = parse_package_permissions(dump, &dangerous);
        assert_eq!(list.len(), 3);

        let camera = list.iter().find(|p| p.name.ends_with("CAMERA")).unwrap();
        assert_eq!(camera.kind, "runtime");
        assert!(!camera.granted);
        assert!(camera.dangerous);
        assert_eq!(
            camera.group.as_deref(),
            Some("android.permission-group.CAMERA")
        );
        assert_eq!(camera.flags, vec!["USER_SET"]);

        let post = list
            .iter()
            .find(|p| p.name.ends_with("POST_NOTIFICATIONS"))
            .unwrap();
        assert_eq!(post.kind, "runtime");
        assert!(post.granted);
        assert_eq!(post.flags.len(), 2);

        let wake = list.iter().find(|p| p.name.ends_with("WAKE_LOCK")).unwrap();
        assert_eq!(wake.kind, "install");
        assert!(wake.granted);
        assert!(!wake.dangerous);
    }
}
