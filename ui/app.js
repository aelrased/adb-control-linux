/* ADB Control — Desktop frontend
   SPDX-License-Identifier: GPL-3.0-or-later */

const T = window.__TAURI__;
const invoke = T?.core?.invoke;
const dialog = T?.dialog;

const $ = (id) => document.getElementById(id);
const t = (key, vars) => window.I18n.t(key, vars);
const tr = (message) => window.I18n.tr(message);

const state = {
  devices: [],
  serial: null,
  apps: [],
  removed: [],
  filter: "all",
  search: "",
  selected: new Set(),
  busy: false,
  info: null,
  platform: "Desktop",
  memory: null,
  processes: [],
};

const consoleEl = $("console");

/* Permissions manager (single app). */
const permState = {
  pkg: null,
  label: "",
  items: [],
  loading: false,
  applying: false,
};

/* UDADB-NG safety classes (bundled list, same as the Android app). */
const UAD_KEYS = {
  recommended: ["uadRecommended", "rec"],
  advanced: ["uadAdvanced", "adv"],
  expert: ["uadExpert", "exp"],
  unsafe: ["uadUnsafe", "uns"],
};

function uadBadge(app) {
  if (!app.uad) return "";
  const key = String(app.uad).toLowerCase();
  const [textKey, cls] = UAD_KEYS[key] || ["uadAdvanced", "adv"];
  return `<span class="uad ${cls}">${escapeHtml(t(textKey))}</span>`;
}

function riskyCount(targets) {
  const keys = new Set(targets);
  return state.apps
    .filter((a) => keys.has(a.package) && /^(expert|unsafe)$/i.test(a.uad || ""))
    .length;
}

function log(message, kind = "") {
  const line = document.createElement("div");
  if (kind) line.className = kind;
  line.textContent = tr(message);
  consoleEl.appendChild(line);
  consoleEl.scrollTop = consoleEl.scrollHeight;
  while (consoleEl.childElementCount > 500) consoleEl.removeChild(consoleEl.firstChild);
}

let toastTimer = null;
function toast(message, isError = false) {
  let el = document.querySelector(".toast");
  if (!el) {
    el = document.createElement("div");
    el.className = "toast";
    document.body.appendChild(el);
  }
  el.textContent = tr(message);
  el.classList.toggle("err", isError);
  el.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove("show"), 3200);
}

function setBusy(on, text) {
  state.busy = on;
  $("progress").classList.toggle("hidden", !on);
  $("statusText").textContent = on ? tr(text || t("working")) : t("ready");
  document.querySelectorAll(".btn, .icon-btn, .quick, .tab").forEach((b) => {
    if (b.id !== "refreshDevices" || !on) b.disabled = on;
  });
  if (!on) updateSelectionUi();
}

async function safe(fn, errPrefix = t("errGeneric")) {
  try {
    return await fn();
  } catch (e) {
    const msg = typeof e === "string" ? e : e?.message ?? String(e);
    log(`${errPrefix}: ${msg}`, "err");
    toast(`${errPrefix}: ${msg.split("\n")[0]}`, true);
    return null;
  }
}

/* ---------------- Devices ---------------- */

async function initAdb() {
  const status = await safe(() => invoke("adb_status"), t("errAdb"));
  const chip = $("adbStatus");
  if (status && status.version) {
    chip.textContent = status.version;
    chip.title = status.path;
    chip.classList.add("ok");
    chip.classList.remove("bad");
  } else {
    chip.textContent = t("adbUnavailable");
    chip.title = status?.path ?? "";
    chip.classList.add("bad");
    chip.classList.remove("ok");
    toast(t("adbNotFoundToast"), true);
  }
}

async function chooseAdb() {
  const picked = await dialog.open({
    multiple: false,
    title: t("chooseAdbTitle"),
  });
  if (!picked) return;
  const version = await safe(() => invoke("set_adb_path", { path: picked }), t("errAdb"));
  if (version) {
    log(`adb: ${picked}\n${version}`, "ok");
    toast(t("adbSetOk"));
    await initAdb();
    await refreshDevices();
  }
}

async function refreshDevices() {
  try {
    const found = await invoke("discover_devices");
    if (Array.isArray(found) && found.length) {
      log(t("discovering", { list: found.join(", ") }), "ok");
    }
  } catch (_) { /* mDNS discovery is not available in some adb builds */ }
  let devices = await safe(() => invoke("list_devices"), t("errDevices"));
  if (!devices) return;
  if (devices.length === 0) {
    const lan = await safe(() => invoke("scan_lan"), t("errLan"));
    if (Array.isArray(lan) && lan.length) {
      log(t("lanFound", { list: lan.join(", ") }), "ok");
      devices = (await safe(() => invoke("list_devices"), t("errDevices"))) || devices;
    }
  }
  state.devices = devices;
  const select = $("deviceSelect");
  select.innerHTML = "";
  if (!devices.length) {
    select.innerHTML = `<option value="">${escapeHtml(t("noDevice"))}</option>`;
    state.serial = null;
    state.info = null;
    state.apps = [];
    state.removed = [];
    stopMemory();
    updateCounts();
    updateConsoleDevice();
    renderDeviceInfo(null);
    renderTable();
    return;
  }
  for (const d of devices) {
    const opt = document.createElement("option");
    opt.value = d.serial;
    const name = d.model ? d.model.replace(/_/g, " ") : d.serial;
    opt.textContent = `${name} · ${d.state}${d.serial.includes(":") ? " (Wi‑Fi)" : ""}`;
    select.appendChild(opt);
  }
  const authorized = devices.filter((d) => d.state === "device");
  const unauthorized = devices.filter((d) => d.state !== "device");
  if (unauthorized.length) {
    log(
      t("unauthorizedLog", { list: unauthorized.map((d) => d.serial).join(", ") }),
      "err"
    );
  }
  const preferred = authorized[0] || null;
  if (!state.serial || !authorized.some((d) => d.serial === state.serial)) {
    state.serial = preferred ? preferred.serial : null;
  }
  if (!state.serial) {
    select.innerHTML = `<option value="">${escapeHtml(t("noAuthorizedOption"))}</option>`;
    $("footerDevice").textContent = t("noDeviceFooter");
    state.info = null;
    state.apps = [];
    state.removed = [];
    stopMemory();
    updateCounts();
    updateConsoleDevice();
    renderDeviceInfo(null);
    renderTable();
    return;
  }
  select.value = state.serial;
  $("footerDevice").textContent = state.serial;
  await onDeviceChanged();
  updateWirelessLabel();
}

