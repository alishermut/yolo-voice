import { useState, useCallback } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type { UpdateStatus } from "../shared/types";

export function useUpdater() {
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
        setStatus("up-to-date");
        setTimeout(() => setStatus("idle"), 5000);
      }
    } catch (e) {
      setError(String(e));
      setStatus("error");
      setTimeout(() => setStatus("idle"), 5000);
    }
  }, []);

  const installUpdate = useCallback(async () => {
    if (updateObj) {
      await updateObj.install();
      await relaunch();
    }
  }, [updateObj]);

  return { status, version, error, checkForUpdates, installUpdate };
}
