import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowClockwiseIcon } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ErrorMessage } from "./controls";
import type { Workbench } from "../lib/use-workbench";
interface CaptureApp {
  packageName: string;
  label: string;
}
export function AppAudioChoice({
  workbench: w,
  disabled,
}: {
  workbench: Workbench;
  disabled: boolean;
}) {
  const [apps, setApps] = useState<CaptureApp[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    if (w.info.browserPreview) return;
    let active = true;
    setLoading(true);
    setError("");
    invoke<{ apps: CaptureApp[] }>("plugin:sonahop-lan|list_capture_apps")
      .then((value) => {
        if (active) setApps(value.apps);
      })
      .catch((reason) => {
        if (active) setError(String(reason));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [revision, w.info.browserPreview]);
  return (
    <div className="app-audio-choice">
      <label className="field-label" htmlFor="capture-scope">
        声音范围
      </label>
      <Select
        value={w.prefs.captureScope}
        disabled={disabled}
        onValueChange={(scope) => {
          if (scope === "app" || scope === "all") w.update({ captureScope: scope });
        }}
      >
        <SelectTrigger id="capture-scope" className="device-select-trigger">
          <SelectValue />
        </SelectTrigger>
        <SelectContent className="device-select-content">
          <SelectGroup>
            <SelectItem className="device-select-item" value="app">
              单个应用
            </SelectItem>
            <SelectItem className="device-select-item" value="all">
              全部允许采集的应用
            </SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
      {w.prefs.captureScope === "app" && (
        <>
          <div className="control-title">
            <label className="field-label" htmlFor="capture-app">
              音源应用
            </label>
            <Button
              className="icon-button"
              disabled={disabled || loading || w.info.browserPreview}
              aria-label="刷新音源应用"
              onClick={() => setRevision((value) => value + 1)}
            >
              <ArrowClockwiseIcon size={17} />
            </Button>
          </div>
          <Select
            value={w.prefs.capturePackageName || "__pick_app__"}
            disabled={disabled || loading}
            onValueChange={(value) =>
              w.update({ capturePackageName: value === "__pick_app__" ? "" : value })
            }
          >
            <SelectTrigger id="capture-app" className="device-select-trigger">
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="device-select-content">
              <SelectGroup>
                <SelectItem className="device-select-item" value="__pick_app__">
                  {loading ? "正在读取应用…" : "请选择要共享声音的应用"}
                </SelectItem>
                {w.prefs.capturePackageName &&
                  !apps.some((app) => app.packageName === w.prefs.capturePackageName) && (
                    <SelectItem
                      className="device-select-item"
                      value={w.prefs.capturePackageName}
                      disabled
                    >
                      已选应用当前不可用
                    </SelectItem>
                  )}
                {apps.map((app) => (
                  <SelectItem
                    className="device-select-item"
                    key={app.packageName}
                    value={app.packageName}
                  >
                    {app.label}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
          <p className="field-help">
            仅转发所选应用的声音。系统授权窗口中可选择同一个应用；目标应用仍需允许音频采集。
          </p>
        </>
      )}
      {w.prefs.captureScope === "all" && (
        <p className="field-help">会转发所有允许采集的应用声音，系统将要求整个屏幕的共享授权。</p>
      )}
      {error && <ErrorMessage message={error} />}
    </div>
  );
}