async function onDeviceChanged() {
  state.selected.clear();
  updateSelectionUi();
  if (!state.serial) {
    stopMemory();
    return;
  }
  startMemory();
  const info = await safe(() => invoke("device_info", { serial: state.serial }), t("errDeviceInfo"));
  state.info = info;
  renderDeviceInfo(info);
  await refreshApps();
  const bridge = await safe(() => invoke("bridge_status", { serial: state.serial }), t("errBridge"));
  if (state.apps.length && (!bridge || bridge.icons === 0)) {
    toast(t("iconsMissingToast"));
  }
  updateWirelessLabel();
  updateConsoleDevice();
}

function renderDeviceInfo(info) {
  const box = $("deviceInfo");
  if (!info) {
    box.innerHTML = `<div class="empty-hint">${escapeHtml(t("noDevice"))}</div>`;
    return;
  }
  const rows = [
    [t("infoManufacturer"), info["ro.product.manufacturer"]],
    [t("infoModel"), info["ro.product.model"]],
    [t("infoAndroid"), info["ro.build.version.release"]],
    [t("infoApi"), info["ro.build.version.sdk"]],
    [t("infoArch"), info["ro.product.cpu.abi"]],
    [t("infoBattery"), info["battery"] ? `${info["battery"]}%` : ""],
    [t("infoSerial"), info["ro.serialno"] || state.serial],
  ];
  box.innerHTML = rows
    .filter(([, v]) => v !== undefined && v !== null && String(v).trim() !== "")
    .map(
      ([k, v]) =>
        `<div class="info-row"><span class="k">${escapeHtml(k)}</span><span class="v" title="${escapeHtml(
          String(v)
        )}">${escapeHtml(String(v))}</span></div>`
    )
    .join("");
}

function escapeHtml(str) {
  return String(str).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

/* ---------------- RAM + Process Manager ---------------- */

let memoryTimer = null;

function updateMemoryUi() {
  const mem = state.memory;
  const text = $("ramText");
  const fill = $("ramFill");
  if (!mem) {
    if (text) text.textContent = "— / — MB";
    if (fill) {
      fill.style.width = "0%";
      fill.className = "ram-fill";
    }
    return;
  }
  if (text) text.textContent = `${mem.used_mb} / ${mem.total_mb} MB`;
  if (fill) {
    const percent = Math.max(0, Math.min(100, Number(mem.percent) || 0));
    fill.style.width = `${percent}%`;
    fill.className = "ram-fill" + (percent >= 90 ? " danger" : percent >= 75 ? " warn" : "");
  }
}

async function refreshMemory() {
  if (!state.serial) return;
  const mem = await safe(
    () => invoke("device_memory", { serial: state.serial }),
    t("errMemory")
  );
  if (mem) {
    state.memory = mem;
    updateMemoryUi();
  }
}

function stopMemory() {
  if (memoryTimer) {
    clearInterval(memoryTimer);
    memoryTimer = null;
  }
  state.memory = null;
  updateMemoryUi();
  const btn = $("btnProcesses");
  if (btn) btn.disabled = true;
}

function startMemory() {
  stopMemory();
  if (!state.serial) return;
  const btn = $("btnProcesses");
  if (btn) btn.disabled = false;
  refreshMemory();
  memoryTimer = setInterval(refreshMemory, 15000);
}

/** Package name usable with `am force-stop`, or null for system daemons. */
function appPackage(name) {
  const base = String(name).split(":")[0];
  return /^[A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)+$/.test(base) ? base : null;
}

function renderProcesses() {
  const body = $("procBody");
  const list = state.processes;
  const summary = $("procSummary");
  if (summary) {
    summary.textContent = state.memory
      ? t("procSummary", {
          used: state.memory.used_mb,
          total: state.memory.total_mb,
          n: list.length,
        })
      : t("procCount", { n: list.length });
  }
  if (!body) return;
  if (!list.length) {
    body.innerHTML = `<tr><td colspan="4" class="empty-row">${escapeHtml(t("procEmpty"))}</td></tr>`;
    return;
  }
  const frag = document.createDocumentFragment();
  for (const proc of list) {
    const tr = document.createElement("tr");
    const pkg = appPackage(proc.name);
    tr.innerHTML = `
      <td>${escapeHtml(String(proc.pid))}</td>
      <td class="proc-name" title="${escapeHtml(proc.name)}">${escapeHtml(proc.name)}</td>
      <td class="proc-ram">${Number(proc.rss_mb).toFixed(1)} MB</td>
      <td>${
        pkg
          ? `<button class="btn danger proc-stop" data-name="${escapeHtml(pkg)}">${escapeHtml(
              t("procStop")
            )}</button>`
          : '<span class="muted">—</span>'
      }</td>`;
    frag.appendChild(tr);
  }
  body.replaceChildren(frag);
}

async function loadProcesses() {
  if (!state.serial) return;
  const list = await safe(
    () => invoke("list_processes", { serial: state.serial }),
    t("errProcesses")
  );
  state.processes = Array.isArray(list) ? list : [];
  renderProcesses();
}

async function openProcesses() {
  if (!state.serial) return toast(t("cleanNoDevice"), true);
  $("procModal").classList.remove("hidden");
  renderProcesses();
  await loadProcesses();
  await refreshMemory();
}

async function stopProcess(name) {
  const ok = await dialog.confirm(t("procStopConfirm", { name }), {
    title: t("confirmTitle"),
    kind: "warning",
  });
  if (!ok) return;
  const res = await safe(
    () => invoke("kill_process", { serial: state.serial, name }),
    t("errKill")
  );
  if (res) {
    log(res, "ok");
    toast(res);
  }
  await loadProcesses();
  await refreshMemory();
}

/* ---------------- Apps ---------------- */

async function refreshApps() {
  if (!state.serial) {
    state.apps = [];
    state.removed = [];
    updateCounts();
    renderTable();
    return;
  }
  setBusy(true, t("loadingApps"));
  const apps = await safe(() => invoke("list_apps", { serial: state.serial }), t("errApps"));
  setBusy(false);
  if (!apps) return;
  state.apps = apps;
  log(t("loadedApps", { n: apps.length, s: state.serial }), "ok");
  updateCounts();
  renderTable();
  await loadRemoved();
}

async function loadRemoved() {
  if (!state.serial) {
    state.removed = [];
    updateCounts();
    return;
  }
  const list = await safe(
    () => invoke("list_removed_apps", { serial: state.serial }),
    t("errRemoved")
  );
  state.removed = Array.isArray(list) ? list : [];
  updateCounts();
  if (state.filter === "removed") renderTable();
}

function updateCounts() {
  const user = state.apps.filter((a) => !a.system).length;
  const system = state.apps.filter((a) => a.system).length;
  const enabled = state.apps.filter((a) => !a.disabled).length;
  const disabled = state.apps.filter((a) => a.disabled).length;
  $("countAll").textContent = state.apps.length;
  $("countEnabled").textContent = enabled;
  $("countUser").textContent = user;
  $("countSystem").textContent = system;
  $("countDisabled").textContent = disabled;
  $("countRemoved").textContent = state.removed.length;
}

function visibleApps() {
  const q = state.search.trim().toLowerCase();
  const source = state.filter === "removed" ? state.removed : state.apps;
  return source.filter((a) => {
    if (state.filter === "enabled" && a.disabled) return false;
    if (state.filter === "user" && a.system) return false;
    if (state.filter === "system" && !a.system) return false;
    if (state.filter === "disabled" && !a.disabled) return false;
    if (!q) return true;
    return a.label.toLowerCase().includes(q) || a.package.toLowerCase().includes(q);
  });
}

const ICON_COLORS = ["#2f81f7", "#8957e5", "#2ea043", "#d29922", "#db61a2", "#1f9cd0", "#e06c35", "#6e7681"];
function iconColor(pkg) {
  let h = 0;
  for (let i = 0; i < pkg.length; i++) h = (h * 31 + pkg.charCodeAt(i)) | 0;
  return ICON_COLORS[Math.abs(h) % ICON_COLORS.length];
}

/* ---------- Real icons (lazy, concurrency-limited) ---------- */
const iconCache = new Map();   // package -> data URL | null
const iconPending = new Set();
const iconQueue = [];
let iconActive = 0;
const ICON_CONCURRENCY = 4;

function pumpIcons() {
  while (iconActive < ICON_CONCURRENCY && iconQueue.length) {
    const task = iconQueue.shift();
    iconActive++;
    task().finally(() => {
      iconActive--;
      pumpIcons();
    });
  }
}

function applyCachedIcon(el) {
  const pkg = el.dataset.pkg;
  if (!pkg || !iconCache.has(pkg)) return false;
  const url = iconCache.get(pkg);
  if (url) {
    el.classList.add("has-icon");
    el.style.background = "transparent";
    el.innerHTML = `<img src="${url}" alt="" />`;
  }
  return true;
}

const iconObserver = new IntersectionObserver(
  (entries) => {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        loadIcon(entry.target);
        iconObserver.unobserve(entry.target);
      }
    }
  },
  { root: document.querySelector(".table-wrap"), rootMargin: "300px" }
);

