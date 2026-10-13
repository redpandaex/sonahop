import { useState } from "react";
import { Button } from "@/components/ui/button";
import { openVirtualAudioWebsite } from "../lib/bridge";
import type { Workbench } from "../lib/use-workbench";
import { virtualAudioRoute, virtualAudioRoutes } from "../lib/virtual-audio";
import { ErrorMessage } from "./controls";

export function VirtualAudioGuide({
  workbench: w,
  deviceId,
}: {
  workbench: Workbench;
  deviceId: string;
}) {
  const [error, setError] = useState("");
  const device = w.devices.find((item) => item.id === deviceId);
  const route = device ? virtualAudioRoute(device, w.captureDevices, w.devices) : null;
  const manual = w.captureDevices.find((item) => item.id === w.prefs.microphoneRecordingDeviceId);
  const manualRequested = Boolean(w.prefs.microphoneRecordingDeviceId);
  const detected = virtualAudioRoutes(w.devices, w.captureDevices);
  const incomplete = !device && detected.length > 0 && detected.every((item) => !item.recording);
  const ambiguous = !device && detected.some((item) => item.status === "ambiguous");
  const openWebsite = async () => {
    setError("");
    try {
      await openVirtualAudioWebsite(route?.product ?? "cable");
    } catch (cause) {
      setError(String(cause));
    }
  };
  return (
    <div className="virtual-audio-guide">
      {manualRequested ? (
        <p className="field-help virtual-audio-recording">
          {manual ? (
            <>
              手动录音端：<strong>{manual.name}</strong>。请确认与播放端属于同一条虚拟音频线。
            </>
          ) : (
            "手动选择的录音端当前不可用，请重新选择。"
          )}
        </p>
      ) : route?.recording ? (
        <p className="field-help virtual-audio-recording">
          录音软件选择：<strong>{route.recording.name}</strong>
        </p>
      ) : route ? (
        <p className="field-help capability-note">
          {w.captureError
            ? "无法读取录音端，请刷新设备。"
            : route.status === "ambiguous"
              ? "发现多个同名端点，请使用其他虚拟设备并手动选择录音端。"
              : `${route.recordingLabel} 当前不可用，请在 Windows 录音设备中启用后刷新。`}
        </p>
      ) : device ? (
        <p className="field-help">
          请在“使用其他虚拟设备”中选择对应录音端，确认属于同一条虚拟音频线。
        </p>
      ) : (
        <p className="field-help">
          {ambiguous
            ? "存在同名端点，请使用其他虚拟设备并手动选择录音端。"
            : incomplete
              ? "已发现虚拟播放端，请启用对应录音端后刷新。"
              : "安装 VB-CABLE 后刷新，选择 CABLE Input。"}
        </p>
      )}
      {route?.product === "voicemeeter" && (
        <p className="field-help">需要运行 Voicemeeter，并启用这个输入对应的 B 总线。</p>
      )}
      <details className="virtual-audio-help">
        <summary>电脑麦克风使用方法</summary>
        <ol>
          <li>按需从厂商官网下载并安装虚拟音频线，完成后刷新设备。</li>
          <li>SonaHop 选择播放端 CABLE Input，并启动手机麦克风音频。</li>
          <li>微信、会议或录音软件选择录音端 CABLE Output。</li>
        </ol>
        <p className="field-help">
          VB-CABLE 由 VB-Audio 提供，采用捐赠授权；使用和授权以厂商说明为准。
        </p>
        <Button className="secondary-button" onClick={() => void openWebsite()}>
          {route?.product === "voicemeeter" ? "Voicemeeter 官网" : "VB-CABLE 官网"}
        </Button>
      </details>
      {error && <ErrorMessage message={error} />}
    </div>
  );
}
