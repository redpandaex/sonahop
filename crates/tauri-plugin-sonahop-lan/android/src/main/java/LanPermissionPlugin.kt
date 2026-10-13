package io.github.redpandaex.sonahop.lan

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.SystemClock
import androidx.activity.result.ActivityResult
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.PermissionState
import app.tauri.plugin.Plugin

@TauriPlugin(
    permissions = [
        Permission(strings = ["android.permission.ACCESS_LOCAL_NETWORK"], alias = "lan"),
        Permission(strings = [Manifest.permission.RECORD_AUDIO], alias = "microphone"),
        Permission(strings = ["android.permission.POST_NOTIFICATIONS"], alias = "notifications"),
    ]
)
class LanPermissionPlugin(private val activity: Activity) : Plugin(activity) {
    private var notificationRequested = false
    // Accessed on the Activity UI thread only. Consent is never a reusable session credential.
    private var preparingAudio: Invoke? = null
    private var preparedProjection: Intent? = null
    private var preparedAt = 0L

    override fun onDestroy() {
        cancelPreparation("Android activity was destroyed; start audio again explicitly")
        AudioBridge.stop()
    }

    private fun foreground(): Boolean {
        // A plugin can load after onResume; window focus at construction is not lifecycle state.
        // Read the owning Activity on the UI thread instead of caching plugin callbacks.
        val lifecycle = (activity as? LifecycleOwner)?.lifecycle ?: return false
        return lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED) &&
            !activity.isFinishing && !activity.isDestroyed && activity.window.decorView.isShown
    }

    @Command
    fun ensurePermission(invoke: Invoke) {
        if (Build.VERSION.SDK_INT < 37 || getPermissionState("lan") == PermissionState.GRANTED) {
            resolvePermission(invoke)
        } else {
            requestPermissionForAlias("lan", invoke, "lanPermissionResult")
        }
    }

    @PermissionCallback
    fun lanPermissionResult(invoke: Invoke) { resolvePermission(invoke) }

    @Command
    fun audioCapabilities(invoke: Invoke) {
        invoke.resolve(JSObject()
            .put("playbackCapture", Build.VERSION.SDK_INT >= 29)
            .put("microphoneCapture", true))
    }

    @Command
    fun prepareAudio(invoke: Invoke) {
        activity.runOnUiThread {
            val mode = invoke.getArgs().optString("mode", "")
            if (mode != "listen" && mode != "microphone" && mode != "mic" && mode != "playback") {
                invoke.reject("Unknown Android audio mode")
                return@runOnUiThread
            }
            if (!foreground()) {
                invoke.reject("请保持 SonaHop 在前台，再点击开始音频")
                return@runOnUiThread
            }
            if (preparingAudio != null || AudioBridge.hasActiveSession(mode != "listen")) {
                invoke.reject("Android audio is already active, stopping or awaiting permission")
                return@runOnUiThread
            }
            // Changing source or preparing again discards any unconsumed older grant.
            preparedProjection = null
            if (mode == "playback" && Build.VERSION.SDK_INT < 29) {
                invoke.reject("Android 内部播放采集需要 Android 10 或更新版本")
                return@runOnUiThread
            }
            preparingAudio = invoke
            try {
                if (mode == "listen" || getPermissionState("microphone") == PermissionState.GRANTED)
                    completeAudioPermission(invoke, true)
                else requestPermissionForAlias("microphone", invoke, "microphonePermissionResult")
            } catch (error: RuntimeException) {
                rejectPreparation(invoke, "Cannot request Android audio permission: ${error.message}")
            }
        }
    }

    @PermissionCallback
    fun microphonePermissionResult(invoke: Invoke) {
        if (preparingAudio !== invoke) return
        completeAudioPermission(invoke, getPermissionState("microphone") == PermissionState.GRANTED)
    }

    private fun completeAudioPermission(invoke: Invoke, granted: Boolean) {
        if (preparingAudio !== invoke) return
        try {
            if (granted && Build.VERSION.SDK_INT >= 33 && !notificationRequested &&
                getPermissionState("notifications") != PermissionState.GRANTED) {
                notificationRequested = true
                requestPermissionForAlias("notifications", invoke, "notificationPermissionResult")
            } else finishAudioPermission(invoke, granted)
        } catch (error: RuntimeException) {
            rejectPreparation(invoke, "Cannot prepare Android audio: ${error.message}")
        }
    }

    @PermissionCallback
    fun notificationPermissionResult(invoke: Invoke) {
        if (preparingAudio !== invoke) return
        val mode = invoke.getArgs().optString("mode", "")
        // Notifications are optional; declining them never silently grants capture access.
        finishAudioPermission(invoke,
            mode == "listen" || getPermissionState("microphone") == PermissionState.GRANTED)
    }

    private fun finishAudioPermission(invoke: Invoke, granted: Boolean) {
        if (preparingAudio !== invoke) return
        if (!granted || invoke.getArgs().optString("mode", "") != "playback") {
            resolveAudioPermission(invoke, granted)
            return
        }
        try {
            if (activity.isFinishing || activity.isDestroyed) {
                rejectPreparation(invoke, "Android activity is no longer available")
                return
            }
            val manager = activity.getSystemService(Context.MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
            // Only explicit prepareAudio(playback) reaches this system-owned consent dialog.
            val intent = manager.createScreenCaptureIntent()
            startActivityForResult(invoke, intent, "playbackCaptureResult")
        } catch (error: RuntimeException) {
            rejectPreparation(invoke, "Cannot request playback capture consent: ${error.message}")
        }
    }

    @ActivityCallback
    fun playbackCaptureResult(invoke: Invoke, result: ActivityResult) {
        // A role stop or Activity destruction invalidates the outstanding request as well.
        if (preparingAudio !== invoke) return
        preparedProjection = null
        if (activity.isFinishing || activity.isDestroyed) {
            rejectPreparation(invoke, "Playback capture consent was cancelled with the activity")
            return
        }
        val data = result.data
        val granted = result.resultCode == Activity.RESULT_OK && data != null &&
            getPermissionState("microphone") == PermissionState.GRANTED
        if (granted) {preparedProjection=data;preparedAt=SystemClock.elapsedRealtime()}
        resolveAudioPermission(invoke, granted)
    }

    @Command
    fun startPlayback(invoke: Invoke) { start(invoke, false) }

    @Command
    fun startCapture(invoke: Invoke) { start(invoke, true) }

    @Command
    fun startSystemCapture(invoke: Invoke) {
        activity.runOnUiThread {
            // Remove before any fallible operation: failed starts must also obtain fresh consent.
            val projectionData = preparedProjection
            val fresh=preparedAt>0 && SystemClock.elapsedRealtime()-preparedAt<30000
            preparedProjection = null;preparedAt=0L
            if (preparingAudio != null || !fresh || activity.isFinishing || activity.isDestroyed) {
                invoke.reject("请保持 SonaHop 在前台，并完成内部播放采集授权后重新开始")
                return@runOnUiThread
            }
            if (Build.VERSION.SDK_INT < 29 || projectionData == null) {
                invoke.reject("请先主动授权内部播放采集；每次开始都需要新的授权")
                return@runOnUiThread
            }
            if (getPermissionState("microphone") != PermissionState.GRANTED) {
                invoke.reject("Playback capture RECORD_AUDIO permission is not granted")
                return@runOnUiThread
            }
            AudioBridge.startSystemCapture(activity, projectionData) { error ->
                if (error != null) invoke.reject(error)
                else invoke.resolve(JSObject().put("started", true))
            }
        }
    }

    private fun start(invoke: Invoke, microphone: Boolean) {
        activity.runOnUiThread {
            preparedProjection = null
            if (preparingAudio != null || !foreground()) {
                invoke.reject("请保持 SonaHop 在前台，再重新开始音频")
                return@runOnUiThread
            }
            if (microphone && getPermissionState("microphone") != PermissionState.GRANTED) {
                invoke.reject("Microphone permission is not granted")
                return@runOnUiThread
            }
            AudioBridge.start(activity, microphone) { error ->
                if (error != null) invoke.reject(error)
                else invoke.resolve(JSObject().put("started", true))
            }
        }
    }

    @Command
    fun stopAudio(invoke: Invoke) {
        activity.runOnUiThread {
            cancelPreparation("Android audio preparation was stopped; obtain fresh consent to start again")
            AudioBridge.stop { error ->
                AudioBridge.finishNotificationStop()
                if (error != null) invoke.reject(error)
                else invoke.resolve(JSObject().put("started", false))
            }
        }
    }

    @Command
    fun stopCapture(invoke: Invoke) {
        activity.runOnUiThread {
            if (preparingAudio?.getArgs()?.optString("mode", "") != "listen")
                cancelPreparation("Android capture preparation was stopped")
            AudioBridge.stopCapture { error ->
                if (error != null) invoke.reject(error) else invoke.resolve(JSObject().put("started", false))
            }
        }
    }
    @Command
    fun stopPlayback(invoke: Invoke) {
        activity.runOnUiThread {
            if (preparingAudio?.getArgs()?.optString("mode", "") == "listen") {
                val pending = preparingAudio
                preparingAudio = null
                pending?.reject("Android playback preparation was stopped")
            }
            AudioBridge.stopPlayback { error ->
                if (error != null) invoke.reject(error) else invoke.resolve(JSObject().put("started", false))
            }
        }
    }
    private fun cancelPreparation(reason: String) {
        preparedProjection = null;preparedAt=0L
        val pending = preparingAudio
        preparingAudio = null
        pending?.reject(reason)
    }

    private fun rejectPreparation(invoke: Invoke, reason: String) {
        if (preparingAudio !== invoke) return
        preparedProjection = null
        preparingAudio = null
        invoke.reject(reason)
    }

    private fun resolveAudioPermission(invoke: Invoke, granted: Boolean) {
        if (preparingAudio !== invoke) return
        preparingAudio = null
        val result = JSObject()
        result.put("granted", granted)
        result.put("deviceName", deviceName())
        invoke.resolve(result)
    }

    private fun resolvePermission(invoke: Invoke) {
        val result = JSObject()
        result.put("granted", Build.VERSION.SDK_INT < 37 || getPermissionState("lan") == PermissionState.GRANTED)
        result.put("deviceName", deviceName())
        invoke.resolve(result)
    }

    private fun deviceName(): String = (Build.MANUFACTURER + " " + Build.MODEL).trim().take(64)
}