function observeIcons() {
  document.querySelectorAll(".app-icon[data-pkg]").forEach((el) => {
    if (!applyCachedIcon(el)) iconObserver.observe(el);
  });
}

function loadIcon(el) {
  const pkg = el.dataset.pkg;
  if (!pkg || !state.serial) return;
  if (iconCache.has(pkg)) {
    applyCachedIcon(el);
    return;
  }
  if (iconPending.has(pkg)) return;
  iconPending.add(pkg);
  const serial = state.serial;
  iconQueue.push(async () => {
    const url = await safe(() => invoke("get_icon", { serial, package: pkg }), t("errIcons"));
    iconCache.set(pkg, url || null);
    iconPending.delete(pkg);
    document.querySelectorAll(`.app-icon[data-pkg="${CSS.escape(pkg)}"]`).forEach(applyCachedIcon);
  });
  pumpIcons();
}

async function syncIcons() {
  if (!state.serial) return;
  const ok = await dialog.confirm(
    t("iconsConfirmText"),
    { title: t("iconsConfirmTitle"), kind: "info" }
  );
  if (!ok) return;
  setBusy(true, t("iconsBusy"));
  const res = await safe(() => invoke("sync_bridge", { serial: state.serial }), t("errIcons"));
  setBusy(false);
  if (res) {
    log(res, "ok");
    toast(res);
    iconCache.clear();
    iconPending.clear();
    await refreshApps();
  }
}

function renderTable() {
  const tbody = $("appsBody");
  const rows = visibleApps();
  if (!state.serial) {
    tbody.innerHTML = `<tr><td colspan="5" class="empty-row">${escapeHtml(t("noDeviceRow"))}</td></tr>`;
    updateSelectionUi();
    return;
  }
  if (!rows.length) {
    const empty = state.filter === "removed" ? t("removedEmpty") : t("noResults");
    tbody.innerHTML = `<tr><td colspan="5" class="empty-row">${escapeHtml(empty)}</td></tr>`;
    updateSelectionUi();
    return;
  }
  const isRemoved = state.filter === "removed";
  const frag = document.createDocumentFragment();
  for (const app of rows) {
    const tr = document.createElement("tr");
    tr.dataset.pkg = app.package;
    if (state.selected.has(app.package)) tr.classList.add("selected");
    const cachedIcon = iconCache.get(app.package);
    const iconStyle = cachedIcon ? "background:transparent" : `background:${iconColor(app.package)}`;
    const iconInner = cachedIcon
      ? '<img src="' + cachedIcon + '" alt="" />'
      : escapeHtml(app.label.trim().charAt(0).toUpperCase() || "?");
    tr.innerHTML = `
      <td class="col-check"><input type="checkbox" ${state.selected.has(app.package) ? "checked" : ""} /></td>
      <td>
        <div class="app-cell">
          <div class="app-icon${cachedIcon ? " has-icon" : ""}" data-pkg="${escapeHtml(
            app.package
          )}" style="${iconStyle}">${iconInner}</div>
          <div class="app-meta">
            <span class="app-name">${escapeHtml(app.label)}</span>
            <span class="app-pkg" title="${escapeHtml(app.package)}">${escapeHtml(app.package)}</span>
            ${uadBadge(app)}
          </div>
        </div>
      </td>
      <td class="col-ver"><span class="ver">${escapeHtml(app.version || "—")}</span></td>
      <td class="col-type"><span class="badge ${
        isRemoved ? "del" : app.system ? "sys" : "user"
      }">${escapeHtml(isRemoved ? t("typeRemoved") : app.system ? t("typeSystem") : t("typeUser"))}</span></td>
      <td class="col-state"><span class="badge ${
        isRemoved ? "del" : app.disabled ? "off" : "on"
      }">${escapeHtml(
        isRemoved ? t("stateRemoved") : app.disabled ? t("stateDisabled") : t("stateEnabled")
      )}</span></td>`;
    frag.appendChild(tr);
  }
  tbody.replaceChildren(frag);
  updateSelectionUi();
  observeIcons();
}

function toggleRow(pkg, on) {
  if (on) state.selected.add(pkg);
  else state.selected.delete(pkg);
  const row = document.querySelector(`tr[data-pkg="${CSS.escape(pkg)}"]`);
  if (row) {
    row.classList.toggle("selected", on);
    const cb = row.querySelector('input[type="checkbox"]');
    if (cb) cb.checked = on;
  }
  updateSelectionUi();
}

