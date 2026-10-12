import { Button } from "@/components/ui/button";
import {
  ArrowRightIcon,
  CaretDownIcon,
  ArrowClockwiseIcon,
  MicrophoneIcon,
  WifiHighIcon,
} from "@phosphor-icons/react";
import { ErrorMessage, StatusBadge } from "./controls";
import type { Workbench } from "../lib/use-workbench";

export function audioStatusLabel(w: Workbench): string {
  if (!w.audio.available) return w.info.browserPreview ? "浏览器布局预览" : "音频服务未就绪";
  const value = w.audio.snapshot;
  if (!value) return "正在读取音频状态";
  if (value.active) {
    if (value.mode === "microphone")
      return value.role === "host" ? "接收手机麦克风" : "麦克风传输中";
    if (value.mode === "listen") return value.role === "host" ? "正在共享系统声音" : "正在收听";
  }
  if (value.waiting) {
    if (value.role === "host")
      return value.mode === "microphone" ? "等待手机麦克风" : "等待设备开始收听";
    return value.mode === "microphone" ? "等待电脑批准麦克风" : "等待电脑开启共享";
  }
  return "音频已停止";
}

export function AudioDiagnostics({ workbench: w }: { workbench: Workbench }) {
  const snapshot = w.audio.snapshot;
  if (!snapshot || w.info.browserPreview) return null;
  return (
    <div className="audio-diagnostics" aria-label="音频链路统计">
      <span>
        发送 <b className="mono">{snapshot.sentPackets}</b>
      </span>
      <span>
        接收 <b className="mono">{snapshot.receivedPackets}</b>
      </span>
      <span>
        丢弃 <b className="mono">{snapshot.droppedPackets}</b>
      </span>
    </div>
  );
}

export function VirtualOutputSelector({ workbench: w }: { workbench: Workbench }) {
  const activeMic = w.audio.snapshot?.active && w.audio.snapshot.mode === "microphone";
  const value = activeMic
    ? (w.audio.snapshot?.outputDeviceId ?? w.micOutputDeviceId)
    : w.micOutputDeviceId;
  return (
    <div className="virtual-device-field">
      <div className="control-title">
        <label className="field-label" htmlFor="microphone-output">
          虚拟音频线播放入口
        </label>
        <Button
          className="icon-button"
          aria-label="刷新虚拟音频入口列表"
          title="刷新播放设备"
          disabled={w.loading}
          onClick={() => void w.refreshDevices()}
        >
          <ArrowClockwiseIcon size={18} weight="bold" className={w.loading ? "refreshing" : ""} />
        </Button>
      </div>
      <div className="select-wrap">
        <select
          id="microphone-output"
          value={value}
          disabled={w.loading || w.audio.busy || Boolean(activeMic) || w.info.browserPreview}
          onChange={(event) => w.setMicOutputDeviceId(event.currentTarget.value)}
        >
          <option value="">请明确选择已安装的虚拟音频线入口</option>
          {value && !w.devices.some((device) => device.id === value) && (
            <option value={value} disabled>
              所选设备当前不可用
            </option>
          )}
          {w.devices.map((device) => (
            <option key={device.id} value={device.id}>
              {device.name}
              {device.isDefault ? " · 系统默认" : ""}
            </option>
          ))}
        </select>
        <CaretDownIcon size={16} weight="bold" aria-hidden />
      </div>
      <p className="field-help">
        需要先安装虚拟音频线。选择它的播放端（如 CABLE
        Input），电脑会议应用再选择对应录音端；不要选择扬声器或耳机。
      </p>
      {activeMic && (
        <p className="field-help">当前麦克风正在使用后台确认的入口；停止后可以重新选择。</p>
      )}
      {w.deviceError && <ErrorMessage message={w.deviceError} />}
    </div>
  );
}

