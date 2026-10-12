import { Button } from "@/components/ui/button";
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
      <section className="panel settings-panel">
        <div className="panel-heading">
          <h2>声音与模式</h2>
          <ShieldCheckIcon size={18} weight="bold" />
        </div>
        <div className="setting-row">
          <div>
            <h3>接收设备独立音量</h3>
            <p>接收音量只作用于这台设备，不改变电脑或其它收听设备的音量。</p>
          </div>
          <span className="quiet-tag">已选定</span>
        </div>
        <div className="setting-row">
          <div>
            <h3>收听 / 麦克风互斥</h3>
            <p>首版只运行一种方向，避免声音回环。</p>
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