function updateSelectionUi() {
  $("selCount").textContent = state.selected.size;
  const has = state.selected.size > 0 && !state.busy;
  const removedMode = state.filter === "removed";
  ["btnDisable", "btnEnable", "btnClear", "btnClearCache", "btnUninstall", "btnSaveApk"].forEach((id) => {
    $(id).disabled = !has || removedMode;
  });
  $("btnBackupZip").disabled = !has || removedMode;
  $("btnPresetSave").disabled = state.busy || !visibleApps().length;
  $("btnPresetLoad").disabled = state.busy || !state.serial || !state.apps.length;
  $("btnRestore").disabled = !has;
  $("btnUninstallDeep").disabled = !has || removedMode;
  const single = state.selected.size === 1 ? [...state.selected][0] : null;
  const singleInstalled =
    single &&
    state.apps.some((a) => a.package === single) &&
    !state.removed.some((a) => a.package === single);
  $("btnPermissions").disabled = state.busy || !singleInstalled;
  $("btnInstallApk").disabled = state.busy || !state.serial;
  const rows = visibleApps();
  const allChecked = rows.length > 0 && rows.every((a) => state.selected.has(a.package));
  $("selectAll").checked = allChecked;
}

/* ---------------- Actions ---------------- */

async function applyToSelection(action, confirmText) {
  const targets = [...state.selected];
  if (!targets.length) return;
  if (confirmText) {
    const ok = await dialog.confirm(confirmText, { title: t("confirmTitle"), kind: "warning" });
    if (!ok) return;
  }
  setBusy(true, t("actionBusy"));
  let done = 0;
  for (const pkg of targets) {
    const res = await safe(() => invoke("app_action", { serial: state.serial, package: pkg, action }), pkg);
    if (res !== null) {
      done++;
      log(`${pkg}: ${res}`, "ok");
    }
  }
  setBusy(false);
  toast(t("actionDone", { done, total: targets.length }));
  state.selected.clear();
  await refreshApps();
}

async function installApk() {
  if (!state.serial) return;
  const picked = await dialog.open({
    multiple: true,
    title: t("installPickTitle"),
    filters: [{ name: "Android Package", extensions: ["apk", "apks", "xapk", "zip"] }],
  });
  if (!picked) return;
  const files = Array.isArray(picked) ? picked : [picked];
  setBusy(true, t("installBusy"));
  const res = await safe(
    () => invoke("install_apks", { serial: state.serial, paths: files, reinstall: true }),
    t("errInstall")
  );
  setBusy(false);
  if (res !== null) {
    log(res, "ok");
    toast(res.split("\n")[0]);
  }
  await refreshApps();
}

async function saveApks() {
  const targets = [...state.selected];
  if (!targets.length) return;
  setBusy(true, t("saveBusy"));
  for (const pkg of targets) {
    const app = state.apps.find((a) => a.package === pkg);
    const res = await safe(() =>
      invoke("extract_apk", {
        serial: state.serial,
        package: pkg,
        label: app?.label ?? null,
        version: app?.version ?? null,
      }), pkg);
    if (res) {
      log(`${pkg} → ${res}`, "ok");
      toast(t("apkSavedToast"));
    }
  }
  setBusy(false);
}

async function backupSelected() {
  if (!state.serial) return toast(t("cleanNoDevice"), true);
  const targets = [...state.selected]
    .map((pkg) => state.apps.find((a) => a.package === pkg))
    .filter(Boolean);
  if (!targets.length) return toast(t("backupNoTargets"), true);
  const ok = await dialog.confirm(t("backupConfirm", { n: targets.length }), {
    title: t("confirmTitle"),
    kind: "info",
  });
  if (!ok) return;
  setBusy(true, t("backupBusy"));
  const res = await safe(
    () =>
      invoke("backup_apps", {
        serial: state.serial,
        apps: targets.map((a) => ({
          package: a.package,
          label: a.label,
          version: a.version ?? null,
        })),
      }),
    t("errBackup")
  );
  setBusy(false);
  if (!res) return;
  const failed = Array.isArray(res.failed) ? res.failed : [];
  log(t("backupDone", { path: res.path }), "ok");
  toast(t("backupDone", { path: res.path }));
  for (const f of failed) log(f, "err");
  if (failed.length) {
    toast(t("backupPartial", { ok: res.ok, n: failed.length }), true);
  }
}

/* ---------------- Presets ---------------- */

/* Original ADB AppControl names presets `<Device>_[<Filter>]_<date>`. */
const PRESET_FILTERS = {
  all: { token: "All", labelKey: "presetFilterAll", arDesc: "كل التطبيقات" },
  enabled: {
    token: "Enabled",
    labelKey: "presetFilterEnabled",
    arDesc: "التطبيقات المفعّلة",
  },
  user: { token: "User", labelKey: "presetFilterUser", arDesc: "تطبيقات المستخدم" },
  system: {
    token: "System",
    labelKey: "presetFilterSystem",
    arDesc: "تطبيقات النظام",
  },
  disabled: {
    token: "Disabled",
    labelKey: "presetFilterDisabled",
    arDesc: "التطبيقات المعطّلة",
  },
  removed: {
    token: "Uninstalled",
    labelKey: "presetFilterRemoved",
    arDesc: "التطبيقات المحذوفة",
  },
  selected: {
    token: "Selected",
    labelKey: "presetFilterSelected",
    arDesc: "التطبيقات المحددة",
  },
};

let presetPending = null;

function deviceDisplayName() {
  const info = state.info || {};
  const rawManufacturer = String(info["ro.product.manufacturer"] || "").trim();
  const manufacturer = rawManufacturer
    ? rawManufacturer.charAt(0).toUpperCase() + rawManufacturer.slice(1)
    : "";
  const model = String(info["ro.product.model"] || "").trim();
  if (!model) return (state.serial || "device").trim();
  if (
    manufacturer &&
    !model.toLowerCase().startsWith(`${manufacturer.toLowerCase()} `)
  ) {
    return `${manufacturer} ${model}`;
  }
  return model;
}

function presetTimestamp() {
  const d = new Date();
  const p = (n) => String(n).padStart(2, "0");
  const date = `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
  const hm = `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
  return {
    file: `${date}_${hm.replace(/:/g, "_")}`,
    header: `${date}_${hm}`,
  };
}

function presetNameFor(device, token, stamp) {
  const safeDevice =
    device
      .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "")
      .replace(/\s+/g, "_")
      .replace(/_+/g, "_")
      .replace(/^_+|_+$/g, "") || "device";
  return `${safeDevice}_[${token}]_${stamp}`;
}

/** Preset source: manual selection when it is a real subset, otherwise the
 *  whole current tab (Enabled/User/System/…), matching the original app.
 *  Each item keeps its enabled/disabled/uninstalled state for the preset file. */
function presetSourceForCurrentView() {
  const visible = visibleApps();
  const selected = [...state.selected];
  const selectedSet = new Set(selected);
  const allVisibleSelected =
    visible.length > 0 && visible.every((a) => selectedSet.has(a.package));
  const useSelection = selected.length > 0 && !allVisibleSelected;
  let items;
  if (useSelection) {
    const byPkg = new Map(
      [...state.apps, ...state.removed].map((a) => [a.package, a])
    );
    items = selected.map(
      (pkg) => byPkg.get(pkg) || { package: pkg, disabled: false, removed: false }
    );
  } else {
    items = visible.map((a) => ({
      package: a.package,
      disabled: !!a.disabled,
      removed: !!a.removed,
    }));
  }
  const key = useSelection
    ? "selected"
    : PRESET_FILTERS[state.filter]
    ? state.filter
    : "all";
  return { items, meta: PRESET_FILTERS[key] };
}

