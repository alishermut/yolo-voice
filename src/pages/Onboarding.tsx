import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MicSelector } from "../components/MicSelector";

interface OnboardingProps {
  onComplete: () => void;
}

type Step = "welcome" | "engine" | "done";

export function Onboarding({ onComplete }: OnboardingProps) {
  const [step, setStep] = useState<Step>("welcome");
  const [deviceIndex, setDeviceIndex] = useState(0);
  const [transcriptionMode, setTranscriptionMode] = useState<"offline" | "cloud">("offline");
  const [cloudProvider, setCloudProvider] = useState("groq");
  const [cloudApiKey, setCloudApiKey] = useState("");
  const [saving, setSaving] = useState(false);

  const handleFinish = async () => {
    setSaving(true);
    try {
      // Get current config, update it, and save
      const config = await invoke<Record<string, unknown>>("get_config");
      const newConfig = {
        ...config,
        device_index: deviceIndex,
        transcription_mode: transcriptionMode,
        cloud_stt_provider: cloudProvider,
        cloud_stt_api_key: cloudApiKey,
        onboarding_completed: true,
      };
      await invoke("save_config_cmd", { newConfig });
      onComplete();
    } catch (e) {
      console.error("Onboarding save error:", e);
      // Complete anyway so user isn't stuck
      onComplete();
    }
  };

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100 flex items-center justify-center p-6">
      <div className="max-w-lg w-full">
        {step === "welcome" && (
          <div className="space-y-6">
            <div className="text-center space-y-2">
              <h1 className="text-3xl font-bold">Welcome to YOLO Voice</h1>
              <p className="text-gray-400">
                Offline voice dictation for Windows. Speak naturally and your
                words appear wherever you type.
              </p>
            </div>

            <div className="space-y-3">
              <h2 className="text-lg font-semibold text-gray-200">
                Select your microphone
              </h2>
              <MicSelector
                deviceIndex={deviceIndex}
                onDeviceChange={setDeviceIndex}
              />
            </div>

            <button
              onClick={() => setStep("engine")}
              className="w-full px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
            >
              Next
            </button>
          </div>
        )}

        {step === "engine" && (
          <div className="space-y-6">
            <div className="text-center space-y-2">
              <h1 className="text-2xl font-bold">Transcription Engine</h1>
              <p className="text-gray-400">
                Choose how your speech gets transcribed.
              </p>
            </div>

            <div className="space-y-3">
              <label
                className={`flex items-start gap-3 p-4 rounded-lg border cursor-pointer transition-colors ${
                  transcriptionMode === "offline"
                    ? "bg-blue-600/10 border-blue-500/50"
                    : "bg-gray-800/50 border-gray-700 hover:border-gray-600"
                }`}
              >
                <input
                  type="radio"
                  name="engine"
                  checked={transcriptionMode === "offline"}
                  onChange={() => setTranscriptionMode("offline")}
                  className="accent-blue-500 mt-1"
                />
                <div>
                  <span className="text-sm font-medium text-gray-200">
                    Offline (Recommended)
                  </span>
                  <p className="text-xs text-gray-500 mt-1">
                    Uses faster-whisper locally. Private, no internet needed.
                    Everything is included — works out of the box.
                  </p>
                </div>
              </label>

              <label
                className={`flex items-start gap-3 p-4 rounded-lg border cursor-pointer transition-colors ${
                  transcriptionMode === "cloud"
                    ? "bg-blue-600/10 border-blue-500/50"
                    : "bg-gray-800/50 border-gray-700 hover:border-gray-600"
                }`}
              >
                <input
                  type="radio"
                  name="engine"
                  checked={transcriptionMode === "cloud"}
                  onChange={() => setTranscriptionMode("cloud")}
                  className="accent-blue-500 mt-1"
                />
                <div>
                  <span className="text-sm font-medium text-gray-200">
                    Cloud
                  </span>
                  <p className="text-xs text-gray-500 mt-1">
                    Uses Groq or Deepgram API. Fast, no GPU needed. Requires
                    API key and internet.
                  </p>
                </div>
              </label>
            </div>

            {transcriptionMode === "cloud" && (
              <div className="space-y-3 pl-2">
                <div className="flex items-center gap-3">
                  <span className="text-sm text-gray-400 w-20">Provider</span>
                  <select
                    value={cloudProvider}
                    onChange={(e) => setCloudProvider(e.target.value)}
                    className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
                  >
                    <option value="groq">Groq (Whisper)</option>
                    <option value="deepgram">Deepgram (Nova-2)</option>
                  </select>
                </div>
                <div className="flex items-center gap-3">
                  <span className="text-sm text-gray-400 w-20">API Key</span>
                  <input
                    type="password"
                    value={cloudApiKey}
                    onChange={(e) => setCloudApiKey(e.target.value)}
                    placeholder="Enter your API key..."
                    className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
                  />
                </div>
              </div>
            )}

            <div className="flex gap-3">
              <button
                onClick={() => setStep("welcome")}
                className="px-4 py-3 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg font-medium transition-colors"
              >
                Back
              </button>
              <button
                onClick={handleFinish}
                disabled={saving}
                className="flex-1 px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors disabled:opacity-50"
              >
                {saving ? "Setting up..." : "Get Started"}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
