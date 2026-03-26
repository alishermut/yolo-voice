import { ModelSelector } from "../ModelSelector";
import type { AppConfig } from "../../shared/types";
import {
  inputStyles,
  sectionHeader,
} from "../ui/styles";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";

interface TranscriptionSectionProps {
  config: AppConfig;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
  setError: (error: string | null) => void;
}

export function TranscriptionSection({
  config,
  updateConfig,
}: TranscriptionSectionProps) {
  return (
    <div className="space-y-8">
      {/* Engine toggle */}
      <div>
        <h3 className={sectionHeader}>Engine</h3>
        <div className="flex gap-3 mb-4">
          <label
            className={`flex-1 flex items-center gap-2 p-3 rounded-lg border cursor-pointer transition-colors ${
              config.transcription_mode === "offline"
                ? "bg-accent-muted border-accent"
                : "bg-bg-raised border-border-default"
            }`}
          >
            <input
              type="radio"
              name="transcription_mode"
              checked={config.transcription_mode === "offline"}
              onChange={() => updateConfig({ transcription_mode: "offline" })}
              className="accent-accent"
            />
            <div>
              <span className="text-sm font-medium text-text-primary">Offline</span>
              <p className="text-xs text-text-muted">Local Parakeet TDT</p>
            </div>
          </label>
          <label
            className={`flex-1 flex items-center gap-2 p-3 rounded-lg border cursor-pointer transition-colors ${
              config.transcription_mode === "cloud"
                ? "bg-accent-muted border-accent"
                : "bg-bg-raised border-border-default"
            }`}
          >
            <input
              type="radio"
              name="transcription_mode"
              checked={config.transcription_mode === "cloud"}
              onChange={() => updateConfig({ transcription_mode: "cloud" })}
              className="accent-accent"
            />
            <div>
              <span className="text-sm font-medium text-text-primary">Cloud</span>
              <p className="text-xs text-text-muted">Groq / Deepgram API</p>
            </div>
          </label>
        </div>

        {/* Offline settings */}
        {config.transcription_mode !== "cloud" && (
          <div className="space-y-4">
            <ModelSelector />

            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-text-primary">
                  Text cleanup
                </span>
                <p className="text-xs text-text-muted">
                  Remove hard fillers, fix restart stutters, and shape joined
                  dictation into cleaner sentences
                </p>
              </div>
              <Switch
                checked={config.text_cleanup_enabled}
                onChange={(checked) =>
                  updateConfig({ text_cleanup_enabled: checked })
                }
                label="Text cleanup"
              />
            </div>
          </div>
        )}

        {/* Cloud settings */}
        {config.transcription_mode === "cloud" && (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-primary w-20">
                Provider
              </span>
              <Select
                value={config.cloud_stt_provider ?? "groq"}
                onChange={(v) => updateConfig({ cloud_stt_provider: v })}
                options={[
                  { value: "groq", label: "Groq (Whisper large-v3-turbo)" },
                  { value: "deepgram", label: "Deepgram (Nova-2)" },
                ]}
                className="flex-1"
              />
            </div>
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-primary w-20">
                API Key
              </span>
              <input
                type="password"
                value={config.cloud_stt_api_key ?? ""}
                onChange={(e) =>
                  updateConfig({ cloud_stt_api_key: e.target.value })
                }
                placeholder="Enter your API key..."
                className={`flex-1 ${inputStyles}`}
              />
            </div>
          </div>
        )}
      </div>

      {/* Diagnostics section hidden — dev-only, not needed in production */}
    </div>
  );
}
