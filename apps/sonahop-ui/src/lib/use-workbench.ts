import { useCallback, useEffect, useState } from "react";
import { getAppInfo, listPlaybackDevices, platformHint, setDesktopTheme } from "./bridge";
import type { AppInfo, PlaybackDevice } from "./bridge";
import { PREFERENCES_KEY, readPreferences, savePreferences } from "./preferences";
import type { Preferences } from "./preferences";

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
    version: "0.1.0",
    uiPreview: true,
    browserPreview: false,
  });
  const [devices, setDevices] = useState<PlaybackDevice[]>([]);
  const [loading, setLoading] = useState(false);
  const [deviceError, setDeviceError] = useState("");
  const [appError, setAppError] = useState("");
  const [saved, setSaved] = useState(true);
  const [systemDark, setSystemDark] = useState(
    () => matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const [page, setPage] = useState(() =>
    location.hash === "#settings" ? "settings" : "workspace",
  );

  const update = useCallback(
    (value: Partial<Preferences>) => setPrefs((previous) => ({ ...previous, ...value })),
    [],
  );
  const refreshDevices = useCallback(async () => {
    setLoading(true);
    setDeviceError("");
    try {
      setDevices(await listPlaybackDevices());
    } catch (error) {
      setDevices([]);
      setDeviceError(String(error));
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    let active = true;
    getAppInfo()
      .then((value) => {
        if (!active) return;
        setInfo(value);
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
    query.addEventListener("change", changed);
    const routeChanged = () => setPage(location.hash === "#settings" ? "settings" : "workspace");
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
      ?.setAttribute("content", dark ? "#171b19" : "#f7f6f3");
    void setDesktopTheme(theme, info.platform).catch(() => {});
  }, [dark, theme, info.platform]);
  const source =
    devices.find((device) => device.id === prefs.sourceId) ??
    devices.find((device) => device.isDefault) ??
    devices[0];
  const navigate = (next: "workspace" | "settings") => {
    location.hash = next;
  };
  return {
    prefs,
    update,
    info,
    devices,
    source,
    loading,
    deviceError,
    appError,
    saved,
    page,
    navigate,
    refreshDevices,
    dark,
  };
}
export type Workbench = ReturnType<typeof useWorkbench>;
