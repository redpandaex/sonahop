export const CONTROL_PORT = 32993;

export function parseLinkAddress(value: string): { address: string; port: number } {
  const match = /^(\d{1,3}(?:\.\d{1,3}){3})$/.exec(value.trim());
  if (!match) throw new Error("请输入 IPv4 地址，例如 192.168.1.10。");
  const octets = match[1].split(".").map(Number);
  if (octets.some((part) => part > 255)) {
    throw new Error("IPv4 地址无效，每一段应为 0–255。");
  }
  return { address: octets.join("."), port: CONTROL_PORT };
}

// Migrate old saved/copied endpoints to IP-only values; the application uses its fixed port.
export function normalizeStoredAddress(value: string): string {
  const address = value.trim().replace(/:\d{1,5}$/, "");
  if (!address) return "";
  try {
    return parseLinkAddress(address).address;
  } catch {
    return "";
  }
}
