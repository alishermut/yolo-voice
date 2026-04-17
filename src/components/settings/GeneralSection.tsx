import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import i18n, { UI_LANGUAGES } from "../../i18n";
import { MicSelector } from "../MicSelector";
import type { AppConfig, SettingsExperienceMode } from "../../shared/types";
import {
  getAvailableSounds,
  previewSound,
  setLaunchOnStartup,
} from "../../shared/platform";
import { buttonVariants, sectionHeader } from "../ui/styles";
import { Select } from "../ui/Select";
import { Switch } from "../ui/Switch";

interface GeneralSectionProps {
  config: AppConfig;
  settingsMode: SettingsExperienceMode;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
  setConfig: React.Dispatch<React.SetStateAction<AppConfig | null>>;
  setError: (error: string | null) => void;
}

export function GeneralSection({
  config,
  settingsMode,
  updateConfig,
  setConfig,
  setError,
}: GeneralSectionProps) {
  const { t } = useTranslation();
  const [availableSounds, setAvailableSounds] = useState<string[]>([]);
  const isAdvanced = settingsMode === "advanced";

  useEffect(() => {
    getAvailableSounds()
      .then(setAvailableSounds)
      .catch(() => {});
  }, []);

  return (
    <div className="space-y-8">
      {/* Language */}
      <div>
        <h3 className={sectionHeader}>{t("general.language.label")}</h3>
        <Select
          value={config.ui_language ?? "en"}
          onChange={(v) => {
            updateConfig({ ui_language: v });
            i18n.changeLanguage(v);
          }}
          options={UI_LANGUAGES.map((l) => ({ value: l.code, label: l.name }))}
        />
      </div>

      {/* Microphone */}
      <div>
        <h3 className={sectionHeader}>{t("general.microphone.heading")}</h3>
        <MicSelector
          deviceIndex={config.device_index}
          onDeviceChange={(index) => updateConfig({ device_index: index })}
        />
      </div>

      {/* Sounds */}
      <div>
        <h3 className={sectionHeader}>{t("general.sounds.heading")}</h3>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <span className="text-sm font-medium text-text-primary">
                {t("general.sounds.enableLabel")}
              </span>
              <p className="text-xs text-text-muted">
                {t("general.sounds.enableDescription")}
              </p>
            </div>
            <Switch
              checked={config.sounds_enabled ?? true}
              onChange={(checked) =>
                updateConfig({ sounds_enabled: checked })
              }
              label={t("general.sounds.enableLabel")}
            />
          </div>
          {config.sounds_enabled !== false && isAdvanced && (
            <>
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-text-primary w-32">
                  {t("general.sounds.startRecording")}
                </span>
                <Select
                  value={config.start_sound ?? "chime"}
                  onChange={(v) => updateConfig({ start_sound: v })}
                  options={availableSounds.map((s) => ({
                    value: s,
                    label: s.replace(/[_-]/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()),
                  }))}
                  className="flex-1"
                />
                <button
                  onClick={() => previewSound(config.start_sound ?? "chime")}
                  className={buttonVariants.icon}
                  aria-label={t("general.sounds.previewAriaLabel")}
                >
                  🔊
                </button>
              </div>
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-text-primary w-32">
                  {t("general.sounds.stopRecording")}
                </span>
                <Select
                  value={config.stop_sound ?? "ding"}
                  onChange={(v) => updateConfig({ stop_sound: v })}
                  options={availableSounds.map((s) => ({
                    value: s,
                    label: s.replace(/[_-]/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()),
                  }))}
                  className="flex-1"
                />
                <button
                  onClick={() => previewSound(config.stop_sound ?? "ding")}
                  className={buttonVariants.icon}
                  aria-label={t("general.sounds.previewAriaLabel")}
                >
                  🔊
                </button>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Startup */}
      <div>
        <h3 className={sectionHeader}>{t("general.startup.heading")}</h3>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <span className="text-sm font-medium text-text-primary">
                {t("general.startup.launchLabel")}
              </span>
              <p className="text-xs text-text-muted">
                {t("general.startup.launchDescription")}
              </p>
            </div>
            <Switch
              checked={config.launch_on_startup ?? false}
              onChange={async (enable) => {
                try {
                  await setLaunchOnStartup(enable);
                  setConfig((prev) =>
                    prev ? { ...prev, launch_on_startup: enable } : prev,
                  );
                } catch (err) {
                  setError(String(err));
                }
              }}
              label={t("general.startup.launchLabel")}
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <span className="text-sm font-medium text-text-primary">
                {t("general.startup.minimizedLabel")}
              </span>
              <p className="text-xs text-text-muted">
                {t("general.startup.minimizedDescription")}
              </p>
            </div>
            <Switch
              checked={config.start_minimized ?? false}
              onChange={(checked) =>
                updateConfig({ start_minimized: checked })
              }
              label={t("general.startup.minimizedLabel")}
            />
          </div>

          {isAdvanced && (
            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-text-primary">
                  {t("general.pill.label")}
                </span>
                <p className="text-xs text-text-muted">
                  {t("general.pill.description")}
                </p>
              </div>
              <Switch
                checked={config.pill_pinned ?? false}
                onChange={(checked) =>
                  updateConfig({ pill_pinned: checked })
                }
                label={t("general.pill.label")}
              />
            </div>
          )}

          {isAdvanced && (
            <div className="flex items-center justify-between">
              <div>
                <span className="text-sm font-medium text-text-primary">
                  {t("general.mediaPause.label")}
                </span>
                <p className="text-xs text-text-muted">
                  {t("general.mediaPause.description")}
                </p>
              </div>
              <Switch
                checked={config.auto_pause_media_enabled ?? false}
                onChange={(checked) =>
                  updateConfig({ auto_pause_media_enabled: checked })
                }
                label={t("general.mediaPause.label")}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
