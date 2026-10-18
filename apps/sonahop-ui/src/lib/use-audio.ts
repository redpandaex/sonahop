import { useCallback, useEffect, useRef, useState } from "react";
import type { AudioSnapshot, ListenOptions } from "./audio-bridge";
import {
  answerMicrophoneRequest,
  getAudioSnapshot,
  prepareAudio,
  requestMicrophone,
  setAudioGain,
  startListening,
  stopAudio,
  stopAudioLane,
} from "./audio-bridge";
import type { Platform } from "./bridge";
import { nativeLinkAvailable } from "./link-bridge";

export function useAudio(platform: Platform, appReady: boolean) {
  const available =
    nativeLinkAvailable() && appReady && (platform === "windows" || platform === "android");
  const [snapshot, setSnapshot] = useState<AudioSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [operationError, setOperationError] = useState("");
  const [pollError, setPollError] = useState("");
  const revision = useRef(0);
  const lifecycle = useRef<object | null>(null);
  const mutating = useRef(false);
  const reading = useRef(false);
  const acceptSnapshot = useCallback((value: AudioSnapshot) => {
    setSnapshot(value);
    setPollError("");
  }, []);

  useEffect(() => {
    const lifetime = {};
    lifecycle.current = lifetime;
    if (!available)
      return () => {
        lifecycle.current = null;
      };
    let timer: ReturnType<typeof setTimeout> | undefined;
    let visibilityRevision = 0;
    const poll = async () => {
      if (lifecycle.current !== lifetime || document.visibilityState !== "visible") return;
      const visibilityVersion = visibilityRevision;
      if (!reading.current && !mutating.current) {
        reading.current = true;
        const version = revision.current;
        const active = () =>
          lifecycle.current === lifetime &&
          revision.current === version &&
          !mutating.current &&
          visibilityRevision === visibilityVersion &&
          document.visibilityState === "visible";
        try {
          const value = await getAudioSnapshot();
          if (active()) acceptSnapshot(value);
        } catch (error) {
          if (active()) setPollError(String(error));
        } finally {
          reading.current = false;
        }
      }
      if (
        lifecycle.current === lifetime &&
        visibilityRevision === visibilityVersion &&
        document.visibilityState === "visible"
      ) {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => void poll(), 1000);
      }
    };
    const visibilityChanged = () => {
      visibilityRevision += 1;
      if (timer) clearTimeout(timer);
      timer = undefined;
      if (document.visibilityState === "visible") void poll();
    };
    document.addEventListener("visibilitychange", visibilityChanged);
    void poll();
    return () => {
      lifecycle.current = null;
      if (timer) clearTimeout(timer);
      document.removeEventListener("visibilitychange", visibilityChanged);
    };
  }, [available, acceptSnapshot]);

  const perform = useCallback(
    async (operation: () => Promise<AudioSnapshot>) => {
      if (!available || !lifecycle.current || mutating.current) return false;
      const lifetime = lifecycle.current;
      const version = ++revision.current;
      const active = () => lifecycle.current === lifetime && revision.current === version;
      mutating.current = true;
      setBusy(true);
      setOperationError("");
      try {
        const value = await operation();
        if (active()) acceptSnapshot(value);
        return true;
      } catch (error) {
        if (active()) setOperationError(String(error));
        try {
          const value = await getAudioSnapshot();
          if (active()) acceptSnapshot(value);
        } catch {
          // Preserve the user action error until retry; polling can recover the snapshot.
        }
        return false;
      } finally {
        mutating.current = false;
        if (active()) setBusy(false);
      }
    },
    [available, acceptSnapshot],
  );

  const refresh = useCallback(() => perform(getAudioSnapshot), [perform]);
  const snapshotRole = snapshot?.role;
  const startListen = useCallback(
    (options: ListenOptions = {}, prepared = false) =>
      perform(async () => {
        if (!snapshotRole) throw new Error("音频状态尚未就绪，请稍后重试。");
        const source = options.source ?? "system";
        if (!prepared)
          await prepareAudio(
            platform,
            snapshotRole === "host"
              ? source === "microphone"
                ? "microphone"
                : "playback"
              : "listen",
          );
        if (snapshotRole === "host" && source === "microphone" && options.gain !== undefined)
          await setAudioGain(options.gain);
        return startListening(options);
      }),
    [perform, platform, snapshotRole],
  );
  const requestMic = useCallback(
    (gain: number, inputDeviceId?: string) =>
      perform(async () => {
        await prepareAudio(platform, "microphone");
        await setAudioGain(gain);
        return requestMicrophone(inputDeviceId);
      }),
    [perform, platform],
  );
  const answerMic = useCallback(
    (peerId: string, requestId: string, approve: boolean, outputDeviceId: string | null) =>
      perform(() => answerMicrophoneRequest(peerId, requestId, approve, outputDeviceId)),
    [perform],
  );
  const stop = useCallback(() => perform(stopAudio), [perform]);
  const stopLane = useCallback(
    (lane: "listen" | "microphone") => perform(() => stopAudioLane(lane)),
    [perform],
  );
  const setGain = useCallback((gain: number) => perform(() => setAudioGain(gain)), [perform]);
  return {
    snapshot,
    available,
    busy,
    error: operationError || pollError || snapshot?.lastError || "",
    refresh,
    startListen,
    requestMic,
    answerMic,
    stop,
    stopLane,
    setGain,
  };
}
