import { useState, useEffect } from "react";
import { MicSelector } from "../MicSelector";
import type { AppConfig } from "../../shared/types";
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
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
  setConfig: React.Dispatch<React.SetStateAction<AppConfig | null>>;
  setError: (error: string | null) => void;
}

export function GeneralSection({
  config,
  updateConfig,
  setConfig,
  setError,
}: GeneralSectionProps) {
  const [availableSounds, setAvailableSounds] = useState<string[]>([]);

  useEffect(() => {
    getAvailableSounds()
      .then(setAvailableSounds)
      .catch(() => {});
  }, []);

  return (
    <div className="space-y-8">
      {/* Microphone */}
      <div>
        <h3 className={sectionHeader}>Microphone</h3>
        <MicSelector
          deviceIndex={config.device_index}
          onDeviceChange={(index) => updateConfig({ device_index: index })}
        />
      </div>

      {/* Sounds */}
      <div>
        <h3 className={sectionHeader}>Sounds</h3>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <span className="text-sm font-medium text-text-primary">
                Enable sounds
              </span>
              <p className="text-xs text-text-muted">
                Play audio feedback when recording starts and stops
              </p>
            </div>
            <Switch
              checked={config.sounds_enabled ?? true}
              onChange={(checked) =>
                updateConfig({ sounds_enabled: checked })
              }
              label="Enable sounds"
            />
          </div>
          {config.sounds_enabled !== false && (
            <>
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-text-primary w-32">
                  Start recording
                </span>
                <Select
                  value={config.start_sound ?? "chime"}
                  onChange={(v) => updateConfig({ start_sound: v })}
                  options={availableSounds.map((s) => ({
                    value: s,
                    label: s.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()),
                  }))}
                  className="flex-1"
                />
                <button
                  onClick={() => previewSound(config.start_sound ?? "chime")}
                  className={buttonVariants.icon}
                  aria-label="Preview sound"
                >
                  🔊
                </button>
              </div>
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-text-primary w-32">
                  Stop recording
                </span>
                <Select
                  value={config.stop_sound ?? "ding"}
                  onChange={(v) => updateConfig({ stop_sound: v })}
                  options={availableSounds.map((s) => ({
                    value: s,
                    label: s.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()),
                  }))}
                  className="flex-1"
                />
                <button
                  onClick={() => previewSound(config.stop_sound ?? "ding")}
                  className={buttonVariants.icon}
                  aria-label="Preview sound"
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
        <h3 className={sectionHeader}>Startup</h3>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <span className="text-sm font-medium text-text-primary">
                Launch on Windows startup
              </span>
              <p className="text-xs text-text-muted">
                Start YOLO Voice automatically when you log in
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
              label="Launch on Windows startup"
            />
          </div>

          <div className="flex items-center justify-between">
            <div>
              <span className="text-sm font-medium text-text-primary">
                Start minimized
              </span>
              <p className="text-xs text-text-muted">
                Hide the main window on launch, only show tray icon
              </p>
            </div>
            <Switch
              checked={config.start_minimized ?? false}
              onChange={(checked) =>
                updateConfig({ start_minimized: checked })
              }
              label="Start minimized"
            />
          </div>
        </div>
      </div>
    </div>
  );
}
