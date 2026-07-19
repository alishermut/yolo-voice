import { useTranslation } from "react-i18next";
import { KeybindingInput } from "../KeybindingInput";
import type { AppConfig, SettingsExperienceMode } from "../../shared/types";
import { sectionHeader } from "../ui/styles";
import { Switch } from "../ui/Switch";

interface HotkeySectionProps {
  config: AppConfig;
  settingsMode: SettingsExperienceMode;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
}

function getVoiceActivationReason(
  config: Pick<
    AppConfig,
    "transcription_mode" | "offline_engine" | "parakeet_segmented_mode_enabled"
  >,
): "offline" | "engine" | "segmented" | null {
  if (config.transcription_mode !== "offline") {
    return "offline";
  }
  if (
    config.offline_engine !== "parakeet" &&
    config.offline_engine !== "parakeet_en"
  ) {
    return "engine";
  }
  if (!config.parakeet_segmented_mode_enabled) {
    return "segmented";
  }
  return null;
}

export function HotkeySection({
  config,
  settingsMode,
  updateConfig,
}: HotkeySectionProps) {
  const { t } = useTranslation();
  const voiceActivationReason = getVoiceActivationReason(config);
  const voiceActivatedAvailable = voiceActivationReason === null;
  const voiceActivatedSelected =
    config.dictation_activation_mode === "voice_activated";
  const isAdvanced = settingsMode === "advanced";

  return (
    <div className="space-y-8">
      <div>
        <h3 className={sectionHeader}>{t("hotkeys.dictationHotkey.heading")}</h3>
        <div className="space-y-3">
          <div className="space-y-1">
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-primary w-24">
                {t("hotkeys.dictationHotkey.label")}
              </span>
              <KeybindingInput
                value={config.hotkey ?? ""}
                onChange={(hotkey) => updateConfig({ hotkey })}
              />
            </div>
            {config.hotkey && config.command_hotkey && config.hotkey === config.command_hotkey && (
              <p className="text-xs text-warning ml-28">
                &#9888; {t("hotkeys.dictationHotkey.conflictWarning")}
              </p>
            )}
          </div>
        </div>
      </div>

      <div>
        <h3 className={sectionHeader}>{t("hotkeys.activationMode.heading")}</h3>
        <div className="space-y-3">
          <label
            className={`flex items-start gap-3 p-4 rounded-lg border cursor-pointer transition-colors ${
              config.dictation_activation_mode === "manual"
                ? "bg-accent-muted border-accent"
                : "bg-bg-raised border-border-default"
            }`}
          >
            <input
              type="radio"
              name="dictation_activation_mode"
              checked={config.dictation_activation_mode === "manual"}
              onChange={() => updateConfig({ dictation_activation_mode: "manual" })}
              className="mt-1 accent-accent"
            />
            <div>
              <span className="text-sm font-medium text-text-primary">
                {t("hotkeys.activationMode.manualLabel")}
              </span>
              <p className="text-xs text-text-muted">
                {t("hotkeys.activationMode.manualDescription")}
              </p>
            </div>
          </label>

          <label
            className={`flex items-start gap-3 p-4 rounded-lg border transition-colors ${
              voiceActivatedSelected
                ? "bg-accent-muted border-accent"
                : "bg-bg-raised border-border-default"
            } ${voiceActivatedAvailable ? "cursor-pointer" : "cursor-not-allowed opacity-60"}`}
          >
            <input
              type="radio"
              name="dictation_activation_mode"
              checked={voiceActivatedSelected}
              disabled={!voiceActivatedAvailable}
              onChange={() =>
                updateConfig({
                  dictation_activation_mode: "voice_activated",
                  continuous_recording_enabled: false,
                })
              }
              className="mt-1 accent-accent"
            />
            <div>
              <span className="text-sm font-medium text-text-primary">
                {t("hotkeys.activationMode.voiceLabel")}
              </span>
              <p className="text-xs text-text-muted">
                {t("hotkeys.activationMode.voiceDescription")}
              </p>
              {!voiceActivatedAvailable && (
                <p className="text-xs text-warning mt-2">
                  {voiceActivationReason === "offline" &&
                    t("hotkeys.activationMode.voiceRequiresOffline")}
                  {voiceActivationReason === "engine" &&
                    t("hotkeys.activationMode.voiceRequiresParakeet")}
                  {voiceActivationReason === "segmented" &&
                    t("hotkeys.activationMode.voiceRequiresSegmented")}
                </p>
              )}
            </div>
          </label>
        </div>
      </div>

      <div>
        <h3 className={sectionHeader}>{t("textActions.hotkey.heading")}</h3>
        <div className="space-y-1">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-text-primary w-24">
              {t("textActions.hotkey.label")}
            </span>
            <KeybindingInput
              value={config.command_hotkey ?? ""}
              onChange={(command_hotkey) => updateConfig({ command_hotkey })}
              chord
            />
          </div>
          {config.hotkey &&
            config.command_hotkey &&
            config.hotkey === config.command_hotkey && (
              <p className="text-xs text-warning ml-28">
                &#9888; {t("textActions.hotkey.conflictWarning")}
              </p>
            )}
        </div>
      </div>

      {isAdvanced && (
        <div>
          <h3 className={sectionHeader}>{t("hotkeys.continuous.heading")}</h3>
          <div className="flex items-center justify-between gap-4">
            <div>
              <span className="text-sm font-medium text-text-primary">
                {t("hotkeys.continuous.label")}
              </span>
              <p className="text-xs text-text-muted">
                {voiceActivatedSelected
                  ? t("hotkeys.continuous.disabledVoiceDescription")
                  : t("hotkeys.continuous.description")}
              </p>
            </div>
            <Switch
              checked={config.continuous_recording_enabled}
              disabled={voiceActivatedSelected}
              onChange={(checked) =>
                updateConfig({ continuous_recording_enabled: checked })
              }
              label={t("hotkeys.continuous.label")}
            />
          </div>
        </div>
      )}
    </div>
  );
}
