package io.github.redpandaex.sonahop.lan

import android.Manifest
import android.app.Activity
import android.os.Build
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
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

    override fun onDestroy() { AudioBridge.stop() }

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
    fun prepareAudio(invoke: Invoke) {
        activity.runOnUiThread {
            val mode = invoke.getArgs().optString("mode", "")
            if (mode != "listen" && mode != "microphone" && mode != "mic") {
                invoke.reject("Unknown Android audio mode")
                return@runOnUiThread
            }
            if (!foreground()) {
                invoke.reject("请保持 SonaHop 在前台，再点击开始收听或申请麦克风")
                return@runOnUiThread
            }
            if (mode == "listen") completeAudioPermission(invoke, true)
            else if (getPermissionState("microphone") == PermissionState.GRANTED) completeAudioPermission(invoke, true)
            else requestPermissionForAlias("microphone", invoke, "microphonePermissionResult")
        }
    }

    @PermissionCallback
    fun microphonePermissionResult(invoke: Invoke) {
        completeAudioPermission(invoke, getPermissionState("microphone") == PermissionState.GRANTED)
    }

    private fun completeAudioPermission(invoke: Invoke, granted: Boolean) {
        if (granted && Build.VERSION.SDK_INT >= 33 && !notificationRequested &&
            getPermissionState("notifications") != PermissionState.GRANTED) {
            notificationRequested = true
            requestPermissionForAlias("notifications", invoke, "notificationPermissionResult")
        } else resolveAudioPermission(invoke, granted)
    }

    @PermissionCallback
    fun notificationPermissionResult(invoke: Invoke) {
        val mode = invoke.getArgs().optString("mode", "")
        // Notifications are optional; declining them never silently grants microphone access.
        resolveAudioPermission(invoke, mode == "listen" || getPermissionState("microphone") == PermissionState.GRANTED)
    }

    @Command
    fun startPlayback(invoke: Invoke) { start(invoke, false) }

    @Command
    fun startCapture(invoke: Invoke) { start(invoke, true) }

    private fun start(invoke: Invoke, microphone: Boolean) {
        activity.runOnUiThread {
            if (!foreground()) {
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
            AudioBridge.stop { error ->
                if (error != null) invoke.reject(error)
                else invoke.resolve(JSObject().put("started", false))
            }
        }
    }

    private fun resolveAudioPermission(invoke: Invoke, granted: Boolean) {
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
