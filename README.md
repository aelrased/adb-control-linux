# ADB Control Linux

بديل حر ومفتوح المصدر لبرنامج **ADB AppControl** (Windows)، مبني بواجهة سطح مكتب
حديثة على **Tauri 2 + WebKitGTK** مع نواة **Rust**، ويعتمد على أداة `adb` الرسمية من
Google (Apache‑2.0).

> 🔗 **نسخة أندرويد:** https://github.com/aelrased/adb-control-android

> الحالة: **MVP** قابل للتشغيل — إدارة التطبيقات، التثبيت والاستخراج، لقطات الشاشة،
> Logcat، وطرفية ADB. التصميم مستوحى من البرنامج الأصلي دون نسخ أي أصول أو أيقونات.

---

## ✨ الميزات (MVP)

- **اكتشاف الأجهزة**: USB وWi‑Fi (`adb connect IP:5555`) مع عرض الطراز وحالة الاتصال.
- **معلومات الجهاز**: الماركة، الطراز، إصدار أندرويد، API، المعمارية، البطارية، الرقم التسلسلي.
- **قائمة التطبيقات** مع: الاسم الحقيقي، أيقونة التطبيق الأصلية، اسم الحزمة، الإصدار، النوع (نظام/مستخدم)، الحالة (مفعّل/معطّل).
- **أيقونات وأسماء حقيقية** عبر تطبيق مساعد صغير (Bridge) يُثبَّت على الجهاز عند الطلب،
  تماماً كأسلوب ADB AppControl الأصلي. يُقرأ 164 أيقونة في ~15 ثانية وتُخزَّن محلياً.
- **تصفية وبحث فوري** + عدّادات لكل فئة + تحديد متعدد وتحديد الكل.
- **إجراءات جماعية**:
  - تعطيل / تفعيل (`pm disable-user` / `pm enable`)
  - حذف للمستخدم (`pm uninstall --user 0`)
  - مسح البيانات (`pm clear`)
  - حفظ/استخراج APK (`pm path` + `adb pull`) إلى `~/Downloads/adb-appcontrol-apks`
    - تطبيق عادي → ملف `اسم-الحزمة.apk` واحد
    - تطبيق App Bundle (مقسّم) → ملف `اسم-الحزمة.apks` واحد يضم base + splits
- **تثبيت APK** عبر منتقي ملفات أصلي: `.apk` أو `.apks`/`.xapk` (يُستخدم
  `adb install -r` أو `adb install-multiple -r` تلقائياً).
- **لقطة شاشة** إلى `~/Pictures/ADB AppControl`.
- **Logcat** (آخر 300 سطر) داخل لوحة الطرفية.
- **طرفية ADB مدمجة** لتنفيذ أوامر شل على الجهاز.
- **إعادة تشغيل** الجهاز من الشريط الجانبي.
- واجهة **داكنة RTL** بنفس روح البرنامج الأصلي (شريط جانبي، تبويبات، جدول، شريط إجراءات).

---

## 🪟 البناء على ويندوز (Windows 10/11)

المتطلبات:

