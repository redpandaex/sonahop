import { Button } from "@/components/ui/button";
import { MonitorIcon } from "@phosphor-icons/react";
import { Brand, PreviewNotice, StatusBadge, ErrorMessage } from "../components/controls";
import { linkStatusLabel } from "../components/lan-connection";
import { audioStatusLabel } from "../components/audio-controls";
import { RoleNavigation, WorkspaceScreen } from "../components/workspace";
import type { Workbench } from "../lib/use-workbench";
import { SettingsScreen } from "./settings";

export function DesktopScreen({ workbench: w }: { workbench: Workbench }) {
  return (
    <div className="desktop-app">
      <aside className="sidebar">
        <Brand />
        <div className="sidebar-section-title">声音</div>
        <RoleNavigation workbench={w} vertical />
        <div className="sidebar-bottom">
          <span className="version-label">SONAHOP / {w.info.version}</span>
          <span className="sidebar-platform">
            <MonitorIcon size={14} weight="bold" />
            {w.info.browserPreview ? "浏览器布局预览" : "Windows"}
          </span>
        </div>
      </aside>
      <main className="desktop-main" id="main-content">
        <header className="desktop-header">
          <div className="breadcrumb">
            工作区 <span>/</span>{" "}
            {w.page === "settings" ? "设置" : w.role === "host" ? "共享" : "收听"}
          </div>
          <StatusBadge label={linkStatusLabel(w)} />
        </header>
        <PreviewNotice browser={w.info.browserPreview} />
        {w.appError && <ErrorMessage message={w.appError} />}
        {w.page === "settings" ? (
          <SettingsScreen workbench={w} />
        ) : (
          <WorkspaceScreen workbench={w} />
        )}
        <footer className="desktop-footer">
          <span>
            <span className="status-dot" />
            {audioStatusLabel(w)}
          </span>
          {!w.saved && <span>设置未保存</span>}
          {w.page === "settings" &&
            w.audio.snapshot &&
            (w.audio.snapshot.active || w.audio.snapshot.waiting) && (
              <Button
                className="secondary-button"
                disabled={w.audio.busy || w.roleBusy}
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
