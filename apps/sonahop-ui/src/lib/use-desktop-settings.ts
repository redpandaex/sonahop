import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import type { Platform } from "./bridge";
import { nativeLinkAvailable } from "./link-bridge";

export type CloseBehavior = "tray" | "exit";
interface DesktopSettings {
  closeBehavior: CloseBehavior;
  trayAvailable: boolean;
}

export function useDesktopSettings(platform: Platform, appReady: boolean) {
  const available = platform === "windows" && appReady && nativeLinkAvailable();
  const [settings, setSettings] = useState<DesktopSettings | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const mounted = useRef(false);
  const writing = useRef(false);

  useEffect(() => {
    mounted.current = true;
    let active = true;
    if (available) {
      setSettings(null);
      void invoke<DesktopSettings>("get_desktop_settings")
        .then((value) => {
          if (active) {
            setSettings(value);
            setError("");
          }
        })
        .catch((failure) => {
          if (active) setError(String(failure));
        });
    }
    return () => {
      active = false;
      mounted.current = false;
    };
  }, [available]);

  const change = useCallback(
    async (closeBehavior: CloseBehavior) => {
      if (!available || !settings || writing.current) return;
      writing.current = true;
      setBusy(true);
      setError("");
      try {
        const value = await invoke<DesktopSettings>("set_close_behavior", { closeBehavior });
        if (mounted.current) setSettings(value);
      } catch (failure) {
        if (mounted.current) setError(String(failure));
      } finally {
        writing.current = false;
        if (mounted.current) setBusy(false);
      }
    },
    [available, settings],
  );

  return { available, settings, busy, error, change };
}
