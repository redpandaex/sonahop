import { Badge } from "@/components/ui/badge";
import {
  WaveformIcon,
  SpeakerHighIcon,
  MicrophoneIcon,
  ArrowRightIcon,
  MonitorIcon,
  DeviceTabletIcon,
  WifiHighIcon,
  WarningCircleIcon,
} from "@phosphor-icons/react";
import type { AudioMode } from "../lib/preferences";
import type { Workbench } from "../lib/use-workbench";

export function Brand({ small = false }: { small?: boolean }) {
  return (
    <div className={small ? "brand brand-small" : "brand"}>
      <span className="brand-mark">
        <WaveformIcon weight="bold" size={23} aria-hidden />
      </span>
      <span className="brand-word">
        SonaHop<span className="brand-caption">跨设备音频</span>
      </span>
    </div>
  );
}
export function PreviewNotice({ browser }: { browser: boolean }) {
  return (
    <div className="preview-notice" id="preview-notice">
      <span className="preview-label">界面预览</span>
      <span>
        {browser ? "浏览器中的音源为示例。" : "可选择音源和调整设置。"}{" "}
        当前尚不支持设备搜索、配对或音频传输。
      </span>
    </div>
  );
}
export function ModeSwitch({
  mode,
  onChange,
  vertical = false,
}: {
  mode: AudioMode;
  onChange: (mode: AudioMode) => void;
  vertical?: boolean;
}) {
  return (
    <fieldset className={vertical ? "mode-switch mode-vertical" : "mode-switch"}>
      <legend className="sr-only">工作模式</legend>
      <label className={mode === "listen" ? "mode-choice selected" : "mode-choice"}>
        <input
          type="radio"
          name="audio-mode"
          checked={mode === "listen"}
          onChange={() => onChange("listen")}
        />
        <SpeakerHighIcon size={20} weight="bold" aria-hidden />
        <span>收听电脑声音</span>
      </label>
      <label className={mode === "microphone" ? "mode-choice selected" : "mode-choice"}>
        <input
          type="radio"
          name="audio-mode"
          checked={mode === "microphone"}
          onChange={() => onChange("microphone")}
        />
        <MicrophoneIcon size={20} weight="bold" aria-hidden />
        <span>手机作为麦克风</span>
      </label>
    </fieldset>
  );
}
export function StatusBadge({ label = "未共享" }: { label?: string }) {
  return (
    <Badge variant="outline" className="status-badge">
      <span className="status-dot" />
      {label}
    </Badge>
  );
}
export function RouteDiagram({ mode, mobile = false }: { mode: AudioMode; mobile?: boolean }) {
  const firstIsPc = mode === "listen";
  return (
    <div className={mobile ? "audio-route mobile-route" : "audio-route"}>
      <div className="route-end">
        <span className="route-icon">
          {firstIsPc ? (
            <MonitorIcon size={29} weight="bold" aria-hidden />
          ) : (
            <MicrophoneIcon size={29} weight="bold" aria-hidden />
          )}
        </span>
        <span>{firstIsPc ? "电脑声音" : "设备麦克风"}</span>
      </div>
      <div className="route-middle">
        <div className="route-line">
          <WifiHighIcon size={17} weight="bold" aria-hidden />
          <ArrowRightIcon size={16} weight="bold" aria-hidden />
        </div>
        <span>本地网络</span>
      </div>
      <div className="route-end">
        <span className="route-icon">
          {firstIsPc ? (
            <DeviceTabletIcon size={29} weight="bold" aria-hidden />
          ) : (
            <MonitorIcon size={29} weight="bold" aria-hidden />
          )}
        </span>
        <span>{firstIsPc ? "手机 / 平板" : "电脑应用"}</span>
      </div>
    </div>
  );
}
export function VolumeControl({
  workbench,
  microphone = false,
}: {
  workbench: Workbench;
  microphone?: boolean;
}) {
  const { prefs, update } = workbench;
  return (
    <section className="volume-control" aria-labelledby="volume-title">
      <div className="control-title">
        <div className="inline-label">
          {microphone ? (
            <MicrophoneIcon size={20} weight="bold" aria-hidden />
          ) : (
            <SpeakerHighIcon size={20} weight="bold" aria-hidden />
          )}
          <h3 id="volume-title">{microphone ? "麦克风增益偏好" : "本设备播放音量"}</h3>
        </div>
        <output htmlFor="device-volume" className="volume-number">
          {prefs.volume}
          <span>%</span>
        </output>
      </div>
      <input
        id="device-volume"
        className="volume-range"
        type="range"
        min="0"
        max="100"
        step="1"
        aria-label={microphone ? "麦克风增益偏好" : "本设备播放音量"}
        value={prefs.volume}
        onChange={(event) => update({ volume: event.currentTarget.valueAsNumber })}
      />
      <div className="range-labels">
        <span>静音</span>
        <span>100%</span>
      </div>
      <p className="control-help">
        {microphone
          ? "保存增益偏好；本版不会启动麦克风。"
          : "独立控制接收音量。本版保存设置，音频接入后生效。"}
      </p>
    </section>
  );
}
export function ErrorMessage({ message }: { message: string }) {
  return (
    <div className="error-message" role="alert">
      <WarningCircleIcon weight="bold" size={19} aria-hidden />
      <span>{message}</span>
    </div>
  );
}
