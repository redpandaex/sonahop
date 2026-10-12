import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DeviceMobileIcon, MonitorIcon } from "@phosphor-icons/react";
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
  if (snapshot.clientState === "awaitingApproval") return "等待共享设备确认";
  if (snapshot.clientState === "connecting") return "正在连接";
  if (snapshot.running) return snapshot.role === "host" ? "可被发现" : "正在搜索";
  return "局域网连接已关闭";
}

export function HostLinkPanel({ workbench: w }: { workbench: Workbench }) {
  const [copyMessage, setCopyMessage] = useState("");
  const { snapshot, busy, available, error } = w.link;
  const running = snapshot?.running ?? false;
  const disabled = !available || !snapshot || busy || w.audio.busy || w.roleBusy || !w.roleReady;
  return (
    <section className="panel peer-panel lan-panel">
      <div className="panel-heading">
        <h2>局域网连接</h2>
        <span className="subtle-count">{snapshot?.peers.length ?? 0} / 2</span>
      </div>
      <div className="lan-service-row">
        <div>
          <h3>{snapshot?.localName || "这台设备"}</h3>
        </div>
        {!running && !w.roleBusy && (
          <Button
            className="secondary-button"
            disabled={disabled}
            onClick={() => void w.retryDiscovery()}
          >
            重试发现
          </Button>
        )}
      </div>
      {snapshot?.localAddresses?.length ? (
        <div className="lan-addresses">
          <h3 className="lan-group-heading">连接地址</h3>
          {snapshot.localAddresses.map((ip) => (
            <div className="lan-address-row" key={ip}>
              <code className="mono">
                {ip}:{snapshot.port}
              </code>
              <Button
                className="secondary-button"
                aria-label={"复制地址 " + ip}
                onClick={() => {
                  const address = ip + ":" + snapshot.port;
                  if (!navigator.clipboard?.writeText) {
                    setCopyMessage("请长按或选中地址复制。");
                    return;
                  }
                  void navigator.clipboard
                    .writeText(address)
                    .then(() => setCopyMessage("地址已复制"))
                    .catch(() => setCopyMessage("复制失败，请手动选择地址。"));
                }}
              >
                复制
              </Button>
            </div>
          ))}
          {copyMessage && (
            <p className="field-help" role="status">
              {copyMessage}
            </p>
          )}
        </div>
      ) : (
        running && <p className="field-help">未检测到可用地址</p>
      )}
      {snapshot?.role === "host" && snapshot.running && snapshot.pairingCode && (
        <div className="lan-confirmation">
          <h3>连接 PIN</h3>
          <output className="lan-code" aria-label="收听设备连接 PIN">
            {snapshot.pairingCode}
          </output>
          <p>在收听设备输入此 PIN。</p>
        </div>
      )}
      {error && <ErrorMessage message={error} />}
      {!w.link.native && <p className="lan-note">请打开原生应用，浏览器无法提供共享设备。</p>}
      {snapshot && snapshot.requests.length > 0 && (
        <div className="lan-group">
          <h3 className="lan-group-heading">待确认请求</h3>
          <p className="lan-note">核对设备名称和编号后接受。</p>
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
                <span className="quiet-tag">已连接</span>
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
          <p className="lan-empty">暂无连接</p>
        )}
      </div>
    </section>
  );
}

export function ClientLinkPanel({ workbench: w }: { workbench: Workbench }) {
  const [address, setAddress] = useState(w.prefs.computerAddress);
  const [addressError, setAddressError] = useState("");
  const [pairingCode, setPairingCode] = useState("");
  const validPin = /^\d{6}$/.test(pairingCode);
  const { snapshot, busy, available, error } = w.link;
  const disabled = !available || !snapshot || busy || w.audio.busy || w.roleBusy || !w.roleReady;
  const state = snapshot?.clientState ?? "idle";
  const connected = state === "connected";
  const pending = state === "connecting" || state === "awaitingApproval";
  return (
    <section className="panel lan-panel client-lan-panel">
      <div className="panel-heading">
        <h2>连接共享设备</h2>
        <StatusBadge label={linkStatusLabel(w)} />
      </div>
      <div className="lan-service-row">
        {pending ? (
          <Button
            className="secondary-button"
            disabled={disabled}
            onClick={() => void w.retryDiscovery()}
          >
            取消请求
          </Button>
        ) : (
          !snapshot?.running && (
            <Button
              className="secondary-button"
              disabled={disabled}
              onClick={() => void w.retryDiscovery()}
            >
              重试发现
            </Button>
          )
        )}
      </div>
      {error && <ErrorMessage message={error} />}
      {!w.link.native && <p className="lan-note">请打开原生应用，浏览器无法搜索或连接共享设备。</p>}
      {pending && (
        <div className="lan-confirmation" role="status">
          <h3>{state === "awaitingApproval" ? "请在共享设备确认连接" : "正在联系共享设备…"}</h3>
          {snapshot?.verificationCode && (
            <output className="lan-code" aria-label="核对编号">
              {snapshot.verificationCode}
            </output>
          )}
          <p>
            {state === "awaitingApproval"
              ? "核对两端的六位编号，然后在共享设备上接受请求。"
              : "收到共享设备回应后，会显示核对编号。"}
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
              对方的连接 PIN
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
          </div>
          <div className="lan-group">
            <h3 className="lan-group-heading">发现的共享设备</h3>
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
              <p className="lan-empty">{snapshot?.running ? "正在搜索…" : "暂无设备"}</p>
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
              端口默认 32993。
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
    </section>
  );
}
