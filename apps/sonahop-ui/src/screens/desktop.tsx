import { Button } from "@/components/ui/button";
import {
  SpeakerHighIcon,
  MicrophoneIcon,
  GearSixIcon,
  MonitorIcon,
  DeviceMobileIcon,
  DeviceTabletIcon,
  ArrowClockwiseIcon,
  CaretDownIcon,
  ArrowRightIcon,
  ShieldCheckIcon,
  CheckCircleIcon,
  WifiHighIcon,
} from "@phosphor-icons/react";
import {
  Brand,
  ModeSwitch,
  PreviewNotice,
  RouteDiagram,
  StatusBadge,
  ErrorMessage,
} from "../components/controls";
import type { Workbench } from "../lib/use-workbench";
import { SettingsScreen } from "./settings";

function EmptyDevices({ microphone }: { microphone: boolean }) {
  return (
    <section className="panel peer-panel">
      <div className="panel-heading">
        <h2>{microphone ? "麦克风来源" : "接收设备"}</h2>
        <span className="subtle-count">0 / {microphone ? "1" : "2"}</span>
      </div>
      <div className="empty-peers">
        <div className="device-illustration">
          <DeviceMobileIcon size={52} weight="bold" />
          <DeviceTabletIcon size={64} weight="bold" />
        </div>
        <h3>设备发现尚未接入</h3>
        <p>
          {microphone
            ? "连接后，选择一台手机或平板作为电脑麦克风。"
            : "手机和平板连接后会显示在这里，最多可同时收听两台。"}
        </p>
      </div>
      <div className="peer-slot">
        <DeviceMobileIcon size={22} weight="bold" aria-hidden />
        <span>手机 / 平板</span>
        <span className="slot-state">未连接</span>
      </div>
      {!microphone && (
        <div className="peer-slot">
          <DeviceTabletIcon size={22} weight="bold" aria-hidden />
          <span>第二台设备</span>
          <span className="slot-state">未连接</span>
        </div>
      )}
      <div className="small-note">
        <ShieldCheckIcon size={17} weight="bold" aria-hidden />
        <span>连接与配对功能随后接入</span>
      </div>
    </section>
  );
}

