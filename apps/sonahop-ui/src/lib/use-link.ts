import { useCallback, useEffect, useRef, useState } from "react";
import type { Platform } from "./bridge";
import type { LinkSnapshot } from "./link-bridge";
import {
  connectLink,
  disconnectLinkPeer,
  ensureNetworkAccess,
  getLinkSnapshot,
  nativeLinkAvailable,
  startLink,
  stopLink,
  switchConnectionRole,
} from "./link-bridge";

type LinkOperation = (publish: (value: LinkSnapshot) => void) => Promise<LinkSnapshot | void>;

export function useLink(
  platform: Platform,
  appReady: boolean,
  stopRevision = 0,
  onAction?: () => void,
) {
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
      onAction?.();
      mutating.current = true;
      setBusy(true);
      setOperationError("");
      try {
        const result = await operation(publish);
        const value = result ?? (await getLinkSnapshot());
        publish(value);
        return active();
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
    [available, acceptSnapshot, onAction],
  );

  const start = useCallback(
    () =>
      perform(async () => {
        const name = await ensureNetworkAccess(platform);
        return startLink(name, stopRevision);
      }),
    [perform, platform, stopRevision],
  );
  const changeRole = useCallback(
    (role: "host" | "client") => perform(() => switchConnectionRole(role, stopRevision)),
    [perform, stopRevision],
  );
  const stop = useCallback(() => perform(() => stopLink(stopRevision)), [perform, stopRevision]);
  const connect = useCallback(
    (address: string, port: number) =>
      perform(async (publish) => {
        const name = await ensureNetworkAccess(platform);
        if (!currentSnapshot.current?.running) publish(await startLink(name, stopRevision));
        await connectLink(address, port, stopRevision);
      }),
    [perform, platform, stopRevision],
  );
  const disconnect = useCallback(
    (peerId: string) => perform(() => disconnectLinkPeer(peerId, stopRevision)),
    [perform, stopRevision],
  );

  const cancelPending = useCallback(() => {
    revision.current += 1;
    setOperationError("");
    setBusy(false);
  }, []);
  const synchronize = useCallback(async () => {
    const lifetime = lifecycle.current;
    if (!available || !lifetime) return false;
    const version = ++revision.current;
    try {
      const value: LinkSnapshot = await getLinkSnapshot();
      if (lifecycle.current !== lifetime || revision.current !== version) return false;
      acceptSnapshot(value);
      setOperationError("");
      setBusy(false);
      return true;
    } catch {
      return false;
    }
  }, [available, acceptSnapshot]);

  return {
    cancelPending,
    synchronize,
    snapshot,
    available,
    native,
    busy,
    error: operationError || pollError || snapshot?.lastError || "",
    changeRole,
    start,
    stop,
    connect,
    disconnect,
  };
}
