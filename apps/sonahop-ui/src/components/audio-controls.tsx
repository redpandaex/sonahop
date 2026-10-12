import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  ArrowRightIcon,
  ArrowClockwiseIcon,
  MicrophoneIcon,
  SpeakerHighIcon,
} from "@phosphor-icons/react";
import { ErrorMessage, StatusBadge, VolumeControl } from "./controls";
import type { Workbench } from "../lib/use-workbench";
import type { PlaybackDevice } from "../lib/bridge";

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
          ? "等待播放器收听"
          : "等待服务器开始共享"
        : value.role === "host"
          ? "等待远端麦克风"
          : "等待服务器批准";
    return "音频已停止";
  }
  if (value.listen.active && value.microphone.active) return "共享中 · 接收远端麦克风";
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
        <div
          className="signal-track"
          role="meter"
          aria-label={label}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={level}
        >
          <div className="signal-fill" style={{ width: `${level}%` }} />
        </div>
      </div>
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
  if (w.info.platform !== "windows")
    return <p className="field-help">使用这台设备的麦克风。只在你主动开始并授权后采集。</p>;
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
        <h2>共享音源</h2>
        <StatusBadge label={audioStatusLabel(w, "listen")} />
      </div>
      <div className="source-identity">
        <span className="device-symbol">
          {microphone ? (
            <MicrophoneIcon size={28} weight="bold" />
          ) : (
            <SpeakerHighIcon size={28} weight="bold" />
          )}
        </span>
        <div>
          <h3>{microphone ? "本机麦克风" : "本机内部声音"}</h3>
          <p>一台服务器，最多两个播放器。</p>
        </div>
      </div>
      <label className="field-label" htmlFor="send-source">
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
              内部播放声音{!w.info.systemCaptureSupported ? " · 当前不可用" : ""}
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
      {started && <p className="field-help">共享期间音源已锁定，停止共享后可以更换。</p>}
      {microphone ? (
        <>
          <InputDevices workbench={w} disabled={disabled || started} />
          <VolumeControl workbench={w} microphone />
        </>
      ) : (
        <p className="field-help">
          {unsupported
            ? w.info.browserPreview
              ? "浏览器无法读取内部声音采集能力。"
              : "当前平台暂不支持内部声音采集，请选择本机麦克风。"
            : w.info.platform === "android"
              ? "开始共享时需要系统音频捕获授权；部分应用可能不允许被采集。"
              : "采集系统进程音频，独立于本地扬声器音量，不改变系统输出设备。"}
        </p>
      )}
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
      {!w.link.snapshot?.running && (
        <p className="field-help">先在连接区开启服务器，再主动开始共享。</p>
      )}
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
  const validOutput =
    !w.prefs.outputDeviceId || w.devices.some((device) => device.id === w.prefs.outputDeviceId);
  return (
    <section className="panel playback-panel player-audio-panel">
      <div className="panel-heading">
        <h2>{microphone ? "上传本机麦克风" : "本地播放"}</h2>
        <StatusBadge label={audioStatusLabel(w, laneName)} />
      </div>
      {microphoneAllowed && (
        <div className="player-direction-tabs" aria-label="播放器音频方向">
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
        </div>
      )}
      <div className="playback-identity">
        <span className="playback-icon">
          {microphone ? (
            <MicrophoneIcon size={37} weight="bold" />
          ) : (
            <SpeakerHighIcon size={37} weight="bold" />
          )}
        </span>
        <div>
          <h3>{microphone ? "本机麦克风" : "本设备扬声器 / 耳机"}</h3>
          <p>
            {microphone
              ? "服务器批准后才开始采集。"
              : w.audio.snapshot?.remoteSource === "microphone"
                ? "正在接收服务器的麦克风音源。"
                : w.audio.snapshot?.remoteSource === "system"
                  ? "正在接收服务器的内部声音音源。"
                  : "连接并开始收听后显示远端音源。"}
          </p>
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
              <DeviceChoice
                id="player-output"
                label="本应用播放出口"
                value={w.prefs.outputDeviceId}
                devices={w.devices}
                defaultLabel="当前默认播放设备"
                disabled={disabled || started || w.loading}
                onChange={(outputDeviceId) => w.update({ outputDeviceId })}
              />
              <p className="field-help">
                只选择 SonaHop 的播放出口，不更改 Windows 默认设备。停止收听后可以更换。
              </p>
              {!validOutput && (
                <p className="field-help capability-note">
                  已保存的播放设备不在当前列表中，请重新选择。
                </p>
              )}
              {w.deviceError && <ErrorMessage message={w.deviceError} />}
            </>
          )}
          <VolumeControl workbench={w} />
        </>
      )}
      {!microphoneAllowed && (
        <p className="field-help">当前服务器不接收远端麦克风，此设备可以收听它共享的音源。</p>
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
                : w.audio.startListen({ outputDeviceId: w.prefs.outputDeviceId || undefined }))
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
                : "开始收听"}
          <ArrowRightIcon size={18} weight="bold" />
        </Button>
        <p>
          {lane?.waiting
            ? microphone
              ? "请在服务器上批准申请；等待期间不采集麦克风。"
              : "等待服务器主动开始共享。"
            : microphone
              ? "这台播放器一次只运行一个音频方向，上传会停止本机收听。"
              : "可收听 Windows 或 Android 服务器，播放音量由本设备系统控制。"}
        </p>
      </div>
      {lane?.lastError && <ErrorMessage message={lane.lastError} />}
      <AudioDiagnostics workbench={w} lane={laneName} />
    </section>
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
        disabled={
          w.loading ||
          w.audio.busy ||
          w.roleBusy ||
          !w.roleReady ||
          Boolean(activeMic) ||
          w.info.browserPreview
        }
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

export function RemoteMicrophonePanel({ workbench: w }: { workbench: Workbench }) {
  if (w.info.platform !== "windows" || w.role !== "host") return null;
  const snapshot = w.audio.snapshot;
  const requests = snapshot?.pendingMicRequests ?? [];
  const active = Boolean(snapshot?.microphone.active || snapshot?.microphone.waiting);
  const validOutput = w.devices.some((device) => device.id === w.micOutputDeviceId);
  const disabled = busy(w);
  const name = w.link.snapshot?.peers.find((peer) => peer.id === snapshot?.micPeerId)?.name;
  return (
    <section className="panel microphone-requests remote-microphone-panel">
      <div className="panel-heading">
        <h2>接收远端麦克风</h2>
        <StatusBadge label={audioStatusLabel(w, "microphone")} />
      </div>
      <p className="field-help">
        Windows 服务器可以接收一台播放器上传的麦克风，同时继续共享本机音源。此入口供会议等应用使用。
      </p>
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
            : "等待播放器主动申请上传麦克风。"}
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
