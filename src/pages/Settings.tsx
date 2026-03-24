import { useState, useEffect } from "react";
import { MicSelector } from "../components/MicSelector";
import { KeybindingInput } from "../components/KeybindingInput";
import { ModelSelector } from "../components/ModelSelector";
import { ProfileEditor } from "../components/ProfileEditor";
import { LLMSettings } from "../components/LLMSettings";
import { ReplacementRules } from "../components/ReplacementRules";
import { IndustryPackSelector } from "../components/IndustryPackSelector";
import type {
  AppConfig,
  TranscriptDiagnosticsStatus,
  UserDictionary,
} from "../shared/types";
import {
  clearTranscriptDiagnostics,
  getConfig,
  getAvailableSounds,
  getTranscriptDiagnosticsStatus,
  getUserDictionary,
  previewSound,
  saveConfig,
  saveUserDictionary,
  setLaunchOnStartup,
} from "../shared/platform";

export function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [userDict, setUserDict] = useState<UserDictionary>({
    version: 2,
    user_vocabulary: [],
    user_normalization_rules: [],
  });
  const [diagnosticsStatus, setDiagnosticsStatus] =
    useState<TranscriptDiagnosticsStatus | null>(null);
  const [vocabInput, setVocabInput] = useState("");
  const [availableSounds, setAvailableSounds] = useState<string[]>([]);

  const loadDict = () => {
    getUserDictionary()
      .then((d) => {
        setUserDict(d);
        setVocabInput(d.user_vocabulary.join(", "));
      })
      .catch(() => {});
  };

  useEffect(() => {
    getConfig()
      .then(setConfig)
      .catch((e) => setError(String(e)));
    getAvailableSounds()
      .then(setAvailableSounds)
      .catch(() => {});
    getTranscriptDiagnosticsStatus()
      .then(setDiagnosticsStatus)
      .catch(() => {});
    loadDict();
  }, []);

  const saveDict = async (dict: UserDictionary) => {
    try {
      await saveUserDictionary(dict);
      setUserDict(dict);
    } catch (e) {
      setError(String(e));
    }
  };

  const updateConfig = async (updates: Partial<AppConfig>) => {
    if (!config) return;
    const newConfig = { ...config, ...updates };
    try {
      await saveConfig(newConfig);
      setConfig(newConfig);
      if ("transcript_diagnostics_enabled" in updates) {
        setDiagnosticsStatus((prev) =>
          prev
            ? { ...prev, enabled: newConfig.transcript_diagnostics_enabled }
            : prev,
        );
      }
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const clearDiagnosticsData = async () => {
    if (!window.confirm("Delete all locally stored transcript diagnostics samples?")) {
      return;
    }

    try {
      const status = await clearTranscriptDiagnostics();
      setDiagnosticsStatus(status);
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

      {config?.show_dictionary_migration_notice && (
        <div className="px-4 py-3 bg-amber-950/50 border border-amber-700 rounded-lg text-amber-200 text-sm">
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="font-medium">Legacy dictionary reset</p>
              <p className="text-xs text-amber-300/90 mt-1">
                An older merged dictionary was backed up and reset so industry packs are scoped correctly now.
                Your active pack setting was kept, but personal terms and rules need to be re-added if they lived only
                in the old merged file.
              </p>
            </div>
            <button
              onClick={() => updateConfig({ show_dictionary_migration_notice: false })}
              className="px-2 py-1 rounded bg-amber-800/60 hover:bg-amber-700/60 text-xs text-amber-100 transition-colors"
            >
              Dismiss
            </button>
          </div>
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
              <p className="text-xs text-gray-500">Local Parakeet TDT</p>
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
          <div className="space-y-4">
            <ModelSelector />

            {/* Text cleanup toggle */}
            <label className="flex items-center justify-between cursor-pointer">
              <div>
                <span className="text-sm text-gray-300">Text cleanup</span>
                <p className="text-xs text-gray-500">
                  Remove hard fillers, fix restart stutters, and shape joined dictation into cleaner sentences
                </p>
              </div>
              <input
                type="checkbox"
                checked={config.text_cleanup_enabled}
                onChange={(e) =>
                  updateConfig({ text_cleanup_enabled: e.target.checked })
                }
                className="accent-blue-500 w-4 h-4 ml-3"
              />
            </label>
          </div>
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
        <h2 className="text-lg font-semibold text-gray-200 mb-3">
          Local Transcript Diagnostics
        </h2>
        <div className="space-y-4">
          <label className="flex items-center justify-between cursor-pointer">
            <div>
              <span className="text-sm text-gray-300">Enable local diagnostics logging</span>
              <p className="text-xs text-gray-500">
                Store text-only pipeline checkpoints on this device to help refine transcription behavior over time.
              </p>
            </div>
            <input
              type="checkbox"
              checked={config?.transcript_diagnostics_enabled ?? false}
              onChange={(e) =>
                updateConfig({ transcript_diagnostics_enabled: e.target.checked })
              }
              className="accent-blue-500 w-4 h-4 ml-3"
            />
          </label>

          <div className="p-3 bg-gray-800/50 border border-gray-700 rounded-lg text-xs text-gray-400 space-y-2">
            <p>
              Local only. Text only. No audio, clipboard contents, or window metadata are captured.
            </p>
            <p>
              Retention is capped at the most recent {diagnosticsStatus?.max_samples ?? 1000} samples.
            </p>
            {diagnosticsStatus && (
              <p className="break-all">
                Stored samples: {diagnosticsStatus.sample_count} • Database: {diagnosticsStatus.db_path}
              </p>
            )}
          </div>

          <div className="flex items-center justify-between gap-3">
            <div>
              <span className="text-sm text-gray-300">Clear stored samples</span>
              <p className="text-xs text-gray-500">
                Remove all previously captured diagnostics data without changing the current toggle.
              </p>
            </div>
            <button
              onClick={clearDiagnosticsData}
              disabled={!diagnosticsStatus || diagnosticsStatus.sample_count === 0}
              className="px-3 py-2 rounded-lg bg-gray-800 border border-gray-700 text-sm text-gray-200 hover:border-red-500 hover:text-red-300 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              Clear data
            </button>
          </div>
        </div>
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
        <h2 className="text-lg font-semibold text-gray-200 mb-3">Sounds</h2>
        <div className="space-y-3">
          <div className="flex items-center gap-3">
            <span className="text-sm text-gray-400 w-32">Start recording</span>
            <select
              value={config?.start_sound ?? "chime"}
              onChange={(e) => updateConfig({ start_sound: e.target.value })}
              className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
            >
              {availableSounds.map((s) => (
                <option key={s} value={s}>
                  {s.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())}
                </option>
              ))}
            </select>
            <button
              onClick={() =>
                previewSound(config?.start_sound ?? "chime")
              }
              className="p-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-300 hover:border-blue-500 hover:text-blue-300 transition-colors"
              title="Preview sound"
            >
              🔊
            </button>
          </div>
          <div className="flex items-center gap-3">
            <span className="text-sm text-gray-400 w-32">Stop recording</span>
            <select
              value={config?.stop_sound ?? "ding"}
              onChange={(e) => updateConfig({ stop_sound: e.target.value })}
              className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
            >
              {availableSounds.map((s) => (
                <option key={s} value={s}>
                  {s.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())}
                </option>
              ))}
            </select>
            <button
              onClick={() =>
                previewSound(config?.stop_sound ?? "ding")
              }
              className="p-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-300 hover:border-blue-500 hover:text-blue-300 transition-colors"
              title="Preview sound"
            >
              🔊
            </button>
          </div>
        </div>
      </section>

      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-3">
          Personal Dictionary & Industry Packs
        </h2>
        <div className="space-y-4">
          <div>
            <h3 className="text-sm font-semibold text-gray-300 mb-2">
              Active Industry Pack
            </h3>
            <p className="text-xs text-gray-500 mb-2">
              Pack terms and pack normalization rules apply only while this scope is active. Activating a pack does not
              copy its entries into your personal dictionary.
            </p>
            <IndustryPackSelector
              activePack={config?.active_industry_pack ?? "general"}
              onApply={() => {
                loadDict();
                getConfig().then(setConfig).catch(() => {});
              }}
            />
          </div>

          <div>
            <h3 className="text-sm font-semibold text-gray-300 mb-2">
              Personal Terms
            </h3>
            <p className="text-xs text-gray-500 mb-2">
              Canonical terms you want the app to preserve. Personal terms now create safe automatic aliases for common
              shape-based variants like <span className="font-mono">type script → TypeScript</span> and
              <span className="font-mono"> next js → Next.js</span>, but they are still not sent to the offline
              recognizer itself.
            </p>
            <p className="text-xs text-gray-500 mb-2">
              Use Personal Normalization Rules below for phonetic mismatches the app cannot infer safely, like
              <span className="font-mono"> super base → Supabase</span>.
            </p>
            <textarea
              value={vocabInput}
              onChange={(e) => {
                setVocabInput(e.target.value);
              }}
              onBlur={() => {
                const words = vocabInput
                  .split(",")
                  .map((w) => w.trim())
                  .filter((w) => w.length > 0);
                saveDict({ ...userDict, user_vocabulary: words });
              }}
              rows={3}
              placeholder="Supabase, Vercel, Kubernetes, ..."
              className="w-full bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 resize-y"
            />
            {userDict.user_vocabulary.length > 0 && (
              <p className="text-xs text-gray-500 mt-1">
                {userDict.user_vocabulary.length} personal term
                {userDict.user_vocabulary.length !== 1 ? "s" : ""}
              </p>
            )}
          </div>

          <div>
            <h3 className="text-sm font-semibold text-gray-300 mb-2">
              Personal Normalization Rules
            </h3>
            <p className="text-xs text-gray-500 mb-2">
              Deterministic corrections that apply after transcription. These stay in your personal dictionary and are
              layered on top of the currently active industry pack.
            </p>
            <ReplacementRules
              rules={userDict.user_normalization_rules}
              onChange={(rules) =>
                saveDict({ ...userDict, user_normalization_rules: rules })
              }
            />
          </div>
        </div>
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
                  await setLaunchOnStartup(enable);
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
