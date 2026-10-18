import { useCallback, useEffect, useRef, useState } from "react";
import type { ListenOptions } from "./audio-bridge";
import { prepareAudio } from "./audio-bridge";
import type { AppInfo, PlaybackDevice } from "./bridge";
import type { ConnectionRole, Preferences, SendSource } from "./preferences";
import type { useAudio } from "./use-audio";
import type { useLink } from "./use-link";
import { virtualMicrophoneReady } from "./virtual-audio";

interface SessionContext {
  info: AppInfo;
  role: ConnectionRole;
  roleReady: boolean;
  prefs: Preferences;
  devices: PlaybackDevice[];
  captureDevices: PlaybackDevice[];
  audio: ReturnType<typeof useAudio>;
  link: ReturnType<typeof useLink>;
  update: (value: Partial<Preferences>) => void;
}
interface ListenIntent {
  epoch: number;
  options: ListenOptions;
}

export function useSessionActions({
  info,
  role,
  roleReady,
  prefs,
  devices,
  captureDevices,
  audio,
  link,
  update,
}: SessionContext) {
  const [listenIntent, setListenIntent] = useState<ListenIntent | null>(null);
  const [connectPreparing, setConnectPreparing] = useState(false);
  const [sharePreparing, setSharePreparing] = useState(false);
  const [sharePaused, setSharePaused] = useState(false);
  const [sessionError, setSessionError] = useState("");
  const epoch = useRef(0);
  const mounted = useRef(true);
  const connectGate = useRef(false);
  const shareGate = useRef(false);
  const hostAttempted = useRef(false);
  const currentRole = useRef(role);
  currentRole.current = role;

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      epoch.current += 1;
    };
  }, []);

  const clearClientIntent = useCallback(() => {
    epoch.current += 1;
    setListenIntent(null);
    setSessionError("");
  }, []);

  const listenOptions = useCallback((): ListenOptions => {
    if (info.platform !== "windows") return {};
    if (prefs.playbackDestination === "microphone") {
      if (
        !virtualMicrophoneReady(
          devices,
          captureDevices,
          prefs.microphoneOutputDeviceId,
          prefs.microphoneRecordingDeviceId,
        )
      )
        throw new Error("请先选择可用的电脑麦克风入口和对应录音端。");
      return { outputDeviceId: prefs.microphoneOutputDeviceId };
    }
    if (prefs.outputDeviceId && !devices.some((device) => device.id === prefs.outputDeviceId))
      throw new Error("已保存的播放设备不可用，请重新选择音频输出。");
    return { outputDeviceId: prefs.outputDeviceId || undefined };
  }, [
    info.platform,
    devices,
    captureDevices,
    prefs.playbackDestination,
    prefs.microphoneOutputDeviceId,
    prefs.microphoneRecordingDeviceId,
    prefs.outputDeviceId,
  ]);

  const connectAndListen = useCallback(
    async (address: string, port: number) => {
      if (
        connectGate.current ||
        role !== "client" ||
        !roleReady ||
        !audio.available ||
        !link.available
      )
        return false;
      connectGate.current = true;
      const attempt = ++epoch.current;
      setListenIntent(null);
      setConnectPreparing(true);
      setSessionError("");
      try {
        const options = listenOptions();
        // Prepare playback from the connection click, never from a microphone/background restart.
        await prepareAudio(info.platform, "listen");
        if (!mounted.current || epoch.current !== attempt || currentRole.current !== "client")
          return false;
        if (!(await link.connect(address, port))) return false;
        if (mounted.current && epoch.current === attempt && currentRole.current === "client")
          setListenIntent({ epoch: attempt, options });
        return true;
      } catch (error) {
        if (mounted.current && epoch.current === attempt) setSessionError(String(error));
        return false;
      } finally {
        connectGate.current = false;
        if (mounted.current) setConnectPreparing(false);
      }
    },
    [role, roleReady, audio.available, link.available, link.connect, info.platform, listenOptions],
  );

  useEffect(() => {
    if (role !== "client" || !roleReady || !link.snapshot?.running) clearClientIntent();
  }, [role, roleReady, link.snapshot?.running, clearClientIntent]);

  useEffect(() => {
    if (
      !listenIntent ||
      listenIntent.epoch !== epoch.current ||
      role !== "client" ||
      connectPreparing ||
      !link.snapshot
    )
      return;
    if (
      link.snapshot.lastError ||
      link.snapshot.clientState === "discovering" ||
      link.snapshot.clientState === "idle"
    ) {
      clearClientIntent();
      return;
    }
    if (
      link.snapshot?.clientState !== "connected" ||
      !link.snapshot.peers.length ||
      !audio.snapshot
    )
      return;
    if (document.visibilityState !== "visible") return;
    const intent = listenIntent;
    // Consume before starting so polling cannot create a second subscription.
    setListenIntent(null);
    if (intent.epoch === epoch.current) void audio.startListen(intent.options, true);
  }, [
    listenIntent,
    connectPreparing,
    clearClientIntent,
    link.snapshot,
    audio.snapshot,
    audio.startListen,
    role,
  ]);

  const retryListening = useCallback(async () => {
    if (role !== "client" || !link.snapshot?.peers.length) return false;
    setSessionError("");
    try {
      return await audio.startListen(listenOptions());
    } catch (error) {
      if (mounted.current) setSessionError(String(error));
      return false;
    }
  }, [role, link.snapshot?.peers.length, audio.startListen, listenOptions]);

  const activateShareSource = useCallback(
    async (source: SendSource) => {
      if (
        shareGate.current ||
        role !== "host" ||
        !roleReady ||
        !audio.available ||
        !link.snapshot?.running
      )
        return false;
      shareGate.current = true;
      hostAttempted.current = true;
      setSharePreparing(true);
      setSessionError("");
      try {
        if (source === "system" && !info.systemCaptureSupported)
          throw new Error("当前设备不支持系统声音采集。");
        if (
          source === "microphone" &&
          info.platform === "windows" &&
          !captureDevices.some(
            (device) => !prefs.inputDeviceId || device.id === prefs.inputDeviceId,
          )
        )
          throw new Error("请选择可用的本机麦克风。");
        const lane = audio.snapshot?.listen;
        if ((lane?.active || lane?.waiting) && audio.snapshot?.sendSource === source) {
          setSharePaused(false);
          return true;
        }
        if ((lane?.active || lane?.waiting) && !(await audio.stopLane("listen"))) return false;
        if (!mounted.current || currentRole.current !== "host") return false;
        update({ sendSource: source });
        const started = await audio.startListen({
          source,
          inputDeviceId: prefs.inputDeviceId || undefined,
          gain: prefs.volume,
        });
        if (mounted.current && started) setSharePaused(false);
        return started;
      } catch (error) {
        if (mounted.current) setSessionError(String(error));
        return false;
      } finally {
        shareGate.current = false;
        if (mounted.current) setSharePreparing(false);
      }
    },
    [
      role,
      roleReady,
      audio.available,
      audio.snapshot,
      audio.stopLane,
      audio.startListen,
      link.snapshot?.running,
      info.systemCaptureSupported,
      info.platform,
      captureDevices,
      prefs.inputDeviceId,
      prefs.volume,
      update,
    ],
  );

  useEffect(() => {
    if (role !== "host" || !roleReady || !link.snapshot?.running) {
      hostAttempted.current = false;
      return;
    }
    if (!audio.available || !audio.snapshot || audio.busy || hostAttempted.current) return;
    hostAttempted.current = true;
    setSharePaused(false);
    // Restoring a saved microphone selection must never restore recording automatically.
    if (prefs.sendSource === "system" && info.systemCaptureSupported)
      void activateShareSource("system");
  }, [
    role,
    roleReady,
    link.snapshot?.running,
    audio.available,
    audio.snapshot,
    audio.busy,
    prefs.sendSource,
    info.systemCaptureSupported,
    activateShareSource,
  ]);

  const pauseSharing = useCallback(async () => {
    hostAttempted.current = true;
    setSharePaused(true);
    return audio.stopLane("listen");
  }, [audio.stopLane]);

  return {
    connectAndListen,
    clearClientIntent,
    retryListening,
    activateShareSource,
    pauseSharing,
    sharePaused,
    sessionError,
    sessionBusy: connectPreparing || sharePreparing,
    connectionPending:
      connectPreparing || Boolean(listenIntent) || link.snapshot?.clientState === "connecting",
  };
}
