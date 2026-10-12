import { invoke } from "@tauri-apps/api/core";
import type { Platform } from "./bridge";
import type { AudioMode } from "./preferences";

export interface AudioSnapshot {
  role: "host" | "client";
  mode: "idle" | "listen" | "microphone";
  active: boolean;
  waiting: boolean;
  micPeerId: string | null;
  outputDeviceId: string | null;
  pendingMicRequests: Array<{ peerId: string; name: string }>;
  sentPackets: number;
  receivedPackets: number;
  droppedPackets: number;
  gain: number;
  lastError: string | null;
}
export const getAudioSnapshot = () => invoke<AudioSnapshot>("get_audio_snapshot");
export const startListening = () => invoke<AudioSnapshot>("start_listening");
export const requestMicrophone = () => invoke<AudioSnapshot>("request_microphone");
export const answerMicrophoneRequest = (
  peerId: string,
  approve: boolean,
  outputDeviceId: string | null,
) => invoke<AudioSnapshot>("answer_microphone_request", { peerId, approve, outputDeviceId });
export const stopAudio = () => invoke<AudioSnapshot>("stop_audio");
export const setAudioGain = (gain: number) => invoke<AudioSnapshot>("set_audio_gain", { gain });

export async function prepareAudio(platform: Platform, mode: AudioMode): Promise<void> {
  if (platform !== "android") return;
  const result = await invoke<{ granted: boolean; deviceName: string }>(
    "plugin:sonahop-lan|prepare_audio",
    { mode },
  );
  if (!result.granted) {
    throw new Error(
      mode === "microphone"
        ? "麦克风或前台音频服务未获授权，请允许所需权限后再申请。当前不会采集声音。"
        : "音频播放未获授权，请允许所需权限后再开始收听。",
    );
  }
}
