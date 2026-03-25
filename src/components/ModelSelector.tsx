import { useState, useEffect } from "react";
import {
  getGpuAvailable,
  getModelStatus,
  onModelStatus,
  downloadModel,
  cancelModelDownload,
  deleteModel,
  reloadModel,
  onModelDownloadProgress,
} from "../shared/platform";

export function ModelSelector() {
  const [gpuAvailable, setGpuAvailable] = useState(false);
  const [modelStatus, setModelStatus] = useState<string>("unknown");
  const [downloading, setDownloading] = useState(false);
  const [initializing, setInitializing] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [downloadStatusText, setDownloadStatusText] = useState("");
  const [downloadSpeed, setDownloadSpeed] = useState("");
  const [downloadEta, setDownloadEta] = useState("");
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [reloading, setReloading] = useState(false);

  useEffect(() => {
    checkGpu();
    checkModelStatus();

    const unlisten = onModelStatus((status) => {
      setModelStatus(status);
      if (status === "ready") {
        checkGpu();
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
              ? `${s}s remaining`
              : `${Math.floor(s / 60)}m ${s % 60}s remaining`,
          );
        } else {
          setDownloadEta("");
        }
      } else if (progress.status === "complete") {
        setDownloadProgress(100);
        setDownloadStatusText("Download complete!");
        setDownloadSpeed("");
        setDownloadEta("");
      } else if (progress.status === "initializing") {
        setDownloading(false);
        setInitializing(true);
        setDownloadStatusText("Loading model into memory...");
        setDownloadSpeed("");
        setDownloadEta("");
      } else if (progress.status === "error") {
        setDownloading(false);
        setInitializing(false);
        setModelStatus("error");
      }
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
    };
  }, []);

  const checkGpu = async () => {
    try {
      const available = await getGpuAvailable();
      setGpuAvailable(available);
    } catch {
      setGpuAvailable(false);
    }
  };

  const checkModelStatus = async () => {
    try {
      const status = await getModelStatus();
      setModelStatus(status);
    } catch {
      setModelStatus("error");
    }
  };

  const handleDownload = () => {
    setDownloading(true);
    setInitializing(false);
    setDownloadProgress(0);
    setDownloadSpeed("");
    setDownloadEta("");
    setDownloadStatusText("Starting...");

    // Fire-and-forget: command returns immediately, progress comes via events
    downloadModel().catch(() => {
      setDownloading(false);
      setModelStatus("error");
    });
  };

  const handleCancelDownload = () => {
    cancelModelDownload().catch(console.error);
    setDownloading(false);
    setDownloadProgress(0);
    setDownloadStatusText("");
    setDownloadSpeed("");
    setDownloadEta("");
  };

  const handleDelete = async () => {
    try {
      await deleteModel();
      setConfirmingDelete(false);
    } catch {
      setConfirmingDelete(false);
    }
  };

  const statusColor =
    modelStatus === "ready"
      ? "bg-green-500"
      : modelStatus === "loading"
        ? "bg-yellow-500"
        : modelStatus === "not-downloaded"
          ? "bg-gray-500"
          : "bg-red-500";

  const statusText =
    modelStatus === "ready"
      ? "Ready"
      : modelStatus === "loading"
        ? "Loading..."
        : modelStatus === "not-downloaded"
          ? "Model not downloaded"
          : modelStatus === "error"
            ? "Error"
            : "Not ready";

  return (
    <div className="space-y-2">
      {/* Engine status */}
      <div className="flex items-center gap-2 text-xs text-gray-500">
        <div className={`w-2 h-2 rounded-full ${statusColor}`} />
        Transcription engine: {statusText}
        {modelStatus === "ready" && (
          <span className={gpuAvailable ? "text-green-500" : "text-yellow-500"}>
            ({gpuAvailable ? "GPU accelerated" : "CPU"})
          </span>
        )}
      </div>

      {/* GPU toggle */}
      {modelStatus === "ready" && !reloading && (
        <div className="text-xs text-gray-400 bg-gray-800/50 border border-gray-700 rounded-lg px-3 py-2 flex items-center justify-between">
          <span>Acceleration: {gpuAvailable ? "GPU (DirectML)" : "CPU"}</span>
          <button
            onClick={() => {
              setReloading(true);
              reloadModel(!gpuAvailable).catch(() => setReloading(false));
            }}
            className="text-blue-400 hover:text-blue-300 ml-2 transition-colors"
          >
            Switch to {gpuAvailable ? "CPU" : "GPU"}
          </button>
        </div>
      )}

      {reloading && (
        <div className="text-xs text-blue-300/80 bg-blue-900/20 border border-blue-800/30 rounded-lg px-3 py-2 flex items-center gap-2">
          <svg className="animate-spin h-3.5 w-3.5 text-blue-400 shrink-0" viewBox="0 0 24 24" fill="none">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
          <span>Reloading model...</span>
        </div>
      )}

      {/* CPU fallback notice */}
      {modelStatus === "ready" && !gpuAvailable && !reloading && (
        <div className="text-xs text-yellow-500/80 bg-yellow-900/20 border border-yellow-800/30 rounded-lg px-3 py-2">
          Using CPU mode, which may be slower than GPU.
        </div>
      )}

      {/* Model ready: option to delete */}
      {modelStatus === "ready" && !downloading && !confirmingDelete && (
        <div className="text-xs text-gray-400 bg-gray-800/50 border border-gray-700 rounded-lg px-3 py-2 flex items-center justify-between">
          <span>Parakeet TDT model (~2.4 GB on disk).</span>
          <button
            onClick={() => setConfirmingDelete(true)}
            className="text-gray-500 hover:text-red-400 ml-2 transition-colors"
          >
            Delete model
          </button>
        </div>
      )}

      {/* Delete confirmation */}
      {confirmingDelete && (
        <div className="text-xs text-red-400/80 bg-red-900/20 border border-red-800/30 rounded-lg px-3 py-2 space-y-2">
          <p>Delete the speech model? This frees ~2.4 GB but you'll need to re-download it to use offline transcription.</p>
          <div className="flex gap-2 justify-end">
            <button
              onClick={() => setConfirmingDelete(false)}
              className="px-2 py-1 text-gray-400 hover:text-gray-200 transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleDelete}
              className="px-2 py-1 bg-red-600 hover:bg-red-700 text-white rounded transition-colors"
            >
              Delete
            </button>
          </div>
        </div>
      )}

      {/* Download progress */}
      {downloading && (
        <div className="space-y-1">
          <div className="w-full bg-gray-800 rounded-full h-2 overflow-hidden">
            <div
              className="bg-blue-600 h-full rounded-full transition-all duration-300"
              style={{ width: `${downloadProgress}%` }}
            />
          </div>
          <p className="text-xs text-gray-500 text-center">
            {downloadStatusText}
          </p>
          {(downloadSpeed || downloadEta) && (
            <p className="text-xs text-gray-600 text-center">
              {downloadSpeed}{downloadSpeed && downloadEta ? " \u2014 " : ""}{downloadEta}
            </p>
          )}
          <button
            onClick={handleCancelDownload}
            className="w-full text-xs text-gray-500 hover:text-gray-300 transition-colors py-1"
          >
            Cancel
          </button>
        </div>
      )}

      {/* Initializing phase */}
      {initializing && (
        <div className="text-xs text-blue-300/80 bg-blue-900/20 border border-blue-800/30 rounded-lg px-3 py-2 flex items-center gap-2">
          <svg className="animate-spin h-3.5 w-3.5 text-blue-400 shrink-0" viewBox="0 0 24 24" fill="none">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
          <span>Loading model into memory... This may take a moment.</span>
        </div>
      )}

      {/* Error: re-download action */}
      {modelStatus === "error" && !downloading && (
        <div className="text-xs text-red-400/80 bg-red-900/20 border border-red-800/30 rounded-lg px-3 py-2 flex items-center justify-between">
          <span>Engine failed to initialize.</span>
          <button
            onClick={handleDownload}
            className="text-red-300 underline hover:text-red-200 ml-2"
          >
            Re-download model
          </button>
        </div>
      )}

      {/* Not downloaded: download action */}
      {modelStatus === "not-downloaded" && !downloading && (
        <div className="text-xs text-gray-400 bg-gray-800/50 border border-gray-700 rounded-lg px-3 py-2 flex items-center justify-between">
          <span>Speech model not downloaded yet (~2.4 GB).</span>
          <button
            onClick={handleDownload}
            className="text-blue-400 underline hover:text-blue-300 ml-2"
          >
            Download
          </button>
        </div>
      )}
    </div>
  );
}
