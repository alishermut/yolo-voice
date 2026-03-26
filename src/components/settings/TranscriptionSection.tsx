import { useTranslation } from "react-i18next";
import { ModelSelector } from "../ModelSelector";
import type { AppConfig } from "../../shared/types";
import {
  inputStyles,
  sectionHeader,
} from "../ui/styles";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import { OfflineInfoCard, CloudInfoCard } from "./EngineInfoCard";

interface TranscriptionSectionProps {
  config: AppConfig;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
  setError: (error: string | null) => void;
}

export function TranscriptionSection({
  config,
  updateConfig,
}: TranscriptionSectionProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-8">
      {/* Engine toggle */}
      <div>
        <h3 className={sectionHeader}>{t("transcription.engine.heading")}</h3>
        <div className="flex gap-3 mb-2">
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
              <span className="text-sm font-medium text-text-primary">{t("transcription.engine.offlineLabel")}</span>
              <p className="text-xs text-text-muted">{t("transcription.engine.offlineDescription")}</p>
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
              <span className="text-sm font-medium text-text-primary">{t("transcription.engine.cloudLabel")}</span>
              <p className="text-xs text-text-muted">{t("transcription.engine.cloudDescription")}</p>
            </div>
          </label>
        </div>

        {/* Info card for selected engine */}
        <div className="mb-4">
          {config.transcription_mode !== "cloud"
            ? <OfflineInfoCard />
            : <CloudInfoCard provider={config.cloud_stt_provider ?? "groq"} />
          }
        </div>

        {/* Offline settings */}
        {config.transcription_mode !== "cloud" && (
          <div className="space-y-4">
            <ModelSelector />

            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-text-primary">
                  {t("transcription.offline.textCleanupLabel")}
                </span>
                <p className="text-xs text-text-muted">
                  {t("transcription.offline.textCleanupDescription")}
                </p>
              </div>
              <Switch
                checked={config.text_cleanup_enabled}
                onChange={(checked) =>
                  updateConfig({ text_cleanup_enabled: checked })
                }
                label={t("transcription.offline.textCleanupLabel")}
              />
            </div>

            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-text-primary">
                  {t("transcription.offline.numeralsLabel")}
                </span>
                <p className="text-xs text-text-muted">
                  {t("transcription.offline.numeralsDescription")}
                </p>
              </div>
              <Switch
                checked={config.numerals_enabled}
                onChange={(checked) =>
                  updateConfig({ numerals_enabled: checked })
                }
                label={t("transcription.offline.numeralsLabel")}
              />
            </div>
          </div>
        )}

        {/* Cloud settings */}
        {config.transcription_mode === "cloud" && (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-primary w-20">
                {t("transcription.cloud.providerLabel")}
              </span>
              <Select
                value={config.cloud_stt_provider ?? "groq"}
                onChange={(v) => updateConfig({ cloud_stt_provider: v })}
                options={[
                  { value: "groq", label: t("transcription.cloud.providerGroq") },
                  { value: "deepgram", label: t("transcription.cloud.providerDeepgram") },
                ]}
                className="flex-1"
              />
            </div>
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-primary w-20">
                {t("transcription.cloud.apiKeyLabel")}
              </span>
              <input
                type="password"
                value={config.cloud_stt_api_key ?? ""}
                onChange={(e) =>
                  updateConfig({ cloud_stt_api_key: e.target.value })
                }
                placeholder={t("transcription.cloud.apiKeyPlaceholder")}
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
