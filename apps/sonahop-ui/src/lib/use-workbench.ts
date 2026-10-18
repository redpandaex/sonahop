import { useCallback, useEffect, useRef, useState } from "react";
import type { AppInfo, PlaybackDevice } from "./bridge";
import {
  getAppInfo,
  listCaptureDevices,
  listPlaybackDevices,
  platformHint,
  setDesktopTheme,
} from "./bridge";
import { nativeLinkAvailable } from "./link-bridge";
import type { AudioMode, ConnectionRole, Preferences } from "./preferences";
import { PREFERENCES_KEY, readPreferences, savePreferences } from "./preferences";
import { useAudio } from "./use-audio";
import { useAutostart } from "./use-autostart";
import { useDesktopSession } from "./use-desktop-session";
import { useDesktopSettings } from "./use-desktop-settings";
import { useLink } from "./use-link";
import { useSessionActions } from "./use-session-actions";

function initialPreferences() {
  try {
    return readPreferences(localStorage.getItem(PREFERENCES_KEY));
  } catch {
    return readPreferences(null);
  }
}
export function useWorkbench() {
  const [prefs, setPrefs] = useState(initialPreferences);
  const [info, setInfo] = useState<AppInfo>({
    platform: platformHint(),
    version: "0.5.0",
    uiPreview: true,
    browserPreview: !nativeLinkAvailable(),
    systemCaptureSupported: false,
  });
  const [devices, setDevices] = useState<PlaybackDevice[]>([]);
  const [captureDevices, setCaptureDevices] = useState<PlaybackDevice[]>([]);
  const [loading, setLoading] = useState(false);
  const [deviceError, setDeviceError] = useState("");
  const [captureError, setCaptureError] = useState("");
  const [appError, setAppError] = useState("");
  const [appReady, setAppReady] = useState(false);
  const [roleError, setRoleError] = useState("");
  const clearRoleError = useCallback(() => setRoleError(""), []);
  const audio = useAudio(info.platform, appReady, clearRoleError);
  const link = useLink(info.platform, appReady, audio.snapshot?.stopRevision ?? 0, clearRoleError);
  const desktopSession = useDesktopSession(info.platform, appReady);
  const autostart = useAutostart(info.platform, appReady);
  const desktop = useDesktopSettings(info.platform, appReady);
  const micOutputDeviceId = prefs.microphoneOutputDeviceId;
  const [roleBusy, setRoleBusy] = useState(false);
  const [roleReady, setRoleReady] = useState(!nativeLinkAvailable());
  const roleGate = useRef(false);
  const initialized = useRef(false);
  const desktopRevision = useRef(0);
  desktopRevision.current = desktopSession.snapshot?.revision ?? 0;
  const mounted = useRef(true);
  const deviceRevision = useRef(0);
  const [saved, setSaved] = useState(true);
  const [systemDark, setSystemDark] = useState(
    () => matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const [page, setPage] = useState(() =>
    location.hash === "#settings" ? "settings" : "workspace",
  );
  const role: ConnectionRole =
    link.native && link.snapshot
      ? link.snapshot.role
      : (prefs.connectionRole ?? (info.platform === "android" ? "client" : "host"));

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const update = useCallback(
    (value: Partial<Preferences>) => setPrefs((previous) => ({ ...previous, ...value })),
    [],
  );
  const setMicOutputDeviceId = useCallback(
    (microphoneOutputDeviceId: string) =>
      setPrefs((previous) => ({
        ...previous,
        microphoneOutputDeviceId,
        microphoneRecordingDeviceId:
          previous.microphoneOutputDeviceId === microphoneOutputDeviceId
            ? previous.microphoneRecordingDeviceId
            : "",
      })),
    [],
  );
  const sessions = useSessionActions({
    info,
    role,
    roleReady,
    desktopSession: desktopSession.snapshot,
    prefs,
    devices,
    captureDevices,
    audio,
    link,
    update,
  });
  const { clearClientIntent } = sessions;
  const switchRole = useCallback(
    async (next: ConnectionRole) => {
      if (roleGate.current) return false;
      clearClientIntent();
      if (!nativeLinkAvailable()) {
        update({ connectionRole: next });
        setRoleReady(true);
        location.hash = "workspace";
        return true;
      }
      if (!link.available || !audio.available) return false;
      const revision = desktopRevision.current;
      const current = () => mounted.current && desktopRevision.current === revision;
      roleGate.current = true;
      setRoleBusy(true);
      setRoleReady(false);
      setRoleError("");
      try {
        // The native operation stops all audio and connections before changing either service role.
        if (!(await link.changeRole(next)))
          throw new Error("连接角色切换失败，请查看连接错误并重试。");
        if (!(await audio.refresh()))
          throw new Error("角色已切换，但音频状态读取失败。请重试角色切换。");
        if (!current()) return false;
        update({ connectionRole: next });
        if (!(await link.start())) throw new Error("自动发现启动失败，请检查本地网络授权并重试。");
        if (current()) {
          update({ connectionRole: next });
          setRoleReady(true);
          location.hash = "workspace";
        }
        return true;
      } catch (error) {
        if (current()) setRoleError(String(error));
        return false;
      } finally {
        roleGate.current = false;
        if (mounted.current) setRoleBusy(false);
      }
    },
    [
      link.available,
      link.changeRole,
      link.start,
      audio.available,
      audio.refresh,
      update,
      clearClientIntent,
    ],
  );
  useEffect(() => {
    if (!appReady || !desktopSession.ready || !link.snapshot || initialized.current || !link.native)
      return;
    initialized.current = true;
    if (desktopSession.snapshot?.revision) return;
    const initialRole = prefs.connectionRole ?? (info.platform === "android" ? "client" : "host");
    void switchRole(initialRole);
  }, [
    appReady,
    desktopSession.ready,
    desktopSession.snapshot,
    link.snapshot,
    link.native,
    prefs.connectionRole,
    info.platform,
    switchRole,
  ]);

  useEffect(() => {
    const session = desktopSession.snapshot;
    if (!session?.revision) return;
    let active = true;
    initialized.current = true;
    clearClientIntent();
    link.cancelPending();
    audio.cancelPending();
    setRoleReady(false);
    setRoleError(session.error ?? "");
    if (!session.busy) {
      void Promise.all([link.synchronize(), audio.synchronize()]).then(() => {
        if (!active) return;
        if (session.role)
          update({
            connectionRole: session.role,
            ...(session.action === "startSharing" && !session.error
              ? { sendSource: "system" as const }
              : {}),
          });
        setRoleReady(true);
      });
    }
    return () => {
      active = false;
    };
  }, [
    desktopSession.snapshot,
    clearClientIntent,
    link.cancelPending,
    audio.cancelPending,
    link.synchronize,
    audio.synchronize,
    update,
  ]);

  const switchMode = useCallback(
    async (mode: AudioMode) => {
      if (mode === prefs.mode) return true;
      update({ mode });
      return true;
    },
    [prefs.mode, update],
  );
  const stopLink = useCallback(async () => {
    clearClientIntent();
    return link.stop();
  }, [link.stop, clearClientIntent]);
  const retryDiscovery = useCallback(async () => {
    if (roleGate.current) return false;
    clearClientIntent();
    // Native stop records the local intent before it waits for audio teardown.
    if (!(await link.stop())) return false;
    return link.start();
  }, [link.stop, link.start, clearClientIntent]);
  const disconnectPeer = useCallback(
    async (peerId: string) => {
      clearClientIntent();
      const disconnected = await link.disconnect(peerId);
      if (audio.available && role === "client") await audio.stop();
      else if (audio.available && audio.snapshot?.micPeerId === peerId)
        await audio.stopLane("microphone");
      return disconnected;
    },
    [
      audio.available,
      audio.stop,
      audio.stopLane,
      audio.snapshot?.micPeerId,
      role,
      link.disconnect,
      clearClientIntent,
    ],
  );
  const refreshDevices = useCallback(async () => {
    const version = ++deviceRevision.current;
    setLoading(true);
    setDeviceError("");
    setCaptureError("");
    const results = await Promise.allSettled([listPlaybackDevices(), listCaptureDevices()]);
    if (!mounted.current || version !== deviceRevision.current) return;
    if (results[0].status === "fulfilled") setDevices(results[0].value);
    else {
      setDevices([]);
      setDeviceError(String(results[0].reason));
    }
    if (results[1].status === "fulfilled") setCaptureDevices(results[1].value);
    else {
      setCaptureDevices([]);
      setCaptureError(String(results[1].reason));
    }
    setLoading(false);
  }, []);
  useEffect(() => {
    let active = true;
    getAppInfo()
      .then((value) => {
        if (!active) return;
        setInfo(value);
        setAppReady(true);
        if (value.platform === "windows") void refreshDevices();
      })
      .catch((error) => {
        if (active) setAppError(String(error));
      });
    return () => {
      active = false;
    };
  }, [refreshDevices]);
  useEffect(() => {
    try {
      setSaved(savePreferences(localStorage, prefs));
    } catch {
      setSaved(false);
    }
  }, [prefs]);
  useEffect(() => {
    const query = matchMedia("(prefers-color-scheme: dark)");
    const changed = () => setSystemDark(query.matches);
    const routeChanged = () => setPage(location.hash === "#settings" ? "settings" : "workspace");
    query.addEventListener("change", changed);
    addEventListener("hashchange", routeChanged);
    return () => {
      query.removeEventListener("change", changed);
      removeEventListener("hashchange", routeChanged);
    };
  }, []);
  const theme = info.platform === "android" ? "system" : prefs.theme;
  const dark = theme === "dark" || (theme === "system" && systemDark);
  useEffect(() => {
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    document.documentElement.style.colorScheme = dark ? "dark" : "light";
    document
      .querySelector('meta[name="theme-color"]')
      ?.setAttribute("content", dark ? "#0f141c" : "#f5f7fb");
    void setDesktopTheme(theme, info.platform).catch(() => {});
  }, [dark, theme, info.platform]);
  const navigate = (next: "workspace" | "settings") => {
    location.hash = next;
  };
  return {
    prefs,
    update,
    info,
    ...sessions,
    link,
    audio,
    autostart,
    desktop,
    role,
    roleBusy: roleBusy || desktopSession.snapshot?.busy === true,
    roleReady,
    roleError: desktopSession.error || roleError,
    switchRole,
    switchMode,
    stopLink,
    retryDiscovery,
    disconnectPeer,
    micOutputDeviceId,
    setMicOutputDeviceId,
    devices,
    captureDevices,
    loading,
    deviceError,
    captureError,
    appError,
    saved,
    page,
    navigate,
    refreshDevices,
    dark,
  };
}
export type Workbench = ReturnType<typeof useWorkbench>;
