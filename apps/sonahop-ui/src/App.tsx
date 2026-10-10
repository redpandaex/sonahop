import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import "./App.css";

type Info = { platform: string; version: string; uiPreview: boolean };
type Device = { id: string; name: string; isDefault: boolean };

export default function App() {
  const [info, setInfo] = useState<Info | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!isTauri()) return;
    invoke<Info>("get_app_info")
      .then(async (value) => {
        setInfo(value);
        if (value.platform === "windows") {
          setDevices(await invoke<Device[]>("list_playback_devices"));
        }
      })
      .catch((reason: unknown) => setError(String(reason)));
  }, []);
  return (
    <main>
      <h1>SonaHop</h1>
      <p>界面预览 · {info ? info.platform : "浏览器"} · {info?.version ?? "0.1.0"}</p>
      {error && <p role="alert">{error}</p>}
      <ul>{devices.map((device) => <li key={device.id}>{device.name}{device.isDefault ? " · 默认" : ""}</li>)}</ul>
    </main>
  );
}
