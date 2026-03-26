import { ProfileEditor } from "../ProfileEditor";
import type { AppConfig } from "../../shared/types";
import { descStyles } from "../ui/styles";

interface ProfilesSectionProps {
  config: AppConfig;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
}

export function ProfilesSection({
  config,
  updateConfig,
}: ProfilesSectionProps) {
  return (
    <div>
      <p className={`${descStyles} mb-4`}>
        Hold dictation key + style's shortcut key to apply a style during recording.
        Uses <span className="text-text-secondary">openai/gpt-oss-120b</span> via Groq.
      </p>
      <ProfileEditor
        activeProfileId={config.active_profile_id ?? ""}
        onProfileChange={(id) => updateConfig({ active_profile_id: id })}
        dictationHotkey={config.hotkey ?? ""}
        commandHotkey={config.command_hotkey ?? ""}
      />
    </div>
  );
}
