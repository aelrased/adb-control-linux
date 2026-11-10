/* ADB AppControl — Linux frontend
   SPDX-License-Identifier: GPL-3.0-or-later */

const T = window.__TAURI__;
const invoke = T?.core?.invoke;
const dialog = T?.dialog;

const $ = (id) => document.getElementById(id);

const state = {
  devices: [],
  serial: null,
  apps: [],
  filter: "all",
  search: "",
  selected: new Set(),
  busy: false,
};

const consoleEl = $("console");

function log(message, kind = "") {
  const line = document.createElement("div");
  if (kind) line.className = kind;
  line.textContent = message;
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
  el.textContent = message;
  el.classList.toggle("err", isError);
  el.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove("show"), 3200);
}

function setBusy(on, text = "جارٍ العمل…") {
  state.busy = on;
  $("progress").classList.toggle("hidden", !on);
  $("statusText").textContent = on ? text : "جاهز";
  document.querySelectorAll(".btn, .icon-btn, .quick, .tab").forEach((b) => {
    if (b.id !== "refreshDevices" || !on) b.disabled = on;
  });
  if (!on) updateSelectionUi();
}

async function safe(fn, errPrefix = "خطأ") {
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
  const status = await safe(() => invoke("adb_status"), "adb");
  const chip = $("adbStatus");
  if (status && status.version) {
    chip.textContent = status.version;
    chip.title = status.path;
    chip.classList.add("ok");
    chip.classList.remove("bad");
  } else {
    chip.textContent = "adb غير متاح — اضغط ⚙️";
    chip.title = status?.path ?? "";
    chip.classList.add("bad");
    chip.classList.remove("ok");
    toast("لم يتم العثور على adb — اضغط «تحديد adb» واختر adb.exe", true);
  }
}

async function chooseAdb() {
  const picked = await dialog.open({
    multiple: false,
    title: "اختر ملف adb (platform-tools/adb.exe على ويندوز)",
  });
  if (!picked) return;
  const version = await safe(() => invoke("set_adb_path", { path: picked }), "adb");
  if (version) {
    log(`adb: ${picked}\n${version}`, "ok");
    toast("تم ضبط adb بنجاح");
    await initAdb();
    await refreshDevices();
  }
}

async function refreshDevices() {
  const devices = await safe(() => invoke("list_devices"), "الأجهزة");
  if (!devices) return;
  state.devices = devices;
  const select = $("deviceSelect");
  select.innerHTML = "";
  if (!devices.length) {
    select.innerHTML = '<option value="">— لا يوجد جهاز —</option>';
    state.serial = null;
    renderDeviceInfo(null);
    return;
  }
  for (const d of devices) {
    const opt = document.createElement("option");
    opt.value = d.serial;
    const name = d.model ? d.model.replace(/_/g, " ") : d.serial;
    opt.textContent = `${name} · ${d.state}${d.serial.includes(":") ? " (Wi‑Fi)" : ""}`;
    select.appendChild(opt);
  }
  const preferred = devices.find((d) => d.state === "device") || devices[0];
  if (!state.serial || !devices.some((d) => d.serial === state.serial)) {
    state.serial = preferred.serial;
  }
  select.value = state.serial;
  $("footerDevice").textContent = state.serial;
  await onDeviceChanged();
}

async function onDeviceChanged() {
  state.selected.clear();
  updateSelectionUi();
  if (!state.serial) return;
  const info = await safe(() => invoke("device_info", { serial: state.serial }), "معلومات الجهاز");
  renderDeviceInfo(info);
  await refreshApps();
  const bridge = await safe(() => invoke("bridge_status", { serial: state.serial }), "حالة الأيقونات");
  if (state.apps.length && (!bridge || bridge.icons === 0)) {
    toast("لا توجد أيقونات محفوظة — اضغط «أيقونات التطبيقات» لقراءتها من الجهاز");
  }
}

