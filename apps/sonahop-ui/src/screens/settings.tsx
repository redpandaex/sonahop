import { Button } from "@/components/ui/button";
import { ErrorMessage } from "../components/controls";
import { DiagnosticSettings } from "../components/diagnostic-settings";
import {
  SunIcon,
  MoonIcon,
  CircleHalfIcon,
  ArrowLeftIcon,
  ShieldCheckIcon,
  MonitorIcon,
} from "@phosphor-icons/react";
import type { Workbench } from "../lib/use-workbench";
import type { ThemePreference } from "../lib/preferences";

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
          <p className="eyebrow">PREFERENCES</p>
          <h1>设置</h1>
          <p>让 SonaHop 适合你的设备。</p>
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
              <p>与系统状态栏、导航栏保持一致。</p>
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
            <h2>Windows 启动</h2>
            <MonitorIcon size={18} weight="bold" />
          </div>
          <div className="setting-row">
            <div>
              <h3>登录 Windows 时启动 SonaHop</h3>
              <p>默认关闭。启动应用后，声音共享和手机麦克风仍由你手动开启。</p>
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
          {w.autostart.enabled && (
            <p className="field-help">
              启动项使用当前程序位置。启用后请保留此程序，移动到新位置后重新关闭、开启一次。
            </p>
          )}
          {w.autostart.error && <ErrorMessage message={w.autostart.error} />}
        </section>
      )}
      <section className="panel settings-panel">
        <div className="panel-heading">
          <h2>声音与模式</h2>
          <ShieldCheckIcon size={18} weight="bold" />
        </div>
        <div className="setting-row">
          <div>
            <h3>设备系统音量</h3>
            <p>使用手机或平板的音量键控制播放，不改变电脑和其它设备的音量。</p>
          </div>
          <span className="quiet-tag">已选定</span>
        </div>
        <div className="setting-row">
          <div>
            <h3>电脑同时收发</h3>
            <p>Windows 可以共享声音并接收手机麦克风。每台 Android 一次运行一个方向。</p>
          </div>
          <span className="quiet-tag">默认</span>
        </div>
        <div className="setting-row">
          <div>
            <h3>界面设置保存</h3>
            <p>
              {w.saved
                ? "设置保存在本机，重新打开后恢复。"
                : "当前无法保存设置，本次调整仍可使用。"}
            </p>
          </div>
          <span className="quiet-tag">{w.saved ? "已保存" : "未保存"}</span>
        </div>
      </section>
      <DiagnosticSettings />
      <section className="panel settings-panel">
        <div className="panel-heading">
          <h2>关于 SonaHop</h2>
          <MonitorIcon size={18} weight="bold" />
        </div>
        <div className="about-row">
          <span>版本</span>
          <span className="mono">{w.info.version} · 首版</span>
        </div>
        <div className="about-row">
          <span>当前平台</span>
          <span>
            {mobile ? "Android" : w.info.platform === "windows" ? "Windows" : "浏览器预览"}
          </span>
        </div>
        <p className="about-note">
          {w.info.browserPreview
            ? "浏览器仅提供布局与偏好预览；连接、系统声音收听及手机麦克风需使用原生应用。"
            : "支持局域网 PIN 连接、系统声音共享与手机麦克风；麦克风需要电脑批准及已安装的虚拟音频线。"}
          切换设置页不停止正在运行的音频。
        </p>
      </section>
    </div>
  );
}
