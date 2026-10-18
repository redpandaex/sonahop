import {
  ArrowLeftIcon,
  CircleHalfIcon,
  MonitorIcon,
  MoonIcon,
  SunIcon,
} from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ErrorMessage } from "../components/controls";
import { DiagnosticSettings } from "../components/diagnostic-settings";
import type { ThemePreference } from "../lib/preferences";
import type { Workbench } from "../lib/use-workbench";

export function SettingsScreen({
  workbench: w,
  mobile = false,
}: {
  workbench: Workbench;
  mobile?: boolean;
}) {
  const themes: { value: ThemePreference; label: string; icon: typeof SunIcon }[] = [
    { value: "light", label: "浅色", icon: SunIcon },
    { value: "dark", label: "深色", icon: MoonIcon },
    { value: "system", label: "系统", icon: CircleHalfIcon },
  ];
  return (
    <div className="settings-content">
      <a href="#workspace" className="back-link">
        <ArrowLeftIcon size={17} weight="bold" />
        返回音频
      </a>
      <div className="page-heading">
        <div>
          <h1>设置</h1>
        </div>
      </div>
      <section className="panel settings-panel">
        <div className="panel-heading">
          <h2>应用外观</h2>
          <span className="quiet-tag">{w.dark ? "深色" : "浅色"}</span>
        </div>
        {mobile ? (
          <div className="system-theme-note">
            <CircleHalfIcon size={25} weight="bold" />
            <div>
              <h3>跟随设备系统主题</h3>
            </div>
          </div>
        ) : (
          <div className="theme-options">
            {themes.map(({ value, label, icon: Icon }) => (
              <Button
                className={w.prefs.theme === value ? "theme-option selected" : "theme-option"}
                key={value}
                aria-pressed={w.prefs.theme === value}
                onClick={() => w.update({ theme: value })}
              >
                <Icon size={24} weight="bold" />
                <span>{label}</span>
              </Button>
            ))}
          </div>
        )}
      </section>
      {!mobile && w.info.platform === "windows" && (
        <section className="panel settings-panel">
          <div className="panel-heading">
            <h2>Windows</h2>
            <MonitorIcon size={18} weight="bold" />
          </div>
          <div className="setting-row">
            <div>
              <h3>登录 Windows 时启动 SonaHop</h3>
            </div>
            <Button
              type="button"
              role="switch"
              aria-checked={w.autostart.enabled === true}
              aria-label="登录 Windows 时启动 SonaHop"
              className={w.autostart.enabled ? "primary-button" : "secondary-button"}
              disabled={!w.autostart.available || w.autostart.enabled === null || w.autostart.busy}
              onClick={() => void w.autostart.change(!w.autostart.enabled)}
            >
              {w.autostart.busy
                ? "设置中…"
                : !w.autostart.available
                  ? "原生应用可用"
                  : w.autostart.enabled === null
                    ? "读取中…"
                    : w.autostart.enabled
                      ? "已开启"
                      : "已关闭"}
            </Button>
          </div>
          <div className="setting-row">
            <h3 id="close-behavior-label">关闭主窗口时</h3>
            <Select
              value={w.desktop.settings?.closeBehavior ?? "tray"}
              disabled={!w.desktop.available || !w.desktop.settings || w.desktop.busy}
              onValueChange={(value) => {
                if (value === "tray" || value === "exit") void w.desktop.change(value);
              }}
            >
              <SelectTrigger
                className="desktop-close-select"
                aria-labelledby="close-behavior-label"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent position="popper">
                <SelectItem value="tray" disabled={w.desktop.settings?.trayAvailable === false}>
                  收起到托盘
                </SelectItem>
                <SelectItem value="exit">直接退出</SelectItem>
              </SelectContent>
            </Select>
          </div>
          {w.desktop.settings?.trayAvailable === false && (
            <p className="field-help">托盘不可用，关闭窗口将退出。</p>
          )}
          {w.desktop.error && <ErrorMessage message={w.desktop.error} />}
          {w.autostart.enabled && <p className="field-help">移动程序后需重新设置启动项。</p>}
          {w.autostart.error && <ErrorMessage message={w.autostart.error} />}
        </section>
      )}
      <DiagnosticSettings />
      <section className="panel settings-panel">
        <div className="panel-heading">
          <h2>关于 SonaHop</h2>
          <MonitorIcon size={18} weight="bold" />
        </div>
        <div className="about-row">
          <span>版本</span>
          <span className="mono">{w.info.version}</span>
        </div>
        <div className="about-row">
          <span>当前平台</span>
          <span>
            {mobile ? "Android" : w.info.platform === "windows" ? "Windows" : "浏览器预览"}
          </span>
        </div>
      </section>
    </div>
  );
}
