import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ArrowRightIcon, ArrowClockwiseIcon, MicrophoneIcon } from "@phosphor-icons/react";
import { ErrorMessage, StatusBadge } from "./controls";
import type { Workbench } from "../lib/use-workbench";

export function audioStatusLabel(w: Workbench): string {
  if (!w.audio.available) return w.info.browserPreview ? "浏览器布局预览" : "音频服务未就绪";
  const value = w.audio.snapshot;
  if (!value) return "正在读取音频状态";
  if (value.role === "host" && value.listen.active && value.microphone.active)
    return "共享声音 · 接收手机麦克风";
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

export function AudioDiagnostics({
  workbench: w,
  lane,
}: {
  workbench: Workbench;
  lane?: "listen" | "microphone";
}) {
  const snapshot = lane ? w.audio.snapshot?.[lane] : w.audio.snapshot;
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
  const activeMic = w.audio.snapshot?.microphone.active;
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
      <Select
        value={value || "__sonahop_no_output__"}
        disabled={w.loading || w.audio.busy || Boolean(activeMic) || w.info.browserPreview}
        onValueChange={(selected) =>
          w.setMicOutputDeviceId(selected === "__sonahop_no_output__" ? "" : selected)
        }
      >
        <SelectTrigger
          id="microphone-output"
          className="device-select-trigger"
          aria-label="虚拟音频线播放入口"
          title={w.devices.find((device) => device.id === value)?.name}
        >
          <SelectValue placeholder="请选择虚拟音频线播放入口" />
        </SelectTrigger>
        <SelectContent position="popper" align="start" className="device-select-content">
          <SelectGroup>
            <SelectItem className="device-select-item" value="__sonahop_no_output__">
              请选择虚拟音频线播放入口
            </SelectItem>
            {value && !w.devices.some((device) => device.id === value) && (
              <SelectItem className="device-select-item" value={value} disabled>
                所选设备当前不可用
              </SelectItem>
            )}
            {w.devices.map((device) => (
              <SelectItem className="device-select-item" key={device.id} value={device.id}>
                {device.name}
                {device.isDefault ? " · 系统默认" : ""}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
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
        只允许一台设备提供麦克风。接受申请时，其它手机或平板可以继续收听电脑声音。
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
  const listening = Boolean(snapshot?.listen.active || snapshot?.listen.waiting);
  const microphone = Boolean(snapshot?.microphone.active || snapshot?.microphone.waiting);
  const busy = !w.audio.available || !snapshot || w.audio.busy || w.link.busy;
  const micName = w.link.snapshot?.peers.find((peer) => peer.id === snapshot?.micPeerId)?.name;
  return (
    <>
      <MicrophoneRequests workbench={w} />
      {w.audio.error && <ErrorMessage message={w.audio.error} />}
      <div className="audio-direction-actions">
        <section className="panel audio-direction-control" aria-label="电脑声音共享">
          <div className="panel-heading">
            <h2>共享电脑声音</h2>
            <StatusBadge label={listening ? "共享中" : "已停止"} />
          </div>
          <p className="field-help">
            最多两台设备收听，接收设备使用系统音量键。停止共享不会停止手机麦克风。
          </p>
          <Button
            className={listening ? "secondary-button" : "primary-button"}
            disabled={busy || (!listening && !w.link.snapshot?.peers.length)}
            onClick={() => void (listening ? w.audio.stopLane("listen") : w.audio.startListen())}
          >
            {w.audio.busy ? "处理中…" : listening ? "停止共享" : "开始共享"}
            <ArrowRightIcon size={18} weight="bold" />
          </Button>
          <AudioDiagnostics workbench={w} lane="listen" />
        </section>
        <section className="panel audio-direction-control" aria-label="手机麦克风接收">
          <div className="panel-heading">
            <h2>接收手机麦克风</h2>
            <StatusBadge label={microphone ? "接收中" : "已停止"} />
          </div>
          <p className="field-help">
            {microphone
              ? `${micName ?? "已选择的手机"}正在提供麦克风；平板可以同时收听。`
              : "由手机主动申请，在电脑选择虚拟音频线入口并批准。"}
          </p>
          <Button
            className="secondary-button"
            disabled={busy || !microphone}
            onClick={() => void w.audio.stopLane("microphone")}
          >
            {w.audio.busy ? "处理中…" : microphone ? "停止麦克风" : "等待手机申请"}
          </Button>
          <AudioDiagnostics workbench={w} lane="microphone" />
        </section>
      </div>
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
              : w.audio.startListen())
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
            ? "只在你主动申请并获电脑批准后采集声音；其它设备可以继续收听。"
            : "使用手机或平板的音量键调节，不改变电脑扬声器音量。"}
      </p>
      <AudioDiagnostics workbench={w} />
    </div>
  );
}