1. **Rust (MSVC)** — من [rustup.rs](https://rustup.rs): اختر `x86_64-pc-windows-msvc`.
2. **Visual Studio Build Tools 2022** مع حزمة **Desktop development with C++**.
3. **WebView2 Runtime** (مثبّت مسبقاً في Windows 11، ومتوفر تلقائياً في التحديثات).
4. **لا تحتاج تثبيت Android Platform Tools**: `adb.exe` الرسمي (Google، Apache‑2.0)
   **مضمّن** داخل نسخة ويندوز في `src-tauri/platform-tools/` ويُحزم مع المثبّت.

```powershell
cd adb-appcontrol-linux
powershell -ExecutionPolicy Bypass -File .\windows.ps1
# أو للتشغيل مباشرة بعد البناء:
powershell -ExecutionPolicy Bypass -File .\windows.ps1 -Run
```

النواتج:

```
src-tauri\target\release\adb-appcontrol.exe
src-tauri\target\release\bundle\nsis\ADB Control Windows_0.1.0_x64-setup.exe
src-tauri\target\release\bundle\msi\ADB Control Windows_0.1.0_x64_en-US.msi
```

### كيف يجد البرنامج adb على ويندوز؟

بالترتيب:

1. متغير البيئة `ADB_PATH`.
2. المسار المحفوظ من زر **«⚙️ تحديد adb»** داخل البرنامج
   (`%APPDATA%\adb-appcontrol\config.json`).
3. **المضمّن**: `platform-tools\adb.exe` بجوار الملف التنفيذي أو في مجلد الموارد.
4. `%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe`.
5. `%USERPROFILE%\AppData\Local\Android\Sdk\platform-tools\adb.exe`.
6. `C:\platform-tools\adb.exe`.
7. `adb` من `PATH`.

كل استدعاءات adb تُشغَّل بـ `CREATE_NO_WINDOW` حتى لا تظهر نوافذ سوداء تومض.

### مسارات الحفظ على ويندوز

| النوع | المسار |
|---|---|
| APK / .apks | `%USERPROFILE%\Downloads\adb-appcontrol-apks` |
| لقطات الشاشة | `%USERPROFILE%\Pictures\ADB AppControl` |
| الكاش (أيقونات) | `%LOCALAPPDATA%\adb-appcontrol` |
| الإعدادات | `%APPDATA%\adb-appcontrol\config.json` |

> التطبيق المساعد `bridge.apk` مضمّن في الملف التنفيذي ويعمل كما هو على أندرويد،
> لذلك لا تحتاج إلى أي أدوات إضافية على ويندوز.

### لماذا لم ننسخ مجلد `adb` من ADB AppControl؟

مجلد `ADBAppControl-1.8.6\adb` يحتوي:

| الملف | المصدر | قرارنا |
|---|---|---|
| `adb.exe` + `AdbWinApi.dll` + `AdbWinUsbApi.dll` | Google (Apache‑2.0) | ✅ نستخدم نسخة **رسمية أحدث** من Platform Tools |
| `fastboot.exe` | Google (Apache‑2.0) | ⏳ لم يُضمَّن (fastboot في خطة M4) |
| `acupdate.exe` | CyberCat (خاص) | ❌ ليس لنا ولا يجوز إعادة توزيعه |
| `com.cybercat.acbridge.apk` | CyberCat (خاص) | ❌ بديلنا مفتوح المصدر: `bridge/` |

كما أن النسخة التي بحوزتك جاءت بجانبها حزمة `SETUP_lNSTALLER_(KEY.3132).zip`
مشبوهة، لذا لا يُنصح بالاعتماد على أي ملف تنفيذي منها.

الملفات المضمّنة موثّقة بالـ SHA‑256 في
`src-tauri/platform-tools/NOTICE.txt`.

### ما الذي يُحزم فعلاً في نسخة ويندوز؟

| الملف داخل المثبّت | الملكية | الترخيص |
|---|---|---|
| `adb-appcontrol.exe` | كودنا | GPL‑3.0 |
| `platform-tools\adb.exe` | Google | Apache‑2.0 |
| `platform-tools\AdbWinApi.dll` | Google | Apache‑2.0 |
| `platform-tools\AdbWinUsbApi.dll` | Google | Apache‑2.0 |
| `platform-tools\NOTICE.txt` | نحن | — |
| `bridge.apk` (مضمّن داخل exe) | كودنا (من `bridge/`) | GPL‑3.0 |

**غير مضمّن إطلاقاً:** `acupdate.exe`، `com.cybercat.acbridge.apk`، أي ملف من
توزيعة ADB AppControl الأصلية. إعدادات الموارد موجودة في
`src-tauri/tauri.windows.conf.json` فقط، فلا تدخل هذه الملفات في حزمة لينكس.

> لا يُدعم البناء المتقاطع من لينكس إلى ويندوز؛ ابنِ النسخة على ويندوز نفسه
> (MSVC + WebView2) كما هو موضح أعلاه.

**مشكلة عدم ظهور الجهاز عبر USB؟** فعّل `USB debugging`، وثبّت *Google USB Driver*
من SDK Manager إن لزم، أو استخدم الاتصال اللاسلكي: أدخل `IP:5555` في الشريط العلوي
(يشترط أن يكون التلفزيون والكمبيوتر على نفس الشبكة وتفعيل Network debugging).

## 📋 المتطلبات

| المكوّن | الإصدار |
|---|---|
| Rust | ≥ 1.77 (مُختبر على 1.96) |
| Node.js | غير مطلوب (الواجهة ثابتة، لا bundler) |
| WebKitGTK | 4.1 + `libgtk-3` + `libsoup-3` |
| adb | Android Platform Tools |

على Debian/Ubuntu/PikaOS:

```bash
sudo apt install adb libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
                 build-essential pkg-config libssl-dev
```

### صلاحيات USB (مرة واحدة)

```bash
sudo usermod -aG plugdev $USER
echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="18d1", MODE="0666", GROUP="plugdev"' \
  | sudo tee /etc/udev/rules.d/51-android.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

ثم على الجهاز: `Settings → About → Build number ×7` ← `Developer options ← USB debugging`.

---

## 🚀 التشغيل

```bash
cd adb-appcontrol-linux/src-tauri
cargo run
```

أو باستخدام Tauri CLI:

```bash
cd adb-appcontrol-linux
cargo tauri dev
```

### تشخيص

```bash
ADBAC_DEBUG=1 cargo run   # يطبع كل أوامر adb وعدد الحزم على stderr
ADB_PATH=/path/to/adb cargo run  # تحديد مسار adb مختلف
```

### البناء للإصدار

```bash
cd adb-appcontrol-linux
cargo tauri build
# النواتج: src-tauri/target/release/adb-appcontrol
# والحزم:  target/release/bundle/deb/*.deb و *.AppImage
```

---

## 🧱 البنية

```
adb-appcontrol-linux/
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json          # نافذة، أمان، حزم
│   ├── capabilities/default.json# صلاحيات core + dialog
│   ├── assets/bridge.apk        # التطبيق المساعد (مضمّن في الملف التنفيذي)
│   ├── icons/
│   └── src/main.rs              # كل أوامر Tauri + منطق adb
├── windows.ps1                  # بناء وتشغيل نسخة ويندوز (NSIS + MSI)
├── install.sh                   # تثبيت نسخة لينكس في قائمة التطبيقات
├── bridge/                      # مصدر التطبيق المساعد (Java، بدون Gradle)
│   ├── AndroidManifest.xml
│   ├── src/com/adbappcontrol/bridge/MainActivity.java
│   └── build.sh
└── ui/
    ├── index.html               # الهيكل (RTL)
    ├── style.css                # الثيم الداكن
    └── app.js                   # الحالة والتفاعل
```

### كيف تعمل الأيقونات؟

1. عند الضغط على **«أيقونات التطبيقات»** يُثبَّت `bridge.apk` (13 KB) على الجهاز.
2. يقرأ التطبيق المساعد `PackageManager` ويكتب PNG بحجم 192×192 + `apps.json`
   إلى `/sdcard/Android/data/com.adbappcontrol.bridge/files/`.
3. يقوم `adb pull` بسحب ~2 MB إلى `~/.cache/adb-appcontrol/icons/<serial>/`.
4. الواجهة تطلب كل أيقونة عند ظهور صفّها فقط (lazy + 4 تحميلات متوازية).

> يظهر على شاشة التلفزيون إطار أسود لثوانٍ أثناء القراءة. التطبيق المساعد يبقى مثبتاً
> لتحديث الأيقونات لاحقاً، ويمكن حذفه من إعدادات التلفزيون.

### بناء التطبيق المساعد

```bash
cd bridge
./build.sh     # يحتاج build-tools;34.0.0 و platforms;android-34
# الناتج: src-tauri/assets/bridge.apk (مضمّن تلقائياً عند cargo build)
```

### اختبار سريع بدون واجهة

```bash
cd src-tauri
cargo run -- --selftest 10.0.0.88:5555
# → تم استيراد 164 أيقونة وبيانات 164 تطبيقاً
```

**المبدأ الأمني**: لا يُستخدم shell ولا تجميع أوامر نصية. كل استدعاء يمر عبر
`Command::new("adb").args([...])` مع التحقق من اسم الحزمة (`[A-Za-z0-9._]`)، لذلك
لا توجد ثغرة حقن أوامر عبر أسماء الحزم.

الأوامر المتاحة في الـ backend:

```
check_adb, list_devices, connect_device, disconnect_device, device_info,
list_apps, app_action, extract_apk, install_apk, screenshot, logcat,
reboot, run_command
```

---

## ❓ لماذا يخرج ملف `.apks` بدل APK واحد؟

معظم تطبيقات أندرويد الحديثة تُنشَر كـ **Android App Bundle**، فيثبّت الجهاز
مجموعة أجزاء (Split APKs):

```
base.apk                    # الكود والموارد الأساسية
split_config.arm64_v8a.apk  # مكتبات المعمارية
split_config.xxhdpi.apk     # موارد الشاشة
split_config.ar.apk         # اللغة
```

هذه الأجزاء **لا يمكن دمجها بمجرد ضغطها** في APK واحد صالح، لأن كل جزء يحمل
`AndroidManifest` وتوقيعاً خاصاً. لذلك يحفظها البرنامج في أرشيف `.apks` واحد
(نفس صيغة SAI / APKMirror Bundles) ويستخدمه عند التثبيت عبر `adb install-multiple`.

يمكنك أيضاً تمرير الأجزاء الأربعة يدوياً لمنتقي الملفات، وسيثبّتها البرنامج معاً.

## 🗺️ خريطة الطريق

| المرحلة | الميزات |
|---|---|
| **M1 (الحالية)** | الجهاز، التطبيقات، تعطيل/تفعيل/حذف/مسح، تثبيت/استخراج APK، لقطة، Logcat، طرفية |
| M2 | أيقونات التطبيقات الحقيقية (سحب APK + استخراج الأيقونة)، أحجام التطبيقات والبيانات والكاش |
| M3 | تكامل **scrcpy** (بث الشاشة والتحكم) + تسجيل |
| M4 | **fastboot** (إعادة تشغيل، تفليش) + وضع الطوارئ |
| M5 | نسخ احتياطي/استعادة كامل، بروفايلات Debloat جاهزة، إدارة الصلاحيات |
| M6 | تحديثات داخلية عبر GitHub Releases، حزم Flatpak/AppImage، دعم تعدد الأجهزة |
| M7 | ترجمة كاملة (عربي/إنجليزي/تركي) + ثيمات |

---

## ⚖️ الترخيص والاعتمادات

- كود هذا المشروع: **GPL‑3.0‑or‑later**.
- [Tauri](https://tauri.app) — MIT / Apache‑2.0.
- [Android Platform Tools](https://developer.android.com/tools/releases/platform-tools) — Apache‑2.0.
- [scrcpy](https://github.com/Genymobile/scrcpy) (خطة M3) — Apache‑2.0.
- التصميم مستوحى من **ADB AppControl** (CyberCat) لكنه إعادة تنفيذ مستقلة:
  لا يُستخدم أي كود أو أصول أو أيقونات من البرنامج الأصلي.
- أيقونة التطبيق رسم أصلي (PIL) لرأس روبوت أندرويد بأسلوب Android 16:
  *The Android robot is reproduced or modified from work created and shared by
  Google and used according to terms described in the Creative Commons 3.0
  Attribution License.* Android is a trademark of Google LLC.

> هذا المشروع غير مرتبط رسمياً بـ ADB AppControl أو CyberCat.