export function MicrophoneRequests({ workbench: w }: { workbench: Workbench }) {
  const requests = w.audio.snapshot?.pendingMicRequests ?? [];
  if (!requests.length) return null;
  const validOutput = w.devices.some((device) => device.id === w.micOutputDeviceId);
  const disabled = !w.audio.available || !w.audio.snapshot || w.audio.busy || w.link.busy;
  return (
    <section className="panel microphone-requests">
      <div className="panel-heading">
        <h2>手机麦克风申请</h2>
        <MicrophoneIcon size={20} weight="bold" />
      </div>
      <p className="lan-note">
        只允许一台设备提供麦克风。接受申请会停止电脑声音共享和其它设备的收听。
      </p>
      <VirtualOutputSelector workbench={w} />
      {requests.map((request) => (
        <div className="lan-device" key={request.peerId}>
          <div className="lan-device-identity">
            <MicrophoneIcon size={23} weight="bold" />
            <div>
              <h3>{request.name}</h3>
              <p>
                {w.link.snapshot?.peers.find((peer) => peer.id === request.peerId)?.address ??
                  "已连接设备"}
              </p>
            </div>
          </div>
          <div className="lan-actions">
            <Button
              className="secondary-button"
              disabled={disabled}
              onClick={() => void w.audio.answerMic(request.peerId, false, null)}
            >
              拒绝
            </Button>
            <Button
              className="primary-button"
              disabled={disabled || !validOutput}
              onClick={() => void w.audio.answerMic(request.peerId, true, w.micOutputDeviceId)}
            >
              接受并启用
            </Button>
          </div>
        </div>
      ))}
    </section>
  );
}

export function HostAudioAction({ workbench: w }: { workbench: Workbench }) {
  const snapshot = w.audio.snapshot;
  const started = Boolean(snapshot?.active || snapshot?.waiting);
  const microphone = w.prefs.mode === "microphone";
  const disabled =
    !w.audio.available ||
    !snapshot ||
    w.audio.busy ||
    w.link.busy ||
    (!started && (microphone || !w.link.snapshot?.peers.length));
  return (
    <>
      <MicrophoneRequests workbench={w} />
      {w.audio.error && <ErrorMessage message={w.audio.error} />}
      <div className="share-bar">
        <div>
          <span className="share-status-icon">
            <WifiHighIcon size={22} weight="bold" />
          </span>
          <div>
            <h3>{audioStatusLabel(w)}</h3>
            <p>
              {microphone
                ? "由手机主动申请，在此选择虚拟音频线并批准。"
                : "采集系统声音，独立于电脑扬声器音量；最多两台设备收听。"}
            </p>
          </div>
        </div>
        <Button
          className={started ? "secondary-button" : "primary-button"}
          disabled={disabled}
          onClick={() => void (started ? w.audio.stop() : w.audio.startListen(w.prefs.volume))}
        >
          {w.audio.busy
            ? "处理中…"
            : started
              ? "停止音频"
              : microphone
                ? "等待手机申请"
                : "开始共享"}
          <ArrowRightIcon size={18} weight="bold" />
        </Button>
      </div>
      <AudioDiagnostics workbench={w} />
    </>
  );
}

export function MobileAudioAction({ workbench: w }: { workbench: Workbench }) {
  const value = w.audio.snapshot;
  const started = Boolean(value?.active || value?.waiting);
  const microphone = w.prefs.mode === "microphone";
  const disabled =
    !w.audio.available ||
    !value ||
    w.audio.busy ||
    w.link.busy ||
    (!started && !w.link.snapshot?.peers.length);
  return (
    <div className="mobile-primary">
      <StatusBadge label={audioStatusLabel(w)} />
      {w.audio.error && <ErrorMessage message={w.audio.error} />}
      <Button
        className={started ? "secondary-button" : "primary-button"}
        disabled={disabled}
        onClick={() =>
          void (started
            ? w.audio.stop()
            : microphone
              ? w.audio.requestMic(w.prefs.volume)
              : w.audio.startListen(w.prefs.volume))
        }
      >
        {w.audio.busy
          ? "处理中…"
          : started
            ? value?.waiting
              ? "取消等待"
              : microphone
                ? "停止麦克风"
                : "停止收听"
            : microphone
              ? "申请使用麦克风"
              : "开始收听"}
        <ArrowRightIcon size={18} weight="bold" />
      </Button>
      <p>
        {value?.waiting
          ? microphone
            ? "在 Windows 上选择虚拟音频线入口并接受申请，批准前不会采集声音。"
            : "请在 Windows 上点击开始共享，收到音频后会开始播放。"
          : microphone
            ? "只在你主动申请并获电脑批准后采集声音；启用后会暂停所有收听。"
            : "此设备音量与电脑本地扬声器音量独立。"}
      </p>
      <AudioDiagnostics workbench={w} />
    </div>
  );
}
