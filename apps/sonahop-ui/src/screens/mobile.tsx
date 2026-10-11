import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import {
  GearSixIcon,
  MonitorIcon,
  CaretRightIcon,
  CheckCircleIcon,
  ArrowRightIcon,
  MicrophoneIcon,
  SpeakerHighIcon,
  HouseIcon,
  PencilSimpleIcon,
} from "@phosphor-icons/react";
import {
  Brand,
  ModeSwitch,
  PreviewNotice,
  RouteDiagram,
  StatusBadge,
  VolumeControl,
  ErrorMessage,
} from "../components/controls";
import type { Workbench } from "../lib/use-workbench";
import { SettingsScreen } from "./settings";

function ComputerCard({ workbench: w }: { workbench: Workbench }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(w.prefs.computerName);
  const [address, setAddress] = useState(w.prefs.computerAddress);
  const [message, setMessage] = useState("");
  const hasAddress = Boolean(w.prefs.computerAddress);
  return (
    <section className="panel computer-panel">
      <div className="panel-heading">
        <h2>电脑信息</h2>
        <StatusBadge label={hasAddress ? "未连接" : "未添加"} />
      </div>
      {editing ? (
        <form
          className="computer-form"
          onSubmit={(event) => {
            event.preventDefault();
            w.update({ computerName: name.trim() || "我的电脑", computerAddress: address.trim() });
            setEditing(false);
            setMessage("电脑信息已保存，当前版本尚不能连接。");
          }}
        >
          <label className="field-label" htmlFor="computer-name">
            电脑名称
          </label>
          <Input
            id="computer-name"
            value={name}
            maxLength={64}
            onChange={(event) => setName(event.currentTarget.value)}
            placeholder="我的电脑"
          />
          <label className="field-label" htmlFor="computer-address">
            电脑地址
          </label>
          <Input
            id="computer-address"
            value={address}
            maxLength={255}
            onChange={(event) => setAddress(event.currentTarget.value)}
            placeholder="例如 192.168.1.10"
            inputMode="url"
            autoCapitalize="off"
            spellCheck={false}
          />
          <div className="form-actions">
            <Button type="button" className="secondary-button" onClick={() => setEditing(false)}>
              取消
            </Button>
            <Button className="primary-button" type="submit" disabled={!address.trim()}>
              保存信息
            </Button>
          </div>
        </form>
      ) : (
        <>
          <div className="computer-identity">
            <span className="device-symbol">
              <MonitorIcon size={28} weight="bold" />
            </span>
            <div>
              <h3>{hasAddress ? w.prefs.computerName : "添加你的 Windows 电脑"}</h3>
              <p className={hasAddress ? "mono" : ""}>
                {hasAddress ? w.prefs.computerAddress : "当前仅保存地址，尚不能搜索或连接"}
              </p>
            </div>
            <Button
              className="icon-button"
              aria-label="编辑电脑信息"
              onClick={() => {
                setName(w.prefs.computerName);
                setAddress(w.prefs.computerAddress);
                setEditing(true);
              }}
            >
              <PencilSimpleIcon size={19} weight="bold" />
            </Button>
          </div>
          <Button
            className="computer-action"
            onClick={() => {
              setName(w.prefs.computerName);
              setAddress(w.prefs.computerAddress);
              setEditing(true);
            }}
          >
            {hasAddress ? "编辑连接信息" : "填写电脑地址"}
            <CaretRightIcon size={19} weight="bold" />
          </Button>
        </>
      )}
      {message && (
        <p className="saved-message" role="status">
          <CheckCircleIcon size={15} weight="bold" />
          {message}
        </p>
      )}
    </section>
  );
}
export function MobileScreen({ workbench: w }: { workbench: Workbench }) {
  const microphone = w.prefs.mode === "microphone";
  return (
    <div className="mobile-app">
      <header className="mobile-header">
        <Brand small />
        <a className="icon-button" href="#settings" aria-label="打开设置">
          <GearSixIcon size={23} weight="bold" />
        </a>
      </header>
      <main className="mobile-main">
        <PreviewNotice browser={w.info.browserPreview} />
        {w.appError && <ErrorMessage message={w.appError} />}
        {w.page === "settings" ? (
          <SettingsScreen workbench={w} mobile />
        ) : (
          <>
            <div className="mobile-heading">
              <p className="eyebrow">YOUR AUDIO, HERE</p>
              <h1>{microphone ? "把麦克风交给电脑" : "让电脑声音来到这里"}</h1>
              <p>
                {microphone
                  ? "使用这台设备的麦克风，为电脑应用提供声音。"
                  : "收听电脑声音，在这台设备上控制音量。"}
              </p>
            </div>
            <ModeSwitch mode={w.prefs.mode} onChange={(mode) => w.update({ mode })} />
            <div className="mobile-panels">
              <div>
                <ComputerCard workbench={w} />
                <section className="panel mobile-flow-panel">
                  <RouteDiagram mode={w.prefs.mode} mobile />
                  <p>
                    {microphone
                      ? "启用麦克风时，电脑声音收听会暂停。"
                      : "收听与麦克风模式分别运行。"}
                  </p>
                </section>
              </div>
              <section className="panel playback-panel">
                <div className="panel-heading">
                  <h2>{microphone ? "麦克风设置" : "播放控制"}</h2>
                  <span className="quiet-tag">{microphone ? "未启用" : "独立音量"}</span>
                </div>
                <div className="playback-identity">
                  <span className="playback-icon">
                    {microphone ? (
                      <MicrophoneIcon size={37} weight="bold" />
                    ) : (
                      <SpeakerHighIcon size={37} weight="bold" />
                    )}
                  </span>
                  <div>
                    <h3>{microphone ? "本设备麦克风" : "本设备扬声器 / 耳机"}</h3>
                    <p>{microphone ? "当前不会采集声音" : "音频输出尚未启动"}</p>
                  </div>
                </div>
                <VolumeControl workbench={w} microphone={microphone} />
                <div className="mobile-primary">
                  <Button className="primary-button" disabled aria-describedby="preview-notice">
                    {microphone ? "开始使用麦克风" : "连接并收听"}
                    <ArrowRightIcon size={18} weight="bold" />
                  </Button>
                  <p>
                    {microphone
                      ? "录音只会在你主动开始后启动。"
                      : "当前版本为界面预览，尚不能连接电脑。"}
                  </p>
                </div>
              </section>
            </div>
          </>
        )}
        {!w.saved && <ErrorMessage message="设置未能保存，下次打开可能恢复默认。" />}
      </main>
      <nav className="mobile-navigation" aria-label="主导航">
        <a
          className={w.page === "workspace" ? "active" : ""}
          href="#workspace"
          aria-current={w.page === "workspace" ? "page" : undefined}
        >
          <HouseIcon size={22} weight="bold" />
          <span>音频</span>
        </a>
        <a
          className={w.page === "settings" ? "active" : ""}
          href="#settings"
          aria-current={w.page === "settings" ? "page" : undefined}
        >
          <GearSixIcon size={22} weight="bold" />
          <span>设置</span>
        </a>
      </nav>
    </div>
  );
}
