import { invoke } from "@tauri-apps/api/core";
import type { Platform } from "./bridge";
import type { AudioMode, SendSource } from "./preferences";

export interface LaneSnapshot {
  active: boolean;
  waiting: boolean;
  lastError: string | null;
  sentPackets: number;
  receivedPackets: number;
  droppedPackets: number;
  peakLevel: number;
}
export interface AudioSnapshot {
  stopRevision: number;
  sendSource: SendSource;
  remoteSource: SendSource | null;
  remoteMicrophoneSupported: boolean | null;
  listen: LaneSnapshot;
  microphone: LaneSnapshot;
  role: "host" | "client";
  mode: "idle" | "listen" | "microphone" | "duplex";
  active: boolean;
  waiting: boolean;
  micPeerId: string | null;
  outputDeviceId: string | null;
  pendingMicRequests: Array<{ peerId: string; name: string; requestId: string }>;
  sentPackets: number;
  receivedPackets: number;
  droppedPackets: number;
  gain: number;
  lastError: string | null;
}
export const getAudioSnapshot = () => invoke<AudioSnapshot>("get_audio_snapshot");
export interface ListenOptions {
  source?: SendSource;
  inputDeviceId?: string;
  outputDeviceId?: string;
  gain?: number;
}
export const startListening = (
  { source, inputDeviceId, outputDeviceId }: ListenOptions = {},
  stopRevision = 0,
) =>
  invoke<AudioSnapshot>("start_listening", { source, inputDeviceId, outputDeviceId, stopRevision });
export const requestMicrophone = (inputDeviceId?: string, stopRevision = 0) =>
  invoke<AudioSnapshot>("request_microphone", { inputDeviceId, stopRevision });
export const answerMicrophoneRequest = (
  peerId: string,
  requestId: string,
  approve: boolean,
  outputDeviceId: string | null,
  stopRevision = 0,
) =>
  invoke<AudioSnapshot>("answer_microphone_request", {
    peerId,
    requestId,
    approve,
    outputDeviceId,
    stopRevision,
  });
export const stopAudio = (stopRevision = 0) =>
  invoke<AudioSnapshot>("stop_audio", { stopRevision });
export const stopAudioLane = (lane: AudioMode, stopRevision = 0) =>
  invoke<AudioSnapshot>("stop_audio_lane", { lane, stopRevision });
export const setAudioGain = (gain: number, stopRevision = 0) =>
  invoke<AudioSnapshot>("set_audio_gain", { gain, stopRevision });

export async function prepareAudio(
  platform: Platform,
  mode: AudioMode | "playback",
): Promise<void> {
  if (platform !== "android") return;
  const result = await invoke<{ granted: boolean; deviceName: string }>(
    "plugin:sonahop-lan|prepare_audio",
    { mode },
  );
  if (!result.granted) {
    throw new Error(
      mode === "microphone"
        ? "麦克风或前台音频服务未获授权，请允许所需权限后再申请。当前不会采集声音。"
        : mode === "playback"
          ? "内部声音采集未获授权，请在系统提示中允许共享音频后重试。"
          : "音频播放未获授权，请允许所需权限后再开始收听。",
    );
  }
}
