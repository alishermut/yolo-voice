import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ModelSelector } from "../ModelSelector";
import type {
  AppConfig,
  SettingsExperienceMode,
  SettingsPresetDefinition,
  StorageOverview,
} from "../../shared/types";
import { openStorageLocation } from "../../shared/platform";
import {
  inputStyles,
  buttonVariants,
  sectionHeader,
} from "../ui/styles";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";
import { CloudInfoCard } from "./EngineInfoCard";
import { TrustCard } from "./TrustCard";
import { ConfigTextInput } from "./ConfigTextInput";

interface TranscriptionSectionProps {
  addToast: (message: string, type?: "success" | "error" | "info") => void;
  config: AppConfig;
  settingsMode: SettingsExperienceMode;
  storageOverview: StorageOverview | null;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
  setError: (error: string | null) => void;
}

export function TranscriptionSection({
  addToast,
  config,
  settingsMode,
  storageOverview,
  updateConfig,
}: TranscriptionSectionProps) {
  const { t } = useTranslation();
  const [trustMessage, setTrustMessage] = useState<{
    tone: "error" | "info" | "success";
    text: string;
  } | null>(null);
  const [applyingPresetId, setApplyingPresetId] = useState<string | null>(null);
  const currentModelPath =
    config.offline_engine === "distil_whisper"
      ? storageOverview?.distil_whisper_models_dir
      : storageOverview?.parakeet_models_dir;
  const cloudProviderName =
    config.cloud_stt_provider === "deepgram" ? "Deepgram" : "Groq";
  const effectiveModeLabel =
    config.transcription_mode === "offline"
      ? t("trust.badge.local", { defaultValue: "Local" })
      : t("trust.badge.cloudVia", {
          defaultValue: "Cloud via {{provider}}",
          provider: cloudProviderName,
        });

  const handleOpenLocation = async (kind: "models" | "app_data", label: string) => {
    try {
      setTrustMessage(null);
      await openStorageLocation(kind);
    } catch (error) {
      setTrustMessage({
        tone: "error",
        text: t("trust.message.openError", {
          defaultValue: "Couldn't open {{label}}: {{error}}",
          label,
          error: String(error),
        }),
        });
    }
  };
  const isAdvanced = settingsMode === "advanced";
  const presets: SettingsPresetDefinition[] = [
    {
      id: "fastest",
      label: t("transcription.presets.fastest.label", {
        defaultValue: "Fastest",
      }),
      description: t("transcription.presets.fastest.description", {
        defaultValue: "Parakeet with fast segmented dictation.",
      }),
      updates: {
        transcription_mode: "offline",
        offline_engine: "parakeet",
        parakeet_segmented_mode_enabled: true,
        dictation_activation_mode: "manual",
        continuous_recording_enabled: false,
      },
    },
    {
      id: "best_quality",
      label: t("transcription.presets.bestQuality.label", {
        defaultValue: "Best Quality",
      }),
      description: t("transcription.presets.bestQuality.description", {
        defaultValue: "Distil-Whisper for higher-quality English dictation.",
      }),
      updates: {
        transcription_mode: "offline",
        offline_engine: "distil_whisper",
        dictation_activation_mode: "manual",
        continuous_recording_enabled: false,
      },
    },
    {
      id: "hands_free",
      label: t("transcription.presets.handsFree.label", {
        defaultValue: "Hands-Free",
      }),
      description: t("transcription.presets.handsFree.description", {
        defaultValue: "Parakeet segmented mode with voice activation.",
      }),
      updates: {
        transcription_mode: "offline",
        offline_engine: "parakeet",
        parakeet_segmented_mode_enabled: true,
        dictation_activation_mode: "voice_activated",
        continuous_recording_enabled: false,
      },
    },
    {
      id: "coding",
      label: t("transcription.presets.coding.label", {
        defaultValue: "Coding",
      }),
      description: t("transcription.presets.coding.description", {
        defaultValue: "Parakeet with rawer output and spoken punctuation.",
      }),
      updates: {
        transcription_mode: "offline",
        offline_engine: "parakeet",
        parakeet_segmented_mode_enabled: false,
        dictation_activation_mode: "manual",
        text_cleanup_enabled: false,
        spoken_punctuation_enabled: true,
        numerals_enabled: false,
      },
    },
  ];

  const handleApplyPreset = async (preset: SettingsPresetDefinition) => {
    setApplyingPresetId(preset.id);
    try {
      await updateConfig(preset.updates);
      addToast(
        t("transcription.presets.applied", {
          defaultValue: "Applied {{preset}} preset.",
          preset: preset.label,
        }),
        "success",
      );
    } finally {
      setApplyingPresetId(null);
    }
  };

  return (
    <div className="space-y-8">
      <div>
        <h3 className={sectionHeader}>{t("transcription.presets.heading", {
          defaultValue: "Recommended presets",
        })}</h3>
        <div className="grid gap-3 sm:grid-cols-2">
          {presets.map((preset) => (
            <button
              key={preset.id}
              onClick={() => handleApplyPreset(preset)}
              disabled={applyingPresetId !== null}
              className={`text-left p-4 rounded-lg border border-border-default bg-bg-raised hover:border-border-hover transition-colors disabled:opacity-60 ${buttonVariants.secondary}`}
            >
              <div className="space-y-1">
                <p className="text-sm font-medium text-text-primary">
                  {preset.label}
                </p>
                <p className="text-xs text-text-muted">
                  {applyingPresetId === preset.id
                    ? t("transcription.presets.applying", {
                        defaultValue: "Applying...",
                      })
                    : preset.description}
                </p>
              </div>
            </button>
          ))}
        </div>
      </div>

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

        {config.transcription_mode === "cloud" && (
          <div className="mb-4">
            <CloudInfoCard provider={config.cloud_stt_provider ?? "groq"} />
          </div>
        )}

        <TrustCard
          title={t("transcription.trust.title", {
            defaultValue: "Storage & privacy",
          })}
          badgeLabel={effectiveModeLabel}
          badgeTone={config.transcription_mode === "offline" ? "local" : "cloud"}
          description={[
            config.transcription_mode === "offline"
              ? t("transcription.trust.offlineLine", {
                  defaultValue:
                    "Offline transcription keeps recorded audio on this device. Local models are stored in your app data folder.",
                })
              : t("transcription.trust.cloudLine", {
                  defaultValue:
                    "Cloud transcription sends recorded audio directly to {{provider}} for transcription.",
                  provider: cloudProviderName,
                }),
            t("transcription.trust.historyLine", {
              defaultValue:
                "Transcript history is still stored locally on this device after transcription finishes.",
            }),
          ]}
          paths={[
            {
              label: t("trust.path.models", {
                defaultValue: "Models folder",
              }),
              value:
                currentModelPath ||
                storageOverview?.models_dir ||
                t("trust.value.unavailable", { defaultValue: "Unavailable" }),
            },
            {
              label: t("trust.path.historyDb", {
                defaultValue: "History database",
              }),
              value:
                storageOverview?.transcript_history_db_path ||
                t("trust.value.unavailable", { defaultValue: "Unavailable" }),
            },
          ]}
          actions={[
            {
              label: t("trust.action.openModels", {
                defaultValue: "Open models folder",
              }),
              onClick: () =>
                handleOpenLocation(
                  "models",
                  t("trust.action.openModels", {
                    defaultValue: "Open models folder",
                  }),
                ),
            },
            {
              label: t("trust.action.openAppData", {
                defaultValue: "Open app data folder",
              }),
              onClick: () =>
                handleOpenLocation(
                  "app_data",
                  t("trust.action.openAppData", {
                    defaultValue: "Open app data folder",
                  }),
                ),
            },
          ]}
          message={trustMessage}
        />

        {/* Offline settings */}
        {config.transcription_mode !== "cloud" && (
          <div className="space-y-4">
            <ModelSelector
              config={config}
              settingsMode={settingsMode}
              updateConfig={updateConfig}
            />

            {isAdvanced &&
              (config.offline_engine === "parakeet" ||
                config.offline_engine === "parakeet_en") && (
              <div className="flex items-center justify-between">
                <div>
                  <span className="text-sm font-medium text-text-primary">
                    {t("transcription.offline.parakeetSegmentedLabel", {
                      defaultValue: "Fast segmented mode",
                    })}
                  </span>
                  <p className="text-xs text-text-muted">
                    {t("transcription.offline.parakeetSegmentedDescription", {
                      defaultValue:
                        "Uses VAD and segmented transcription for much faster response, but can reduce quality on natural dictation.",
                    })}
                  </p>
                </div>
                <Switch
                  checked={config.parakeet_segmented_mode_enabled}
                  onChange={(checked) =>
                    updateConfig({ parakeet_segmented_mode_enabled: checked })
                  }
                  label={t("transcription.offline.parakeetSegmentedLabel", {
                    defaultValue: "Fast segmented mode",
                  })}
                />
              </div>
            )}

            {isAdvanced && (
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
            )}

            {isAdvanced && (
            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-text-primary">
                  {t("transcription.offline.numeralsLabel")}
                </span>
                <p className="text-xs text-text-muted">
                  {config.offline_engine === "distil_whisper"
                    ? t("transcription.offline.distilNumeralsDescription", {
                        defaultValue:
                          "Optional for Distil-Whisper. Leave it off to preserve raw wording, or enable it if you prefer digit-style output.",
                      })
                    : t("transcription.offline.numeralsDescription")}
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
            )}

            {isAdvanced && (
            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-text-primary">
                  {t("transcription.offline.hallucinationFilterLabel")}
                </span>
                <p className="text-xs text-text-muted">
                  {t("transcription.offline.hallucinationFilterDescription")}
                </p>
              </div>
              <Switch
                checked={config.hallucination_filter_enabled}
                onChange={(checked) =>
                  updateConfig({ hallucination_filter_enabled: checked })
                }
                label={t("transcription.offline.hallucinationFilterLabel")}
              />
            </div>
            )}

            {isAdvanced && (
            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-text-primary">
                  {t("transcription.offline.spokenPunctuationLabel")}
                </span>
                <p className="text-xs text-text-muted">
                  {config.offline_engine === "distil_whisper"
                    ? t("transcription.offline.distilSpokenPunctuationDescription", {
                        defaultValue:
                          "Optional for Distil-Whisper. Leave it off for raw punctuation, or enable it if you explicitly dictate punctuation words like comma or period.",
                      })
                    : t("transcription.offline.spokenPunctuationDescription")}
                </p>
              </div>
              <Switch
                checked={config.spoken_punctuation_enabled}
                onChange={(checked) =>
                  updateConfig({ spoken_punctuation_enabled: checked })
                }
                label={t("transcription.offline.spokenPunctuationLabel")}
              />
            </div>
            )}
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
              <ConfigTextInput
                type="password"
                value={config.cloud_stt_api_key ?? ""}
                onCommit={(cloud_stt_api_key) =>
                  updateConfig({ cloud_stt_api_key })
                }
                placeholder={t("transcription.cloud.apiKeyPlaceholder")}
                className={`flex-1 ${inputStyles}`}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
