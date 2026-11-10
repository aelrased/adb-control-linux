package com.adbappcontrol.bridge;

import android.app.Activity;
import android.os.Bundle;

/** Optional manual launcher; the provider is the headless entry point. */
public class MainActivity extends Activity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        new Thread(() -> {
            try {
                Exporter.export(this);
            } catch (Throwable ignored) {
            } finally {
                runOnUiThread(this::finish);
            }
        }).start();
    }
}
