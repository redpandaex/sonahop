import { GearSixIcon, MonitorIcon, SpeakerHighIcon } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import type { ConnectionRole } from "../lib/preferences";
import type { Workbench } from "../lib/use-workbench";
import { PlayerAudioPanel, RemoteMicrophonePanel, ServerAudioPanel } from "./audio-controls";
import { ErrorMessage } from "./controls";
import { ClientLinkPanel, HostLinkPanel } from "./lan-connection";

export function RoleNavigation({
  workbench: w,
  vertical = false,
}: {
  workbench: Workbench;
  vertical?: boolean;
}) {
  const roles: Array<{ role: ConnectionRole; label: string; icon: typeof MonitorIcon }> = [
    { role: "host", label: "共享", icon: MonitorIcon },
    { role: "client", label: "收听", icon: SpeakerHighIcon },
  ];
  const disabled =
    w.roleBusy ||
    w.audio.busy ||
    w.sessionBusy ||
    w.link.busy ||
    (w.link.native && !w.link.available);
  return (
    <nav
      className={vertical ? "role-navigation role-navigation-vertical" : "role-navigation"}
      aria-label="共享、收听与设置"
    >
      {roles.map(({ role, label, icon: Icon }) => (
        <Button
          key={role}
          className={
            w.page !== "settings" && w.role === role ? "role-choice selected" : "role-choice"
          }
          aria-pressed={w.page !== "settings" && w.role === role}
          title={w.role !== role ? "切换会结束当前连接" : undefined}
          disabled={disabled}
          onClick={() => {
            if (w.role === role && w.roleReady) w.navigate("workspace");
            else void w.switchRole(role);
          }}
        >
          <Icon size={21} weight="bold" />
          <span>{label}</span>
        </Button>
      ))}
      <a
        href="#settings"
        className={w.page === "settings" ? "role-choice selected" : "role-choice"}
        aria-current={w.page === "settings" ? "page" : undefined}
      >
        <GearSixIcon size={21} weight="bold" />
        <span>设置</span>
      </a>
    </nav>
  );
}

export function WorkspaceScreen({ workbench: w }: { workbench: Workbench }) {
  const host = w.role === "host";
  return (
    <div className="workspace-content shared-workspace">
      <div className="page-heading">
        <div>
          <h1>{host ? "共享声音" : "收听声音"}</h1>
        </div>
        <span className="mode-symbol">
          {host ? (
            <MonitorIcon size={28} weight="bold" />
          ) : (
            <SpeakerHighIcon size={28} weight="bold" />
          )}
        </span>
      </div>
      {w.roleBusy && (
        <p className="role-state-note" role="status">
          正在停止旧会话并切换连接角色…
        </p>
      )}
      {!w.roleReady && !w.roleBusy && !w.roleError && w.link.native && (
        <p className="role-state-note" role="status">
          正在准备连接…
        </p>
      )}
      {w.roleError && (
        <>
          <ErrorMessage message={w.roleError} />
          <Button
            className="secondary-button"
            disabled={w.roleBusy || w.audio.busy || w.link.busy}
            onClick={() => void w.switchRole(w.prefs.connectionRole ?? w.role)}
          >
            重试角色初始化
          </Button>
        </>
      )}
      {w.audio.error && <ErrorMessage message={w.audio.error} />}
      {w.sessionError && <ErrorMessage message={w.sessionError} />}
      {host ? (
        <>
          <div className="workspace-panels">
            <ServerAudioPanel workbench={w} />
            <HostLinkPanel workbench={w} />
          </div>
          {w.info.platform === "windows" && (
            <details
              className="advanced-audio"
              open={Boolean(
                w.audio.snapshot?.pendingMicRequests.length || w.audio.snapshot?.microphone.active,
              )}
            >
              <summary>提供给电脑应用的麦克风</summary>
              <RemoteMicrophonePanel workbench={w} />
            </details>
          )}
        </>
      ) : (
        <div className="workspace-panels">
          <ClientLinkPanel workbench={w} />
          <PlayerAudioPanel workbench={w} />
        </div>
      )}
    </div>
  );
}
