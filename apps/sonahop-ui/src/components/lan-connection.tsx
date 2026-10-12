import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DeviceMobileIcon, MonitorIcon, WifiHighIcon } from "@phosphor-icons/react";
import { ErrorMessage, StatusBadge } from "./controls";
import { parseLinkAddress } from "../lib/link-bridge";
import type { Workbench } from "../lib/use-workbench";
import { audioStatusLabel } from "./audio-controls";

export function linkStatusLabel(w: Workbench): string {
  if (!w.link.native) return "原生应用才能连接";
  if (!w.link.available) return "连接服务未就绪";
  const snapshot = w.link.snapshot;
  if (!snapshot) return "正在读取连接状态";
  if (snapshot.peers.length > 0)
    return w.audio.snapshot?.active || w.audio.snapshot?.waiting
      ? audioStatusLabel(w)
      : "已连接 · 音频已停止";
  if (snapshot.clientState === "awaitingApproval") return "等待电脑确认";
  if (snapshot.clientState === "connecting") return "正在连接";
  if (snapshot.running) return snapshot.role === "host" ? "允许局域网发现" : "正在搜索电脑";
  return "局域网连接已关闭";
}

export function HostLinkPanel({ workbench: w }: { workbench: Workbench }) {
  const { snapshot, busy, available, error } = w.link;
  const running = snapshot?.running ?? false;
  const disabled = !available || !snapshot || busy || w.audio.busy;
  return (
    <section className="panel peer-panel lan-panel">
      <div className="panel-heading">
        <h2>局域网连接</h2>
        <span className="subtle-count">{snapshot?.peers.length ?? 0} / 2</span>
      </div>
      <div className="lan-service-row">
        <div>
          <h3>{running ? snapshot?.localName : "允许手机和平板发现这台电脑"}</h3>
          <p>
            {running
              ? `连接端口 ${snapshot?.port}`
              : "主动开启后，同一局域网内的设备可以请求连接。"}
          </p>
        </div>
        <Button
          className={running ? "secondary-button" : "primary-button"}
          role="switch"
          aria-checked={running}
          aria-label="允许局域网发现"
          disabled={disabled}
          onClick={() => void (running ? w.stopLink() : w.link.start())}
        >
          <WifiHighIcon size={18} weight="bold" />
          {busy ? "处理中…" : running ? "停止联网" : "开启联网"}
        </Button>
      </div>
      {snapshot?.role === "host" && snapshot.running && snapshot.pairingCode && (
        <div className="lan-confirmation">
          <h3>手机连接 PIN</h3>
          <output className="lan-code" aria-label="手机连接 PIN">
            {snapshot.pairingCode}
          </output>
          <p>在手机输入此六位 PIN 后才能发起有效请求；停止并重新联网会更换 PIN。</p>
        </div>
      )}
      {error && <ErrorMessage message={error} />}
      {!w.link.native && (
        <p className="lan-note">请打开 Windows 原生应用，浏览器无法发现或连接设备。</p>
      )}
      {snapshot && snapshot.requests.length > 0 && (
        <div className="lan-group">
          <h3 className="lan-group-heading">待确认请求</h3>
          <p className="lan-note">
            连接 PIN 已由后台验证；核对手机上的请求编号和名称后，再接受请求。
          </p>
          {snapshot.requests.map((request) => (
            <div className="lan-device" key={request.id}>
              <div className="lan-device-identity">
                <DeviceMobileIcon size={23} weight="bold" aria-hidden />
                <div>
                  <h3>{request.name}</h3>
                  <p className="mono">{request.address}</p>
                </div>
              </div>
              <output className="lan-code" aria-label="核对编号">
                {request.verificationCode}
              </output>
              <div className="lan-actions">
                <Button
                  className="secondary-button"
                  disabled={disabled}
                  onClick={() => void w.link.answer(request.id, false)}
                >
                  拒绝
                </Button>
                <Button
                  className="primary-button"
                  disabled={disabled || snapshot.peers.length >= 2}
                  onClick={() => void w.link.answer(request.id, true)}
                >
                  接受
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}
      <div className="lan-group">
        <h3 className="lan-group-heading">已连接设备</h3>
        {snapshot?.peers.length ? (
          snapshot.peers.map((peer) => (
            <div className="lan-device" key={peer.id}>
              <div className="lan-device-identity">
                <DeviceMobileIcon size={23} weight="bold" aria-hidden />
                <div>
                  <h3>{peer.name}</h3>
                  <p className="mono">{peer.address}</p>
                </div>
              </div>
              <div className="lan-device-footer">
                <span className="quiet-tag">控制已连接</span>
                <Button
                  className="secondary-button"
                  disabled={disabled}
                  onClick={() => void w.disconnectPeer(peer.id)}
                >
                  断开
                </Button>
              </div>
            </div>
          ))
        ) : (
          <p className="lan-empty">
            {running ? "在手机上搜索并发起连接，请求会显示在这里。" : "尚无已连接设备。"}
          </p>
        )}
      </div>
      <p className="lan-note">最多连接两台设备；收听或麦克风需在音频区单独启动。</p>
    </section>
  );
}

export function ClientLinkPanel({ workbench: w }: { workbench: Workbench }) {
  const [address, setAddress] = useState(w.prefs.computerAddress);
  const [addressError, setAddressError] = useState("");
  const [pairingCode, setPairingCode] = useState("");
  const validPin = /^\d{6}$/.test(pairingCode);
  const { snapshot, busy, available, error } = w.link;
  const disabled = !available || !snapshot || busy || w.audio.busy;
  const state = snapshot?.clientState ?? "idle";
  const connected = state === "connected";
  const pending = state === "connecting" || state === "awaitingApproval";
  return (
    <section className="panel lan-panel client-lan-panel">
      <div className="panel-heading">
        <h2>连接电脑</h2>
        <StatusBadge label={linkStatusLabel(w)} />
      </div>
      <div className="lan-service-row">
        <p>
          {snapshot?.running
            ? "搜索与连接在切换设置页后仍保持。"
            : "先在 Windows 上开启联网，再搜索同一局域网内的电脑。"}
        </p>
        <Button
          className={snapshot?.running ? "secondary-button" : "primary-button"}
          disabled={disabled}
          onClick={() => void (snapshot?.running ? w.stopLink() : w.link.start())}
        >
          {busy
            ? "处理中…"
            : snapshot?.running
              ? pending
                ? "取消连接"
                : connected
                  ? "停止连接"
                  : "停止搜索"
              : "搜索电脑"}
        </Button>
      </div>
      {error && <ErrorMessage message={error} />}
      {!w.link.native && (
        <p className="lan-note">请打开 Android 原生应用，浏览器无法搜索或连接电脑。</p>
      )}
      {pending && (
        <div className="lan-confirmation" role="status">
          <h3>{state === "awaitingApproval" ? "请在 Windows 上确认连接" : "正在联系电脑…"}</h3>
          {snapshot?.verificationCode && (
            <output className="lan-code" aria-label="核对编号">
              {snapshot.verificationCode}
            </output>
          )}
          <p>
            {state === "awaitingApproval"
              ? "核对两端的六位编号，然后在电脑上接受请求。"
              : "收到电脑回应后，会显示核对编号。"}
          </p>
        </div>
      )}
      {connected &&
        snapshot?.peers.map((peer) => (
          <div className="lan-device" key={peer.id}>
            <div className="lan-device-identity">
              <MonitorIcon size={23} weight="bold" />
              <div>
                <h3>{peer.name}</h3>
                <p className="mono">{peer.address}</p>
              </div>
            </div>
            <div className="lan-device-footer">
              <span className="quiet-tag">{audioStatusLabel(w)}</span>
              <Button
                className="secondary-button"
                disabled={disabled}
                onClick={() => void w.disconnectPeer(peer.id)}
              >
                断开
              </Button>
            </div>
          </div>
        ))}
      {!connected && !pending && (
        <>
          <div className="lan-pin-field">
            <label className="field-label" htmlFor="connection-pin">
              Windows 显示的连接 PIN
            </label>
            <Input
              id="connection-pin"
              value={pairingCode}
              maxLength={6}
              pattern="[0-9]{6}"
              inputMode="numeric"
              autoComplete="one-time-code"
              placeholder="六位数字"
              onChange={(event) =>
                setPairingCode(event.currentTarget.value.replace(/\D/g, "").slice(0, 6))
              }
            />
            <p className="field-help">
              输入电脑联网区显示的六位 PIN，再选择电脑或填写地址。PIN 不保存到本机。
            </p>
          </div>
          <div className="lan-group">
            <h3 className="lan-group-heading">发现的电脑</h3>
            {snapshot?.discovered.length ? (
              snapshot.discovered.map((host) => (
                <div className="lan-device" key={host.id}>
                  <div className="lan-device-identity">
                    <MonitorIcon size={23} weight="bold" />
                    <div>
                      <h3>{host.name}</h3>
                      <p className="mono">
                        {host.address}:{host.port}
                      </p>
                    </div>
                  </div>
                  <Button
                    className="secondary-button"
                    disabled={disabled || !validPin}
                    onClick={() => void w.link.connect(host.address, host.port, pairingCode)}
                  >
                    连接
                  </Button>
                </div>
              ))
            ) : (
              <p className="lan-empty">
                {snapshot?.running ? "正在搜索…也可以手动填写电脑地址。" : "搜索结果会显示在这里。"}
              </p>
            )}
          </div>
          <form
            className="lan-manual-form"
            onSubmit={(event) => {
              event.preventDefault();
              setAddressError("");
              try {
                const target = parseLinkAddress(address);
                void w.link.connect(target.address, target.port, pairingCode);
              } catch (validationError) {
                setAddressError(String(validationError));
              }
            }}
          >
            <label className="field-label" htmlFor="lan-address">
              手动连接
            </label>
            <Input
              id="lan-address"
              value={address}
              maxLength={21}
              onChange={(event) => {
                setAddress(event.currentTarget.value);
                setAddressError("");
              }}
              placeholder="192.168.1.10:32993"
              autoCapitalize="off"
              autoComplete="off"
              inputMode="url"
              spellCheck={false}
              aria-describedby="lan-address-help"
            />
            <p id="lan-address-help" className="field-help">
              可只填 IPv4，默认端口 32993。填写地址不会自动连接。
            </p>
            {addressError && <ErrorMessage message={addressError} />}
            <div className="lan-actions">
              {w.prefs.computerAddress && w.prefs.computerAddress !== address && (
                <Button
                  type="button"
                  className="secondary-button"
                  onClick={() => {
                    setAddress(w.prefs.computerAddress);
                    setAddressError("");
                  }}
                >
                  使用已保存地址
                </Button>
              )}
              <Button
                type="submit"
                className="primary-button"
                disabled={disabled || !address.trim() || !validPin}
              >
                连接此地址
              </Button>
            </div>
          </form>
        </>
      )}
      <p className="lan-note">连接后可单独开始收听或申请麦克风；切换设置页不停止连接和音频。</p>
    </section>
  );
}
