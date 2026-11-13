/* ADB AppControl — runtime localization (Arabic source, English overlay).
   SPDX-License-Identifier: GPL-3.0-or-later */

(function () {
  const STORAGE_KEY = "adbcontrol.lang";

  /* [Arabic, English] */
  const S = {
    /* --- static UI --- */
    device: ["الجهاز", "Device"],
    deviceSelectTitle: ["اختر الجهاز المتصل", "Choose the connected device"],
    refreshDevices: ["تحديث الأجهزة", "Refresh devices"],
    connectWifi: ["اتصال Wi‑Fi", "Wi‑Fi connect"],
    deviceInfo: ["معلومات الجهاز", "Device info"],
    noDevice: ["لم يتم اختيار جهاز", "No device selected"],
    ramTitle: ["الذاكرة", "Memory"],
    processManager: ["مدير العمليات", "Process Manager"],
    consoleTitle: ["Console — ADB Shell", "Console — ADB Shell"],
    consoleOpen: ["فتح Console", "Open Console"],
    consoleCopy: ["نسخ", "Copy"],
    consoleClear: ["مسح", "Clear"],
    consoleHint: [
      "Enter للتنفيذ · ↑/↓ لسجل الأوامر · Ctrl+L للمسح",
      "Enter to run · ↑/↓ for history · Ctrl+L to clear",
    ],
    consoleCopied: ["تم نسخ مخرجات Console", "Console output copied"],
    processesTitle: ["مدير العمليات", "Process Manager"],
    procRefresh: ["تحديث", "Refresh"],
    procName: ["العملية", "Process"],
    procRam: ["الذاكرة", "Memory"],
    procAction: ["إجراء", "Action"],
    procStop: ["إيقاف", "Stop"],
    procStopConfirm: ["إيقاف العملية {name}؟ (am force-stop)", "Stop process {name}? (am force-stop)"],
    procSummary: [
      "RAM {used} / {total} MB · {n} عملية",
      "RAM {used} / {total} MB · {n} processes",
    ],
    procCount: ["{n} عملية", "{n} processes"],
    procEmpty: ["لا توجد عمليات", "No processes"],
    errMemory: ["الذاكرة", "Memory"],
    errProcesses: ["العمليات", "Processes"],
    errKill: ["إيقاف العملية", "Stop process"],
    quickTools: ["أدوات سريعة", "Quick tools"],
    screenshot: ["لقطة شاشة", "Screenshot"],
    refreshList: ["تحديث القائمة", "Refresh list"],
    appIcons: ["أيقونات التطبيقات", "App icons"],
    chooseAdb: ["تحديد adb", "Choose adb"],
    reboot: ["إعادة تشغيل", "Reboot"],
    adbTerminal: ["طرفية ADB", "ADB terminal"],
    run: ["تنفيذ", "Run"],
    all: ["الكل", "All"],
    enabled: ["المفعّلة", "Enabled"],
    user: ["مستخدم", "User"],
    system: ["نظام", "System"],
    disabled: ["معطّل", "Disabled"],
    removed: ["محذوفة", "Removed"],
    removedTabTitle: [
      "تطبيقات محذوفة للمستخدم 0 ويمكن استعادتها",
      "Apps removed for user 0 that can be restored",
    ],
    search: ["ابحث بالاسم أو اسم الحزمة…", "Search by name or package…"],
    selectAll: ["تحديد الكل", "Select all"],
    colApp: ["التطبيق", "App"],
    colVersion: ["الإصدار", "Version"],
    colType: ["النوع", "Type"],
    colState: ["الحالة", "State"],
    selected: ["محدد", "selected"],
    btnDisable: ["تعطيل", "Disable"],
    btnEnable: ["تفعيل", "Enable"],
    btnClearCache: ["مسح الكاش", "Clear cache"],
    btnClearData: ["مسح البيانات", "Clear data"],
    btnUninstall: ["حذف", "Uninstall"],
    btnRestore: ["استعادة", "Restore"],
    btnRestoreTitle: [
      "pm install-existing — استعادة تطبيق محذوف للمستخدم 0",
      "pm install-existing — restore an app removed for user 0",
    ],
    btnUninstallDeep: ["حذف نهائي", "Deep uninstall"],
    btnUninstallDeepTitle: [
      "إزالة تحديثات النظام ثم الحذف للمستخدم 0 — نسخة /system تتطلب روت",
      "Remove system updates then uninstall for user 0 — the /system copy requires root",
    ],
    btnSaveApk: ["حفظ APK", "Save APK"],
    btnBackupZip: ["نسخة ZIP", "ZIP backup"],
    btnBackupZipTitle: [
      "نسخة احتياطية ZIP تحوي ملفات APK الأصلية للتطبيقات المحددة",
      "ZIP backup containing the original APK files of the selected apps",
    ],
    backupConfirm: [
      "إنشاء نسخة احتياطية ZIP لـ {n} تطبيق؟\n\nسيتم حفظ ملفات APK الأصلية فقط، بدون بيانات التطبيقات أو إعداداتها.",
      "Create a ZIP backup of {n} app(s)?\n\nOnly the original APK files are saved — app data and settings are not included.",
    ],
    backupBusy: ["سحب ملفات APK وإنشاء النسخة الاحتياطية…", "Pulling APK files and creating the backup…"],
    backupDone: ["تم حفظ النسخة الاحتياطية: {path}", "Backup saved: {path}"],
    backupPartial: [
      "تم نسخ {ok} تطبيقاً، وتعذّر نسخ {n}",
      "Backed up {ok} app(s); {n} failed",
    ],
    backupNoTargets: ["لا توجد تطبيقات صالحة للنسخ", "No apps available to back up"],
    errBackup: ["النسخة الاحتياطية", "Backup"],
    btnPresetSave: ["حفظ Preset", "Save preset"],
    btnPresetLoad: ["تحميل Preset", "Load preset"],
    btnPresetSaveTitle: [
      "حفظ التطبيقات المحددة في ملف Preset نصي",
      "Save the selected apps to a preset text file",
    ],
    btnPresetLoadTitle: [
      "تحميل Preset وتحديد تطبيقاته تلقائياً",
      "Load a preset and select its apps automatically",
    ],
    presetSaveTitle: ["حفظ Preset", "Save preset"],
    presetLoadTitle: ["اختر ملف Preset", "Choose a preset file"],
    presetName: ["اسم الـ Preset", "Preset name"],
    presetComment: ["الوصف (اختياري)", "Description (optional)"],
    presetSave: ["حفظ", "Save"],
    presetSelectedCount: [
      "{n} تطبيقاً سيُحفظ في الملف — المرشّح: {filter}",
      "{n} app(s) will be saved — filter: {filter}",
    ],
    presetEmptyFilter: [
      "لا توجد تطبيقات في هذا التبويب لحفظها",
      "No apps in this tab to save",
    ],
    presetFilterAll: ["الكل", "All"],
    presetFilterEnabled: ["المفعّلة", "Enabled"],
    presetFilterUser: ["المستخدمة", "User"],
    presetFilterSystem: ["النظام", "System"],
    presetFilterDisabled: ["المعطّلة", "Disabled"],
    presetFilterRemoved: ["المحذوفة", "Uninstalled"],
    presetFilterSelected: ["المحددة", "Selected"],
    presetHint: [
      "ملف نصي بسيط بأسماء الحزم، يمكن مشاركته واستخدامه على أجهزة أخرى.",
      "A simple text file of package names that can be shared and reused on other devices.",
    ],
    presetNameRequired: ["أدخل اسماً للـ Preset", "Enter a preset name"],
    presetNoSelection: ["حدد تطبيقاً واحداً على الأقل", "Select at least one app"],
    presetSaved: ["تم حفظ Preset: {path}", "Preset saved: {path}"],
    presetLoaded: ["تم تحديد {n} تطبيقاً من Preset", "Selected {n} app(s) from the preset"],
    presetMissing: [
      "تعذّر العثور على {n} حزمة من Preset على هذا الجهاز",
      "{n} preset package(s) were not found on this device",
    ],
    errPreset: ["Preset", "Preset"],
    btnPermissions: ["الصلاحيات", "Permissions"],
    btnPermissionsTitle: [
      "عرض صلاحيات تطبيق واحد ومنحها أو سحبها",
      "View, grant, or revoke permissions for a single app",
    ],
    permTitle: ["إدارة الصلاحيات", "Permissions"],
    permHelp: [
      "منح أو سحب صلاحيات التشغيل لهذا التطبيق. الصلاحيات المثبّتة وقت التثبيت للقراءة فقط.",
      "Grant or revoke this app's runtime permissions. Install-time permissions are read-only.",
    ],
    permSearch: ["ابحث عن صلاحية…", "Search permissions…"],
    permOnlyGranted: ["الممنوحة فقط", "Granted only"],
    permRefresh: ["تحديث", "Refresh"],
    permGrantAll: ["منح الكل", "Grant all"],
    permRevokeAll: ["سحب الكل", "Revoke all"],
    permEmpty: ["لا توجد صلاحيات مطابقة", "No matching permissions"],
    permLoading: ["قراءة الصلاحيات من الجهاز…", "Reading permissions from the device…"],
    permSummary: [
      "{total} صلاحية · {granted} ممنوحة · {runtime} قابلة للتغيير",
      "{total} permissions · {granted} granted · {runtime} changeable",
    ],
    permGranted: ["ممنوحة", "Granted"],
    permDenied: ["غير ممنوحة", "Not granted"],
    permKindRuntime: ["تشغيل", "Runtime"],
    permKindInstall: ["تثبيت", "Install"],
    permKindRequested: ["مطلوبة", "Requested"],
    permDangerous: ["خطرة", "Dangerous"],
    permReadonly: ["للقراءة فقط", "Read-only"],
    permOneApp: ["اختر تطبيقاً واحداً مثبتاً", "Select exactly one installed app"],
    permRemoved: [
      "لا يمكن إدارة صلاحيات تطبيق محذوف — استعده أولاً",
      "Cannot manage permissions for a removed app — restore it first",
    ],
    permRevokeAllConfirm: [
      "سحب جميع صلاحيات التشغيل من هذا التطبيق؟ قد يتوقف عن العمل بشكل صحيح.",
      "Revoke all runtime permissions from this app? It may stop working correctly.",
    ],
    permGrantedToast: ["تم منح {n} صلاحية", "Granted {n} permission(s)"],
    permRevokedToast: ["تم سحب {n} صلاحية", "Revoked {n} permission(s)"],
    permFailedToast: ["تعذّر تغيير {n} صلاحية", "Could not change {n} permission(s)"],
    permNoChanges: ["لا توجد صلاحيات لتغييرها", "Nothing to change"],
    permApplying: ["تطبيق تغييرات الصلاحيات…", "Applying permission changes…"],
    permLoadErr: ["الصلاحيات", "Permissions"],
    permGrantAllConfirm: [
      "منح جميع صلاحيات التشغيل المطلوبة لهذا التطبيق؟",
      "Grant all requested runtime permissions to this app?",
    ],
    btnInstallApk: ["تثبيت APK", "Install APK"],
    ready: ["جاهز", "Ready"],
    cleanTitle: ["CleanYourTV — تنظيف التلفزيون", "CleanYourTV — TV cleanup"],
    cytFetch: ["تحديث القوائم (اختياري)", "Update lists (optional)"],
    cytDisable: ["تعطيل الحزم", "Disable packages"],
    cytEnable: ["استعادة الحزم", "Restore packages"],
    close: ["إغلاق", "Close"],
    cleanNote: [
      "تعطيل فقط عبر pm disable-user — لا حذف. استخدم «استعادة الحزم» للتراجع.",
      "Disable only via pm disable-user — no uninstall. Use “Restore packages” to undo.",
    ],

    /* --- dynamic UI --- */
    openSource: ["مفتوح المصدر", "Open Source"],
    adbUnavailable: ["adb غير متاح — اضغط ⚙️", "adb unavailable — click ⚙️"],
    adbNotFoundToast: [
      "لم يتم العثور على adb — اضغط «تحديد adb» واختر adb.exe",
      "adb not found — click “Choose adb” and pick adb.exe",
    ],
    chooseAdbTitle: [
      "اختر ملف adb (platform-tools/adb.exe على ويندوز)",
      "Pick the adb file (platform-tools/adb.exe on Windows)",
    ],
    adbSetOk: ["تم ضبط adb بنجاح", "adb configured successfully"],
    discovering: ["اكتشاف تلقائي: {list}", "Auto-discovery: {list}"],
    lanFound: ["تم العثور على أجهزة في الشبكة: {list}", "Devices found on the network: {list}"],
    unauthorizedLog: [
      "أجهزة غير مصرح بها: {list} — وافق على Allow debugging على الجهاز",
      "Unauthorized devices: {list} — accept “Allow debugging” on the device",
    ],
    noAuthorizedOption: ["— لا يوجد جهاز مصرح —", "— No authorized device —"],
    noDeviceFooter: ["لا جهاز", "No device"],
    loadingApps: ["قراءة قائمة التطبيقات…", "Reading app list…"],
    loadedApps: ["تم تحميل {n} تطبيقاً من {s}", "Loaded {n} apps from {s}"],
    iconsMissingToast: [
      "لا توجد أيقونات محفوظة — اضغط «أيقونات التطبيقات» لقراءتها من الجهاز",
      "No saved icons — click “App icons” to read them from the device",
    ],
    iconsConfirmTitle: ["أيقونات التطبيقات", "App icons"],
    iconsConfirmText: [
      "سيتم تثبيت تطبيق مساعد صغير على الجهاز (ADB AppControl Bridge) لقراءة الأيقونات والأسماء الحقيقية للتطبيقات.\n\nستظهر شاشة سوداء على التلفزيون لثوانٍ أثناء القراءة. متابعة؟",
      "A small helper app (ADB AppControl Bridge) will be installed on the device to read real app icons and names.\n\nA black screen will appear on the TV for a few seconds. Continue?",
    ],
    iconsBusy: ["قراءة الأيقونات من الجهاز…", "Reading icons from the device…"],
    confirmTitle: ["تأكيد", "Confirm"],
    working: ["جارٍ العمل…", "Working…"],
    actionBusy: ["تنفيذ الإجراء…", "Applying action…"],
    actionDone: ["تم تنفيذ الإجراء على {done}/{total}", "Applied to {done}/{total}"],
    installBusy: ["تثبيت…", "Installing…"],
    installPickTitle: [
      "اختر APK أو حزمة .apks / .xapk",
      "Choose an APK or .apks / .xapk bundle",
    ],
    saveBusy: ["سحب APK…", "Pulling APK…"],
    apkSavedToast: ["تم حفظ APK في Downloads", "APK saved to Downloads"],
    screenshotBusy: ["لقطة شاشة…", "Taking screenshot…"],
    screenshotSaved: ["تم الحفظ: {path}", "Saved: {path}"],
    screenshotLog: ["لقطة محفوظة: {path}", "Screenshot saved: {path}"],
    logcatLoaded: ["تم تحميل logcat", "Logcat loaded"],
    rebootConfirm: ["إعادة تشغيل الجهاز الآن؟", "Reboot the device now?"],
    rebootBusy: ["إعادة تشغيل…", "Rebooting…"],
    rebootSent: ["تم إرسال أمر إعادة التشغيل", "Reboot command sent"],
    cleanNoDevice: ["اختر جهازاً أولاً", "Select a device first"],
    cleanDeviceLine: ["الجهاز: {s}", "Device: {s}"],
    cleanProfiles: ["{n} ملف تعريف مضمّن في البرنامج", "{n} profiles bundled in the app"],
    cleanNoProfiles: [
      "لا توجد قوائم — اضغط «تحميل / تحديث القوائم»",
      "No profiles — click “Download / Update lists”",
    ],
    cleanProfileCount: ["{n} حزمة في هذا الملف", "{n} packages in this profile"],
    cleanDownloading: ["تنزيل قوائم CleanYourTV…", "Downloading CleanYourTV lists…"],
    cleanPrepared: ["تم تجهيز {n} ملف تعريف", "{n} profiles ready"],
    cleanPickProfile: ["اختر ملف تعريف", "Select a profile"],
    cleanConfirmEnable: [
      "إعادة تفعيل الحزم الموجودة في هذا الملف؟",
      "Re-enable the packages in this profile?",
    ],
    cleanConfirmDisable: [
      "تعطيل كل الحزم في هذا الملف؟ لن يتم حذف أي تطبيق، ويمكن الاستعادة لاحقاً.",
      "Disable all packages in this profile? Nothing is uninstalled and you can restore them later.",
    ],
    cleanRestoring: ["استعادة الحزم…", "Restoring packages…"],
    cleanDisabling: ["تعطيل الحزم…", "Disabling packages…"],
    wirelessEnabling: ["تفعيل ADB اللاسلكي…", "Enabling wireless ADB…"],
    wirelessDisabling: ["إيقاف ADB اللاسلكي…", "Disabling wireless ADB…"],
    wirelessEnableLabel: ["تفعيل ADB لاسلكي", "Enable wireless ADB"],
    wirelessDisableLabel: ["إيقاف ADB اللاسلكي", "Disable wireless ADB"],
    wirelessEnabled: ["تم التفعيل: {ep}", "Enabled: {ep}"],
    wirelessLog: ["ADB لاسلكي: {ep}", "Wireless ADB: {ep}"],
    errWireless: ["ADB اللاسلكي", "Wireless ADB"],
    enterIp: ["أدخل عنوان IP", "Enter an IP address"],
    connecting: ["اتصال…", "Connecting…"],
    confirmDisable: [
      "تعطيل التطبيقات المحددة؟ يمكن تفعيلها لاحقاً.",
      "Disable the selected apps? You can re-enable them later.",
    ],
    confirmClearData: [
      "مسح بيانات التطبيقات المحددة؟ لا يمكن التراجع.",
      "Clear data of the selected apps? This cannot be undone.",
    ],
    confirmUninstall: [
      "حذف التطبيقات المحددة لهذا المستخدم 0؟ يمكن استعادتها لاحقاً من تبويب «محذوفة».",
      "Uninstall the selected apps for user 0? You can restore them later from the “Removed” tab.",
    ],
    confirmRestore: [
      "استعادة التطبيقات المحددة (pm install-existing)؟",
      "Restore the selected apps (pm install-existing)?",
    ],
    confirmDeep: [
      "⚠️ حذف نهائي (نسخة المصنع):\n\nسيتم إزالة تحديثات النظام للتطبيقات المحددة، إيقافها، ثم حذفها للمستخدم 0.\n\n• نسخة /system الأصلية قد تبقى في النظام، وإزالتها الكاملة تتطلب روت.\n• قد تؤثر على عمل الجهاز إذا حذفت مكوّنات أساسية — تأكد من وظيفة الحزمة قبل المتابعة.\n\nهل أنت متأكد؟",
      "⚠️ Deep uninstall (factory copy):\n\nSystem updates will be removed for the selected apps, they will be force-stopped, then uninstalled for user 0.\n\n• The original /system copy may remain; full removal requires root.\n• Removing essential components can break the device — make sure you know what the package does.\n\nAre you sure?",
    ],
    deepRisky: [
      "⚠️ تنبيه UDADB: منها {n} تطبيقاً مصنّفاً «خبير/غير آمن» — قد يتعطّل الجهاز.",
      "⚠️ UDADB warning: {n} selected app(s) are “Expert/Unsafe” — the device may break.",
    ],
    removedEmpty: [
      "لا توجد تطبيقات محذوفة قابلة للاستعادة",
      "No removed apps available to restore",
    ],
    noResults: ["لا توجد نتائج", "No results"],
    noDeviceRow: ["لم يتم اختيار جهاز", "No device selected"],
    initHint: [
      "اختر جهازاً ثم اضغط «تحديث القائمة»",
      "Pick a device, then click “Refresh list”",
    ],
    typeSystem: ["نظام", "System"],
    typeUser: ["مستخدم", "User"],
    typeRemoved: ["محذوف", "Removed"],
    stateDisabled: ["معطّل", "Disabled"],
    stateEnabled: ["مفعّل", "Enabled"],
    stateRemoved: ["غير مثبّت", "Not installed"],
    infoManufacturer: ["الماركة", "Manufacturer"],
    infoModel: ["الطراز", "Model"],
    infoAndroid: ["أندرويد", "Android"],
    infoApi: ["API", "API"],
    infoArch: ["المعمارية", "Architecture"],
    infoBattery: ["البطارية", "Battery"],
    infoSerial: ["الرقم التسلسلي", "Serial"],
    errGeneric: ["خطأ", "Error"],
    errDevices: ["الأجهزة", "Devices"],
    errDeviceInfo: ["معلومات الجهاز", "Device info"],
    errApps: ["التطبيقات", "Apps"],
    errIcons: ["الأيقونات", "Icons"],
    errBridge: ["حالة الأيقونات", "Icon status"],
    errScreenshot: ["لقطة الشاشة", "Screenshot"],
    errLogcat: ["logcat", "logcat"],
    errReboot: ["إعادة التشغيل", "Reboot"],
    errCommand: ["الأمر", "Command"],
    errConnection: ["الاتصال", "Connection"],
    errInstall: ["التثبيت", "Install"],
    errClean: ["CleanYourTV", "CleanYourTV"],
    errMatch: ["مطابقة الجهاز", "Device match"],
    errPackages: ["قراءة القائمة", "Read list"],
    errDownload: ["التنزيل", "Download"],
    errAdb: ["adb", "adb"],
    errLan: ["فحص الشبكة", "Network scan"],
    errRemoved: ["قائمة المحذوفة", "Removed list"],
    tauriUnavailable: [
      "Tauri API غير متاح — شغّل التطبيق عبر cargo run",
      "Tauri API unavailable — run the app via cargo run",
    ],
    uadRecommended: ["UDADB: موصى به — آمن للحذف", "UDADB: Recommended — safe to remove"],
    uadAdvanced: ["UDADB: متقدم — للحذر", "UDADB: Advanced — be careful"],
    uadExpert: ["UDADB: خبير — قد يؤثر على النظام", "UDADB: Expert — may affect the system"],
    uadUnsafe: ["UDADB: غير آمن — لا تحذفه", "UDADB: Unsafe — do not remove"],
  };

  /* Backend messages (Arabic → English). Exact matches first, then patterns. */
  const BACKEND_EXACT = {
    "الملف غير موجود": "File not found",
    "هذا الملف ليس adb صالحاً": "This file is not a valid adb",
    "عنوان غير صالح": "Invalid address",
    "اسم حزمة غير صالح": "Invalid package name",
    "إجراء غير معروف": "Unknown action",
    "تم تنظيف كاش جميع التطبيقات (مسح كاش تطبيق واحد متاح من Android 11+)":
      "Cleared cache for all apps (per-app cache clearing requires Android 11+)",
  };

  const BACKEND_PATTERNS = [
    [/^تم الحذف النهائي: (.+)$/s, "Permanently removed: $1"],
    [
      /^أُزيل (.+) للمستخدم 0 — نسخة المصنع تبقى في النظام \(إزالتها الكاملة تتطلب روت\)$/s,
      "$1 removed for user 0 — the factory copy remains in /system (full removal requires root)",
    ],
    [
      /^أُزيلت تحديثات النظام، لكن تعذّر الحذف الكامل — نسخة \/system تتطلب روت\.\n?/s,
      "System updates removed, but full removal failed — the /system copy requires root.\n",
    ],
    [/^تم تثبيت (\d+)\/(\d+) حزمة/, "Installed $1/$2 packages"],
    [/^تعذّر تشغيل adb: /, "Could not run adb: "],
    [/^تعذّر تشغيل الملف: /, "Could not run the file: "],
    [/^تعذّر كتابة bridge.apk: /, "Could not write bridge.apk: "],
    [/^خطأ داخلي: /, "Internal error: "],
    [/^تم تعطيل /, "Disabled "],
    [/^تم تفعيل /, "Enabled "],
    [/^تم حذف /, "Uninstalled "],
    [/^تم مسح كاش /, "Cleared cache of "],
    [/^تم مسح بيانات /, "Cleared data of "],
    [/^تمت استعادة /, "Restored "],
    [/^جارٍ /, "Working: "],
    [/^فشل: /, "Failed: "],
  ];

  let lang;
  try {
    lang = localStorage.getItem(STORAGE_KEY) === "en" ? "en" : "ar";
  } catch (_) {
    lang = "ar";
  }

  function fill(text, vars) {
    if (!vars) return text;
    return text.replace(/\{(\w+)\}/g, (m, key) => (vars[key] !== undefined ? String(vars[key]) : m));
  }

  function t(key, vars) {
    const row = S[key];
    const text = row ? row[lang === "en" ? 1 : 0] : key;
    return fill(text, vars);
  }

  /** Translate a message that came from the Rust backend. */
  function tr(message) {
    if (lang !== "en" || message === null || message === undefined) return message;
    const text = String(message);
    if (BACKEND_EXACT[text]) return BACKEND_EXACT[text];
    for (const [pattern, replacement] of BACKEND_PATTERNS) {
      if (pattern.test(text)) return text.replace(pattern, replacement);
    }
    return text;
  }

  function apply(root) {
    const doc = root || document;
    doc.documentElement.lang = lang;
    doc.documentElement.dir = lang === "ar" ? "rtl" : "ltr";
    doc.querySelectorAll("[data-i18n]").forEach((el) => {
      el.textContent = t(el.getAttribute("data-i18n"));
    });
    doc.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
      el.placeholder = t(el.getAttribute("data-i18n-placeholder"));
    });
    doc.querySelectorAll("[data-i18n-title]").forEach((el) => {
      el.title = t(el.getAttribute("data-i18n-title"));
    });
    const btn = doc.getElementById("langBtn");
    if (btn) {
      btn.textContent = lang === "ar" ? "English" : "العربية";
      btn.title = "اللغة / Language";
    }
    const status = doc.getElementById("statusText");
    if (status && /^(جاهز|Ready)$/.test(status.textContent.trim())) {
      status.textContent = t("ready");
    }
  }

  function setLang(next) {
    lang = next === "en" ? "en" : "ar";
    try {
      localStorage.setItem(STORAGE_KEY, lang);
    } catch (_) {}
    apply(document);
    window.dispatchEvent(new CustomEvent("langchange", { detail: lang }));
  }

  function toggle() {
    setLang(lang === "ar" ? "en" : "ar");
  }

  window.I18n = {
    get lang() {
      return lang;
    },
    get isEnglish() {
      return lang === "en";
    },
    t,
    tr,
    apply,
    setLang,
    toggle,
  };

  apply(document);
})();
