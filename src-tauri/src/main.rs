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

#[tauri::command]
async fn app_action(serial: String, package: String, action: String) -> Result<String, String> {
    if !valid_package(&package) {
        return Err("اسم حزمة غير صالح".into());
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

fn extract_apk_blocking(serial: &str, package: &str) -> Result<String, String> {
    if !valid_package(package) {
        return Err("اسم حزمة غير صالح".into());
    }
    let paths = run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("pm"),
        s("path"),
        s(package),
    ])?;

    let dir = download_dir().join("adb-appcontrol-apks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let tmp = cache_dir()
        .join("tmp")
        .join(format!("{}-{}", safe_serial(package), timestamp()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    let mut parts: Vec<(String, std::path::PathBuf)> = Vec::new();
    for line in paths.lines() {
        let Some(remote) = line.trim().strip_prefix("package:") else {
            continue;
        };
        let name = sanitize_file_name(remote.rsplit('/').next().unwrap_or("part.apk"));
        let dest = tmp.join(&name);
        let out = run_adb(&[
            s("-s"),
            serial.to_string(),
            s("pull"),
            s(remote),
            dest.to_string_lossy().to_string(),
        ])?;
        if out.to_lowercase().contains("error") {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(out);
        }
        parts.push((name, dest));
    }

    if parts.is_empty() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err("لم يتم العثور على مسار APK".into());
    }

    let result = if parts.len() == 1 {
        let dest = dir.join(format!("{package}.apk"));
        let _ = std::fs::remove_file(&dest);
        std::fs::copy(&parts[0].1, &dest).map_err(|e| e.to_string())?;
        dest.to_string_lossy().to_string()
    } else {
        // Split APK (Android App Bundle): pack all parts into one .apks archive.
        let dest = dir.join(format!("{package}.apks"));
        let file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, path) in &parts {
            writer
                .start_file(name.clone(), options)
                .map_err(|e| e.to_string())?;
            let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut writer).map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
        format!(
            "{}  ({} أجزاء مدمجة في ملف واحد)",
            dest.display(),
            parts.len()
        )
    };

    let _ = std::fs::remove_dir_all(&tmp);
    Ok(result)
}

#[tauri::command]
async fn extract_apk(serial: String, package: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || extract_apk_blocking(&serial, &package))
        .await
        .map_err(|e| format!("خطأ داخلي: {e}"))?
}

fn extract_apks_from_zip(
    zip_path: &str,
    dest: &std::path::Path,
    out: &mut Vec<String>,
) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("ملف حزمة غير صالح: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if !name.to_lowercase().ends_with(".apk") {
            continue;
        }
        let file_name = sanitize_file_name(name.rsplit('/').next().unwrap_or("part.apk"));
        let target = dest.join(file_name);
        let mut output = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
        out.push(target.to_string_lossy().to_string());
    }
    Ok(())
}

fn install_apks_blocking(
    serial: &str,
    paths: Vec<String>,
    reinstall: bool,
) -> Result<String, String> {
    let mut apks: Vec<String> = Vec::new();
    let mut temp_dirs: Vec<std::path::PathBuf> = Vec::new();

    for path in paths {
        let lower = path.to_lowercase();
        if lower.ends_with(".apk") {
            if !std::path::Path::new(&path).exists() {
                return Err(format!("الملف غير موجود: {path}"));
            }
            apks.push(path);
        } else if lower.ends_with(".apks") || lower.ends_with(".xapk") || lower.ends_with(".zip") {
            if !std::path::Path::new(&path).exists() {
                return Err(format!("الملف غير موجود: {path}"));
            }
            let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let tmp = cache_dir().join("install").join(format!("{}-{n}", timestamp()));
            std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
            extract_apks_from_zip(&path, &tmp, &mut apks)?;
            temp_dirs.push(tmp);
        } else {
            return Err(format!("صيغة غير مدعومة: {path}"));
        }
    }

    if apks.is_empty() {
        for dir in &temp_dirs {
            let _ = std::fs::remove_dir_all(dir);
        }
        return Err("لم يتم العثور على ملفات APK للتثبيت".into());
    }

    let mut args = vec![s("-s"), serial.to_string()];
    if apks.len() == 1 {
        args.push(s("install"));
        if reinstall {
            args.push(s("-r"));
        }
        args.push(apks[0].clone());
    } else {
        args.push(s("install-multiple"));
        if reinstall {
            args.push(s("-r"));
        }
        args.extend(apks.iter().cloned());
    }

    let out = run_adb(&args)?;
    for dir in &temp_dirs {
        let _ = std::fs::remove_dir_all(dir);
    }
    if out.contains("Failure") {
        return Err(out.trim().to_string());
    }
    Ok(out.trim().to_string())
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
        let out = run_adb(&[
            s("-s"),
            serial.to_string(),
            s("install"),
            s("-r"),
            apk_path.to_string_lossy().to_string(),
        ])?;
        if out.contains("Failure") {
            return Err(out);
        }
    }

    let remote = bridge_remote_dir();
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
    run_adb(&[
        s("-s"),
        serial.to_string(),
        s("shell"),
        s("am"),
        s("start"),
        s("-n"),
        s("com.adbappcontrol.bridge/.MainActivity"),
    ])?;

    let mut finished = false;
    for _ in 0..140 {
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
    if args.iter().any(|a| a == "--extract") {
        match (args.get(2), args.get(3)) {
            (Some(serial), Some(package)) => match extract_apk_blocking(serial, package) {
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
            disconnect_device,
            device_info,
            list_apps,
            app_action,
            extract_apk,
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
