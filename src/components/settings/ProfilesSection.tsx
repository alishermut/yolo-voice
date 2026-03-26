import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();

  return (
    <div>
      <p className={`${descStyles} mb-4`}>
        {t("profiles.description")}
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
