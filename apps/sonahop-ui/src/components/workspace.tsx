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
    { role: "host", label: "服务器", icon: MonitorIcon },
    { role: "client", label: "播放器", icon: SpeakerHighIcon },
  ];
  const disabled =
    w.roleBusy || w.audio.busy || w.link.busy || (w.link.native && !w.link.available);
  return (
    <nav
      className={vertical ? "role-navigation role-navigation-vertical" : "role-navigation"}
      aria-label="连接角色与设置"
    >
      {roles.map(({ role, label, icon: Icon }) => (
        <Button
          key={role}
          className={
            w.page !== "settings" && w.role === role ? "role-choice selected" : "role-choice"
          }
          aria-pressed={w.page !== "settings" && w.role === role}
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
        <h2>保存的服务器</h2>
        <Button
          className="icon-button"
          aria-label="编辑保存的服务器"
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
              computerName: name.trim() || "我的服务器",
              computerAddress: address.trim(),
            });
            setEditing(false);
            setSaved(true);
          }}
        >
          <label className="field-label" htmlFor="saved-server-name">
            服务器名称
          </label>
          <Input
            id="saved-server-name"
            maxLength={64}
            value={name}
            onChange={(event) => setName(event.currentTarget.value)}
            placeholder="我的服务器"
          />
          <label className="field-label" htmlFor="saved-server-address">
            服务器地址
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
          <h3>{w.prefs.computerAddress ? w.prefs.computerName : "还没有保存服务器"}</h3>
          <p className={w.prefs.computerAddress ? "mono" : ""}>
            {w.prefs.computerAddress || "搜索发现或手动连接 Windows / Android 服务器。"}
          </p>
        </div>
      )}
      {saved && (
        <p className="saved-message" role="status">
          <CheckCircleIcon size={15} weight="bold" />
          已保存到本机，保存信息不会发起连接。
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
          <p className="eyebrow">{host ? "AUDIO SERVER" : "AUDIO PLAYER"}</p>
          <h1>{host ? "共享这台设备的声音" : "收听服务器的声音"}</h1>
          <p>
            {host
              ? "选择本机音源，让 Windows 或 Android 播放器收听。"
              : "连接 Windows 或 Android 服务器，在这台设备播放音频。"}
          </p>
        </div>
        <span className="mode-symbol">
          {host ? (
            <MonitorIcon size={28} weight="bold" />
          ) : (
            <SpeakerHighIcon size={28} weight="bold" />
          )}
        </span>
      </div>
      <p className="role-change-note">
        切换服务器 / 播放器会停止音频并断开所有连接。切换设置页会保持当前会话。
      </p>
      {w.roleBusy && (
        <p className="role-state-note" role="status">
          正在停止旧会话并切换连接角色…
        </p>
      )}
      {!w.roleReady && !w.roleBusy && !w.roleError && w.link.native && (
        <p className="role-state-note" role="status">
          正在准备连接角色；尚未开启发现或音频。
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
          <RemoteMicrophonePanel workbench={w} />
        </>
      ) : (
        <div className="workspace-panels">
          <div className="workspace-connection-column">
            <ClientLinkPanel workbench={w} />
            <SavedServer workbench={w} />
          </div>
          <PlayerAudioPanel workbench={w} />
        </div>
      )}
    </div>
  );
}
