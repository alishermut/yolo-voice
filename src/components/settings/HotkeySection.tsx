import { KeybindingInput } from "../KeybindingInput";
import type { AppConfig } from "../../shared/types";
import { sectionHeader } from "../ui/styles";

interface HotkeySectionProps {
  config: AppConfig;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
}

export function HotkeySection({ config, updateConfig }: HotkeySectionProps) {
  return (
    <div className="space-y-8">
      {/* Hotkey binding */}
      <div>
        <h3 className={sectionHeader}>Dictation Hotkey</h3>
        <div className="space-y-3">
          <div className="space-y-1">
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-primary w-24">
                Key binding
              </span>
              <KeybindingInput
                value={config.hotkey ?? ""}
                onChange={(hotkey) => updateConfig({ hotkey })}
              />
            </div>
            {config.hotkey && config.command_hotkey && config.hotkey === config.command_hotkey && (
              <p className="text-xs text-warning ml-28">
                &#9888; Same as command hotkey
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Recording mode info */}
      <div>
        <h3 className={sectionHeader}>Recording Modes</h3>
        <div className="space-y-2 text-sm">
          <div className="flex items-start gap-3 p-3 bg-bg-raised border border-border-default rounded-lg">
            <span className="text-green-400 font-bold mt-0.5">1</span>
            <div>
              <span className="text-text-primary font-medium">Hold to record</span>
              <p className="text-xs text-text-muted">
                Press and hold the hotkey &rarr; speak &rarr; release to stop and transcribe
              </p>
            </div>
          </div>
          <div className="flex items-start gap-3 p-3 bg-bg-raised border border-border-default rounded-lg">
            <span className="text-blue-400 font-bold mt-0.5">2</span>
            <div>
              <span className="text-text-primary font-medium">Double-tap to toggle</span>
              <p className="text-xs text-text-muted">
                Quick double-press &rarr; recording persists &rarr; press again to stop
              </p>
            </div>
          </div>
          <p className="text-xs text-text-muted italic">
            Both modes work automatically with the same hotkey — no need to choose.
          </p>
        </div>
      </div>
    </div>
  );
}
