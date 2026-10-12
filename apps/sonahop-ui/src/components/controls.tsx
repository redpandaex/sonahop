import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  WaveformIcon,
  SpeakerHighIcon,
  MicrophoneIcon,
  ArrowRightIcon,
  ArrowUpRightIcon,
  ArrowDownLeftIcon,
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
      <span className="preview-label">首版</span>
      <span>
        {browser
          ? "浏览器仅供布局与偏好预览，连接与音频需使用原生应用。"
          : "同一局域网内连接电脑与移动设备，再主动开始音频。"}{" "}
        控制连接成功不代表正在收听，麦克风需电脑另行批准。
      </span>
    </div>
  );
}
export function ModeSwitch({
  mode,
  onChange,
  vertical = false,
  disabled = false,
  senderMode = "listen",
}: {
  mode: AudioMode;
  onChange: (mode: AudioMode) => void;
  vertical?: boolean;
  disabled?: boolean;
  senderMode?: AudioMode;
}) {
  const receiverMode: AudioMode = senderMode === "listen" ? "microphone" : "listen";
  const choices = [
    { value: senderMode, label: "发送音频", icon: ArrowUpRightIcon },
    { value: receiverMode, label: "接收音频", icon: ArrowDownLeftIcon },
  ];
  return (
    <fieldset
      disabled={disabled}
      className={vertical ? "mode-switch mode-vertical" : "mode-switch"}
    >
      <legend className="sr-only">音频方向</legend>
      {choices.map(({ value, label, icon: Icon }) => (
        <label key={value} className={mode === value ? "mode-choice selected" : "mode-choice"}>
          <input
            type="radio"
            name="audio-mode"
            checked={mode === value}
            onChange={() => onChange(value)}
          />
          <Icon size={20} weight="bold" aria-hidden />
          <span>{label}</span>
        </label>
      ))}
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
  if (!microphone)
    return (
      <section className="system-volume-note" aria-label="系统媒体音量">
        <SpeakerHighIcon size={23} weight="bold" aria-hidden />
        <div>
          <h3>使用设备音量键调节</h3>
          <p>播放音量由手机或平板的系统媒体音量控制，SonaHop 不再额外衰减声音。</p>
        </div>
      </section>
    );
  return (
    <section className="volume-control" aria-labelledby="volume-title">
      <div className="control-title">
        <div className="inline-label">
          {microphone ? (
            <MicrophoneIcon size={20} weight="bold" aria-hidden />
          ) : (
            <SpeakerHighIcon size={20} weight="bold" aria-hidden />
          )}
          <h3 id="volume-title">{microphone ? "麦克风增益" : "本设备播放音量"}</h3>
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
        aria-label={microphone ? "麦克风增益" : "本设备播放音量"}
        value={prefs.volume}
        disabled={workbench.audio.busy}
        onChange={(event) => update({ volume: event.currentTarget.valueAsNumber })}
        onPointerUp={(event) => {
          if (workbench.audio.available)
            void workbench.audio.setGain(event.currentTarget.valueAsNumber);
        }}
        onKeyUp={(event) => {
          if (workbench.audio.available)
            void workbench.audio.setGain(event.currentTarget.valueAsNumber);
        }}
      />
      <div className="range-labels">
        <span>静音</span>
        <span>100%</span>
      </div>
      <p className="control-help">
        {workbench.info.browserPreview
          ? "浏览器只保存音量偏好。"
          : microphone
            ? "调整本设备麦克风增益；录音只在申请获得批准后启动。"
            : "调整这台设备的接收音量，不改变电脑扬声器或另一台设备的音量。"}
        {workbench.audio.snapshot && !workbench.info.browserPreview && (
          <> 当前生效：{workbench.audio.snapshot.gain}%。</>
        )}
      </p>
      {workbench.audio.available && (
        <Button
          className="secondary-button volume-apply"
          disabled={workbench.audio.busy || !workbench.audio.snapshot}
          onClick={() => void workbench.audio.setGain(prefs.volume)}
        >
          应用音量
        </Button>
      )}
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
