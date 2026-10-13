import type { PlaybackDevice } from "./bridge";

export interface VirtualAudioRoute {
  playback: PlaybackDevice;
  product: "cable" | "voicemeeter";
  recording: PlaybackDevice | null;
  recordingLabel: string;
  status: "ready" | "missing" | "ambiguous";
}
function identity(name: string) {
  const cable = /^(CABLE(?:[- ]?[A-D])?)\s+Input\b/i.exec(name.trim());
  if (cable) return { product: "cable" as const, stem: cable[1], bus: 0 };
  const mixer = /^Voicemeeter(?:\s+(AUX|VAIO3))?\s+Input\b/i.exec(name.trim());
  if (!mixer) return null;
  return {
    product: "voicemeeter" as const,
    stem: `Voicemeeter${mixer[1] ? ` ${mixer[1]}` : ""}`,
    bus: mixer[1]?.toUpperCase() === "AUX" ? 2 : mixer[1] ? 3 : 1,
  };
}
export function virtualAudioRoute(
  playback: PlaybackDevice,
  captureDevices: PlaybackDevice[],
  playbackDevices: PlaybackDevice[] = [playback],
): VirtualAudioRoute | null {
  const route = identity(playback.name);
  if (!route) return null;
  const key = route.stem.toLowerCase().replace(/[- ]/g, "");
  const playbackCount = playbackDevices.filter(
    (device) => identity(device.name)?.stem.toLowerCase().replace(/[- ]/g, "") === key,
  ).length;
  const expectedName = `${route.stem} Output`;
  const candidates = captureDevices.filter((device) => {
    const name = device.name.trim();
    const oldName = name.slice(0, expectedName.length);
    const suffix = name.slice(expectedName.length);
    const oldMatch =
      oldName.toLowerCase() === expectedName.toLowerCase() &&
      (suffix === "" || /^[\s(]/.test(suffix));
    const busName = `Voicemeeter Out B${route.bus}`;
    const busSuffix = name.slice(busName.length);
    return (
      oldMatch ||
      (route.product === "voicemeeter" &&
        name.slice(0, busName.length).toLowerCase() === busName.toLowerCase() &&
        (busSuffix === "" || /^[\s(]/.test(busSuffix)))
    );
  });
  const ambiguous = playbackCount > 1 || candidates.length > 1;
  const recording = !ambiguous && candidates.length === 1 ? candidates[0] : null;
  return {
    playback,
    product: route.product,
    recording,
    recordingLabel: recording?.name ?? expectedName,
    status: ambiguous ? "ambiguous" : recording ? "ready" : "missing",
  };
}
export function virtualAudioRoutes(
  devices: PlaybackDevice[],
  captureDevices: PlaybackDevice[],
): VirtualAudioRoute[] {
  return devices.flatMap((device) => {
    const route = virtualAudioRoute(device, captureDevices, devices);
    return route ? [route] : [];
  });
}
export function virtualMicrophoneReady(
  devices: PlaybackDevice[],
  captureDevices: PlaybackDevice[],
  id: string,
  recordingId = "",
): boolean {
  const device = devices.find((item) => item.id === id);
  if (!device) return false;
  // Manual pairing is an explicit user configuration, not inferred routing.
  if (recordingId) return captureDevices.some((item) => item.id === recordingId);
  return virtualAudioRoute(device, captureDevices, devices)?.status === "ready";
}
