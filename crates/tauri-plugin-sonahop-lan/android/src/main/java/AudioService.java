package io.github.redpandaex.sonahop.lan;

import android.Manifest;
import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.pm.ServiceInfo;
import android.media.AudioAttributes;
import android.media.AudioFocusRequest;
import android.media.AudioFormat;
import android.media.AudioManager;
import android.media.AudioPlaybackCaptureConfiguration;
import android.media.AudioRecord;
import android.media.AudioRecordingConfiguration;
import android.media.AudioTrack;
import android.media.MediaRecorder;
import android.media.projection.MediaProjection;
import android.media.projection.MediaProjectionManager;
import android.os.Build;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.os.PowerManager;
import android.os.Process;
import android.os.SystemClock;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.concurrent.atomic.AtomicBoolean;

/** Owns native audio and the foreground notification; never restarts recording. */
public final class AudioService extends Service {
    static final String ACTION_START = "io.github.redpandaex.sonahop.START_AUDIO";
    static final String ACTION_STOP = "io.github.redpandaex.sonahop.STOP_AUDIO";
    static final String EXTRA_SESSION = "session";
    private static final String CHANNEL = "sonahop_audio";
    private static final int NOTIFICATION = 2401;
    private static final Handler MAIN = new Handler(Looper.getMainLooper());
    private Session owned;

    @Override public IBinder onBind(Intent intent) { return null; }

