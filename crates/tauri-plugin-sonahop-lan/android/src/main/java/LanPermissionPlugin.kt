package io.github.redpandaex.sonahop.lan

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.media.projection.MediaProjectionManager
import android.media.projection.MediaProjectionConfig
import android.os.Build
import android.os.SystemClock
import android.os.Process
import org.json.JSONArray
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
    private var preparingUid = -1
    private var preparedUid = -1
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
    fun listCaptureApps(invoke: Invoke) {
        try {
            val pm=activity.packageManager
            val intent=Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
            val apps=pm.queryIntentActivities(intent,0).mapNotNull { item ->
                val info=item.activityInfo.applicationInfo
                if(info.uid==Process.myUid() || (pm.getPackagesForUid(info.uid)?.size ?: 0)>1) null
                else Pair(info.packageName,pm.getApplicationLabel(info).toString())
            }.distinctBy {it.first}.sortedBy {it.second.lowercase()}
            val result=JSONArray()
            for(app in apps) result.put(JSObject().put("packageName",app.first).put("label",app.second))
            invoke.resolve(JSObject().put("apps",result))
        } catch(error:RuntimeException) {invoke.reject("读取应用列表失败："+error.message)}
    }
    private fun targetUid(invoke: Invoke): Int {
        val pkg=invoke.getArgs().optString("packageName","").trim()
        if(pkg.isEmpty() || pkg=="null") return -1
        val pm=activity.packageManager
        val info=try {pm.getApplicationInfo(pkg,0)} catch(error:android.content.pm.PackageManager.NameNotFoundException) {throw IllegalArgumentException("音源应用已不可用，请刷新列表")}
        require(info.uid!=Process.myUid()) {"请选择其它应用作为声音来源"}
        require((pm.getPackagesForUid(info.uid)?.size ?: 0)==1) {"该应用共享音频身份，无法单独采集"}
        return info.uid
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
            if (preparingAudio != null || AudioBridge.hasActiveSession()) {
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
                preparingUid=if(mode=="playback") targetUid(invoke) else -1
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
            val intent = if (Build.VERSION.SDK_INT >= 34 && preparingUid < 0)
                manager.createScreenCaptureIntent(MediaProjectionConfig.createConfigForDefaultDisplay())
            else manager.createScreenCaptureIntent()
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
        if (granted) {preparedProjection=data;preparedUid=preparingUid;preparedAt=SystemClock.elapsedRealtime()}
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
            val uid=preparedUid
            val fresh=preparedAt>0 && SystemClock.elapsedRealtime()-preparedAt<30000
            preparedProjection = null;preparedUid=-1;preparedAt=0L
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
            AudioBridge.startSystemCapture(activity, projectionData, uid) { error ->
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
                if (error != null) invoke.reject(error)
                else invoke.resolve(JSObject().put("started", false))
            }
        }
    }

    private fun cancelPreparation(reason: String) {
        preparedProjection = null;preparedUid=-1;preparedAt=0L;preparingUid=-1
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
