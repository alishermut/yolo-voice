import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ModelSelectorProps {
  whisperModel: string;
  device: string;
  computeType: string;
  onModelChange: (model: string, device: string, computeType: string) => void;
}

export function ModelSelector({
  whisperModel,
  device,
  computeType: _computeType,
  onModelChange,
}: ModelSelectorProps) {
  const [gpuAvailable, setGpuAvailable] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sidecarStatus, setSidecarStatus] = useState<string>("unknown");

  useEffect(() => {
    checkGpu();
    checkSidecar();

    // Poll sidecar status until it's running (it spawns in background thread)
    const interval = setInterval(async () => {
      try {
        const status = await invoke<string>("get_sidecar_status");
        setSidecarStatus(status);
        if (status === "running") {
          clearInterval(interval);
          // Re-check GPU once sidecar is ready
          checkGpu();
        }
      } catch {
        // ignore
      }
    }, 2000);

    return () => clearInterval(interval);
  }, []);

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

  const handleDeviceChange = async (newDevice: string) => {
    const newComputeType = newDevice === "cpu" ? "int8" : "float16";
    setError(null);
    try {
      await invoke("set_whisper_model", {
        model: whisperModel,
        device: newDevice,
        computeType: newComputeType,
      });
    } catch (e) {
      setError(String(e));
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

      {/* Engine status */}
      <div className="flex items-center gap-2 text-xs text-gray-500">
        <div
          className={`w-2 h-2 rounded-full ${
            sidecarStatus === "running" ? "bg-green-500" : "bg-red-500"
          }`}
        />
        Transcription engine: {sidecarStatus}
        {!gpuAvailable && sidecarStatus === "running" && (
          <span className="text-yellow-500">(CPU only)</span>
        )}
      </div>

      {/* Device toggle - only show if GPU is available */}
      {gpuAvailable && (
        <div className="flex items-center gap-3">
          <span className="text-sm text-gray-400 w-20">Device</span>
          <div className="flex gap-2">
            {["auto", "cuda", "cpu"].map((d) => (
              <button
                key={d}
                onClick={() => handleDeviceChange(d)}
                className={`px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors ${
                  device === d
                    ? "bg-blue-600/20 border-blue-500 text-blue-300"
                    : "bg-gray-800 border-gray-700 text-gray-300 hover:border-gray-500"
                }`}
              >
                {d === "auto" ? "Auto" : d === "cuda" ? "GPU (CUDA)" : "CPU"}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