export function DesktopScreen({ workbench: w }: { workbench: Workbench }) {
  const microphone = w.prefs.mode === "microphone";
  return (
    <div className="desktop-app">
      <aside className="sidebar">
        <Brand />
        <div className="sidebar-section-title">工作模式</div>
        <ModeSwitch
          vertical
          mode={w.prefs.mode}
          onChange={(mode) => {
            w.update({ mode });
            w.navigate("workspace");
          }}
        />
        <div className="sidebar-divider" />
        <a
          className={w.page === "settings" ? "sidebar-link active" : "sidebar-link"}
          href="#settings"
          aria-current={w.page === "settings" ? "page" : undefined}
        >
          <GearSixIcon size={21} weight="bold" />
          设置
        </a>
        <div className="sidebar-bottom">
          <span className="version-label">SONAHOP / {w.info.version}</span>
          <p>连接、共享与控制</p>
          <span className="sidebar-platform">
            <MonitorIcon size={14} weight="bold" />
            Windows
          </span>
        </div>
      </aside>
      <main className="desktop-main">
        <header className="desktop-header">
          <div className="breadcrumb">
            工作区 <span>/</span>{" "}
            {w.page === "settings" ? "设置" : microphone ? "手机麦克风" : "共享音频"}
          </div>
          <StatusBadge />
        </header>
        <PreviewNotice browser={w.info.browserPreview} />
        {w.appError && <ErrorMessage message={w.appError} />}
        {w.page === "settings" ? (
          <SettingsScreen workbench={w} />
        ) : (
          <div className="workspace-content">
            <div className="page-heading">
              <div>
                <p className="eyebrow">AUDIO WORKSPACE</p>
                <h1>{microphone ? "手机作为电脑麦克风" : "共享电脑声音"}</h1>
                <p>
                  {microphone
                    ? "让移动设备采集声音，通过虚拟音频入口供电脑应用使用。"
                    : "电脑作为音源，手机和平板独立控制播放音量。"}
                </p>
              </div>
              <span className="mode-symbol">
                {microphone ? (
                  <MicrophoneIcon size={28} weight="bold" />
                ) : (
                  <SpeakerHighIcon size={28} weight="bold" />
                )}
              </span>
            </div>
            <div className="desktop-panels">
              <section className="panel source-panel">
                <div className="panel-heading">
                  <h2>{microphone ? "电脑接收入口" : "音频来源"}</h2>
                  {!microphone && (
                    <Button
                      className="icon-button"
                      aria-label="刷新播放设备"
                      title="刷新播放设备"
                      disabled={w.loading}
                      onClick={() => void w.refreshDevices()}
                    >
                      <ArrowClockwiseIcon
                        size={19}
                        weight="bold"
                        className={w.loading ? "refreshing" : ""}
                      />
                    </Button>
                  )}
                </div>
                <div className="source-identity">
                  <span className="device-symbol">
                    <MonitorIcon size={28} weight="bold" />
                  </span>
                  <div>
                    <h3>{microphone ? "Windows 虚拟音频设备" : "Windows 系统声音"}</h3>
                    <p>{microphone ? "供会议、游戏或录音应用选择" : "来自你所选播放设备的声音"}</p>
                  </div>
                </div>
                {microphone ? (
                  <div className="virtual-device-field">
                    <label>虚拟音频入口</label>
                    <div className="inactive-field">
                      尚未配置
                      <CaretDownIcon size={16} />
                    </div>
                    <p className="field-help">后续兼容已有虚拟音频设备。</p>
                  </div>
                ) : (
                  <>
                    <label className="field-label" htmlFor="audio-source">
                      播放设备
                    </label>
                    <div className="select-wrap">
                      <select
                        id="audio-source"
                        value={w.source?.id ?? ""}
                        disabled={w.loading || w.devices.length === 0}
                        onChange={(event) => w.update({ sourceId: event.currentTarget.value })}
                      >
                        {w.devices.length === 0 && (
                          <option value="">
                            {w.loading ? "正在读取设备…" : "没有可用的播放设备"}
                          </option>
                        )}
                        {w.devices.map((device) => (
                          <option key={device.id} value={device.id}>
                            {device.name}
                            {device.isDefault ? " · 默认" : ""}
                          </option>
                        ))}
                      </select>
                      <CaretDownIcon size={16} weight="bold" aria-hidden />
                    </div>
                    {w.deviceError ? (
                      <ErrorMessage message={w.deviceError} />
                    ) : (
                      <p className="field-help">
                        {w.loading ? (
                          "正在读取 Windows 播放设备"
                        ) : w.source ? (
                          <>
                            <CheckCircleIcon size={14} weight="bold" />
                            已选择{w.source.isDefault ? "系统默认输出" : "此输出设备"}
                          </>
                        ) : (
                          "连接或启用播放设备后，可点击刷新。"
                        )}
                      </p>
                    )}
                  </>
                )}
                <RouteDiagram mode={w.prefs.mode} />
                <div className="source-footnote">
                  <span className="mini-dot" />
                  {microphone
                    ? "仅一台设备提供麦克风，收听模式暂停。"
                    : "接收设备的音量与电脑本地播放音量分开控制。"}
                </div>
              </section>
              <EmptyDevices microphone={microphone} />
            </div>
            <div className="share-bar">
              <div>
                <span className="share-status-icon">
                  <WifiHighIcon size={22} weight="bold" />
                </span>
                <div>
                  <h3>连接功能尚未开放</h3>
                  <p>本版可查看音源与调整界面设置，尚未开放音频连接。</p>
                </div>
              </div>
              <Button className="primary-button" disabled aria-describedby="preview-notice">
                {microphone ? "启用手机麦克风" : "开始共享"}
                <ArrowRightIcon size={18} weight="bold" />
              </Button>
            </div>
          </div>
        )}
        <footer className="desktop-footer">
          <span>
            <span className="status-dot" />
            音频流未启动
          </span>
          <span>{w.saved ? "界面设置已保存到本机" : "设置未保存，下次打开可能恢复默认"}</span>
        </footer>
      </main>
    </div>
  );
}
