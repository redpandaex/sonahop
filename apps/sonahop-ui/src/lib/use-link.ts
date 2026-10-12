import { useCallback, useEffect, useRef, useState } from "react";
import type { Platform } from "./bridge";
import {
  answerLinkRequest,
  connectLink,
  disconnectLinkPeer,
  ensureNetworkAccess,
  getLinkSnapshot,
  nativeLinkAvailable,
  startLink,
  switchConnectionRole,
  stopLink,
} from "./link-bridge";
import type { LinkSnapshot } from "./link-bridge";

type LinkOperation = (publish: (value: LinkSnapshot) => void) => Promise<LinkSnapshot | void>;

export function useLink(platform: Platform, appReady: boolean) {
  const native = nativeLinkAvailable();
  const available = native && appReady && (platform === "windows" || platform === "android");
  const [snapshot, setSnapshot] = useState<LinkSnapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [operationError, setOperationError] = useState("");
  const [pollError, setPollError] = useState("");
  const currentSnapshot = useRef<LinkSnapshot | null>(null);
  const revision = useRef(0);
  const lifecycle = useRef<object | null>(null);
  const mutating = useRef(false);
  const reading = useRef(false);

  const acceptSnapshot = useCallback((value: LinkSnapshot) => {
    currentSnapshot.current = value;
    setSnapshot(value);
    setPollError("");
  }, []);

  useEffect(() => {
    const lifetime = {};
    lifecycle.current = lifetime;
    if (!available)
      return () => {
        lifecycle.current = null;
      };
    let timer: ReturnType<typeof setTimeout> | undefined;
    let visibilityRevision = 0;
    const poll = async () => {
      if (lifecycle.current !== lifetime || document.visibilityState !== "visible") return;
      const visibilityVersion = visibilityRevision;
      if (!reading.current && !mutating.current) {
        reading.current = true;
        const version = revision.current;
        const active = () =>
          lifecycle.current === lifetime &&
          revision.current === version &&
          !mutating.current &&
          visibilityRevision === visibilityVersion &&
          document.visibilityState === "visible";
        try {
          const value = await getLinkSnapshot();
          if (active()) acceptSnapshot(value);
        } catch (error) {
          if (active()) setPollError(String(error));
        } finally {
          reading.current = false;
        }
      }
      if (
        lifecycle.current === lifetime &&
        visibilityRevision === visibilityVersion &&
        document.visibilityState === "visible"
      ) {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => void poll(), 1000);
      }
    };
    const visibilityChanged = () => {
      visibilityRevision += 1;
      if (timer) clearTimeout(timer);
      timer = undefined;
      if (document.visibilityState === "visible") void poll();
    };
    document.addEventListener("visibilitychange", visibilityChanged);
    void poll();
    return () => {
      lifecycle.current = null;
      if (timer) clearTimeout(timer);
      document.removeEventListener("visibilitychange", visibilityChanged);
    };
  }, [available, acceptSnapshot]);

  const perform = useCallback(
    async (operation: LinkOperation) => {
      if (!available || !lifecycle.current || mutating.current) return false;
      const lifetime = lifecycle.current;
      const version = ++revision.current;
      const active = () => lifecycle.current === lifetime && revision.current === version;
      const publish = (value: LinkSnapshot) => {
        if (active()) acceptSnapshot(value);
      };
      mutating.current = true;
      setBusy(true);
      setOperationError("");
      try {
        const result = await operation(publish);
        const value = result ?? (await getLinkSnapshot());
        publish(value);
        return true;
      } catch (error) {
        if (active()) setOperationError(String(error));
        // A failed command can still change backend state (for example a refused connection).
        try {
          publish(await getLinkSnapshot());
        } catch {
          // Keep the command error visible until the user retries.
        }
        return false;
      } finally {
        mutating.current = false;
        if (active()) setBusy(false);
      }
    },
    [available, acceptSnapshot],
  );

  const start = useCallback(
    () =>
      perform(async () => {
        const name = await ensureNetworkAccess(platform);
        return startLink(name);
      }),
    [perform, platform],
  );
  const changeRole = useCallback(
    (role: "host" | "client") => perform(() => switchConnectionRole(role)),
    [perform],
  );
  const stop = useCallback(() => perform(stopLink), [perform]);
  const connect = useCallback(
    (address: string, port: number, pairingCode: string) =>
      perform(async (publish) => {
        if (!/^\d{6}$/.test(pairingCode)) throw new Error("请输入服务器显示的六位连接 PIN。");
        const name = await ensureNetworkAccess(platform);
        if (!currentSnapshot.current?.running) publish(await startLink(name));
        await connectLink(address, port, pairingCode);
      }),
    [perform, platform],
  );
  const answer = useCallback(
    (requestId: string, approve: boolean) => perform(() => answerLinkRequest(requestId, approve)),
    [perform],
  );
  const disconnect = useCallback(
    (peerId: string) => perform(() => disconnectLinkPeer(peerId)),
    [perform],
  );

  return {
    snapshot,
    available,
    native,
    busy,
    error: operationError || pollError || snapshot?.lastError || "",
    changeRole,
    start,
    stop,
    connect,
    answer,
    disconnect,
  };
}
