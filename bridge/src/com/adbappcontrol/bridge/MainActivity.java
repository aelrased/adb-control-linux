package com.adbappcontrol.bridge;

import android.app.Activity;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.drawable.AdaptiveIconDrawable;
import android.graphics.drawable.Drawable;
import android.os.Bundle;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.io.FileOutputStream;
import java.io.FileWriter;
import java.util.List;

/**
 * ADB AppControl Bridge
 * Exports real launcher icons + labels + versions to
 * /sdcard/Android/data/com.adbappcontrol.bridge/files/
 * so the desktop app can read them over adb pull.
 */
public class MainActivity extends Activity {

    private static final int ICON_SIZE = 192;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        // Headless export: no UI, so the host app is never covered by a black screen.
        new Thread(this::exportAll).start();
    }

    private void setStatus(String text) {
        // Headless: progress is polled through the "ready" file instead of a UI.
    }

    private void exportAll() {
        File base = getExternalFilesDir(null);
        if (base == null) {
            setStatus("External storage unavailable");
            return;
        }
        File dir = new File(base, "icons");
        deleteChildren(dir);
        if (!dir.exists() && !dir.mkdirs()) {
            setStatus("Cannot create output directory");
            return;
        }
        // Remove stale marker first so the desktop app never reads an old run.
        new File(base, "ready").delete();

        PackageManager pm = getPackageManager();
        List<ApplicationInfo> apps = pm.getInstalledApplications(0);
        JSONArray out = new JSONArray();
        int done = 0;
        int icons = 0;

        for (ApplicationInfo ai : apps) {
            try {
                JSONObject o = new JSONObject();
                o.put("package", ai.packageName);
                o.put("label", String.valueOf(pm.getApplicationLabel(ai)));
                o.put("system", (ai.flags & ApplicationInfo.FLAG_SYSTEM) != 0);
                o.put("enabled", ai.enabled);
                try {
                    o.put("version", String.valueOf(pm.getPackageInfo(ai.packageName, 0).versionName));
                } catch (Throwable ignored) {
                    o.put("version", "");
                }

                long sizeBytes = 0L;
                try {
                    if (ai.sourceDir != null) {
                        sizeBytes += new File(ai.sourceDir).length();
                    }
                    if (ai.splitSourceDirs != null) {
                        for (String split : ai.splitSourceDirs) {
                            sizeBytes += new File(split).length();
                        }
                    }
                } catch (Throwable ignored) {
                }
                o.put("sizeBytes", sizeBytes);

                try {
                    Drawable d = pm.getApplicationIcon(ai);
                    Bitmap bmp = toBitmap(d);
                    if (bmp != null) {
                        File f = new File(dir, ai.packageName + ".png");
                        try (FileOutputStream fos = new FileOutputStream(f)) {
                            bmp.compress(Bitmap.CompressFormat.PNG, 100, fos);
                        }
                        icons++;
                    }
                } catch (Throwable ignored) {
                    // Keep the app even if its icon cannot be rendered.
                }

                out.put(o);
            } catch (Throwable ignored) {
            }
            done++;
            if (done % 25 == 0) {
                setStatus("Exported " + done + "/" + apps.size() + " apps…");
            }
        }

        try (FileWriter w = new FileWriter(new File(base, "apps.json"))) {
            w.write(out.toString());
        } catch (Throwable t) {
            setStatus("Failed to write apps.json: " + t);
            return;
        }

        try (FileWriter w = new FileWriter(new File(base, "ready"))) {
            w.write("{\"apps\":" + out.length() + ",\"icons\":" + icons
                    + ",\"time\":" + (System.currentTimeMillis() / 1000L) + "}");
        } catch (Throwable ignored) {
        }

        setStatus("Done: " + out.length() + " apps, " + icons + " icons");
        try {
            Thread.sleep(400);
        } catch (InterruptedException ignored) {
        }
        runOnUiThread(this::finish);
    }

    private static void deleteChildren(File dir) {
        File[] files = dir.listFiles();
        if (files == null) return;
        for (File f : files) {
            if (f.isDirectory()) deleteChildren(f);
            f.delete();
        }
    }

    private static Bitmap toBitmap(Drawable d) {
        if (d == null) return null;
        int w;
        int h;
        if (d instanceof AdaptiveIconDrawable) {
            w = ICON_SIZE;
            h = ICON_SIZE;
        } else {
            w = d.getIntrinsicWidth();
            h = d.getIntrinsicHeight();
            if (w <= 0 || h <= 0) {
                w = ICON_SIZE;
                h = ICON_SIZE;
            }
            float scale = Math.min(1f, (float) ICON_SIZE / Math.max(w, h));
            w = Math.max(1, (int) (w * scale));
            h = Math.max(1, (int) (h * scale));
        }
        Bitmap bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888);
        Canvas canvas = new Canvas(bmp);
        d.setBounds(0, 0, w, h);
        d.draw(canvas);
        return bmp;
    }
}
