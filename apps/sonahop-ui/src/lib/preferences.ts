export type AudioMode = "listen" | "microphone";
export type ConnectionRole = "host" | "client";
export type SendSource = "system" | "microphone";
export type ThemePreference = "light" | "dark" | "system";
export interface Preferences {
  version: 1;
  mode: AudioMode;
  volume: number;
  theme: ThemePreference;
  sourceId: string;
  connectionRole: ConnectionRole | null;
  sendSource: SendSource;
  inputDeviceId: string;
  outputDeviceId: string;
  microphoneOutputDeviceId: string;
  microphoneRecordingDeviceId: string;
  playbackDestination: "speakers" | "microphone";
}
export const PREFERENCES_KEY = "sonahop.preferences.v1";
export const DEFAULT_PREFERENCES: Readonly<Preferences> = Object.freeze({
  version: 1,
  mode: "listen",
  volume: 70,
  theme: "light",
  sourceId: "",
  connectionRole: null,
  sendSource: "system",
  inputDeviceId: "",
  outputDeviceId: "",
  microphoneOutputDeviceId: "",
  microphoneRecordingDeviceId: "",
  playbackDestination: "speakers",
});
function text(value: unknown, fallback: string, maxLength: number): string {
  return typeof value === "string" ? value.trim().slice(0, maxLength) : fallback;
}
export function readPreferences(raw: string | null): Preferences {
  const fallback = { ...DEFAULT_PREFERENCES };
  if (!raw) return fallback;
  try {
    const value: unknown = JSON.parse(raw);
    if (!value || typeof value !== "object" || Array.isArray(value)) return fallback;
    const data = value as Record<string, unknown>;
    if (data.version !== 1) return fallback;
    return {
      version: 1,
      mode: data.mode === "microphone" ? "microphone" : "listen",
      theme: data.theme === "dark" || data.theme === "system" ? data.theme : "light",
      volume:
        typeof data.volume === "number" && Number.isFinite(data.volume)
          ? Math.round(Math.min(100, Math.max(0, data.volume)))
          : 70,
      sourceId: text(data.sourceId, "", 512),
      connectionRole:
        data.connectionRole === "host" || data.connectionRole === "client"
          ? data.connectionRole
          : null,
      sendSource: data.sendSource === "microphone" ? "microphone" : "system",
      inputDeviceId: text(data.inputDeviceId, "", 512),
      outputDeviceId: text(data.outputDeviceId, "", 512),
      microphoneOutputDeviceId: text(data.microphoneOutputDeviceId, "", 512),
      microphoneRecordingDeviceId: text(data.microphoneRecordingDeviceId, "", 512),
      playbackDestination: data.playbackDestination === "microphone" ? "microphone" : "speakers",
    };
  } catch {
    return fallback;
  }
}
export function savePreferences(storage: Pick<Storage, "setItem">, value: Preferences): boolean {
  try {
    storage.setItem(PREFERENCES_KEY, JSON.stringify(value));
    return true;
  } catch {
    return false;
  }
}
