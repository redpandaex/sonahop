import {
  ArrowClockwiseIcon,
  ArrowRightIcon,
  MicrophoneIcon,
  SpeakerHighIcon,
} from "@phosphor-icons/react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { PlaybackDevice } from "../lib/bridge";
import type { Workbench } from "../lib/use-workbench";
import { virtualAudioRoutes, virtualMicrophoneReady } from "../lib/virtual-audio";
import { ErrorMessage, StatusBadge, VolumeControl } from "./controls";
import { VirtualAudioGuide } from "./virtual-audio-guide";

export function audioStatusLabel(w: Workbench, lane?: "listen" | "microphone"): string {
  if (!w.audio.available) return w.info.browserPreview ? "浏览器布局预览" : "音频服务未就绪";
  const value = w.audio.snapshot;
  if (!value) return "正在读取音频状态";
  const status = lane ? value[lane] : null;
  if (status) {
    if (status.active)
      return lane === "listen"
        ? value.role === "host"
          ? value.sendSource === "microphone"
            ? "正在共享麦克风"
            : "正在共享内部声音"
          : "正在收听"
        : value.role === "host"
          ? "接收远端麦克风"
          : "正在上传麦克风";
    if (status.waiting)
      return lane === "listen"
        ? value.role === "host"
          ? "等待收听设备收听"
          : "等待共享设备开始共享"
        : value.role === "host"
          ? "等待远端麦克风"
          : "等待共享设备批准";
    return "音频已停止";
  }
  if (value.listen.active && value.microphone.active)
    return value.role === "host" ? "共享中 · 接收远端麦克风" : "收听中 · 上传麦克风";
  if (value.listen.active || value.listen.waiting) return audioStatusLabel(w, "listen");
  if (value.microphone.active || value.microphone.waiting) return audioStatusLabel(w, "microphone");
  return "音频已停止";
}
export function AudioDiagnostics({
  workbench: w,
  lane = "listen",
}: {
  workbench: Workbench;
  lane?: "listen" | "microphone";
}) {
  const snapshot = w.audio.snapshot?.[lane];
  if (!snapshot || w.info.browserPreview) return null;
  const peak = Math.max(0, Math.min(1, snapshot.peakLevel ?? 0));
  const level = peak > 0 ? Math.round(Math.max(0, (Math.log10(peak) * 20 + 60) / 60) * 100) : 0;
  const label =
    lane === "microphone"
      ? w.role === "host"
        ? "远端麦克风信号"
        : "本机麦克风信号"
      : w.role === "client"
        ? "接收音频信号"
        : w.audio.snapshot?.sendSource === "microphone"
          ? "本机麦克风信号"
          : "内部声音信号";
  return (
    <div className="audio-telemetry">
      <div className="microphone-signal">
        <div className="control-title">
          <span>{label}</span>
          <span>
            {snapshot.active
              ? peak > 0.001
                ? "检测到声音"
                : "暂未检测到声音"
              : snapshot.waiting
                ? "等待音频"
                : "未启动"}
          </span>
        </div>
        <meter className="sr-only" aria-label={label} min={0} max={100} value={level} />
        <div className="signal-track" aria-hidden="true">
          <div className="signal-fill" style={{ width: `${level}%` }} />
        </div>
      </div>
      <section className="audio-diagnostics" aria-label="音频链路统计">
        <span>
          发送 <b className="mono">{snapshot.sentPackets}</b>
        </span>
        <span>
          接收 <b className="mono">{snapshot.receivedPackets}</b>
        </span>
        <span>
          丢弃 <b className="mono">{snapshot.droppedPackets}</b>
        </span>
      </section>
    </div>
  );
}