    @Override public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent == null) {
            stopSelf(startId);
            return START_NOT_STICKY;
        }
        long id = intent.getLongExtra(EXTRA_SESSION, -1);
        Session session = AudioBridge.session(id);
        if (ACTION_STOP.equals(intent.getAction())) {
            if (session != null) AudioBridge.fail(session, "Audio stopped from the Android notification");
            else if (owned == null) stopSelf(startId);
            return START_NOT_STICKY;
        }
        if (!ACTION_START.equals(intent.getAction()) || session == null) {
            if (owned == null) stopSelf(startId);
            return START_NOT_STICKY;
        }
        if (owned == session) return START_NOT_STICKY;
        owned = session;
        session.service = this;
        session.startId = startId;
        try {
            if (session.captureChannels != 0 && checkSelfPermission(Manifest.permission.RECORD_AUDIO)
                    != PackageManager.PERMISSION_GRANTED) {
                throw new SecurityException("Audio capture RECORD_AUDIO permission is not granted");
            }
            Notification notification = notification(session);
            if (Build.VERSION.SDK_INT >= 29) {
                int type = session.systemCapture ? ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION
                        : session.microphone
                            ? (Build.VERSION.SDK_INT >= 30 ? ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE : 0)
                            : ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK;
                // MediaProjection is obtained only after this foreground service is established.
                startForeground(NOTIFICATION, notification, type);
            } else startForeground(NOTIFICATION, notification);
            session.worker = new Thread(session::initialize, "sonahop-android-audio");
            session.worker.start();
        } catch (RuntimeException error) {
            AudioBridge.fail(session, "Android foreground audio start failed: " + error.getMessage());
        }
        return START_NOT_STICKY;
    }

    private Notification notification(Session session) {
        NotificationManager manager = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        if (Build.VERSION.SDK_INT >= 26) {
            NotificationChannel channel = new NotificationChannel(CHANNEL, "SonaHop audio",
                    NotificationManager.IMPORTANCE_LOW);
            channel.setDescription("Listening, microphone and playback capture controls");
            channel.setSound(null, null);
            manager.createNotificationChannel(channel);
        }
        Intent stop = new Intent(this, AudioService.class).setAction(ACTION_STOP)
                .putExtra(EXTRA_SESSION, session.id);
        PendingIntent stopAction = PendingIntent.getService(this, (int) session.id, stop,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        Notification.Builder builder = Build.VERSION.SDK_INT >= 26
                ? new Notification.Builder(this, CHANNEL) : new Notification.Builder(this);
        builder.setSmallIcon(session.microphone ? android.R.drawable.ic_btn_speak_now
                : android.R.drawable.ic_media_play)
                .setContentTitle("SonaHop")
                .setContentText(session.systemCapture ? "正在共享本机播放声音 · 点停止结束"
                        : session.microphone ? "正在共享麦克风 · 点停止结束" : "正在接收音频 · 点停止结束")
                .setCategory(Notification.CATEGORY_SERVICE).setOngoing(true).setOnlyAlertOnce(true)
                .addAction(new Notification.Action.Builder(android.R.drawable.ic_media_pause,
                        "停止", stopAction).build());
        Intent launch = getPackageManager().getLaunchIntentForPackage(getPackageName());
        if (launch != null) builder.setContentIntent(PendingIntent.getActivity(this, 0, launch,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE));
        if (Build.VERSION.SDK_INT >= 31) {
            builder.setForegroundServiceBehavior(Notification.FOREGROUND_SERVICE_IMMEDIATE);
        }
        return builder.build();
    }

    void finished(Session session) {
        if (owned == session) {
            owned = null;
            stopForeground(STOP_FOREGROUND_REMOVE);
            stopSelfResult(session.startId);
        }
    }

    @Override public void onDestroy() {
        Session session = owned;
        owned = null;
        if (session != null && session.running) {
            AudioBridge.fail(session, "Android audio foreground service was stopped");
        }
        super.onDestroy();
    }

    static final class Session {
        final long id;
        final Context context;
        final boolean microphone;
        final boolean systemCapture;
        final int captureChannels;
        final Object ioLock = new Object();
        final AtomicBoolean disposing = new AtomicBoolean();
        final ArrayList<AudioBridge.StartResult> stopCallbacks = new ArrayList<>();
        volatile boolean running = true;
        volatile boolean ready;
        volatile AudioService service;
        volatile Thread worker;
        int startId;
        AudioBridge.StartResult callback;
        AudioTrack track;
        AudioRecord recorder;
        Intent projectionData;
        MediaProjection projection;
        MediaProjection.Callback projectionCallback;
        AudioManager manager;
        AudioFocusRequest focus;
        AudioManager.OnAudioFocusChangeListener focusListener;
        boolean focusHeld;
        PowerManager.WakeLock wakeLock;

        Session(long id, Context context, boolean microphone, boolean systemCapture,
                Intent projectionData, AudioBridge.StartResult callback) {
            this.id = id;
            this.context = context;
            this.microphone = microphone;
            this.systemCapture = systemCapture;
            this.captureChannels = systemCapture ? 2 : microphone ? 1 : 0;
            this.projectionData = projectionData;
            this.callback = callback;
        }

        void initialize() {
            try {
                Process.setThreadPriority(Process.THREAD_PRIORITY_AUDIO);
                synchronized (ioLock) {
                    if (!running) return;
                    AudioAttributes attributes = new AudioAttributes.Builder()
                            .setUsage(microphone ? AudioAttributes.USAGE_VOICE_COMMUNICATION : AudioAttributes.USAGE_MEDIA)
                            .setContentType(microphone ? AudioAttributes.CONTENT_TYPE_SPEECH : AudioAttributes.CONTENT_TYPE_MUSIC)
                            .build();
                    // Internal playback capture must not steal focus and pause the captured music app.
                    if (!systemCapture) {
                        manager = (AudioManager) context.getSystemService(Context.AUDIO_SERVICE);
                        focusListener = change -> {
                            if (change < 0) AudioBridge.fail(this, "Android audio focus was lost");
                        };
                        int gain = microphone ? AudioManager.AUDIOFOCUS_GAIN_TRANSIENT_EXCLUSIVE : AudioManager.AUDIOFOCUS_GAIN;
                        int result;
                        if (Build.VERSION.SDK_INT >= 26) {
                            focus = new AudioFocusRequest.Builder(gain).setAudioAttributes(attributes)
                                    .setAcceptsDelayedFocusGain(false).setWillPauseWhenDucked(true)
                                    .setOnAudioFocusChangeListener(focusListener, MAIN).build();
                            result = manager.requestAudioFocus(focus);
                        } else result = manager.requestAudioFocus(focusListener, AudioManager.STREAM_MUSIC, gain);
                        if (result != AudioManager.AUDIOFOCUS_REQUEST_GRANTED) {
                            throw new IllegalStateException("Android did not grant audio focus");
                        }
                        focusHeld = true;
                    }
                    // No global AudioManager mode, speaker route or stream volume is changed.
                    if (systemCapture) initializeSystemCapture();
                    else if (microphone) initializeCapture();
                    else initializePlayback(attributes);
                    if (!running) return;
                    PowerManager power = (PowerManager) context.getSystemService(Context.POWER_SERVICE);
                    wakeLock = power.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "SonaHop:Audio");
                    wakeLock.setReferenceCounted(false);
                    wakeLock.acquire();
                }
                AudioBridge.ready(this);
                if (captureChannels != 0) captureLoop();
            } catch (RuntimeException error) {
                AudioBridge.fail(this, "Android native audio failed: " + error.getMessage());
            }
        }

        private void initializePlayback(AudioAttributes attributes) {
            int minimum = AudioTrack.getMinBufferSize(48000, AudioFormat.CHANNEL_OUT_STEREO,
                    AudioFormat.ENCODING_PCM_16BIT);
            if (minimum <= 0) throw new IllegalStateException("48 kHz stereo playback is unavailable: " + minimum);
            AudioTrack.Builder builder = new AudioTrack.Builder().setAudioAttributes(attributes)
                    .setAudioFormat(new AudioFormat.Builder().setSampleRate(48000)
                            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                            .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO).build())
                    .setTransferMode(AudioTrack.MODE_STREAM).setBufferSizeInBytes(Math.max(minimum, 3 * 3840));
            if (Build.VERSION.SDK_INT >= 26) builder.setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY);
            track = builder.build();
            if (track.getState() != AudioTrack.STATE_INITIALIZED || track.getSampleRate() != 48000
                    || track.getChannelCount() != 2) throw new IllegalStateException("AudioTrack did not initialize at 48 kHz stereo");
            int bufferResult = track.setBufferSizeInFrames(Math.max((minimum + 3) / 4, 3 * 960));
            if (bufferResult <= 0) throw new IllegalStateException("Cannot set AudioTrack buffer: " + bufferResult);
            track.play();
            if (track.getPlayState() != AudioTrack.PLAYSTATE_PLAYING) throw new IllegalStateException("AudioTrack did not start");
        }

        private void initializeSystemCapture() {
            if (Build.VERSION.SDK_INT < 29) throw new IllegalStateException("Playback capture requires Android 10+");
            if (context.checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED)
                throw new SecurityException("Playback capture RECORD_AUDIO permission was revoked");
            Intent data = projectionData;
            projectionData = null; // Consume once, including failed initialization; never reuse this grant.
            if (data == null) throw new IllegalStateException("Playback capture needs fresh user consent");
            MediaProjectionManager projections = (MediaProjectionManager)
                    context.getSystemService(Context.MEDIA_PROJECTION_SERVICE);
            projection = projections.getMediaProjection(Activity.RESULT_OK, data);
            if (projection == null) throw new IllegalStateException("Playback capture consent is no longer valid");
            projectionCallback = new MediaProjection.Callback() {
                @Override public void onStop() {
                    AudioBridge.fail(Session.this,
                            "Android playback capture authorization was revoked; start again with fresh consent");
                }
            };
            projection.registerCallback(projectionCallback, MAIN);
            AudioPlaybackCaptureConfiguration.Builder captureBuilder = new AudioPlaybackCaptureConfiguration.Builder(projection)
                    .addMatchingUsage(AudioAttributes.USAGE_MEDIA)
                    .addMatchingUsage(AudioAttributes.USAGE_GAME)
                    .addMatchingUsage(AudioAttributes.USAGE_UNKNOWN);
            captureBuilder.excludeUid(Process.myUid());
            AudioPlaybackCaptureConfiguration capture=captureBuilder.build();
            int minimum = AudioRecord.getMinBufferSize(48000, AudioFormat.CHANNEL_IN_STEREO,
                    AudioFormat.ENCODING_PCM_16BIT);
            if (minimum <= 0) throw new IllegalStateException("48 kHz stereo playback capture is unavailable: " + minimum);
            recorder = new AudioRecord.Builder()
                    .setAudioPlaybackCaptureConfig(capture)
                    .setAudioFormat(new AudioFormat.Builder().setSampleRate(48000)
                            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                            .setChannelMask(AudioFormat.CHANNEL_IN_STEREO).build())
                    .setBufferSizeInBytes(Math.max(minimum, 3 * 3840)).build();
            if (recorder.getState() != AudioRecord.STATE_INITIALIZED || recorder.getSampleRate() != 48000
                    || recorder.getChannelCount() != 2)
                throw new IllegalStateException("AudioRecord did not initialize playback capture at 48 kHz stereo");
            recorder.startRecording();
            if (recorder.getRecordingState() != AudioRecord.RECORDSTATE_RECORDING)
                throw new IllegalStateException("Playback capture AudioRecord did not start");
        }

        private void initializeCapture() {
            if (context.checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED)
                throw new SecurityException("Audio capture RECORD_AUDIO permission was revoked");
            int minimum = AudioRecord.getMinBufferSize(48000, AudioFormat.CHANNEL_IN_MONO,
                    AudioFormat.ENCODING_PCM_16BIT);
            if (minimum <= 0) throw new IllegalStateException("48 kHz mono recording is unavailable: " + minimum);
            recorder = new AudioRecord.Builder().setAudioSource(MediaRecorder.AudioSource.VOICE_RECOGNITION)
                    .setAudioFormat(new AudioFormat.Builder().setSampleRate(48000)
                            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                            .setChannelMask(AudioFormat.CHANNEL_IN_MONO).build())
                    .setBufferSizeInBytes(Math.max(minimum, 3 * 1920)).build();
            if (recorder.getState() != AudioRecord.STATE_INITIALIZED || recorder.getSampleRate() != 48000
                    || recorder.getChannelCount() != 1) throw new IllegalStateException("AudioRecord did not initialize at 48 kHz mono");
            recorder.startRecording();
            if (recorder.getRecordingState() != AudioRecord.RECORDSTATE_RECORDING)
                throw new IllegalStateException("AudioRecord did not start");
        }

        private void captureLoop() {
            byte[] packet = new byte[960 * captureChannels * 2]; // 20 ms PCM16: mono microphone or stereo playback capture.
            int offset = 0;
            long permissionCheck = 0;
            while (running) {
                int count;
                synchronized (ioLock) {
                    if (!running || recorder == null) break;
                    long now = SystemClock.elapsedRealtime();
                    if (now >= permissionCheck) {
                        permissionCheck = now + 250;
                        if (context.checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED)
                            throw new SecurityException("Audio capture RECORD_AUDIO permission was revoked");
                        if (microphone && Build.VERSION.SDK_INT >= 29) {
                            AudioRecordingConfiguration configuration = recorder.getActiveRecordingConfiguration();
                            if (configuration != null && configuration.isClientSilenced())
                                throw new IllegalStateException("Android silenced microphone recording");
                        }
                    }
                    count = recorder.read(packet, offset, packet.length - offset, AudioRecord.READ_NON_BLOCKING);
                }
                if (count < 0 || count % 2 != 0) throw new IllegalStateException("AudioRecord read failed: " + count);
                offset += count;
                if (offset == packet.length) {
                    AudioBridge.captured(this, Arrays.copyOf(packet, packet.length));
                    offset = 0;
                } else if (count == 0) SystemClock.sleep(2);
            }
        }

        void dispose() {
            if (!disposing.compareAndSet(false, true)) return;
            new Thread(() -> {
                synchronized (ioLock) {
                    projectionData = null;
                    if (recorder != null) {
                        try { recorder.stop(); } catch (RuntimeException ignored) { }
                        try { recorder.release(); } catch (RuntimeException ignored) { }
                        recorder = null;
                    }
                    if (track != null) {
                        try { track.pause(); track.flush(); track.stop(); } catch (RuntimeException ignored) { }
                        try { track.release(); } catch (RuntimeException ignored) { }
                        track = null;
                    }
                    if (projection != null) {
                        if (projectionCallback != null) {
                            try { projection.unregisterCallback(projectionCallback); } catch (RuntimeException ignored) { }
                        }
                        try { projection.stop(); } catch (RuntimeException ignored) { }
                        projection = null;
                        projectionCallback = null;
                    }
                    if (manager != null && focusHeld) {
                        try {
                            if (Build.VERSION.SDK_INT >= 26 && focus != null) manager.abandonAudioFocusRequest(focus);
                            else manager.abandonAudioFocus(focusListener);
                        } catch (RuntimeException ignored) { }
                        focusHeld = false;
                    }
                    if (wakeLock != null && wakeLock.isHeld()) {
                        try { wakeLock.release(); } catch (RuntimeException ignored) { }
                    }
                }
                MAIN.post(() -> {
                    AudioService owner = service;
                    if (owner != null) owner.finished(this);
                    AudioBridge.released(this);
                });
            }, "sonahop-android-audio-stop").start();
        }
    }
}
