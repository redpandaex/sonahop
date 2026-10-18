import {
  MicrophoneIcon,
  SpeakerHighIcon,
  WarningCircleIcon,
  WaveformIcon,
} from "@phosphor-icons/react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { StatusTone } from "../lib/status-presentation";
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
  if (!browser) return null;
  return (
    <div className="preview-notice" id="preview-notice">
      <span className="preview-label">预览</span>
      <span>浏览器仅供布局与偏好预览，连接和音频需使用原生应用。</span>
    </div>
  );
}
export function StatusBadge({
  label = "未共享",
  tone = "neutral",
}: {
  label?: string;
  tone?: StatusTone;
}) {
  return (
    <Badge variant="outline" className="status-badge" data-tone={tone} role="status">
      <span className="status-dot" aria-hidden="true" />
      {label}
    </Badge>
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
  if (!microphone) return null;
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
            ? workbench.role === "host"
              ? "调整本设备麦克风增益；选择麦克风后启用。"
              : "调整本设备麦克风增益；申请获得共享设备批准后启动。"
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
