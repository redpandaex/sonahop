package io.github.redpandaex.sonahop.lan;

import android.app.Activity;
import android.content.Intent;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;

/** PCM crosses JNI directly. This class never queues audio in the WebView. */
public final class AudioBridge {
    private AudioBridge() {}
    private static final Handler MAIN = new Handler(Looper.getMainLooper());
    private static long nextId;
    private static AudioService.Session current;
    private static boolean registered;

    public interface StartResult { void complete(String error); }

    private static native void nativeRegister(Class<?> owner);
    private static native void nativeStarted(long sessionId, int captureChannels);
    private static native void nativeCaptured(byte[] pcm, long sessionId);
    private static native void nativeAudioFailure(String message, long sessionId);
    private static native void nativeStopped(long sessionId);

    private static void register() {
        if (registered) return;
        try {
            nativeRegister(AudioBridge.class);
        } catch (UnsatisfiedLinkError unloaded) {
            // Commands normally run after Tauri has loaded this very library.
            System.loadLibrary("sonahop_ui_lib");
            nativeRegister(AudioBridge.class);
        }
        registered = true;
    }

    public static synchronized boolean hasActiveSession() { return current != null; }

    public static void start(Activity activity, boolean microphone, StartResult callback) {
        start(activity, microphone, false, null, callback);
    }

    public static void startSystemCapture(Activity activity, Intent projectionData, StartResult callback) {
        if (Build.VERSION.SDK_INT < 29 || projectionData == null) {
            MAIN.post(() -> callback.complete("A fresh Android 10+ playback capture grant is required"));
            return;
        }
        start(activity, false, true, projectionData, callback);
    }

    private static void start(Activity activity, boolean microphone, boolean systemCapture,
            Intent projectionData, StartResult callback) {
        final AudioService.Session session;
        synchronized (AudioBridge.class) {
            if (current != null) {
                MAIN.post(() -> callback.complete("Android audio is already active or stopping"));
                return;
            }
            try {
                register();
                nativeStarted(nextId + 1, systemCapture ? 2 : microphone ? 1 : 0);
            } catch (LinkageError | RuntimeException error) {
                MAIN.post(() -> callback.complete("Audio JNI registration failed: " + error.getMessage()));
                return;
            }
            session = new AudioService.Session(++nextId, activity.getApplicationContext(), microphone, systemCapture, projectionData, callback);
            current = session;
        }
        Intent intent = new Intent(activity, AudioService.class)
                .setAction(AudioService.ACTION_START).putExtra(AudioService.EXTRA_SESSION, session.id);
        try {
            if (Build.VERSION.SDK_INT >= 26) activity.startForegroundService(intent);
            else activity.startService(intent);
            MAIN.postDelayed(() -> {
                synchronized (AudioBridge.class) {
                    if (current != session || session.ready || !session.running) return;
                }
                fail(session, "Android audio initialization timed out");
            }, 8000);
        } catch (RuntimeException error) {
            fail(session, "Cannot start Android audio foreground service: " + error.getMessage());
        }
    }

    static synchronized AudioService.Session session(long id) {
        return current != null && current.id == id && current.running ? current : null;
    }

    static void ready(AudioService.Session session) {
        synchronized (AudioBridge.class) {
            if (current != session || !session.running) return;
            session.ready = true;
        }
        complete(session, null);
    }

    private static void complete(AudioService.Session session, String error) {
        MAIN.post(() -> {
            StartResult callback;
            boolean cancelled;
            synchronized (AudioBridge.class) {
                callback = session.callback;
                session.callback = null;
                cancelled = error == null && (current != session || !session.running);
            }
            if (callback != null) callback.complete(cancelled ? "Android audio start was cancelled" : error);
        });
    }


    static synchronized void captured(AudioService.Session session, byte[] bytes) {
        if (current == session && session.running && session.ready) nativeCaptured(bytes, session.id);
    }

    static void fail(AudioService.Session session, String message) {
        synchronized (AudioBridge.class) {
            if (current != session || !session.running) return;
            nativeAudioFailure(message, session.id);
        }
        stop(session, message);
    }

    public static void stop() { stop((StartResult) null); }

    public static void stop(StartResult callback) {
        AudioService.Session session;
        synchronized (AudioBridge.class) {
            session = current;
            if (session != null && callback != null) session.stopCallbacks.add(callback);
        }
        if (session != null) stop(session, null);
        else if (callback != null) MAIN.post(() -> callback.complete(null));
    }

    static void stop(AudioService.Session session, String reason) {
        synchronized (AudioBridge.class) {
            if (current != session || !session.running) return;
            session.running = false;
            // A stopped session cannot publish PCM or an error into its successor.
            nativeStopped(session.id);
        }
        complete(session, reason == null ? "Android audio start was cancelled" : reason);
        session.dispose();
    }

    static void released(AudioService.Session session) {
        java.util.ArrayList<StartResult> callbacks;
        synchronized (AudioBridge.class) {
            if (current == session) current = null;
            callbacks = new java.util.ArrayList<>(session.stopCallbacks);
            session.stopCallbacks.clear();
        }
        for (StartResult callback : callbacks) callback.complete(null);
    }

    /** Called only by the Rust playback worker; bounded to a 20 ms deadline. */
    public static int writePlayback(byte[] pcm) {
        long id;
        synchronized (AudioBridge.class) { id = current == null ? -1 : current.id; }
        return writePlayback(pcm, id);
    }

    public static int writePlayback(byte[] pcm, long expectedSession) {
        final AudioService.Session session;
        synchronized (AudioBridge.class) { session = current; }
        if (session == null || session.id != expectedSession || session.captureChannels != 0 || !session.running || !session.ready
                || pcm == null || pcm.length == 0 || pcm.length % 4 != 0 || pcm.length > 7680) return -1;
        int written = 0;
        long deadline = SystemClock.elapsedRealtimeNanos() + 20_000_000L;
        try {
            while (written < pcm.length && session.running) {
                int count;
                synchronized (session.ioLock) {
                    if (!session.running || session.track == null) return -1;
                    count = session.track.write(pcm, written, pcm.length - written,
                            android.media.AudioTrack.WRITE_NON_BLOCKING);
                }
                if (count < 0) {
                    fail(session, "AudioTrack write failed: " + count);
                    return count;
                }
                written += count;
                if (written == pcm.length) return written;
                if (SystemClock.elapsedRealtimeNanos() >= deadline) {
                    fail(session, "AudioTrack could not consume one packet within 20 ms");
                    return -1;
                }
                SystemClock.sleep(1);
            }
        } catch (RuntimeException error) {
            fail(session, "AudioTrack write failed: " + error.getMessage());
        }
        return -1;
    }
}
