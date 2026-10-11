import { invoke, isTauri } from "@tauri-apps/api/core";
export type Platform = "windows" | "android" | "other";
export interface AppInfo { platform: Platform; version: string; uiPreview: boolean; browserPreview: boolean }
export interface PlaybackDevice { id: string; name: string; isDefault: boolean }
export function platformHint(): Platform {
  if (isTauri()) return navigator.userAgent.includes("Android") ? "android" : "windows";
  const choice = new URLSearchParams(location.search).get("platform");
  return choice === "android" || choice === "windows" ? choice : innerWidth < 900 ? "android" : "windows";
}
export async function getAppInfo(): Promise<AppInfo> {
  if (!isTauri()) return { platform: platformHint(), version: "0.1.0", uiPreview: true, browserPreview: true };
  const value = await invoke<Omit<AppInfo, "browserPreview">>("get_app_info");
  return { ...value, browserPreview: false };
}
export async function listPlaybackDevices(): Promise<PlaybackDevice[]> {
  if (!isTauri()) return [
    { id: "preview-default", name: "默认扬声器（示例）", isDefault: true },
    { id: "preview-headphones", name: "耳机输出（示例）", isDefault: false },
  ];
  return invoke<PlaybackDevice[]>("list_playback_devices");
}
export async function setDesktopTheme(theme: "light" | "dark" | "system", platform: Platform): Promise<void> {
  if (!isTauri() || platform !== "windows") return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().setTheme(theme === "system" ? null : theme);
}
