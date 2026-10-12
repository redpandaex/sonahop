import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CopyIcon, ArrowClockwiseIcon, FileTextIcon } from "@phosphor-icons/react";
import { Button } from "@/components/ui/button";
import { ErrorMessage } from "./controls";
import { nativeLinkAvailable } from "../lib/link-bridge";

interface DiagnosticReport {
  logPath: string | null;
  logError: string | null;
  recentLog: string;
  version: string;
}

export function DiagnosticSettings() {
  const [report, setReport] = useState<DiagnosticReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const available = nativeLinkAvailable();

  async function refresh() {
    setBusy(true);
    setError("");
    setCopied(false);
    try {
      setReport(await invoke<DiagnosticReport>("get_diagnostics"));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  async function copy() {
    if (!report) return;
    setError("");
    setCopied(false);
    try {
      if (!navigator.clipboard?.writeText)
        throw new Error("此设备无法直接复制，请长按或选择下方日志文字复制。");
      await navigator.clipboard.writeText(
        "SonaHop " +
          report.version +
          "\n" +
          (report.logPath ?? "仅保留本次运行的最近日志") +
          "\n" +
          report.recentLog,
      );
      setCopied(true);
    } catch (reason) {
      setError(String(reason));
    }
  }

  return (
    <section className="panel settings-panel diagnostic-settings">
      <div className="panel-heading">
        <h2>连接诊断</h2>
        <FileTextIcon size={18} weight="bold" />
      </div>
      <p className="field-help">
        断开后读取日志，可查看连接结束原因、收发统计和发送耗时。日志不包含 PIN、密钥或录音。
      </p>
      <div className="diagnostic-actions">
        <Button
          className="secondary-button"
          disabled={!available || busy}
          onClick={() => void refresh()}
        >
          <ArrowClockwiseIcon size={16} weight="bold" />
          {busy ? "读取中…" : report ? "刷新日志" : "读取最近日志"}
        </Button>
        {report && (
          <Button
            className="secondary-button"
            disabled={busy || !report.recentLog}
            onClick={() => void copy()}
          >
            <CopyIcon size={16} weight="bold" />
            {copied ? "已复制" : "复制日志"}
          </Button>
        )}
      </div>
      {!available && <p className="field-help">请在 Windows 或 Android 原生应用中读取日志。</p>}
      {report && (
        <div className="diagnostic-report">
          <p className="field-help">
            {report.logPath
              ? "本地日志：" + report.logPath
              : "本地文件不可用，以下保留本次运行的最近日志。"}
          </p>
          {report.logError && <ErrorMessage message={"日志写入失败：" + report.logError} />}
          <label className="field-label" htmlFor="connection-diagnostic-log">
            最近 256 条记录 · SonaHop {report.version}
          </label>
          <textarea
            id="connection-diagnostic-log"
            className="diagnostic-log mono"
            readOnly
            spellCheck={false}
            value={report.recentLog || "当前没有诊断记录。"}
          />
        </div>
      )}
      {error && <ErrorMessage message={error} />}
    </section>
  );
}
