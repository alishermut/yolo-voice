import { useState, useEffect } from "react";
import {
  getGpuAvailable,
  getModelStatus,
  onModelStatus,
  downloadModel,
  onModelDownloadProgress,
} from "../shared/platform";

export function ModelSelector() {
  const [gpuAvailable, setGpuAvailable] = useState(false);
  const [modelStatus, setModelStatus] = useState<string>("unknown");
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [downloadStatusText, setDownloadStatusText] = useState("");

  useEffect(() => {
    checkGpu();
    checkModelStatus();

    const unlisten = onModelStatus((status) => {
      setModelStatus(status);
      if (status === "ready") {
        checkGpu();
        setDownloading(false);
      }
    });

    const unlistenProgress = onModelDownloadProgress((progress) => {
      setDownloadProgress(progress.percent);
      if (progress.status === "downloading") {
        setDownloadStatusText(
          `${progress.downloaded_mb} / ${progress.total_mb} MB`
        );
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

  const handleDownload = async () => {
    setDownloading(true);
    setDownloadProgress(0);
    setDownloadStatusText("Starting...");
    try {
      await downloadModel();
    } catch {
      setDownloading(false);
      setModelStatus("error");
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

      {/* CPU fallback notice */}
      {modelStatus === "ready" && !gpuAvailable && (
        <div className="text-xs text-yellow-500/80 bg-yellow-900/20 border border-yellow-800/30 rounded-lg px-3 py-2">
          GPU acceleration unavailable — using CPU, which may be slower.
          Ensure your GPU drivers are up to date.
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
