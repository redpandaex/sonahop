import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Platform } from "./bridge";

export interface DiscoveredHost {
  id: string;
  name: string;
  address: string;
  port: number;
}
export interface LinkPeer {
  id: string;
  name: string;
  address: string;
}
export interface LinkSnapshot {
  role: "host" | "client";
  running: boolean;
  localName: string;
  localAddresses: string[];
  port: number;
  discovered: DiscoveredHost[];
  peers: LinkPeer[];
  clientState: "idle" | "discovering" | "connecting" | "connected";
  lastError: string | null;
}
export const nativeLinkAvailable = () => isTauri();
export const getLinkSnapshot = () => invoke<LinkSnapshot>("get_link_snapshot");
export const switchConnectionRole = (role: "host" | "client", stopRevision = 0) =>
  invoke<LinkSnapshot>("switch_connection_role", { role, stopRevision });
export const startLink = (name: string, stopRevision = 0) =>
  invoke<LinkSnapshot>("start_link", { name, stopRevision });
export const stopLink = (stopRevision = 0) => invoke<void>("stop_link", { stopRevision });
export const connectLink = (address: string, port: number, stopRevision = 0) =>
  invoke<void>("connect_link", { address, port, stopRevision });
export const disconnectLinkPeer = (peerId: string, stopRevision = 0) =>
  invoke<void>("disconnect_link_peer", { peerId, stopRevision });

export async function ensureNetworkAccess(platform: Platform): Promise<string> {
  if (platform !== "android") return "";
  const permission = await invoke<{ granted: boolean; deviceName: string }>(
    "plugin:sonahop-lan|ensure_permission",
  );
  if (!permission.granted) {
    throw new Error(
      "局域网访问未获授权。请在设备设置中允许 SonaHop 访问本地网络，再重试搜索或连接。",
    );
  }
  return permission.deviceName;
}

export { parseLinkAddress } from "./connection-address.ts";
