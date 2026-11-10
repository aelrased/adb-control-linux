package com.adbappcontrol.bridge;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/**
 * Headless trigger: `adb shell content call --uri content://com.adbappcontrol.bridge.export --method export`
 * Runs the export in a background thread and returns immediately; the host polls the ready file.
 */
public class ExportProvider extends ContentProvider {

    private static final ExecutorService EXECUTOR = Executors.newSingleThreadExecutor();

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public Bundle call(String method, String arg, Bundle extras) {
        if (!"export".equals(method)) {
            return null;
        }
        final Context context = getContext();
        if (context != null) {
            EXECUTOR.execute(() -> {
                try {
                    Exporter.export(context);
                } catch (Throwable ignored) {
                }
            });
        }
        Bundle result = new Bundle();
        result.putBoolean("started", true);
        return result;
    }

    @Override
    public Cursor query(Uri uri, String[] projection, String selection, String[] selectionArgs, String sortOrder) {
        return null;
    }

    @Override
    public String getType(Uri uri) {
        return null;
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        return null;
    }

    @Override
    public int delete(Uri uri, String selection, String[] selectionArgs) {
        return 0;
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] selectionArgs) {
        return 0;
    }
}
