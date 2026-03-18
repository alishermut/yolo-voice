import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MicSelector } from "../components/MicSelector";
import { KeybindingInput } from "../components/KeybindingInput";
import { ModelSelector } from "../components/ModelSelector";
import { ProfileEditor } from "../components/ProfileEditor";
import { LLMSettings } from "../components/LLMSettings";

interface AppConfig {
  hotkey: string;
  record_mode: "hold" | "toggle";
  device_index: number;
  whisper_model: string;
  device: string;
  compute_type: string;
  language: string;
  post_processing_enabled: boolean;
  active_profile_id: string;
  llm_provider: string;
  llm_model: string;
  llm_api_key: string;
  llm_base_url: string;
  transcription_mode: string;
  cloud_stt_provider: string;
  cloud_stt_api_key: string;
  onboarding_completed: boolean;
  launch_on_startup: boolean;
  start_minimized: boolean;
}

export function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppConfig>("get_config")
      .then(setConfig)
      .catch((e) => setError(String(e)));
  }, []);

  const updateConfig = async (updates: Partial<AppConfig>) => {
    if (!config) return;
    const newConfig = { ...config, ...updates };
    try {
      await invoke("save_config_cmd", { newConfig });
      setConfig(newConfig);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="space-y-6">
      {error && (
        <div className="px-3 py-2 bg-red-900/50 border border-red-700 rounded-lg text-red-300 text-sm">
          {error}
        </div>
      )}

      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-3">
          Microphone
        </h2>
        <MicSelector
          deviceIndex={config?.device_index}
          onDeviceChange={(index) => updateConfig({ device_index: index })}
        />
      </section>

      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-3">Hotkey</h2>
        <div className="space-y-3">
          <div className="flex items-center gap-3">
            <span className="text-sm text-gray-400 w-24">Key binding</span>
            <KeybindingInput
              value={config?.hotkey ?? ""}
              onChange={(hotkey) => updateConfig({ hotkey })}
            />
          </div>
        </div>
      </section>

      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-3">
          Recording Mode
        </h2>
        <div className="space-y-2 text-sm">
          <div className="flex items-start gap-3 p-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-green-400 font-bold mt-0.5">1</span>
            <div>
              <span className="text-gray-200 font-medium">Hold to record</span>
              <p className="text-xs text-gray-500">
                Press and hold the hotkey → speak → release to stop and transcribe
              </p>
            </div>
          </div>
          <div className="flex items-start gap-3 p-3 bg-gray-800/50 border border-gray-700 rounded-lg">
            <span className="text-blue-400 font-bold mt-0.5">2</span>
            <div>
              <span className="text-gray-200 font-medium">Double-tap to toggle</span>
              <p className="text-xs text-gray-500">
                Quick double-press → recording persists → press again to stop
              </p>
            </div>
          </div>
          <p className="text-xs text-gray-500 italic">
            Both modes work automatically with the same hotkey — no need to choose.
          </p>
        </div>
      </section>

      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-3">
          Transcription Engine
        </h2>

        {/* Mode toggle */}
        <div className="flex gap-3 mb-4">
          <label
            className={`flex-1 flex items-center gap-2 p-3 rounded-lg border cursor-pointer transition-colors ${
              config?.transcription_mode === "offline"
                ? "bg-blue-600/10 border-blue-500/50"
                : "bg-gray-800/50 border-gray-700"
            }`}
          >
            <input
              type="radio"
              name="transcription_mode"
              checked={config?.transcription_mode === "offline"}
              onChange={() => updateConfig({ transcription_mode: "offline" })}
              className="accent-blue-500"
            />
            <div>
              <span className="text-sm font-medium text-gray-200">Offline</span>
              <p className="text-xs text-gray-500">Local faster-whisper</p>
            </div>
          </label>
          <label
            className={`flex-1 flex items-center gap-2 p-3 rounded-lg border cursor-pointer transition-colors ${
              config?.transcription_mode === "cloud"
                ? "bg-blue-600/10 border-blue-500/50"
                : "bg-gray-800/50 border-gray-700"
            }`}
          >
            <input
              type="radio"
              name="transcription_mode"
              checked={config?.transcription_mode === "cloud"}
              onChange={() => updateConfig({ transcription_mode: "cloud" })}
              className="accent-blue-500"
            />
            <div>
              <span className="text-sm font-medium text-gray-200">Cloud</span>
              <p className="text-xs text-gray-500">Groq / Deepgram API</p>
            </div>
          </label>
        </div>

        {/* Offline settings */}
        {config?.transcription_mode !== "cloud" && config && (
          <ModelSelector
            whisperModel={config.whisper_model}
            device={config.device}
            computeType={config.compute_type}
            onModelChange={(model, device, computeType) =>
              setConfig((prev) =>
                prev
                  ? {
                      ...prev,
                      whisper_model: model,
                      device,
                      compute_type: computeType,
                    }
                  : prev,
              )
            }
          />
        )}

        {/* Cloud settings */}
        {config?.transcription_mode === "cloud" && (
          <div className="space-y-3">
            <div className="flex items-center gap-3">
              <span className="text-sm text-gray-400 w-20">Provider</span>
              <select
                value={config?.cloud_stt_provider ?? "groq"}
                onChange={(e) =>
                  updateConfig({ cloud_stt_provider: e.target.value })
                }
                className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
              >
                <option value="groq">Groq (Whisper large-v3-turbo)</option>
                <option value="deepgram">Deepgram (Nova-2)</option>
              </select>
            </div>
            <div className="flex items-center gap-3">
              <span className="text-sm text-gray-400 w-20">API Key</span>
              <input
                type="password"
                value={config?.cloud_stt_api_key ?? ""}
                onChange={(e) =>
                  updateConfig({ cloud_stt_api_key: e.target.value })
                }
                placeholder="Enter your API key..."
                className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
              />
            </div>
          </div>
        )}
      </section>

      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-3">Language</h2>
        <select
          value={config?.language ?? "en"}
          onChange={(e) => updateConfig({ language: e.target.value })}
          className="bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
        >
          <option value="en">English</option>
          <option value="auto">Auto-detect</option>
          <option value="es">Spanish</option>
          <option value="fr">French</option>
          <option value="de">German</option>
          <option value="it">Italian</option>
          <option value="pt">Portuguese</option>
          <option value="nl">Dutch</option>
          <option value="ja">Japanese</option>
          <option value="ko">Korean</option>
          <option value="zh">Chinese</option>
          <option value="ar">Arabic</option>
          <option value="hi">Hindi</option>
          <option value="ru">Russian</option>
        </select>
      </section>

      <section>
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-lg font-semibold text-gray-200">
            LLM Post-Processing
          </h2>
          <label className="flex items-center gap-2 cursor-pointer">
            <span className="text-sm text-gray-400">
              {config?.post_processing_enabled ? "Enabled" : "Disabled"}
            </span>
            <input
              type="checkbox"
              checked={config?.post_processing_enabled ?? false}
              onChange={(e) =>
                updateConfig({ post_processing_enabled: e.target.checked })
              }
              className="accent-blue-500 w-4 h-4"
            />
          </label>
        </div>

        {config?.post_processing_enabled && (
          <div className="space-y-6">
            <div>
              <h3 className="text-sm font-semibold text-gray-300 mb-2">
                Context Profile
              </h3>
              <ProfileEditor
                activeProfileId={config.active_profile_id}
                onProfileChange={(id) =>
                  updateConfig({ active_profile_id: id })
                }
              />
            </div>

            <div>
              <h3 className="text-sm font-semibold text-gray-300 mb-2">
                LLM Provider
              </h3>
              <LLMSettings
                provider={config.llm_provider}
                model={config.llm_model}
                apiKey={config.llm_api_key}
                baseUrl={config.llm_base_url}
                onUpdate={(updates) => updateConfig(updates)}
              />
            </div>
          </div>
        )}
      </section>

      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-3">Startup</h2>
        <div className="space-y-3">
          <label className="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={config?.launch_on_startup ?? false}
              onChange={async (e) => {
                const enable = e.target.checked;
                try {
                  await invoke("set_launch_on_startup", { enable });
                  setConfig((prev) =>
                    prev ? { ...prev, launch_on_startup: enable } : prev,
                  );
                } catch (err) {
                  setError(String(err));
                }
              }}
              className="accent-blue-500 w-4 h-4"
            />
            <div>
              <span className="text-sm text-gray-200">
                Launch on Windows startup
              </span>
              <p className="text-xs text-gray-500">
                Start YOLO Voice automatically when you log in
              </p>
            </div>
          </label>

          <label className="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={config?.start_minimized ?? false}
              onChange={(e) =>
                updateConfig({ start_minimized: e.target.checked })
              }
              className="accent-blue-500 w-4 h-4"
            />
            <div>
              <span className="text-sm text-gray-200">Start minimized</span>
              <p className="text-xs text-gray-500">
                Hide the main window on launch, only show tray icon
              </p>
            </div>
          </label>
        </div>
      </section>
    </div>
  );
}