function joinPath(dir, name) {
  const sep = dir.includes("\\") ? "\\" : "/";
  return dir.replace(/[\\/]+$/, "") + sep + name;
}

function openSavePreset() {
  if (!state.serial) return toast(t("cleanNoDevice"), true);
  const { items, meta } = presetSourceForCurrentView();
  if (!items.length) return toast(t("presetEmptyFilter"), true);
  const device = deviceDisplayName();
  const stamp = presetTimestamp();
  presetPending = { items, meta, device, stamp };
  $("presetCount").textContent = t("presetSelectedCount", {
    n: items.length,
    filter: t(meta.labelKey),
  });
  $("presetName").value = presetNameFor(device, meta.token, stamp.file);
  $("presetComment").value = `${device} ${meta.token} applications`;
  $("presetModal").classList.remove("hidden");
  $("presetName").focus();
  $("presetName").select();
}

function closeSavePreset() {
  $("presetModal").classList.add("hidden");
  presetPending = null;
}

async function savePreset() {
  const name = $("presetName").value.trim();
  const comment = $("presetComment").value.trim();
  if (!name) return toast(t("presetNameRequired"), true);
  const items = presetPending?.items ?? [];
  if (!items.length) {
    closeSavePreset();
    return toast(t("presetEmptyFilter"), true);
  }
  const dir = await safe(() => invoke("presets_dir"), t("errPreset"));
  if (!dir) return;
  const safeName =
    name
      .replace(/[<>:"/\\|?*\u0000-\u001f]/g, " ")
      .replace(/\s+/g, " ")
      .trim() || "preset";
  let target = await dialog.save({
    title: t("presetSaveTitle"),
    defaultPath: joinPath(dir, `${safeName}.txt`),
    filters: [{ name: "Preset", extensions: ["txt", "preset"] }],
  });
  if (Array.isArray(target)) target = target[0];
  if (!target) return;
  if (!/\.[^\\/]+$/.test(target)) target += ".txt";
  const res = await safe(
    () =>
      invoke("save_preset", {
        path: target,
        apps: items.map((a) => ({
          package: a.package,
          disabled: !!a.disabled,
          removed: !!a.removed,
        })),
        stamp: presetPending?.stamp?.header ?? "",
        comment,
      }),
    t("errPreset")
  );
  if (res) {
    closeSavePreset();
    log(res, "ok");
    toast(t("presetSaved", { path: target }));
  }
}

async function loadPreset() {
  if (!state.serial) return toast(t("cleanNoDevice"), true);
  let picked = await dialog.open({
    title: t("presetLoadTitle"),
    multiple: false,
    filters: [{ name: "Preset", extensions: ["txt", "preset"] }],
  });
  if (Array.isArray(picked)) picked = picked[0];
  if (!picked) return;
  const list = await safe(
    () => invoke("load_preset", { path: picked }),
    t("errPreset")
  );
  if (!Array.isArray(list)) return;
  const installed = new Set(state.apps.map((a) => a.package));
  state.selected.clear();
  let found = 0;
  let missing = 0;
  for (const pkg of list) {
    if (installed.has(pkg)) {
      state.selected.add(pkg);
      found++;
    } else {
      missing++;
    }
  }
  // Show the selection in the main list.
  state.filter = "all";
  state.search = "";
  $("searchInput").value = "";
  document
    .querySelectorAll(".tab")
    .forEach((el) => el.classList.toggle("active", el.dataset.filter === "all"));
  renderTable();
  updateSelectionUi();
  log(t("presetLoaded", { n: found }), "ok");
  toast(t("presetLoaded", { n: found }));
  if (missing) log(t("presetMissing", { n: missing }), "err");
}

/* ---------------- Permissions ---------------- */

function permKindLabel(kind) {
  if (kind === "runtime") return t("permKindRuntime");
  if (kind === "install") return t("permKindInstall");
  return t("permKindRequested");
}

function renderPermissions() {
  const open = !$("permModal").classList.contains("hidden");
  if (!open) return;

  $("permApp").textContent = permState.pkg
    ? `${permState.label || permState.pkg} · ${permState.pkg}`
    : "";

  const all = permState.items;
  const runtime = all.filter((p) => p.kind === "runtime");
  const granted = all.filter((p) => p.granted).length;
  $("permSummary").textContent = permState.loading
    ? t("permLoading")
    : permState.applying
    ? t("permApplying")
    : t("permSummary", { total: all.length, granted, runtime: runtime.length });

  $("permGrantAll").disabled =
    permState.loading || permState.applying || !runtime.some((p) => !p.granted);
  $("permRevokeAll").disabled =
    permState.loading || permState.applying || !runtime.some((p) => p.granted);
  $("permRefresh").disabled = permState.loading || permState.applying;

  const q = $("permSearch").value.trim().toLowerCase();
  const onlyGranted = $("permOnlyGranted").checked;
  const rows = all.filter((p) => {
    if (onlyGranted && !p.granted) return false;
    if (!q) return true;
    return p.name.toLowerCase().includes(q) || (p.group || "").toLowerCase().includes(q);
  });

  if (permState.loading) {
    $("permList").innerHTML = `<div class="perm-empty">${escapeHtml(t("permLoading"))}</div>`;
    return;
  }
  if (!rows.length) {
    $("permList").innerHTML = `<div class="perm-empty">${escapeHtml(t("permEmpty"))}</div>`;
    return;
  }

  const frag = document.createDocumentFragment();
  for (const p of rows) {
    const changeable = p.kind === "runtime";
    const row = document.createElement("div");
    row.className = `perm-row${p.granted ? " granted" : ""}${changeable ? "" : " readonly"}`;
    const groupRaw = p.group
      ? p.group.replace(/^android\.permission-group\./, "").replace(/_/g, " ")
      : "";
    const group = /^UNDEFINED$/i.test(groupRaw.trim()) ? "" : groupRaw;
    row.innerHTML = `
      <label class="perm-toggle" title="${escapeHtml(
        changeable ? (p.granted ? t("permGranted") : t("permDenied")) : t("permReadonly")
      )}">
        <input type="checkbox" data-perm="${escapeHtml(p.name)}" ${
          p.granted ? "checked" : ""
        } ${changeable && !permState.applying ? "" : "disabled"} />
        <span class="perm-switch"></span>
      </label>
      <div class="perm-info">
        <span class="perm-name" dir="ltr">${escapeHtml(p.name)}</span>
        <span class="perm-tags">
          ${
            p.dangerous
              ? `<span class="badge perm-danger">${escapeHtml(t("permDangerous"))}</span>`
              : ""
          }
          <span class="badge perm-kind k-${escapeHtml(p.kind)}">${escapeHtml(
            permKindLabel(p.kind)
          )}</span>
          ${
            changeable
              ? ""
              : `<span class="badge perm-ro">${escapeHtml(t("permReadonly"))}</span>`
          }
          ${group ? `<span class="perm-group">${escapeHtml(group)}</span>` : ""}
        </span>
      </div>
      <span class="perm-state ${p.granted ? "on" : "off"}">${escapeHtml(
        p.granted ? t("permGranted") : t("permDenied")
      )}</span>`;
    frag.appendChild(row);
  }
  $("permList").replaceChildren(frag);
}

async function loadPermissions() {
  if (!permState.pkg || !state.serial) return;
  permState.loading = true;
  renderPermissions();
  const list = await safe(
    () => invoke("list_permissions", { serial: state.serial, package: permState.pkg }),
    t("permLoadErr")
  );
  permState.loading = false;
  if (Array.isArray(list)) permState.items = list;
  renderPermissions();
}

async function applyPermissionChanges(permissions, granted) {
  if (!permState.pkg || !permissions.length) {
    toast(t("permNoChanges"));
    return;
  }
  permState.applying = true;
  renderPermissions();
  const res = await safe(
    () =>
      invoke("set_permissions", {
        serial: state.serial,
        package: permState.pkg,
        permissions,
        granted,
      }),
    t("permLoadErr")
  );
  permState.applying = false;
  if (!res) {
    renderPermissions();
    return;
  }
  const ok = Number(res.ok) || 0;
  const failed = Array.isArray(res.failed) ? res.failed : [];
  if (ok > 0) {
    const msg = t(granted ? "permGrantedToast" : "permRevokedToast", { n: ok });
    log(`${permState.pkg}: ${msg}`, "ok");
    toast(msg);
  }
  for (const f of failed) {
    log(`${permState.pkg} · ${f.permission}: ${f.error}`, "err");
  }
  if (failed.length) {
    toast(t("permFailedToast", { n: failed.length }), true);
  }
  await loadPermissions();
}

async function openPermissions() {
  if (!state.serial) return toast(t("cleanNoDevice"), true);
  const targets = [...state.selected];
  if (targets.length !== 1) return toast(t("permOneApp"), true);
  const pkg = targets[0];
  const app = state.apps.find((a) => a.package === pkg);
  if (!app || state.removed.some((a) => a.package === pkg)) {
    return toast(t("permRemoved"), true);
  }
  permState.pkg = pkg;
  permState.label = app.label || pkg;
  permState.items = [];
  permState.loading = true;
  permState.applying = false;
  $("permSearch").value = "";
  $("permOnlyGranted").checked = false;
  $("permModal").classList.remove("hidden");
  renderPermissions();
  await loadPermissions();
}

function closePermissions() {
  $("permModal").classList.add("hidden");
  permState.pkg = null;
  permState.items = [];
  permState.loading = false;
  permState.applying = false;
}

async function grantAllPermissions() {
  const perms = permState.items
    .filter((p) => p.kind === "runtime" && !p.granted)
    .map((p) => p.name);
  if (!perms.length) return toast(t("permNoChanges"));
  const ok = await dialog.confirm(t("permGrantAllConfirm"), {
    title: t("confirmTitle"),
    kind: "info",
  });
  if (!ok) return;
  await applyPermissionChanges(perms, true);
}

async function revokeAllPermissions() {
  const perms = permState.items
    .filter((p) => p.kind === "runtime" && p.granted)
    .map((p) => p.name);
  if (!perms.length) return toast(t("permNoChanges"));
  const ok = await dialog.confirm(t("permRevokeAllConfirm"), {
    title: t("confirmTitle"),
    kind: "warning",
  });
  if (!ok) return;
  await applyPermissionChanges(perms, false);
}

async function takeScreenshot() {
  if (!state.serial) return;
  setBusy(true, t("screenshotBusy"));
  const path = await safe(() => invoke("screenshot", { serial: state.serial }), t("errScreenshot"));
  setBusy(false);
  if (path) {
    log(t("screenshotLog", { path }), "ok");
    toast(t("screenshotSaved", { path }));
  }
}

async function showLogcat() {
  if (!state.serial) return;
  log("— logcat —", "cmd");
  const out = await safe(() => invoke("logcat", { serial: state.serial, lines: 300 }), t("errLogcat"));
  if (out) {
    for (const line of out.split("\n")) log(line);
    toast(t("logcatLoaded"));
  }
}

async function rebootDevice() {
  if (!state.serial) return;
  const ok = await dialog.confirm(t("rebootConfirm"), { title: t("confirmTitle"), kind: "warning" });
  if (!ok) return;
  setBusy(true, t("rebootBusy"));
  const out = await safe(() => invoke("reboot", { serial: state.serial, mode: "system" }), t("errReboot"));
  setBusy(false);
  if (out !== null) toast(t("rebootSent"));
}

async function runCustomCommand() {
  const input = $("cmdInput");
  const command = input.value.trim();
  if (!command || !state.serial) return;
  log(`$ adb shell ${command}`, "cmd");
  const out = await safe(() => invoke("run_command", { serial: state.serial, command }), t("errCommand"));
  if (out !== null && out.trim() !== "") {
    for (const line of out.split("\n")) log(line);
  }
  input.value = "";
}


/* ---------------- Full Console ---------------- */

const CONSOLE_QUICK = [
  "pm list packages -3",
  "pm list packages -d",
  "dumpsys battery",
  "getprop ro.product.model",
  "dumpsys meminfo",
  "logcat -d -t 200",
  "wm size",
  "pm list users",
];

const consoleLines = [];
const consoleHistory = [];
let historyIndex = -1;

function appendConsole(text, kind = "") {
  const out = $("consoleOut");
  if (!out) return;
  const line = document.createElement("div");
  if (kind) line.className = kind;
  line.textContent = text;
  out.appendChild(line);
  while (out.childElementCount > 1000) out.removeChild(out.firstChild);
  out.scrollTop = out.scrollHeight;
  consoleLines.push(text);
  while (consoleLines.length > 1000) consoleLines.shift();
}

function renderConsoleQuick() {
  const box = $("consoleQuick");
  if (!box) return;
  box.innerHTML = "";
  for (const cmd of CONSOLE_QUICK) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "console-chip";
    button.textContent = cmd;
    button.addEventListener("click", () => runConsoleCommand(cmd));
    box.appendChild(button);
  }
}

function updateConsoleDevice() {
  const el = $("consoleDevice");
  if (!el) return;
  el.textContent = state.serial
    ? t("cleanDeviceLine", { s: state.serial })
    : t("noDeviceFooter");
}

async function runConsoleCommand(raw) {
  const input = $("consoleInput");
  const cmd = String(raw ?? input.value).trim();
  if (!cmd) return;
  if (!state.serial) {
    toast(t("cleanNoDevice"), true);
    return;
  }
  input.value = "";
  appendConsole(`$ adb shell ${cmd}`, "cmd");
  if (consoleHistory[consoleHistory.length - 1] !== cmd) consoleHistory.push(cmd);
  if (consoleHistory.length > 100) consoleHistory.shift();
  historyIndex = -1;
  $("consoleRun").disabled = true;
  try {
    const out = await invoke("run_command", { serial: state.serial, command: cmd });
    const text = String(out ?? "").trimEnd();
    if (text) appendConsole(text);
  } catch (e) {
    appendConsole(String(e), "err");
  } finally {
    $("consoleRun").disabled = false;
    input.focus();
  }
}

function clearConsole() {
  consoleLines.length = 0;
  const out = $("consoleOut");
  if (out) out.innerHTML = "";
}

async function copyConsole() {
  const text = consoleLines.join("\n");
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch (_) {
    const area = document.createElement("textarea");
    area.value = text;
    document.body.appendChild(area);
    area.select();
    document.execCommand("copy");
    area.remove();
  }
  toast(t("consoleCopied"));
}

function openConsole() {
  $("consoleModal").classList.remove("hidden");
  updateConsoleDevice();
  const input = $("consoleInput");
  if (input) input.focus();
}

function closeConsole() {
  $("consoleModal").classList.add("hidden");
}

function wireConsole() {
  $("btnConsole").addEventListener("click", openConsole);
  $("consoleClose").addEventListener("click", closeConsole);
  $("consoleClear").addEventListener("click", clearConsole);
  $("consoleCopy").addEventListener("click", copyConsole);
  $("consoleRun").addEventListener("click", () => runConsoleCommand());
  $("consoleInput").addEventListener("keydown", (e) => {
    const input = $("consoleInput");
    if (e.key === "Enter") {
      e.preventDefault();
      runConsoleCommand();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (!consoleHistory.length) return;
      historyIndex =
        historyIndex < 0 ? consoleHistory.length - 1 : Math.max(0, historyIndex - 1);
      input.value = consoleHistory[historyIndex];
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (historyIndex < 0) return;
      historyIndex += 1;
      if (historyIndex >= consoleHistory.length) {
        historyIndex = -1;
        input.value = "";
      } else {
        input.value = consoleHistory[historyIndex];
      }
    } else if (e.key === "l" && e.ctrlKey) {
      e.preventDefault();
      clearConsole();
    }
  });
  renderConsoleQuick();
  updateConsoleDevice();
}

/* ---------- CleanYourTV ---------- */
const cleanModal = $("cleanModal");

async function openClean() {
  if (!state.serial) {
    toast(t("cleanNoDevice"), true);
    return;
  }
  cleanModal.classList.remove("hidden");
  $("cleanDevice").textContent = t("cleanDeviceLine", { s: state.serial });
  await loadCytProfiles();
}

async function loadCytProfiles() {
  const profiles = await safe(() => invoke("debloat_profiles"), t("errClean"));
  const select = $("cytProfile");
  select.innerHTML = "";
  for (const p of profiles || []) {
    const option = document.createElement("option");
    option.value = p.id;
    option.textContent = `${p.title}${p.os ? " · " + p.os : ""}`;
    select.appendChild(option);
  }
  $("cytStatus").textContent = profiles?.length
    ? t("cleanProfiles", { n: profiles.length })
    : t("cleanNoProfiles");
  if (!profiles?.length) {
    $("cytCount").textContent = "";
    return;
  }
  const matched = await safe(() => invoke("debloat_match", { serial: state.serial }), t("errMatch"));
  if (matched) select.value = matched;
  await updateCytCount();
}

async function updateCytCount() {
  const profile = $("cytProfile").value;
  if (!profile) return;
  const packages = await safe(() => invoke("debloat_packages", { profile }), t("errPackages"));
  $("cytCount").textContent = packages ? t("cleanProfileCount", { n: packages.length }) : "";
}

async function fetchCyt() {
  setBusy(true, t("cleanDownloading"));
  const count = await safe(() => invoke("debloat_fetch"), t("errDownload"));
  setBusy(false);
  if (count !== null) {
    toast(t("cleanPrepared", { n: count }));
    await loadCytProfiles();
  }
}

async function applyCyt(enable) {
  const profile = $("cytProfile").value;
  if (!profile) return toast(t("cleanPickProfile"), true);
  const ok = await dialog.confirm(
    enable ? t("cleanConfirmEnable") : t("cleanConfirmDisable"),
    { title: "CleanYourTV", kind: enable ? "info" : "warning" }
  );
  if (!ok) return;
  setBusy(true, enable ? t("cleanRestoring") : t("cleanDisabling"));
  const res = await safe(
    () => invoke("debloat_apply", { serial: state.serial, profile, enable }),
    t("errClean")
  );
  setBusy(false);
  if (res !== null) {
    log(res, "ok");
    toast(res);
    await refreshApps();
  }
}

function wireClean() {
  $("btnClean")?.addEventListener("click", () => openClean());
  $("btnWireless")?.addEventListener("click", () => toggleWireless());
  $("cytClose")?.addEventListener("click", () => cleanModal.classList.add("hidden"));
  $("cytFetch")?.addEventListener("click", () => fetchCyt());
  $("cytProfile")?.addEventListener("change", () => updateCytCount());
  $("cytDisable")?.addEventListener("click", () => applyCyt(false));
  $("cytEnable")?.addEventListener("click", () => applyCyt(true));
}


async function toggleWireless() {
  if (!state.serial) return toast(t("cleanNoDevice"), true);
  const isNetwork = state.serial.includes(":");
  setBusy(true, isNetwork ? t("wirelessDisabling") : t("wirelessEnabling"));
  if (isNetwork) {
    const res = await safe(() => invoke("disable_wireless", { serial: state.serial }), t("errWireless"));
    setBusy(false);
    if (res) {
      log(res, "ok");
      toast(res);
      await refreshDevices();
    }
  } else {
    const endpoint = await safe(() => invoke("enable_wireless", { serial: state.serial }), t("errWireless"));
    setBusy(false);
    if (endpoint) {
      log(t("wirelessLog", { ep: endpoint }), "ok");
      toast(t("wirelessEnabled", { ep: endpoint }));
      await refreshDevices();
      if (state.devices.some((d) => d.serial === endpoint)) {
        state.serial = endpoint;
        $("deviceSelect").value = endpoint;
        await onDeviceChanged();
      }
    }
  }
  updateWirelessLabel();
}

function updateWirelessLabel() {
  const label = $("wirelessLabel");
  if (!label) return;
  label.textContent = state.serial && state.serial.includes(":")
    ? t("wirelessDisableLabel")
    : t("wirelessEnableLabel");
}

/* ---------------- Wiring ---------------- */

function wire() {
  wireClean();
  wireConsole();
  document.addEventListener("keydown", (e) => {
    if (e.key !== "Escape") return;
    if (!$("consoleModal").classList.contains("hidden")) closeConsole();
    if (!$("procModal").classList.contains("hidden")) $("procModal").classList.add("hidden");
    if (!$("permModal").classList.contains("hidden")) closePermissions();
    if (!$("presetModal").classList.contains("hidden")) closeSavePreset();
  });
  $("deviceSelect").addEventListener("change", async (e) => {
    state.serial = e.target.value || null;
    $("footerDevice").textContent = state.serial || t("noDeviceFooter");
    await onDeviceChanged();
  });

  $("refreshDevices").addEventListener("click", () => refreshDevices());

  $("langBtn").addEventListener("click", () => window.I18n.toggle());

  $("connectBtn").addEventListener("click", async () => {
    const addr = $("connectAddr").value.trim();
    if (!addr) return toast(t("enterIp"), true);
    setBusy(true, t("connecting"));
    const out = await safe(() => invoke("connect_device", { addr }), t("errConnection"));
    setBusy(false);
    if (out) {
      log(out, out.includes("connected") ? "ok" : "err");
      toast(out.trim());
      await refreshDevices();
    }
  });

  $("connectAddr").addEventListener("keydown", (e) => {
    if (e.key === "Enter") $("connectBtn").click();
  });

  document.querySelectorAll(".tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      document.querySelectorAll(".tab").forEach((el) => el.classList.remove("active"));
      tab.classList.add("active");
      state.filter = tab.dataset.filter;
      renderTable();
    });
  });

  $("searchInput").addEventListener("input", (e) => {
    state.search = e.target.value;
    renderTable();
  });

  $("appsBody").addEventListener("click", (e) => {
    const row = e.target.closest("tr[data-pkg]");
    if (!row) return;
    const pkg = row.dataset.pkg;
    if (e.target.matches('input[type="checkbox"]')) {
      toggleRow(pkg, e.target.checked);
    } else if (!e.target.closest("input")) {
      toggleRow(pkg, !state.selected.has(pkg));
    }
  });

  $("selectAll").addEventListener("change", (e) => {
    const rows = visibleApps();
    if (e.target.checked) rows.forEach((a) => state.selected.add(a.package));
    else rows.forEach((a) => state.selected.delete(a.package));
    renderTable();
  });

  $("btnDisable").addEventListener("click", () =>
    applyToSelection("disable", t("confirmDisable"))
  );
  $("btnEnable").addEventListener("click", () => applyToSelection("enable"));
  $("btnClearCache").addEventListener("click", () => applyToSelection("clear_cache"));
  $("btnClear").addEventListener("click", () =>
    applyToSelection("clear_data", t("confirmClearData"))
  );
  $("btnUninstall").addEventListener("click", () =>
    applyToSelection("uninstall", t("confirmUninstall"))
  );
  $("btnRestore").addEventListener("click", () =>
    applyToSelection("restore", t("confirmRestore"))
  );
  $("btnUninstallDeep").addEventListener("click", () => {
    const risky = riskyCount([...state.selected]);
    const extra = risky ? `\n\n${t("deepRisky", { n: risky })}` : "";
    return applyToSelection("uninstall_deep", t("confirmDeep") + extra);
  });
  $("btnSaveApk").addEventListener("click", () => saveApks());
  $("btnBackupZip").addEventListener("click", () => backupSelected());
  $("btnPermissions").addEventListener("click", () => openPermissions());
  $("btnInstallApk").addEventListener("click", () => installApk());
  $("btnScreenshot").addEventListener("click", () => takeScreenshot());
  $("btnLogcat").addEventListener("click", () => showLogcat());
  $("btnRefreshApps").addEventListener("click", () => refreshApps());
  $("btnIcons").addEventListener("click", () => syncIcons());
  $("btnAdb").addEventListener("click", () => chooseAdb());
  $("btnReboot").addEventListener("click", () => rebootDevice());
  $("btnPresetSave").addEventListener("click", () => openSavePreset());
  $("btnPresetLoad").addEventListener("click", () => loadPreset());
  $("presetSave").addEventListener("click", () => savePreset());
  $("presetCancel").addEventListener("click", () => closeSavePreset());
  $("presetName").addEventListener("keydown", (e) => {
    if (e.key === "Enter") savePreset();
  });
  $("btnProcesses").addEventListener("click", () => openProcesses());
  $("procClose").addEventListener("click", () => $("procModal").classList.add("hidden"));
  $("procRefresh").addEventListener("click", async () => {
    await refreshMemory();
    await loadProcesses();
  });
  $("procBody").addEventListener("click", (e) => {
    const btn = e.target.closest(".proc-stop");
    if (btn) stopProcess(btn.dataset.name);
  });
  $("cmdRun").addEventListener("click", () => runCustomCommand());
  $("cmdInput").addEventListener("keydown", (e) => {
    if (e.key === "Enter") runCustomCommand();
  });

  $("permClose").addEventListener("click", () => closePermissions());
  $("permRefresh").addEventListener("click", () => loadPermissions());
  $("permSearch").addEventListener("input", () => renderPermissions());
  $("permOnlyGranted").addEventListener("change", () => renderPermissions());
  $("permGrantAll").addEventListener("click", () => grantAllPermissions());
  $("permRevokeAll").addEventListener("click", () => revokeAllPermissions());
  $("permList").addEventListener("change", (e) => {
    const cb = e.target.closest('input[data-perm]');
    if (!cb) return;
    applyPermissionChanges([cb.dataset.perm], cb.checked);
  });
}

function platformName() {
  return /Windows/i.test(navigator.userAgent) ? "Windows" : "Desktop";
}

function refreshShellText() {
  const sub = document.getElementById("brandSub");
  if (sub) sub.textContent = `${state.platform} · ${t("openSource")}`;
  document.title = "ADB Control " + state.platform;
  const footer = $("footerDevice");
  if (footer) footer.textContent = state.serial || t("noDeviceFooter");
}

async function boot() {
  state.platform = platformName();
  window.I18n.apply(document);
  wire();
  refreshShellText();
  window.addEventListener("langchange", () => {
    refreshShellText();
    renderDeviceInfo(state.info);
    renderTable();
    updateSelectionUi();
    updateWirelessLabel();
    updateMemoryUi();
    renderProcesses();
    renderPermissions();
    updateConsoleDevice();
  });
  if (!invoke) {
    log(t("tauriUnavailable"), "err");
    return;
  }
  log(`ADB AppControl — ${state.platform} (MVP)`, "ok");
  await initAdb();
  await refreshDevices();
  updateSelectionUi();
}

boot();