function DeviceChoice({
  id,
  label,
  value,
  devices,
  defaultLabel,
  disabled,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  devices: PlaybackDevice[];
  defaultLabel: string;
  disabled: boolean;
  onChange: (value: string) => void;
}) {
  const defaultValue = "__sonahop_default_device__";
  return (
    <div className="audio-device-field">
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <Select
        value={value || defaultValue}
        disabled={disabled}
        onValueChange={(selected) => onChange(selected === defaultValue ? "" : selected)}
      >
        <SelectTrigger
          id={id}
          className="device-select-trigger"
          title={devices.find((device) => device.id === value)?.name}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent position="popper" align="start" className="device-select-content">
          <SelectGroup>
            <SelectItem className="device-select-item" value={defaultValue}>
              {defaultLabel}
            </SelectItem>
            {value && !devices.some((device) => device.id === value) && (
              <SelectItem className="device-select-item" value={value} disabled>
                已保存设备当前不可用
              </SelectItem>
            )}
            {devices.map((device) => (
              <SelectItem className="device-select-item" key={device.id} value={device.id}>
                {device.name}
                {device.isDefault ? " · 默认" : ""}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </div>
  );
}
function InputDevices({ workbench: w, disabled }: { workbench: Workbench; disabled: boolean }) {
  if (w.info.platform !== "windows") return null;
  return (
    <>
      <div className="control-title">
        <h3 className="field-label">本机录音设备</h3>
        <Button
          className="icon-button"
          disabled={w.loading}
          aria-label="刷新录音设备"
          onClick={() => void w.refreshDevices()}
        >
          <ArrowClockwiseIcon size={18} weight="bold" className={w.loading ? "refreshing" : ""} />
        </Button>
      </div>
      <DeviceChoice
        id="capture-input"
        label="麦克风入口"
        value={w.prefs.inputDeviceId}
        devices={w.captureDevices}
        defaultLabel={
          w.captureDevices.length
            ? "当前默认录音设备"
            : w.loading
              ? "正在读取录音设备…"
              : "没有可用录音设备"
        }
        disabled={disabled || w.loading || !w.captureDevices.length}
        onChange={(inputDeviceId) => w.update({ inputDeviceId })}
      />
      {!w.captureDevices.length && !w.loading && (
        <p className="field-help capability-note">
          没有可用录音设备，麦克风采集已禁用。连接或启用麦克风后刷新。
        </p>
      )}
      {w.prefs.inputDeviceId &&
        !w.captureDevices.some((device) => device.id === w.prefs.inputDeviceId) && (
          <p className="field-help capability-note">已保存的录音设备不在当前列表中，请重新选择。</p>
        )}
      {w.captureError && <ErrorMessage message={w.captureError} />}
    </>
  );
}
function inputAvailable(w: Workbench): boolean {
  return (
    w.info.platform !== "windows" ||
    (w.captureDevices.length > 0 &&
      (!w.prefs.inputDeviceId ||
        w.captureDevices.some((device) => device.id === w.prefs.inputDeviceId)))
  );
}
function busy(w: Workbench): boolean {
  return (
    !w.audio.available ||
    !w.audio.snapshot ||
    !w.roleReady ||
    w.roleBusy ||
    w.audio.busy ||
    w.link.busy
  );
}

export function ServerAudioPanel({ workbench: w }: { workbench: Workbench }) {
  const lane = w.audio.snapshot?.listen;
  const started = Boolean(lane?.active || lane?.waiting);
  const source = started
    ? (w.audio.snapshot?.sendSource ?? w.prefs.sendSource)
    : w.prefs.sendSource;
  const microphone = source === "microphone";
  const unsupported = !microphone && !w.info.systemCaptureSupported;
  const disabled = busy(w);
  return (
    <section className="panel source-panel">
      <div className="panel-heading">
        <h2>音源</h2>
        <StatusBadge label={audioStatusLabel(w, "listen")} />
      </div>
      <label className="sr-only" htmlFor="send-source">
        音频来源
      </label>
      <Select
        value={source}
        disabled={disabled || started}
        onValueChange={(sendSource) => {
          if (sendSource === "system" || sendSource === "microphone") w.update({ sendSource });
        }}
      >
        <SelectTrigger id="send-source" className="device-select-trigger">
          <SelectValue />
        </SelectTrigger>
        <SelectContent position="popper" align="start" className="device-select-content">
          <SelectGroup>
            <SelectItem
              className="device-select-item"
              value="system"
              disabled={!w.info.systemCaptureSupported}
            >
              {w.info.platform === "android" ? "应用声音" : "系统声音"}
              {!w.info.systemCaptureSupported ? " · 不可用" : ""}
            </SelectItem>
            <SelectItem
              className="device-select-item"
              value="microphone"
              disabled={w.info.platform === "windows" && !w.captureDevices.length}
            >
              本机麦克风
              {w.info.platform === "windows" && !w.captureDevices.length ? " · 无录音设备" : ""}
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
      {microphone ? (
        <>
          <InputDevices workbench={w} disabled={disabled || started} />
          <VolumeControl workbench={w} microphone />
        </>
      ) : unsupported ? (
        <p className="field-help capability-note">当前设备不支持系统声音采集。</p>
      ) : null}
      <Button
        className={started ? "secondary-button" : "primary-button"}
        disabled={
          disabled ||
          (!started &&
            (!w.link.snapshot?.running || unsupported || (microphone && !inputAvailable(w))))
        }
        onClick={() =>
          void (started
            ? w.audio.stopLane("listen")
            : w.audio.startListen({
                source,
                inputDeviceId: w.prefs.inputDeviceId || undefined,
                gain: w.prefs.volume,
              }))
        }
      >
        {w.audio.busy ? "处理中…" : started ? "停止共享" : "开始共享"}
        <ArrowRightIcon size={18} weight="bold" />
      </Button>
      {w.audio.snapshot?.listen.lastError && (
        <ErrorMessage message={w.audio.snapshot.listen.lastError} />
      )}
      <AudioDiagnostics workbench={w} lane="listen" />
    </section>
  );
}

export function PlayerAudioPanel({ workbench: w }: { workbench: Workbench }) {
  const microphoneAllowed = w.audio.snapshot?.remoteMicrophoneSupported !== false;
  const microphone =
    w.prefs.mode === "microphone" &&
    (microphoneAllowed ||
      Boolean(w.audio.snapshot?.microphone.active || w.audio.snapshot?.microphone.waiting));
  const laneName = microphone ? "microphone" : "listen";
  const lane = w.audio.snapshot?.[laneName];
  const started = Boolean(lane?.active || lane?.waiting);
  const disabled = busy(w);
  const virtualMicrophone =
    w.info.platform === "windows" && w.prefs.playbackDestination === "microphone";
  const outputDeviceId = virtualMicrophone ? w.micOutputDeviceId : w.prefs.outputDeviceId;
  const validOutput = virtualMicrophone
    ? virtualMicrophoneReady(
        w.devices,
        w.captureDevices,
        outputDeviceId,
        w.prefs.microphoneRecordingDeviceId,
      )
    : !outputDeviceId || w.devices.some((device) => device.id === outputDeviceId);
  return (
    <section className="panel playback-panel player-audio-panel">
      <div className="panel-heading">
        <h2>{microphone ? "上传本机麦克风" : virtualMicrophone ? "作为电脑麦克风" : "本地播放"}</h2>
        <StatusBadge label={audioStatusLabel(w, laneName)} />
      </div>
      {microphoneAllowed && (
        <section className="player-direction-tabs" aria-label="收听设备音频方向">
          <Button
            className={!microphone ? "direction-tab selected" : "direction-tab"}
            aria-pressed={!microphone}
            disabled={disabled}
            onClick={() => void w.switchMode("listen")}
          >
            <SpeakerHighIcon size={18} weight="bold" />
            收听
          </Button>
          <Button
            className={microphone ? "direction-tab selected" : "direction-tab"}
            aria-pressed={microphone}
            disabled={disabled}
            onClick={() => void w.switchMode("microphone")}
          >
            <MicrophoneIcon size={18} weight="bold" />
            上传麦克风
          </Button>
        </section>
      )}
      <div className="playback-identity">
        <span className="playback-icon">
          {microphone || virtualMicrophone ? (
            <MicrophoneIcon size={37} weight="bold" />
          ) : (
            <SpeakerHighIcon size={37} weight="bold" />
          )}
        </span>
        <div>
          <h3>
            {microphone
              ? "本机麦克风"
              : virtualMicrophone
                ? "供电脑应用使用"
                : w.info.platform === "windows"
                  ? "本机音频输出"
                  : "本设备扬声器 / 耳机"}
          </h3>
          {!microphone && started && w.audio.snapshot?.remoteSource && (
            <p>{w.audio.snapshot.remoteSource === "microphone" ? "麦克风声音" : "应用声音"}</p>
          )}
        </div>
      </div>
      {microphone ? (
        <>
          <InputDevices workbench={w} disabled={disabled || started} />
          <VolumeControl workbench={w} microphone />
        </>
      ) : (
        <>
          {w.info.platform === "windows" && (
            <>
              <section
                className="player-direction-tabs playback-destination"
                aria-label="接收音频用途"
              >
                <Button
                  className={!virtualMicrophone ? "direction-tab selected" : "direction-tab"}
                  aria-pressed={!virtualMicrophone}
                  disabled={disabled || started}
                  onClick={() => w.update({ playbackDestination: "speakers" })}
                >
                  <SpeakerHighIcon size={18} weight="bold" />
                  扬声器 / 耳机
                </Button>
                <Button
                  className={virtualMicrophone ? "direction-tab selected" : "direction-tab"}
                  aria-pressed={virtualMicrophone}
                  disabled={disabled || started}
                  onClick={() => w.update({ playbackDestination: "microphone" })}
                >
                  <MicrophoneIcon size={18} weight="bold" />
                  电脑麦克风
                </Button>
              </section>
              {virtualMicrophone ? (
                <VirtualOutputSelector workbench={w} />
              ) : (
                <DeviceChoice
                  id="player-output"
                  label="音频输出设备"
                  value={w.prefs.outputDeviceId}
                  devices={w.devices}
                  defaultLabel="当前默认播放设备"
                  disabled={disabled || started || w.loading}
                  onChange={(outputDeviceId) => w.update({ outputDeviceId })}
                />
              )}
              {!validOutput && !virtualMicrophone && (
                <p className="field-help capability-note">
                  已保存的播放设备不在当前列表中，请重新选择。
                </p>
              )}
              {w.deviceError && !virtualMicrophone && <ErrorMessage message={w.deviceError} />}
            </>
          )}
          <VolumeControl workbench={w} />
        </>
      )}
      <div className="mobile-primary">
        <Button
          className={started ? "secondary-button" : "primary-button"}
          disabled={
            disabled ||
            (!started &&
              (!w.link.snapshot?.peers.length ||
                (microphone ? !inputAvailable(w) : w.info.platform === "windows" && !validOutput)))
          }
          onClick={() =>
            void (started
              ? w.audio.stopLane(laneName)
              : microphone
                ? w.audio.requestMic(w.prefs.volume, w.prefs.inputDeviceId || undefined)
                : w.audio.startListen({ outputDeviceId: outputDeviceId || undefined }))
          }
        >
          {w.audio.busy
            ? "处理中…"
            : started
              ? lane?.waiting
                ? "取消等待"
                : microphone
                  ? "停止上传"
                  : "停止收听"
              : microphone
                ? "申请上传麦克风"
                : virtualMicrophone
                  ? "开始接收"
                  : "开始收听"}
          <ArrowRightIcon size={18} weight="bold" />
        </Button>
        {lane?.waiting && <p>{microphone ? "等待对方批准" : "等待对方开始共享"}</p>}
      </div>
      {microphoneAllowed &&
        (microphone
          ? w.audio.snapshot?.listen.active || w.audio.snapshot?.listen.waiting
          : w.audio.snapshot?.microphone.active || w.audio.snapshot?.microphone.waiting) && (
          <div className="parallel-audio-status">
            <span>{microphone ? "收听仍在运行" : "麦克风仍在运行"}</span>
            <Button
              className="secondary-button"
              disabled={disabled}
              onClick={() => void w.audio.stopLane(microphone ? "listen" : "microphone")}
            >
              {microphone ? "停止收听" : "停止麦克风"}
            </Button>
          </div>
        )}
      {lane?.lastError && <ErrorMessage message={lane.lastError} />}
      <AudioDiagnostics workbench={w} lane={laneName} />
    </section>
  );
}
export function VirtualOutputSelector({ workbench: w }: { workbench: Workbench }) {
  const [showOtherDevices, setShowOtherDevices] = useState(false);
  const lane = w.role === "host" ? w.audio.snapshot?.microphone : w.audio.snapshot?.listen;
  const active = Boolean(lane?.active || lane?.waiting);
  const value = active
    ? (w.audio.snapshot?.outputDeviceId ?? w.micOutputDeviceId)
    : w.micOutputDeviceId;
  const routes = virtualAudioRoutes(w.devices, w.captureDevices);
  const knownIds = new Set(routes.map((route) => route.playback.id));
  const otherDevices = showOtherDevices
    ? w.devices.filter((device) => !knownIds.has(device.id))
    : [];
  const choices = [...routes.map((route) => route.playback), ...otherDevices];
  const selected = w.devices.find((device) => device.id === value);
  const disabled = w.loading || busy(w) || active || w.info.browserPreview;
  return (
    <div className="virtual-device-field">
      <div className="control-title">
        <label className="field-label" htmlFor="microphone-output">
          电脑麦克风入口
        </label>
        <Button
          className="icon-button"
          aria-label="刷新虚拟音频设备"
          disabled={w.loading}
          onClick={() => void w.refreshDevices()}
        >
          <ArrowClockwiseIcon size={18} weight="bold" className={w.loading ? "refreshing" : ""} />
        </Button>
      </div>
      <Select
        value={value || "__sonahop_no_output__"}
        disabled={disabled}
        onValueChange={(chosen) =>
          w.setMicOutputDeviceId(chosen === "__sonahop_no_output__" ? "" : chosen)
        }
      >
        <SelectTrigger
          id="microphone-output"
          className="device-select-trigger"
          title={selected?.name}
        >
          <SelectValue placeholder="选择虚拟音频线播放端" />
        </SelectTrigger>
        <SelectContent position="popper" align="start" className="device-select-content">
          <SelectGroup>
            <SelectItem className="device-select-item" value="__sonahop_no_output__">
              选择虚拟音频线播放端
            </SelectItem>
            {value && !choices.some((device) => device.id === value) && (
              <SelectItem className="device-select-item" value={value} disabled={!selected}>
                {selected ? `${selected.name} · 手动选择` : "所选设备当前不可用"}
              </SelectItem>
            )}
            {routes.map((route) => (
              <SelectItem
                className="device-select-item"
                key={route.playback.id}
                value={route.playback.id}
                disabled={!route.recording && !showOtherDevices}
              >
                {route.playback.name}
                {route.status === "ambiguous"
                  ? " · 需手动配对"
                  : !route.recording
                    ? " · 需选择录音端"
                    : ""}
                {route.status === "ambiguous" ? ` · ${route.playback.id.slice(-9, -1)}` : ""}
              </SelectItem>
            ))}
            {otherDevices.map((device) => (
              <SelectItem className="device-select-item" key={device.id} value={device.id}>
                {device.name} · 手动配置
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      <Button
        className="secondary-button virtual-audio-advanced"
        disabled={disabled}
        aria-expanded={showOtherDevices}
        onClick={() => setShowOtherDevices((shown) => !shown)}
      >
        {showOtherDevices ? "收起其他设备" : "使用其他虚拟设备"}
      </Button>
      {(showOtherDevices || w.prefs.microphoneRecordingDeviceId) && (
        <DeviceChoice
          id="microphone-recording"
          label="录音端（手动配置）"
          value={w.prefs.microphoneRecordingDeviceId}
          devices={w.captureDevices.map((device) => ({
            ...device,
            name: `${device.name} · ${device.id.slice(-9, -1)}`,
          }))}
          defaultLabel="自动识别对应录音端"
          disabled={disabled}
          onChange={(microphoneRecordingDeviceId) => w.update({ microphoneRecordingDeviceId })}
        />
      )}
      <VirtualAudioGuide workbench={w} deviceId={value} />
      {w.deviceError && <ErrorMessage message={w.deviceError} />}
      {w.captureError && <ErrorMessage message={w.captureError} />}
    </div>
  );
}

export function RemoteMicrophonePanel({ workbench: w }: { workbench: Workbench }) {
  if (w.info.platform !== "windows" || w.role !== "host") return null;
  const snapshot = w.audio.snapshot;
  const requests = snapshot?.pendingMicRequests ?? [];
  const active = Boolean(snapshot?.microphone.active || snapshot?.microphone.waiting);
  const validOutput = virtualMicrophoneReady(
    w.devices,
    w.captureDevices,
    w.micOutputDeviceId,
    w.prefs.microphoneRecordingDeviceId,
  );
  const disabled = busy(w);
  const name = w.link.snapshot?.peers.find((peer) => peer.id === snapshot?.micPeerId)?.name;
  return (
    <section className="panel microphone-requests remote-microphone-panel">
      <div className="panel-heading">
        <h2>接收远端麦克风</h2>
        <StatusBadge label={audioStatusLabel(w, "microphone")} />
      </div>
      <VirtualOutputSelector workbench={w} />
      {requests.map((request) => (
        <div className="lan-device" key={request.requestId}>
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
              onClick={() => void w.audio.answerMic(request.peerId, request.requestId, false, null)}
            >
              拒绝
            </Button>
            <Button
              className="primary-button"
              disabled={disabled || !validOutput}
              onClick={() =>
                void w.audio.answerMic(request.peerId, request.requestId, true, w.micOutputDeviceId)
              }
            >
              接受并启用
            </Button>
          </div>
        </div>
      ))}
      <p className="field-help">
        {active
          ? `${name ?? "已选择的设备"}正在提供麦克风。`
          : requests.length
            ? "选择虚拟入口后再批准申请。"
            : "等待收听设备主动申请上传麦克风。"}
      </p>
      <Button
        className="secondary-button"
        disabled={disabled || !active}
        onClick={() => void w.audio.stopLane("microphone")}
      >
        停止远端麦克风
      </Button>
      {snapshot?.microphone.lastError && <ErrorMessage message={snapshot.microphone.lastError} />}
      <AudioDiagnostics workbench={w} lane="microphone" />
    </section>
  );
}
