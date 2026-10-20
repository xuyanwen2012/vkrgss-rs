package io.github.xuyanwen2012.vkrgss;

import android.app.NativeActivity;
import android.content.Intent;
import android.graphics.Insets;
import android.net.Uri;
import android.os.Bundle;
import android.view.View;
import android.view.WindowInsets;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

/**
 * The activity of the library, whose screen the native code draws. It adds to NativeActivity
 * what the native code cannot do alone: the system's file picker, whose result arrives in
 * onActivityResult, the copy of what the picker chose, the start of a game in
 * {@link Game}, an activity in a process of its own, and the window's insets.
 */
public class VkrgssActivity extends NativeActivity {
    static {
        // So that the native method below is found; NativeActivity loads the same library.
        System.loadLibrary("vkrgss_android");
    }

    /** An extra of the intent: a file to import, as a URI, in place of the picker's choice. */
    static final String EXTRA_IMPORT = "import";
    /** An extra of the intent, with EXTRA_IMPORT: whether the file is a zip of the RTP. */
    static final String EXTRA_RTP = "rtp";
    /** An extra of the intent, with EXTRA_IMPORT: whether the file holds built-in scripts. */
    static final String EXTRA_BUILTIN = "builtin";
    /** The extra of the intent of Game: the directory of the game. */
    static final String EXTRA_GAME = "game";
    /**
     * An extra of the intent of Game: whether the game runs under the validation layer of
     * Vulkan, which an APK of cargo xtask android-run --validation carries.
     */
    static final String EXTRA_VALIDATION = "validation";

    /** What the picker is opened for, as the native code numbers it (Pick of java.rs). */
    private static final int PICK_GAME = 1;
    private static final int PICK_RTP = 2;
    private static final int PICK_BUILTIN = 3;

    /** Hands the native code a file to import, which holds what the PICK_* what says. */
    private static native void nativeImport(String uri, int what);

    /** What the system's bars and the display's cutout cover: left, top, right, bottom. */
    private volatile int[] insets = new int[4];

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        getWindow().getDecorView().setOnApplyWindowInsetsListener(
                new View.OnApplyWindowInsetsListener() {
                    @Override
                    public WindowInsets onApplyWindowInsets(View view, WindowInsets given) {
                        Insets covered = given.getInsets(WindowInsets.Type.systemBars()
                                | WindowInsets.Type.displayCutout());
                        insets = new int[] {
                            covered.left, covered.top, covered.right, covered.bottom
                        };
                        return view.onApplyWindowInsets(given);
                    }
                });
        importFrom(getIntent());
    }

    /**
     * The pixels at the window's edges that the system's bars and the display's cutout cover:
     * left, top, right, bottom; called by the native code from its thread.
     */
    public int[] insets() {
        return insets;
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        importFrom(intent);
    }

    private void importFrom(Intent intent) {
        Uri uri = intent.getParcelableExtra(EXTRA_IMPORT, Uri.class);
        if (uri != null) {
            int what = intent.getBooleanExtra(EXTRA_BUILTIN, false) ? PICK_BUILTIN
                    : intent.getBooleanExtra(EXTRA_RTP, false) ? PICK_RTP : PICK_GAME;
            nativeImport(uri.toString(), what);
        }
    }

    /**
     * Opens the system's picker for a zip or, for PICK_BUILTIN, any file, since a Ruby file
     * has no MIME type that every provider gives; called by the native code from its thread.
     */
    public void openPicker(final int what) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                if (what == PICK_BUILTIN) {
                    intent.setType("*/*");
                } else {
                    intent.setType("application/zip");
                    intent.putExtra(Intent.EXTRA_MIME_TYPES,
                            new String[] {"application/zip", "application/x-zip-compressed"});
                }
                startActivityForResult(intent, what);
            }
        });
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if ((request == PICK_GAME || request == PICK_RTP || request == PICK_BUILTIN)
                && result == RESULT_OK && data != null && data.getData() != null) {
            nativeImport(data.getData().toString(), request);
        }
    }

    /**
     * Copies what uri names into the file import of the cache directory and returns the
     * file's path; called by the native code from a thread of its own.
     */
    public String copyToCache(String uri) throws IOException {
        File file = new File(getCacheDir(), "import");
        try (InputStream in = getContentResolver().openInputStream(Uri.parse(uri));
                OutputStream out = new FileOutputStream(file)) {
            if (in == null) {
                throw new IOException("the system gives nothing to read for " + uri);
            }
            byte[] buffer = new byte[1 << 16];
            int read;
            while ((read = in.read(buffer)) > 0) {
                out.write(buffer, 0, read);
            }
        }
        return file.getPath();
    }

    /** Starts the game of the directory dir in the activity of games. */
    public void startGame(String dir) {
        startActivity(new Intent(this, Game.class).putExtra(EXTRA_GAME, dir));
    }

    /**
     * The directory of the game this activity runs: null for the library, the empty string
     * for the game of the files directory, which tests fill with adb.
     */
    public String gameDir() {
        return null;
    }

    /** Whether the game runs under the validation layer: never for the library. */
    public boolean validation() {
        return false;
    }

    /** The activity of a game, in a process of its own, which ends with the game. */
    public static final class Game extends VkrgssActivity {
        @Override
        public String gameDir() {
            String dir = getIntent().getStringExtra(EXTRA_GAME);
            return dir == null ? "" : dir;
        }

        @Override
        public boolean validation() {
            return getIntent().getBooleanExtra(EXTRA_VALIDATION, false);
        }
    }
}
