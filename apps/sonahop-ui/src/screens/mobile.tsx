import { GearSixIcon } from "@phosphor-icons/react";
import { Brand, PreviewNotice, ErrorMessage, StatusBadge } from "../components/controls";
import { audioStatusLabel } from "../components/audio-controls";
import { RoleNavigation, WorkspaceScreen } from "../components/workspace";
import type { Workbench } from "../lib/use-workbench";
import { SettingsScreen } from "./settings";

export function MobileScreen({ workbench: w }: { workbench: Workbench }) {
  return (
    <div className="mobile-app multi-role-mobile">
      <header className="mobile-header">
        <Brand small />
        <a className="icon-button" href="#settings" aria-label="打开设置">
          <GearSixIcon size={23} weight="bold" />
        </a>
      </header>
      <main className="mobile-main" id="main-content">
        <RoleNavigation workbench={w} />
        <PreviewNotice browser={w.info.browserPreview} />
        {w.appError && <ErrorMessage message={w.appError} />}
        {w.page === "settings" ? (
          <SettingsScreen workbench={w} mobile />
        ) : (
          <WorkspaceScreen workbench={w} />
        )}
        <div className="mobile-session-status">
          <StatusBadge label={audioStatusLabel(w)} />
        </div>
        {!w.saved && <ErrorMessage message="设置未能保存，下次打开可能恢复默认。" />}
      </main>
    </div>
  );
}
