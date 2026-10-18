import { DeviceMobileIcon, MonitorIcon } from "@phosphor-icons/react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { normalizeStoredAddress, parseLinkAddress } from "../lib/connection-address.ts";
import type { Workbench } from "../lib/use-workbench";
import { audioStatusLabel } from "./audio-controls";
import { ErrorMessage, StatusBadge } from "./controls";

export { linkStatusLabel, linkStatusTone } from "../lib/status-presentation";

import { audioStatusTone, linkStatusLabel, linkStatusTone } from "../lib/status-presentation";

export function HostLinkPanel({ workbench: w }: { workbench: Workbench }) {
  const [copyMessage, setCopyMessage] = useState("");
  const { snapshot, busy, available, error } = w.link;
  const running = snapshot?.running ?? false;
  const disabled =
    !available || !snapshot || busy || w.audio.busy || w.sessionBusy || w.roleBusy || !w.roleReady;
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
          <h3 className="lan-group-heading">
            {w.info.platform === "windows" ? "Wi-Fi 地址" : "连接地址"}
          </h3>
          {snapshot.localAddresses.map((ip) => (
            <div className="lan-address-row" key={ip}>
              <code className="mono">{ip}</code>
              <Button
                className="secondary-button"
                aria-label={"复制地址 " + ip}
                onClick={() => {
                  const address = ip;
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
        running && (
          <p className="field-help">
            {w.info.platform === "windows" ? "未检测到 Wi-Fi 地址" : "未检测到可用地址"}
          </p>
        )
      )}
      {error && <ErrorMessage message={error} />}
      {!w.link.native && <p className="lan-note">请打开原生应用，浏览器无法提供共享设备。</p>}
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
                <StatusBadge label="已连接" tone="success" />
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
  const [address, setAddress] = useState("");
  const [addressError, setAddressError] = useState("");
  const { snapshot, busy, available, error } = w.link;
  const disabled =
    !available || !snapshot || busy || w.audio.busy || w.sessionBusy || w.roleBusy || !w.roleReady;
  const state = snapshot?.clientState ?? "idle";
  const connected = state === "connected";
  const pending = state === "connecting" || w.connectionPending;
  useEffect(() => {
    if (connected) {
      setAddress("");
      setAddressError("");
    }
  }, [connected]);
  return (
    <section className="panel lan-panel client-lan-panel">
      <div className="panel-heading">
        <h2>连接共享设备</h2>
        <StatusBadge label={linkStatusLabel(w)} tone={linkStatusTone(w)} />
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
        <p className="lan-empty" role="status">
          正在建立加密连接…
        </p>
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
              <StatusBadge label={audioStatusLabel(w)} tone={audioStatusTone(w)} />
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
          <div className="lan-group">
            <h3 className="lan-group-heading">发现的共享设备</h3>
            {snapshot?.discovered.length ? (
              snapshot.discovered.map((host) => (
                <div className="lan-device" key={host.id}>
                  <div className="lan-device-identity">
                    <MonitorIcon size={23} weight="bold" />
                    <div>
                      <h3>{host.name}</h3>
                      <p className="mono">{host.address}</p>
                    </div>
                  </div>
                  <Button
                    className="secondary-button"
                    disabled={disabled}
                    onClick={() => void w.connectAndListen(host.address, host.port)}
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
                void w.connectAndListen(target.address, target.port);
              } catch (validationError) {
                setAddressError(String(validationError));
              }
            }}
          >
            <label className="field-label" htmlFor="lan-address">
              IP 地址
            </label>
            <Input
              id="lan-address"
              aria-invalid={Boolean(addressError)}
              value={address}
              maxLength={15}
              onChange={(event) => {
                setAddress(event.currentTarget.value);
                setAddressError("");
              }}
              placeholder="192.168.1.10"
              onPaste={(event) => {
                const pasted = normalizeStoredAddress(event.clipboardData.getData("text"));
                if (pasted) {
                  event.preventDefault();
                  setAddress(pasted);
                  setAddressError("");
                }
              }}
              autoCapitalize="off"
              autoComplete="off"
              inputMode="decimal"
              spellCheck={false}
            />
            {addressError && <ErrorMessage message={addressError} />}
            <div className="lan-actions">
              <Button
                type="submit"
                className="primary-button"
                disabled={disabled || !address.trim()}
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
