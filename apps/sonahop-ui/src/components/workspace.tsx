import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  MonitorIcon,
  SpeakerHighIcon,
  GearSixIcon,
  PencilSimpleIcon,
  CheckCircleIcon,
} from "@phosphor-icons/react";
import { ErrorMessage } from "./controls";
import { HostLinkPanel, ClientLinkPanel } from "./lan-connection";
import { ServerAudioPanel, PlayerAudioPanel, RemoteMicrophonePanel } from "./audio-controls";
import type { Workbench } from "../lib/use-workbench";
import type { ConnectionRole } from "../lib/preferences";

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
    w.roleBusy || w.audio.busy || w.link.busy || (w.link.native && !w.link.available);
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

function SavedServer({ workbench: w }: { workbench: Workbench }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(w.prefs.computerName);
  const [address, setAddress] = useState(w.prefs.computerAddress);
  const [saved, setSaved] = useState(false);
  return (
    <section className="panel computer-panel saved-server-panel">
      <div className="panel-heading">
        <h2>保存的共享设备</h2>
        <Button
          className="icon-button"
          aria-label="编辑保存的共享设备"
          onClick={() => {
            setName(w.prefs.computerName);
            setAddress(w.prefs.computerAddress);
            setEditing(!editing);
            setSaved(false);
          }}
        >
          <PencilSimpleIcon size={18} weight="bold" />
        </Button>
      </div>
      {editing ? (
        <form
          className="computer-form"
          onSubmit={(event) => {
            event.preventDefault();
            w.update({
              computerName: name.trim() || "我的共享设备",
              computerAddress: address.trim(),
            });
            setEditing(false);
            setSaved(true);
          }}
        >
          <label className="field-label" htmlFor="saved-server-name">
            共享设备名称
          </label>
          <Input
            id="saved-server-name"
            maxLength={64}
            value={name}
            onChange={(event) => setName(event.currentTarget.value)}
            placeholder="我的共享设备"
          />
          <label className="field-label" htmlFor="saved-server-address">
            共享设备地址
          </label>
          <Input
            id="saved-server-address"
            maxLength={255}
            value={address}
            onChange={(event) => setAddress(event.currentTarget.value)}
            placeholder="192.168.1.10:32993"
            inputMode="url"
            autoCapitalize="off"
            autoComplete="off"
            spellCheck={false}
          />
          <div className="form-actions">
            <Button className="secondary-button" type="button" onClick={() => setEditing(false)}>
              取消
            </Button>
            <Button className="primary-button" type="submit">
              保存信息
            </Button>
          </div>
        </form>
      ) : (
        <div className="saved-server-identity">
          <h3>{w.prefs.computerAddress ? w.prefs.computerName : "还没有保存共享设备"}</h3>
          <p className={w.prefs.computerAddress ? "mono" : ""}>
            {w.prefs.computerAddress || "搜索发现或手动连接 Windows / Android 共享设备。"}
          </p>
        </div>
      )}
      {saved && (
        <p className="saved-message" role="status">
          <CheckCircleIcon size={15} weight="bold" />
          已保存
        </p>
      )}
    </section>
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
          <div className="workspace-connection-column">
            <ClientLinkPanel workbench={w} />
            <details className="saved-server-details">
              <summary>保存或编辑设备地址</summary>
              <SavedServer workbench={w} />
            </details>
          </div>
          <PlayerAudioPanel workbench={w} />
        </div>
      )}
    </div>
  );
}
