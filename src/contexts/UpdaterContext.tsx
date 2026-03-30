import { createContext, useContext, useState, useCallback, useEffect } from "react";
import type { ReactNode } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type { UpdateStatus } from "../shared/types";

interface UpdaterContextValue {
  status: UpdateStatus;
  version: string | null;
  error: string | null;
  checkForUpdates: () => Promise<void>;
  installUpdate: () => Promise<void>;
  dismissError: () => void;
}

const UpdaterContext = createContext<UpdaterContextValue | null>(null);

export function UpdaterProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [version, setVersion] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [updateObj, setUpdateObj] = useState<Awaited<
    ReturnType<typeof check>
  > | null>(null);

  const checkForUpdates = useCallback(async () => {
    setStatus("checking");
    setError(null);
    try {
      const update = await check();
      if (update?.available) {
        setVersion(update.version);
        setStatus("downloading");
        await update.download();
        setUpdateObj(update);
        setStatus("ready");
      } else {
        setUpdateObj(null);
        setVersion(null);
        setStatus("up-to-date");
        setTimeout(() => setStatus("idle"), 5000);
      }
    } catch (e) {
      const msg = String(e);
      console.error("[updater] Check failed:", msg);
      setUpdateObj(null);
      setError(msg);
      setStatus("error");
    }
  }, []);

  const installUpdate = useCallback(async () => {
    if (!updateObj) {
      return;
    }

    setError(null);
    try {
      await updateObj.install();
      await relaunch();
    } catch (e) {
      const msg = String(e);
      console.error("[updater] Install failed:", msg);
      setError(msg);
      setStatus("error");
    }
  }, [updateObj]);

  const dismissError = useCallback(() => {
    setError(null);
    setStatus("idle");
  }, []);

  // Auto-check for updates 10s after mount to avoid blocking startup
  useEffect(() => {
    const timer = setTimeout(checkForUpdates, 10_000);
    return () => clearTimeout(timer);
  }, [checkForUpdates]);

  return (
    <UpdaterContext.Provider
      value={{ status, version, error, checkForUpdates, installUpdate, dismissError }}
    >
      {children}
    </UpdaterContext.Provider>
  );
}

export function useUpdaterContext(): UpdaterContextValue {
  const ctx = useContext(UpdaterContext);
  if (!ctx) {
    throw new Error("useUpdaterContext must be used within <UpdaterProvider>");
  }
  return ctx;
}
