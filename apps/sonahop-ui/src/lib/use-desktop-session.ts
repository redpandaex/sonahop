import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { Platform } from "./bridge";
import { nativeLinkAvailable } from "./link-bridge";
import type { ConnectionRole } from "./preferences";

export interface DesktopSession {
  revision: number;
  action: "startSharing" | "stopSharing" | "connectDevices" | "disconnect" | null;
  busy: boolean;
  role: ConnectionRole | null;
  error: string | null;
}

export function useDesktopSession(platform: Platform, appReady: boolean) {
  const available = platform === "windows" && appReady && nativeLinkAvailable();
  const [snapshot, setSnapshot] = useState<DesktopSession | null>(null);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!available) return;
    let active = true;
    let unlisten: (() => void) | undefined;
    const accept = (value: DesktopSession) => {
      if (!active) return;
      setSnapshot((previous) => {
        if (
          previous &&
          (previous.revision > value.revision ||
            (previous.revision === value.revision && !previous.busy && value.busy))
        )
          return previous;
        return value;
      });
      setReady(true);
      setError("");
    };
    void (async () => {
      try {
        const stop = await listen<DesktopSession>("sonahop://desktop-session", ({ payload }) =>
          accept(payload),
        );
        if (!active) {
          stop();
          return;
        }
        unlisten = stop;
        accept(await invoke<DesktopSession>("get_desktop_session"));
      } catch (failure) {
        if (active) {
          setError(String(failure));
          setReady(true);
        }
      }
    })();
    return () => {
      active = false;
      unlisten?.();
    };
  }, [available]);
  return { snapshot, ready: !available || ready, error };
}
