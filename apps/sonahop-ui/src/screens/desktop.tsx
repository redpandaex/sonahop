import { Button } from "@/components/ui/button";
import { SpeakerHighIcon, MicrophoneIcon, GearSixIcon, MonitorIcon } from "@phosphor-icons/react";
import {
  Brand,
  ModeSwitch,
  PreviewNotice,
  RouteDiagram,
  StatusBadge,
  ErrorMessage,
} from "../components/controls";
import { HostLinkPanel, linkStatusLabel } from "../components/lan-connection";
import {
  HostAudioAction,
  VirtualOutputSelector,
  audioStatusLabel,
} from "../components/audio-controls";
import type { Workbench } from "../lib/use-workbench";
import { SettingsScreen } from "./settings";

export function DesktopScreen({ workbench: w }: { workbench: Workbench }) {
  const microphone = w.prefs.mode === "microphone";
  const pendingMicrophones = Boolean(w.audio.snapshot?.pendingMicRequests.length);
  return (
    <div className="desktop-app">
      <aside className="sidebar">
        <Brand />
        <div className="sidebar-section-title">音频功能</div>
        <ModeSwitch
          vertical
          mode={w.prefs.mode}
          disabled={w.audio.busy || w.link.busy}
          onChange={(mode) => {
            void w.switchMode(mode).then((changed) => {
              if (changed) w.navigate("workspace");
            });
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
          <StatusBadge label={linkStatusLabel(w)} />
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
                    ? "手机主动申请，通过你选择的虚拟音频线供电脑应用使用。"
                    : "采集系统声音，独立于电脑扬声器音量；手机和平板各自控制收听音量。"}
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
                  <StatusBadge label={audioStatusLabel(w)} />
                </div>
                <div className="source-identity">
                  <span className="device-symbol">
                    <MonitorIcon size={28} weight="bold" />
                  </span>
                  <div>
                    <h3>{microphone ? "Windows 虚拟音频设备" : "Windows 系统声音"}</h3>
                    <p>
                      {microphone
                        ? "选择虚拟音频线的播放端，会议应用使用其录音端。"
                        : "通过进程音频捕获共享系统声音。"}
                    </p>
                  </div>
                </div>
                {microphone ? (
                  pendingMicrophones ? (
                    <p className="field-help">请在下方麦克风申请区选择虚拟音频线入口并批准设备。</p>
                  ) : (
                    <VirtualOutputSelector workbench={w} />
                  )
                ) : (
                  <div className="system-audio-source">
                    <span className="field-label">系统音频采集</span>
                    <div className="inactive-field">系统声音 · 独立于本地扬声器音量</div>
                    <p className="field-help">
                      此模式采集系统进程音频，不绑定某个播放设备。点击开始共享后才启动采集。
                    </p>
                  </div>
                )}
                <RouteDiagram mode={w.prefs.mode} />
                <div className="source-footnote">
                  <span className="mini-dot" />
                  {microphone
                    ? "只接受一台设备作为麦克风，其它设备可以继续收听。"
                    : "最多两台设备连接，播放音量使用各自设备的音量键。"}
                </div>
              </section>
              <HostLinkPanel workbench={w} />
            </div>
            <HostAudioAction workbench={w} />
          </div>
        )}
        <footer className="desktop-footer">
          <span>
            <span className="status-dot" />
            {audioStatusLabel(w)}
          </span>
          <span>{w.saved ? "界面设置已保存到本机" : "设置未保存，下次打开可能恢复默认"}</span>
          {w.page === "settings" &&
            w.audio.snapshot &&
            (w.audio.snapshot.active || w.audio.snapshot.waiting) && (
              <Button
                className="secondary-button"
                disabled={w.audio.busy}
                onClick={() => void w.audio.stop()}
              >
                停止全部音频
              </Button>
            )}
        </footer>
      </main>
    </div>
  );
}