function renderDeviceInfo(info) {
  const box = $("deviceInfo");
  if (!info) {
    box.innerHTML = '<div class="empty-hint">لم يتم اختيار جهاز</div>';
    return;
  }
  const rows = [
    ["الماركة", info["ro.product.manufacturer"]],
    ["الطراز", info["ro.product.model"]],
    ["أندرويد", info["ro.build.version.release"]],
    ["API", info["ro.build.version.sdk"]],
    ["المعمارية", info["ro.product.cpu.abi"]],
    ["البطارية", info["battery"] ? `${info["battery"]}%` : ""],
    ["الرقم التسلسلي", info["ro.serialno"] || state.serial],
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

/* ---------------- Apps ---------------- */

async function refreshApps() {
  if (!state.serial) {
    state.apps = [];
    renderTable();
    return;
  }
  setBusy(true, "قراءة قائمة التطبيقات…");
  const apps = await safe(() => invoke("list_apps", { serial: state.serial }), "التطبيقات");
  setBusy(false);
  if (!apps) return;
  state.apps = apps;
  log(`تم تحميل ${apps.length} تطبيقاً من ${state.serial}`, "ok");
  updateCounts();
  renderTable();
}

function updateCounts() {
  const user = state.apps.filter((a) => !a.system).length;
  const system = state.apps.filter((a) => a.system).length;
  const disabled = state.apps.filter((a) => a.disabled).length;
  $("countAll").textContent = state.apps.length;
  $("countUser").textContent = user;
  $("countSystem").textContent = system;
  $("countDisabled").textContent = disabled;
}

function visibleApps() {
  const q = state.search.trim().toLowerCase();
  return state.apps.filter((a) => {
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
    const url = await safe(() => invoke("get_icon", { serial, package: pkg }), "أيقونة");
    iconCache.set(pkg, url || null);
    iconPending.delete(pkg);
    document.querySelectorAll(`.app-icon[data-pkg="${CSS.escape(pkg)}"]`).forEach(applyCachedIcon);
  });
  pumpIcons();
}

async function syncIcons() {
  if (!state.serial) return;
  const ok = await dialog.confirm(
    "سيتم تثبيت تطبيق مساعد صغير على الجهاز (ADB AppControl Bridge) لقراءة الأيقونات والأسماء الحقيقية للتطبيقات.\n\nستظهر شاشة سوداء على التلفزيون لثوانٍ أثناء القراءة. متابعة؟",
    { title: "أيقونات التطبيقات", kind: "info" }
  );
  if (!ok) return;
  setBusy(true, "قراءة الأيقونات من الجهاز…");
  const res = await safe(() => invoke("sync_bridge", { serial: state.serial }), "الأيقونات");
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
    tbody.innerHTML = '<tr><td colspan="5" class="empty-row">لم يتم اختيار جهاز</td></tr>';
    updateSelectionUi();
    return;
  }
  if (!rows.length) {
    tbody.innerHTML = '<tr><td colspan="5" class="empty-row">لا توجد نتائج</td></tr>';
    updateSelectionUi();
    return;
  }
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
          </div>
        </div>
      </td>
      <td class="col-ver"><span class="ver">${escapeHtml(app.version || "—")}</span></td>
      <td class="col-type"><span class="badge ${app.system ? "sys" : "user"}">${
        app.system ? "نظام" : "مستخدم"
      }</span></td>
      <td class="col-state"><span class="badge ${app.disabled ? "off" : "on"}">${
        app.disabled ? "معطّل" : "مفعّل"
      }</span></td>`;
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
  ["btnDisable", "btnEnable", "btnClear", "btnUninstall", "btnSaveApk"].forEach((id) => {
    $(id).disabled = !has;
  });
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
    const ok = await dialog.confirm(confirmText, { title: "تأكيد", kind: "warning" });
    if (!ok) return;
  }
  setBusy(true, "تنفيذ الإجراء…");
  let done = 0;
  for (const pkg of targets) {
    const res = await safe(() => invoke("app_action", { serial: state.serial, package: pkg, action }), pkg);
    if (res !== null) {
      done++;
      log(`${pkg}: ${res}`, "ok");
    }
  }
  setBusy(false);
  toast(`تم تنفيذ الإجراء على ${done}/${targets.length}`);
  state.selected.clear();
  await refreshApps();
}

async function installApk() {
  if (!state.serial) return;
  const picked = await dialog.open({
    multiple: true,
    title: "اختر APK أو حزمة .apks / .xapk",
    filters: [{ name: "Android Package", extensions: ["apk", "apks", "xapk", "zip"] }],
  });
  if (!picked) return;
  const files = Array.isArray(picked) ? picked : [picked];
  setBusy(true, "تثبيت…");
  const res = await safe(
    () => invoke("install_apks", { serial: state.serial, paths: files, reinstall: true }),
    "التثبيت"
  );
  setBusy(false);
  if (res !== null) {
    log(res, "ok");
    toast("تم التثبيت بنجاح");
  }
  await refreshApps();
}

async function saveApks() {
  const targets = [...state.selected];
  if (!targets.length) return;
  setBusy(true, "سحب APK…");
  for (const pkg of targets) {
    const res = await safe(() => invoke("extract_apk", { serial: state.serial, package: pkg }), pkg);
    if (res) {
      log(`${pkg} → ${res}`, "ok");
      toast("تم حفظ APK في Downloads");
    }
  }
  setBusy(false);
}

async function takeScreenshot() {
  if (!state.serial) return;
  setBusy(true, "لقطة شاشة…");
  const path = await safe(() => invoke("screenshot", { serial: state.serial }), "لقطة الشاشة");
  setBusy(false);
  if (path) {
    log(`لقطة محفوظة: ${path}`, "ok");
    toast(`تم الحفظ: ${path}`);
  }
}

async function showLogcat() {
  if (!state.serial) return;
  log("— logcat —", "cmd");
  const out = await safe(() => invoke("logcat", { serial: state.serial, lines: 300 }), "logcat");
  if (out) {
    for (const line of out.split("\n")) log(line);
    toast("تم تحميل logcat");
  }
}

async function rebootDevice() {
  if (!state.serial) return;
  const ok = await dialog.confirm("إعادة تشغيل الجهاز الآن؟", { title: "تأكيد", kind: "warning" });
  if (!ok) return;
  setBusy(true, "إعادة تشغيل…");
  const out = await safe(() => invoke("reboot", { serial: state.serial, mode: "system" }), "إعادة التشغيل");
  setBusy(false);
  if (out !== null) toast("تم إرسال أمر إعادة التشغيل");
}

async function runCustomCommand() {
  const input = $("cmdInput");
  const command = input.value.trim();
  if (!command || !state.serial) return;
  log(`$ adb shell ${command}`, "cmd");
  const out = await safe(() => invoke("run_command", { serial: state.serial, command }), "الأمر");
  if (out !== null && out.trim() !== "") {
    for (const line of out.split("\n")) log(line);
  }
  input.value = "";
}

/* ---------------- Wiring ---------------- */

function wire() {
  $("deviceSelect").addEventListener("change", async (e) => {
    state.serial = e.target.value || null;
    $("footerDevice").textContent = state.serial || "لا جهاز";
    await onDeviceChanged();
  });

  $("refreshDevices").addEventListener("click", () => refreshDevices());

  $("connectBtn").addEventListener("click", async () => {
    const addr = $("connectAddr").value.trim();
    if (!addr) return toast("أدخل عنوان IP", true);
    setBusy(true, "اتصال…");
    const out = await safe(() => invoke("connect_device", { addr }), "الاتصال");
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
      document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
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
    applyToSelection("disable", "تعطيل التطبيقات المحددة؟ يمكن تفعيلها لاحقاً.")
  );
  $("btnEnable").addEventListener("click", () => applyToSelection("enable"));
  $("btnClear").addEventListener("click", () =>
    applyToSelection("clear_data", "مسح بيانات التطبيقات المحددة؟ لا يمكن التراجع.")
  );
  $("btnUninstall").addEventListener("click", () =>
    applyToSelection("uninstall", "حذف التطبيقات المحددة لهذا المستخدم؟ (تطبيقات النظام قد تفشل)")
  );
  $("btnSaveApk").addEventListener("click", () => saveApks());
  $("btnInstallApk").addEventListener("click", () => installApk());
  $("btnScreenshot").addEventListener("click", () => takeScreenshot());
  $("btnLogcat").addEventListener("click", () => showLogcat());
  $("btnRefreshApps").addEventListener("click", () => refreshApps());
  $("btnIcons").addEventListener("click", () => syncIcons());
  $("btnAdb").addEventListener("click", () => chooseAdb());
  $("btnReboot").addEventListener("click", () => rebootDevice());
  $("cmdRun").addEventListener("click", () => runCustomCommand());
  $("cmdInput").addEventListener("keydown", (e) => {
    if (e.key === "Enter") runCustomCommand();
  });
}

async function boot() {
  wire();
  const platform = /Windows/i.test(navigator.userAgent) ? "Windows" : "Linux";
  const sub = document.getElementById("brandSub");
  if (sub) sub.textContent = platform + " · Open Source";
  document.title = "ADB Control " + platform;
  if (!invoke) {
    log("Tauri API غير متاح — شغّل التطبيق عبر cargo run", "err");
    return;
  }
  log("ADB AppControl — Linux (MVP)", "ok");
  await initAdb();
  await refreshDevices();
  updateSelectionUi();
}

boot();
