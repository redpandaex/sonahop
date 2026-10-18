import type { Workbench } from "./use-workbench";

export type StatusTone = "neutral" | "info" | "pending" | "success" | "danger";
interface Presentation {
  label: string;
  tone: StatusTone;
}

export function audioStatus(w: Workbench, lane?: "listen" | "microphone"): Presentation {
  if (!w.audio.available)
    return w.info.browserPreview
      ? { label: "浏览器布局预览", tone: "neutral" }
      : { label: "音频服务未就绪", tone: "pending" };
  const value = w.audio.snapshot;
  if (!value) return { label: "正在读取音频状态", tone: "pending" };
  const status = lane ? value[lane] : null;
  if (
    status?.lastError ||
    (lane === "listen" &&
      !status?.active &&
      !status?.waiting &&
      (w.audio.error || w.sessionError)) ||
    (!lane && (w.audio.error || w.sessionError || value.lastError))
  )
    return { label: "音频异常", tone: "danger" };
  if (status?.active)
    return {
      label:
        lane === "listen"
          ? value.role === "host"
            ? value.sendSource === "microphone"
              ? "正在共享麦克风"
              : "正在共享声音"
            : "正在收听"
          : value.role === "host"
            ? "接收远端麦克风"
            : "正在上传麦克风",
      tone: "success",
    };
  if (status?.waiting)
    return {
      label:
        lane === "listen"
          ? value.role === "host"
            ? "等待收听设备"
            : "等待音源就绪"
          : value.role === "host"
            ? "等待远端麦克风"
            : "等待麦克风授权",
      tone: "pending",
    };
  if (!lane) {
    if (value.listen.active && value.microphone.active)
      return {
        label: value.role === "host" ? "共享中 · 接收麦克风" : "收听中 · 上传麦克风",
        tone: "success",
      };
    if (value.listen.active) return audioStatus(w, "listen");
    if (value.microphone.active) return audioStatus(w, "microphone");
    if (value.listen.waiting) return audioStatus(w, "listen");
    if (value.microphone.waiting) return audioStatus(w, "microphone");
  }
  if ((!lane || lane === "listen") && (w.sessionBusy || w.connectionPending || w.roleBusy))
    return { label: w.role === "host" ? "正在准备共享" : "正在连接音源", tone: "pending" };
  if ((!lane || lane === "listen") && w.sharePaused && w.role === "host")
    return { label: "共享已暂停", tone: "neutral" };
  return { label: "音频已停止", tone: "neutral" };
}
export const audioStatusLabel = (w: Workbench, lane?: "listen" | "microphone") =>
  audioStatus(w, lane).label;
export const audioStatusTone = (w: Workbench, lane?: "listen" | "microphone") =>
  audioStatus(w, lane).tone;

export function linkStatus(w: Workbench): Presentation {
  if (!w.link.native) return { label: "原生应用才能连接", tone: "neutral" };
  if (w.link.error || w.roleError) return { label: "连接异常", tone: "danger" };
  if (!w.link.available || !w.link.snapshot) return { label: "正在准备连接", tone: "pending" };
  if (w.roleBusy || w.connectionPending || w.link.snapshot.clientState === "connecting")
    return { label: "正在连接", tone: "pending" };
  if (w.link.snapshot.peers.length > 0) return { label: "已连接", tone: "success" };
  if (w.link.snapshot.running)
    return { label: w.link.snapshot.role === "host" ? "可被发现" : "正在搜索", tone: "info" };
  return { label: "连接已关闭", tone: "neutral" };
}
export const linkStatusLabel = (w: Workbench) => linkStatus(w).label;
export const linkStatusTone = (w: Workbench) => linkStatus(w).tone;
