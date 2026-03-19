import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  const [sidecarSetup, setSidecarSetup] = useState<boolean | null>(null);
  const [settingUp, setSettingUp] = useState(false);
  const [setupMessage, setSetupMessage] = useState("");
  const [setupPercent, setSetupPercent] = useState(0);

  useEffect(() => {
    checkSetup();
  }, []);

  useEffect(() => {
    if (sidecarSetup === true) {
      checkGpu();
      checkSidecar();
    }
  }, [sidecarSetup]);

  // Listen for sidecar setup progress
  useEffect(() => {
    const unlisten = listen<{
      step: string;
      message: string;
      percent: number;
    }>("sidecar-setup-progress", (event) => {
      setSetupMessage(event.payload.message);
      setSetupPercent(event.payload.percent);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const checkSetup = async () => {
    try {
      const ready = await invoke<boolean>("get_sidecar_setup_status");
      setSidecarSetup(ready);
    } catch {
      setSidecarSetup(false);
    }
  };

  const handleSetup = async () => {
    setError(null);
    setSettingUp(true);
    setSetupMessage("Starting setup...");
    setSetupPercent(0);
    try {
      await invoke("setup_sidecar_cmd");
      setSidecarSetup(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setSettingUp(false);
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

  // Show setup UI if Python environment is not ready
  if (sidecarSetup === false) {
    return (
      <div className="space-y-4">
        {error && (
          <div className="px-3 py-2 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-sm">
            {error}
          </div>
        )}

        <div className="p-4 bg-yellow-900/30 border border-yellow-700/50 rounded-lg space-y-3">
          <div className="text-sm text-yellow-200 font-medium">
            Transcription Engine Setup Required
          </div>
          <p className="text-xs text-gray-400">
            YOLO Voice needs to download and set up a Python runtime for offline
            transcription. This is a one-time setup (~150 MB download).
          </p>

          {settingUp ? (
            <div className="space-y-2">
              <div className="text-xs text-gray-300">{setupMessage}</div>
              <div className="h-1.5 bg-gray-700 rounded-full overflow-hidden">
                <div
                  className="h-full bg-yellow-500 rounded-full transition-all duration-300"
                  style={{ width: `${setupPercent}%` }}
                />
              </div>
              <span className="text-xs text-gray-400">
                {Math.round(setupPercent)}%
              </span>
            </div>
          ) : (
            <button
              onClick={handleSetup}
              className="px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg text-sm font-medium transition-colors"
            >
              Set Up Transcription Engine
            </button>
          )}
        </div>
      </div>
    );
  }

  // Still checking setup status
  if (sidecarSetup === null) {
    return (
      <div className="text-sm text-gray-400">Checking setup status...</div>
    );
  }

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
