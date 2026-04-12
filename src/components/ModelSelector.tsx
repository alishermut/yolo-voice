import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AppConfig, DistilWhisperModelStatus } from "../shared/types";
import {
  cancelModelDownload,
  deleteDistilWhisperModel,
  deleteModel,
  downloadDistilWhisperModel,
  downloadModel,
  getDistilWhisperModelStatus,
  getGpuAvailable,
  getModelStatus,
  onModelDownloadProgress,
  onModelStatus,
  onDistilWhisperStatus,
  openDistilWhisperModelPage,
  prepareDistilWhisperModel,
  reloadDistilWhisperModel,
  reloadModel,
} from "../shared/platform";
import { OfflineInfoCard } from "./settings/EngineInfoCard";
import { Select } from "./ui/Select";

interface ModelSelectorProps {
  config: AppConfig;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
}

export function ModelSelector({ config, updateConfig }: ModelSelectorProps) {
  const { t } = useTranslation();
  const [gpuAvailable, setGpuAvailable] = useState(false);
  const [parakeetStatus, setParakeetStatus] = useState<string>("unknown");
  const [distilStatus, setDistilStatus] = useState<DistilWhisperModelStatus | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [initializing, setInitializing] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [downloadStatusText, setDownloadStatusText] = useState("");
  const [downloadSpeed, setDownloadSpeed] = useState("");
  const [downloadEta, setDownloadEta] = useState("");
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [distilBusy, setDistilBusy] =
    useState<"download" | "prepare" | "reload" | "delete" | "open" | null>(null);
  const [distilError, setDistilError] = useState<string | null>(null);

  const selectedEngine =
    config.offline_engine === "distil_whisper" ? "distil_whisper" : "parakeet";

  const refreshParakeetGpu = async () => {
    try {
      setGpuAvailable(await getGpuAvailable());
    } catch {
      setGpuAvailable(false);
    }
  };

  const refreshParakeetStatus = async () => {
    try {
      setParakeetStatus(await getModelStatus());
    } catch {
      setParakeetStatus("error");
    }
    await refreshParakeetGpu();
  };

  const refreshDistilStatus = async () => {
    try {
      setDistilStatus(await getDistilWhisperModelStatus());
    } catch (err) {
      setDistilStatus({
        status: "error",
        downloaded: false,
        ready: false,
        gpu_available: false,
        runtime: "transformers-distil-whisper",
        message: String(err),
      });
    }
  };

  useEffect(() => {
    refreshParakeetStatus().catch(() => {});
    refreshDistilStatus().catch(() => {});

    const unlisten = onModelStatus((status) => {
      setParakeetStatus(status);
      if (status === "ready") {
        refreshParakeetGpu().catch(() => {});
        setDownloading(false);
        setInitializing(false);
        setReloading(false);
      } else if (status === "error") {
        setDownloading(false);
        setInitializing(false);
        setReloading(false);
      }
    });

    const unlistenProgress = onModelDownloadProgress((progress) => {
      if (progress.status === "downloading") {
        setDownloadProgress(progress.percent);
        const dlMB = (progress.downloaded_bytes / 1_048_576).toFixed(0);
        const totalMB = (progress.total_bytes / 1_048_576).toFixed(0);
        setDownloadStatusText(`${dlMB} / ${totalMB} MB`);
        if (progress.speed_bytes_per_sec && progress.speed_bytes_per_sec > 0) {
          const speed = progress.speed_bytes_per_sec;
          setDownloadSpeed(
            speed >= 1_048_576
              ? `${(speed / 1_048_576).toFixed(1)} MB/s`
              : `${(speed / 1024).toFixed(0)} KB/s`,
          );
        }
        if (progress.eta_seconds !== undefined && progress.eta_seconds > 0) {
          const s = progress.eta_seconds;
          setDownloadEta(
            s < 60
              ? t("model.download.remainingSeconds", { seconds: s })
              : t("model.download.remainingMinutes", {
                  minutes: Math.floor(s / 60),
                  seconds: s % 60,
                }),
          );
        } else {
          setDownloadEta("");
        }
      } else if (progress.status === "complete") {
        setDownloadProgress(100);
        setDownloadStatusText(t("model.download.complete"));
        setDownloadSpeed("");
        setDownloadEta("");
      } else if (progress.status === "initializing") {
        setDownloading(false);
        setInitializing(true);
        setDownloadStatusText(t("model.download.loadingMemory"));
        setDownloadSpeed("");
        setDownloadEta("");
      } else if (progress.status === "error") {
        setDownloading(false);
        setInitializing(false);
        setParakeetStatus("error");
      }
    });

    const unlistenDistil = onDistilWhisperStatus((status) => {
      setDistilStatus((current) =>
        current
          ? {
              ...current,
              status,
              ready: status === "ready",
            }
          : {
              status,
              downloaded: status !== "not-downloaded",
              ready: status === "ready",
              gpu_available: false,
              runtime: "transformers-distil-whisper",
              message: null,
            },
      );
      if (status === "preparing") {
        setDistilBusy("prepare");
      } else {
        setDistilBusy((current) => (current === "prepare" ? null : current));
        refreshDistilStatus().catch(() => {});
      }
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
      unlistenDistil.then((fn) => fn());
    };
  }, [t]);

  useEffect(() => {
    if (selectedEngine !== "distil_whisper") {
      return;
    }

    refreshDistilStatus().catch(() => {});

    if (
      !distilStatus?.downloaded ||
      distilStatus.ready ||
      distilBusy ||
      distilStatus.status === "error"
    ) {
      return;
    }

    if (distilStatus.status !== "preparing") {
      handleDistilAction("prepare", prepareDistilWhisperModel).catch(() => {});
    }
  }, [
    selectedEngine,
    distilStatus?.downloaded,
    distilStatus?.ready,
    distilStatus?.status,
    distilBusy,
  ]);

  useEffect(() => {
    if (selectedEngine !== "distil_whisper") {
      return;
    }
    if (
      !distilStatus?.downloaded ||
      distilStatus.ready ||
      distilStatus.status !== "preparing"
    ) {
      return;
    }

    const timer = window.setInterval(() => {
      refreshDistilStatus().catch(() => {});
    }, 2000);

    return () => window.clearInterval(timer);
  }, [selectedEngine, distilStatus?.downloaded, distilStatus?.ready, distilStatus?.status]);

  const handleParakeetDownload = () => {
    setDownloading(true);
    setInitializing(false);
    setDownloadProgress(0);
    setDownloadSpeed("");
    setDownloadEta("");
    setDownloadStatusText(t("model.download.starting"));

    downloadModel().catch(() => {
      setDownloading(false);
      setParakeetStatus("error");
    });
  };

  const handleDistilAction = async (
    action: "download" | "prepare" | "reload" | "delete" | "open",
    runner: () => Promise<unknown>,
  ) => {
    setDistilBusy(action);
    setDistilError(null);
    try {
      await runner();
      await refreshDistilStatus();
    } catch (err) {
      setDistilError(String(err));
    } finally {
      setDistilBusy(null);
    }
  };

  const parakeetStatusColor =
    parakeetStatus === "ready"
      ? "bg-success"
      : parakeetStatus === "loading"
        ? "bg-warning"
        : parakeetStatus === "not-downloaded"
          ? "bg-text-muted"
          : "bg-error";

  const parakeetStatusText =
    parakeetStatus === "ready"
      ? t("model.status.ready")
      : parakeetStatus === "loading"
        ? t("model.status.loading")
        : parakeetStatus === "not-downloaded"
          ? t("model.status.notDownloaded")
          : parakeetStatus === "error"
            ? t("model.status.error")
            : t("model.status.notReady");

  const distilStatusColor =
    distilStatus?.ready
      ? "bg-success"
      : distilStatus?.status === "preparing"
        ? "bg-accent"
      : distilStatus?.downloaded
        ? "bg-warning"
        : distilStatus?.status === "error"
          ? "bg-error"
          : "bg-text-muted";

  const offlineOptions = [
    {
      value: "parakeet",
      label: t("transcription.offline.parakeetLabel", {
        defaultValue: "Parakeet (Default)",
      }),
    },
    {
      value: "distil_whisper",
      label: t("transcription.offline.distilLabel", {
        defaultValue: "Distil-Whisper",
      }),
    },
  ];
  const distilOnGpu = Boolean(
    distilStatus?.device && distilStatus.device.toLowerCase().startsWith("cuda"),
  );
  const distilGpuAvailable = Boolean(distilStatus?.gpu_available);
  const showDistilAccelerationRow =
    Boolean(distilStatus?.downloaded) &&
    (Boolean(distilStatus?.ready) ||
      distilBusy === "reload" ||
      (distilStatus?.status === "preparing" && Boolean(distilStatus?.device)));

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary w-24">
            {t("transcription.offline.modelLabel", { defaultValue: "Offline model" })}
          </span>
          <Select
            value={selectedEngine}
            onChange={(value) => updateConfig({ offline_engine: value })}
            options={offlineOptions}
            className="flex-1"
          />
        </div>

        <div className="rounded-lg border border-border-default bg-bg-raised p-3 space-y-3">
          {selectedEngine === "parakeet" ? (
            <>
              <div className="space-y-1">
                <p className="text-sm font-medium text-text-primary">
                  {t("transcription.offline.parakeetLabel", { defaultValue: "Parakeet (Default)" })}
                </p>
                <p className="text-xs text-text-muted">
                  {t("transcription.offline.parakeetDescription", {
                    defaultValue:
                      "Fast, broad-device offline dictation with auto language detection.",
                  })}
                </p>
              </div>

              <div className="flex items-center gap-2 text-xs text-text-muted">
                <div className={`w-2 h-2 rounded-full ${parakeetStatusColor}`} />
                {t("model.status.prefix")} {parakeetStatusText}
                {parakeetStatus === "ready" && (
                  <span className={gpuAvailable ? "text-success" : "text-warning"}>
                    ({gpuAvailable ? t("model.status.gpuAccelerated") : t("model.status.cpu")})
                  </span>
                )}
              </div>

              <div className="text-xs text-text-secondary bg-bg-base border border-border-default rounded-lg px-3 py-2">
                {config.parakeet_segmented_mode_enabled
                  ? t("transcription.offline.parakeetSegmentedSummary", {
                      defaultValue:
                        "Fast segmented mode is on. Parakeet uses VAD-based batching for much faster response.",
                    })
                  : t("transcription.offline.parakeetOneShotSummary", {
                      defaultValue:
                        "Fast segmented mode is off. Parakeet waits until stop, then transcribes the whole clip for cleaner wording.",
                    })}
              </div>

              {parakeetStatus === "ready" && !reloading && (
                <div className="text-xs text-text-secondary bg-bg-base border border-border-default rounded-lg px-3 py-2 flex items-center justify-between">
                  <span>
                    {gpuAvailable ? t("model.acceleration.gpuLabel") : t("model.acceleration.cpuLabel")}
                  </span>
                  <button
                    onClick={() => {
                      setReloading(true);
                      reloadModel(!gpuAvailable).catch(() => setReloading(false));
                    }}
                    className="text-accent hover:text-accent-hover ml-2 transition-colors"
                  >
                    {gpuAvailable ? t("model.acceleration.switchToCpu") : t("model.acceleration.switchToGpu")}
                  </button>
                </div>
              )}

              {reloading && (
                <div className="text-xs text-accent bg-accent-muted border border-accent/30 rounded-lg px-3 py-2">
                  {t("model.reloading")}
                </div>
              )}

              {parakeetStatus === "ready" && !downloading && !confirmingDelete && (
                <div className="text-xs text-text-secondary bg-bg-base border border-border-default rounded-lg px-3 py-2 flex items-center justify-between">
                  <span>{t("model.diskSize")}</span>
                  <button
                    onClick={() => setConfirmingDelete(true)}
                    className="text-text-muted hover:text-error ml-2 transition-colors"
                  >
                    {t("model.deleteModel")}
                  </button>
                </div>
              )}

              {confirmingDelete && (
                <div className="text-xs text-error bg-error-muted border border-error/30 rounded-lg px-3 py-2 space-y-2">
                  <p>{t("model.deleteConfirm")}</p>
                  <div className="flex gap-2 justify-end">
                    <button
                      onClick={() => setConfirmingDelete(false)}
                      className="px-2 py-1 text-text-secondary hover:text-text-primary"
                    >
                      {t("model.deleteCancel")}
                    </button>
                    <button
                      onClick={async () => {
                        await deleteModel().catch(() => {});
                        setConfirmingDelete(false);
                        refreshParakeetStatus().catch(() => {});
                      }}
                      className="px-2 py-1 bg-error hover:bg-error text-white rounded"
                    >
                      {t("model.deleteConfirmButton")}
                    </button>
                  </div>
                </div>
              )}

              {downloading && (
                <div className="space-y-1">
                  <div className="w-full bg-bg-base rounded-full h-2 overflow-hidden">
                    <div
                      className="bg-accent h-full rounded-full transition-all duration-300"
                      style={{ width: `${downloadProgress}%` }}
                    />
                  </div>
                  <p className="text-xs text-text-muted text-center">{downloadStatusText}</p>
                  {(downloadSpeed || downloadEta) && (
                    <p className="text-xs text-text-muted text-center">
                      {downloadSpeed}
                      {downloadSpeed && downloadEta ? " - " : ""}
                      {downloadEta}
                    </p>
                  )}
                  <button
                    onClick={() => cancelModelDownload().catch(() => {})}
                    className="w-full text-xs text-text-muted hover:text-text-primary transition-colors py-1"
                  >
                    {t("model.download.cancel")}
                  </button>
                </div>
              )}

              {initializing && (
                <div className="text-xs text-accent bg-accent-muted border border-accent/30 rounded-lg px-3 py-2">
                  {t("model.download.initializingMessage")}
                </div>
              )}

              {parakeetStatus === "error" && !downloading && (
                <div className="text-xs text-error bg-error-muted border border-error/30 rounded-lg px-3 py-2 flex items-center justify-between">
                  <span>{t("model.error.failedInitialize")}</span>
                  <button
                    onClick={handleParakeetDownload}
                    className="text-error underline hover:text-error ml-2"
                  >
                    {t("model.error.redownload")}
                  </button>
                </div>
              )}

              {parakeetStatus === "not-downloaded" && !downloading && (
                <div className="text-xs text-text-secondary bg-bg-base border border-border-default rounded-lg px-3 py-2 flex items-center justify-between">
                  <span>{t("model.notDownloaded.message")}</span>
                  <button
                    onClick={handleParakeetDownload}
                    className="text-accent hover:text-accent-hover underline ml-2"
                  >
                    {t("model.notDownloaded.download")}
                  </button>
                </div>
              )}
            </>
          ) : (
            <>
              <div className="space-y-1">
                <p className="text-sm font-medium text-text-primary">
                  {t("transcription.offline.distilLabel", {
                    defaultValue: "Distil-Whisper",
                  })}
                </p>
                <p className="text-xs text-text-muted">
                  {t("transcription.offline.distilDescription", {
                    defaultValue:
                      "English-focused quality option. Uses whole-clip transcription with speech compaction first.",
                  })}
                </p>
              </div>

              <div className="flex items-center gap-2 text-xs text-text-muted">
                <div className={`w-2 h-2 rounded-full ${distilStatusColor}`} />
                {t("transcription.offline.distilStatusPrefix", {
                  defaultValue: "Distil-Whisper status:",
                })}{" "}
                {distilStatus?.status ?? "unknown"}
              </div>

              <div className="text-xs text-text-secondary bg-bg-base border border-border-default rounded-lg px-3 py-2 space-y-1">
                <div>
                  {t("transcription.offline.distilQualityNote", {
                    defaultValue:
                      "Better raw English dictation quality than Parakeet, but it is slower because it transcribes after stop.",
                  })}
                </div>
                <div>
                  {t("transcription.offline.distilHardwareNote", {
                    defaultValue:
                      "Runs on CPU or GPU. GPU is strongly recommended. CPU works as a fallback, but it is significantly slower on longer clips.",
                    })}
                  </div>
                <div>
                  {t("transcription.offline.distilChunkNote", {
                    defaultValue:
                      "Whole-clip transcription stays single-pass up to 30s, then switches to native chunking.",
                  })}
                </div>
              </div>

              {distilStatus?.message && (
                <div className="text-xs text-warning bg-warning-muted border border-warning/30 rounded-lg px-3 py-2">
                  {distilStatus.message}
                </div>
              )}

              {distilError && (
                <div className="text-xs text-error bg-error-muted border border-error/30 rounded-lg px-3 py-2">
                  {distilError}
                </div>
              )}

              {showDistilAccelerationRow && (
                <div className="text-xs text-text-secondary bg-bg-base border border-border-default rounded-lg px-3 py-2 flex items-center justify-between gap-3">
                  <span>
                    {distilOnGpu
                      ? t("model.acceleration.gpuLabel")
                      : t("model.acceleration.cpuLabel")}
                    {distilStatus?.device ? ` (${distilStatus.device})` : ""}
                  </span>
                  {distilBusy === "reload" || (distilStatus?.status === "preparing" && !distilStatus?.ready) ? (
                    <span className="text-accent ml-2">
                      {t("transcription.offline.distilPreparing", {
                        defaultValue: "Switching acceleration...",
                      })}
                    </span>
                  ) : !distilOnGpu && !distilGpuAvailable ? (
                    <span className="text-warning text-right">
                      {t("transcription.offline.distilGpuUnavailable", {
                        defaultValue: "GPU unavailable. Distil-Whisper needs CUDA support in the bundled runtime.",
                      })}
                    </span>
                  ) : (
                    <button
                      onClick={() =>
                        handleDistilAction("reload", () =>
                          reloadDistilWhisperModel(!distilOnGpu),
                        )
                      }
                      className="text-accent hover:text-accent-hover ml-2 transition-colors"
                    >
                      {distilOnGpu
                        ? t("model.acceleration.switchToCpu")
                        : t("model.acceleration.switchToGpu")}
                    </button>
                  )}
                </div>
              )}

              {distilStatus?.downloaded && !distilGpuAvailable && (
                <div className="text-xs text-warning bg-warning-muted border border-warning/30 rounded-lg px-3 py-2">
                  {t("transcription.offline.distilGpuUnavailableDetail", {
                    defaultValue:
                      "Distil-Whisper GPU mode uses CUDA PyTorch, not DirectML. This installation currently only exposes CPU support for the Distil runtime, so Switch to GPU is unavailable.",
                  })}
                </div>
              )}

              {!distilStatus?.downloaded ? (
                <div className="text-xs text-text-secondary bg-bg-base border border-border-default rounded-lg px-3 py-2 flex items-center justify-between">
                  <span>
                    {t("transcription.offline.distilDownloadPrompt", {
                      defaultValue: "Download Distil-Whisper to use it for offline English dictation.",
                    })}
                  </span>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => handleDistilAction("open", openDistilWhisperModelPage)}
                      className="text-text-muted hover:text-text-primary transition-colors"
                    >
                      {t("transcription.offline.openModelPage", { defaultValue: "Open model page" })}
                    </button>
                    <button
                      onClick={() => handleDistilAction("download", downloadDistilWhisperModel)}
                      className="text-accent hover:text-accent-hover underline"
                    >
                      {t("model.notDownloaded.download")}
                    </button>
                  </div>
                </div>
              ) : (
                <div className="text-xs text-text-secondary bg-bg-base border border-border-default rounded-lg px-3 py-2 flex items-center justify-between">
                  <span>
                    {distilStatus.ready
                      ? t("transcription.offline.distilReadyMessage", {
                          defaultValue: "Distil-Whisper is loaded and ready.",
                        })
                      : distilStatus.status === "preparing"
                        ? t("transcription.offline.distilPreparingPrompt", {
                            defaultValue:
                              "Distil-Whisper is preparing in the background so it is ready for the next dictation.",
                          })
                      : distilStatus.status === "error"
                        ? t("transcription.offline.distilErrorPrompt", {
                            defaultValue:
                              "Distil-Whisper failed to prepare. Fix the runtime issue below, then retry.",
                          })
                      : t("transcription.offline.distilPreparedPrompt", {
                          defaultValue:
                            "Distil-Whisper is downloaded and will prepare automatically when selected.",
                        })}
                    {distilStatus.device ? ` (${distilStatus.device})` : ""}
                  </span>
                  <div className="flex items-center gap-2">
                    {!distilStatus.ready && distilStatus.status !== "preparing" && (
                      <button
                        onClick={() => handleDistilAction("prepare", prepareDistilWhisperModel)}
                        className="text-accent hover:text-accent-hover underline"
                      >
                        {t("transcription.offline.prepareModel", { defaultValue: "Prepare model" })}
                      </button>
                    )}
                    <button
                      onClick={() => handleDistilAction("delete", deleteDistilWhisperModel)}
                      className="text-text-muted hover:text-error transition-colors"
                    >
                      {t("model.deleteModel")}
                    </button>
                  </div>
                </div>
              )}

              {distilBusy && distilBusy !== "reload" && (
                <div className="text-xs text-accent bg-accent-muted border border-accent/30 rounded-lg px-3 py-2">
                  {distilBusy === "download"
                    ? t("transcription.offline.distilDownloading", {
                        defaultValue: "Downloading Distil-Whisper...",
                      })
                    : distilBusy === "prepare"
                      ? t("transcription.offline.distilPreparing", {
                          defaultValue: "Preparing Distil-Whisper...",
                        })
                      : distilBusy === "delete"
                        ? t("transcription.offline.distilDeleting", {
                            defaultValue: "Removing Distil-Whisper...",
                          })
                        : t("transcription.offline.distilOpening", {
                            defaultValue: "Opening model page...",
                          })}
                </div>
              )}
            </>
          )}

          <OfflineInfoCard engine={selectedEngine} />
        </div>
      </div>
    </div>
  );
}
