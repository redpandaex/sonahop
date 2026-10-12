import { useCallback, useEffect, useRef, useState } from "react";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";
import type { Platform } from "./bridge";
import { nativeLinkAvailable } from "./link-bridge";

export function useAutostart(platform: Platform, appReady: boolean) {
  const available = platform === "windows" && appReady && nativeLinkAvailable();
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const mounted = useRef(false);
  const writing = useRef(false);
  useEffect(() => {
    mounted.current = true;
    let active = true;
    if (available) {
      setEnabled(null);
      void isEnabled()
        .then((value) => {
          if (active) {
            setEnabled(value);
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
    async (value: boolean) => {
      if (!available || enabled === null || writing.current) return;
      writing.current = true;
      setBusy(true);
      setError("");
      try {
        if (value) await enable();
        else await disable();
        const actual = await isEnabled();
        if (mounted.current) setEnabled(actual);
        if (actual !== value) throw new Error("Windows 启动项没有应用成功，请重试。");
      } catch (failure) {
        if (mounted.current) setError(String(failure));
      } finally {
        writing.current = false;
        if (mounted.current) setBusy(false);
      }
    },
    [available, enabled],
  );
  return { available, enabled, busy, error, change };
}
