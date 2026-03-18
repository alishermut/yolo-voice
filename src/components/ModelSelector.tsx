import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ModelInfo {
  name: string;
  size_mb: number;
}

interface ModelSelectorProps {
  whisperModel: string;
  device: string;
  computeType: string;
  onModelChange: (model: string, device: string, computeType: string) => void;
}

const MODELS = [
  { id: "tiny", name: "Tiny", size: "~39 MB", desc: "Fastest, least accurate" },
  { id: "base", name: "Base", size: "~142 MB", desc: "Good balance for short phrases" },
  { id: "small", name: "Small", size: "~466 MB", desc: "Recommended for most users" },
  { id: "medium", name: "Medium", size: "~1.5 GB", desc: "High accuracy" },
  {
    id: "large-v3-turbo",
    name: "Large v3 Turbo",
    size: "~3 GB",
    desc: "Best accuracy, needs GPU",
  },
];

export function ModelSelector({
  whisperModel,
  device,
  computeType,
  onModelChange,
}: ModelSelectorProps) {
  const [downloadedModels, setDownloadedModels] = useState<string[]>([]);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [downloadPercent, setDownloadPercent] = useState(0);
  const [loading, setLoading] = useState<string | null>(null);
  const [gpuAvailable, setGpuAvailable] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sidecarStatus, setSidecarStatus] = useState<string>("unknown");

  // Fetch initial state
  useEffect(() => {
    refreshModels();
    checkGpu();
    checkSidecar();
  }, []);

  // Listen for download progress
  useEffect(() => {
    const unlisten = listen<{
      percent: number;
      model: string;
      downloaded_mb: number;
      total_mb: number;
    }>("model-download-progress", (event) => {
      setDownloadPercent(event.payload.percent);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const refreshModels = async () => {
    try {
      const models = await invoke<ModelInfo[]>("get_models");
      setDownloadedModels(models.map((m) => m.name));
    } catch (e) {
      // Sidecar might not be running yet
      console.error("Failed to get models:", e);
    }
  };

  const checkGpu = async () => {
    try {
      const available = await invoke<boolean>("get_gpu_available");
      setGpuAvailable(available);
    } catch {
      setGpuAvailable(false);
    }
  };

  const checkSidecar = async () => {
    try {
      const status = await invoke<string>("get_sidecar_status");
      setSidecarStatus(status);
    } catch {
      setSidecarStatus("error");
    }
  };

  const handleDownload = async (modelId: string) => {
    setError(null);
    setDownloading(modelId);
    setDownloadPercent(0);
    try {
      await invoke("download_model_cmd", { model: modelId });
      await refreshModels();
    } catch (e) {
      setError(String(e));
    } finally {
      setDownloading(null);
    }
  };

  const handleSelect = async (modelId: string) => {
    if (!downloadedModels.includes(modelId)) return;
    setError(null);
    setLoading(modelId);
    try {
      await invoke("set_whisper_model", {
        model: modelId,
        device,
        computeType,
      });
      onModelChange(modelId, device, computeType);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(null);
    }
  };

  const handleDeviceChange = async (newDevice: string) => {
    const newComputeType =
      newDevice === "cpu" ? "int8" : "float16";
    setError(null);

    // Only send to sidecar if a model is already loaded
    if (downloadedModels.includes(whisperModel)) {
      setLoading(whisperModel);
      try {
        await invoke("set_whisper_model", {
          model: whisperModel,
          device: newDevice,
          computeType: newComputeType,
        });
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(null);
      }
    }
    onModelChange(whisperModel, newDevice, newComputeType);
  };

  return (
    <div className="space-y-4">
      {error && (
        <div className="px-3 py-2 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-sm">
          {error}
        </div>
      )}

      {/* Sidecar status */}
      <div className="flex items-center gap-2 text-xs text-gray-500">
        <div
          className={`w-2 h-2 rounded-full ${
            sidecarStatus === "running"
              ? "bg-green-500"
              : "bg-red-500"
          }`}
        />
        Transcription engine: {sidecarStatus}
        {!gpuAvailable && sidecarStatus === "running" && (
          <span className="text-yellow-500">(CPU only)</span>
        )}
      </div>

      {/* Device toggle */}
      <div className="flex items-center gap-3">
        <span className="text-sm text-gray-400 w-20">Device</span>
        <div className="flex gap-2">
          {["auto", "cuda", "cpu"].map((d) => (
            <button
              key={d}
              onClick={() => handleDeviceChange(d)}
              disabled={d === "cuda" && !gpuAvailable}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors ${
                device === d
                  ? "bg-blue-600/20 border-blue-500 text-blue-300"
                  : d === "cuda" && !gpuAvailable
                    ? "bg-gray-800/50 border-gray-700 text-gray-600 cursor-not-allowed"
                    : "bg-gray-800 border-gray-700 text-gray-300 hover:border-gray-500"
              }`}
            >
              {d === "auto" ? "Auto" : d === "cuda" ? "GPU (CUDA)" : "CPU"}
            </button>
          ))}
        </div>
      </div>

      {/* Model list */}
      <div className="space-y-2">
        {MODELS.map((model) => {
          const isDownloaded = downloadedModels.includes(model.id);
          const isActive = whisperModel === model.id && isDownloaded;
          const isDownloading = downloading === model.id;
          const isLoading = loading === model.id;

          return (
            <div
              key={model.id}
              className={`flex items-center justify-between p-3 rounded-lg border transition-colors ${
                isActive
                  ? "bg-blue-600/10 border-blue-500/50"
                  : "bg-gray-800/50 border-gray-700"
              }`}
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-gray-200">
                    {model.name}
                  </span>
                  <span className="text-xs text-gray-500">{model.size}</span>
                  {isActive && (
                    <span className="text-xs bg-blue-600/30 text-blue-300 px-2 py-0.5 rounded-full">
                      Active
                    </span>
                  )}
                  {isDownloaded && !isActive && (
                    <span className="text-xs bg-green-600/20 text-green-400 px-2 py-0.5 rounded-full">
                      Downloaded
                    </span>
                  )}
                </div>
                <p className="text-xs text-gray-500 mt-0.5">{model.desc}</p>

                {/* Download progress bar */}
                {isDownloading && (
                  <div className="mt-2">
                    <div className="h-1.5 bg-gray-700 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-blue-500 rounded-full transition-all duration-300"
                        style={{ width: `${downloadPercent}%` }}
                      />
                    </div>
                    <span className="text-xs text-gray-400 mt-1">
                      {Math.round(downloadPercent)}%
                    </span>
                  </div>
                )}
              </div>

              <div className="ml-3 shrink-0">
                {!isDownloaded && !isDownloading && (
                  <button
                    onClick={() => handleDownload(model.id)}
                    className="px-3 py-1.5 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg text-xs font-medium transition-colors"
                  >
                    Download
                  </button>
                )}
                {isDownloading && (
                  <span className="text-xs text-gray-400">Downloading...</span>
                )}
                {isDownloaded && !isActive && (
                  <button
                    onClick={() => handleSelect(model.id)}
                    disabled={isLoading}
                    className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-medium transition-colors disabled:opacity-50"
                  >
                    {isLoading ? "Loading..." : "Use"}
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
