import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppConfig } from "../shared/types";
import { getConfig, saveConfig, quitApp, getAppInfo } from "../shared/platform";
import type { AppInfo } from "../shared/types";
import { useToast, ToastContainer } from "../components/Toast";
import { focusRing } from "../components/ui/styles";
import { useUpdaterContext } from "../contexts/UpdaterContext";
import { GeneralSection } from "../components/settings/GeneralSection";
import { HotkeySection } from "../components/settings/HotkeySection";
import { CommandSection } from "../components/settings/CommandSection";
import { TranscriptionSection } from "../components/settings/TranscriptionSection";
import { VocabularySection } from "../components/settings/VocabularySection";
import { ProfilesSection } from "../components/settings/ProfilesSection";
import { AboutSection } from "../components/settings/AboutSection";
import { HistorySection } from "../components/settings/HistorySection";

type SettingsSection =
  | "general"
  | "hotkeys"
  | "transcription"
  | "command"
  | "vocabulary"
  | "profiles"
  | "history"
  | "about";

const SECTIONS: { key: SettingsSection; labelKey: string; icon: string }[] = [
  { key: "general", labelKey: "settings.sidebar.section.general", icon: "\u2699" },
  { key: "hotkeys", labelKey: "settings.sidebar.section.hotkeys", icon: "\u2328" },
  { key: "transcription", labelKey: "settings.sidebar.section.transcription", icon: "\uD83C\uDFA4" },
  { key: "command", labelKey: "settings.sidebar.section.commandMode", icon: "\u26A1" },
  { key: "vocabulary", labelKey: "settings.sidebar.section.vocabulary", icon: "\uD83D\uDCD6" },
  { key: "profiles", labelKey: "settings.sidebar.section.dictationStyles", icon: "\uD83C\uDFA8" },
  { key: "history", labelKey: "settings.sidebar.section.history", icon: "\uD83D\uDCCB" },
];

const SECTION_TITLE_KEYS: Record<SettingsSection, string> = {
  general: "settings.sectionTitle.general",
  hotkeys: "settings.sectionTitle.hotkeys",
  transcription: "settings.sectionTitle.transcription",
  command: "settings.sectionTitle.command",
  vocabulary: "settings.sectionTitle.vocabulary",
  profiles: "settings.sectionTitle.profiles",
  history: "settings.sectionTitle.history",
  about: "settings.sectionTitle.about",
};

export function Settings() {
  const { t } = useTranslation();
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [activeSection, setActiveSection] =
    useState<SettingsSection>("general");
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const { toasts, addToast } = useToast();
  const { status: updateStatus, version: updateVersion, installUpdate } = useUpdaterContext();
  const [updateDismissed, setUpdateDismissed] = useState(false);

  useEffect(() => {
    getConfig()
      .then(setConfig)
      .catch((e) => setError(String(e)));
    getAppInfo().then(setAppInfo).catch(() => {});
  }, []);

  const updateConfig = async (updates: Partial<AppConfig>) => {
    if (!config) return;
    const newConfig = { ...config, ...updates };
    try {
      await saveConfig(newConfig);
      setConfig(newConfig);
      setError(null);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      addToast(msg, "error");
    }
  };

  // Keyboard shortcuts: press 1-7 to jump between sections
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      const index = parseInt(e.key, 10);
      if (index >= 1 && index <= SECTIONS.length) {
        e.preventDefault();
        setActiveSection(SECTIONS[index - 1].key);
      }
      if (e.key === "7") {
        e.preventDefault();
        setActiveSection("about");
      }
    },
    [],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  if (!config) {
    return (
      <div className="h-screen bg-bg-base text-text-primary flex items-center justify-center">
        <p className="text-text-muted text-sm">{t("settings.loading")}</p>
      </div>
    );
  }

  return (
    <div className="h-screen flex bg-bg-base text-text-primary">
      <ToastContainer toasts={toasts} />

      {/* Sidebar */}
      <nav className="w-52 flex-shrink-0 border-r border-border-default bg-bg-base flex flex-col">
        {/* Header */}
        <div className="px-4 pt-5 pb-4 flex items-center gap-2.5">
          <img src="/app-icon.svg" alt={t("settings.sidebar.appName")} width="28" height="28" className="shrink-0 rounded-md" />
          <div>
            <h1 className="text-sm font-semibold text-text-primary leading-tight">
              {t("settings.sidebar.appName")}
            </h1>
            <p className="text-xs text-text-muted">
              {t("settings.sidebar.version", { version: appInfo?.version ?? "..." })}
            </p>
          </div>
        </div>

        {/* Sections */}
        <div className="flex-1 overflow-y-auto px-2 space-y-0.5">
          {SECTIONS.map(({ key, labelKey, icon }) => (
            <button
              key={key}
              onClick={() => setActiveSection(key)}
              className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium transition-all text-left ${focusRing} ${
                activeSection === key
                  ? "bg-bg-raised text-text-primary border-l-2 border-accent"
                  : "text-text-secondary hover:text-text-primary hover:bg-bg-hover border-l-2 border-transparent"
              }`}
            >
              <span className="text-base w-5 text-center opacity-70">{icon}</span>
              {t(labelKey)}
            </button>
          ))}

          {/* Separator + About */}
          <div className="border-t border-border-default my-2" />
          <button
            onClick={() => setActiveSection("about")}
            className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium transition-all text-left ${focusRing} ${
              activeSection === "about"
                ? "bg-bg-raised text-text-primary border-l-2 border-accent"
                : "text-text-secondary hover:text-text-primary hover:bg-bg-hover border-l-2 border-transparent"
            }`}
          >
            <span className="text-base w-5 text-center opacity-70">&#9432;</span>
            {t("settings.sidebar.section.about")}
            {(updateStatus === "ready" || updateStatus === "downloading") && (
              <span className="ml-auto w-2 h-2 rounded-full bg-success animate-pulse" />
            )}
          </button>
        </div>

        {/* Footer */}
        <div className="px-3 py-3 border-t border-border-default flex gap-2">
          <button
            onClick={() => getCurrentWindow().hide()}
            className={`flex-1 px-2 py-1.5 rounded-lg text-xs font-medium text-text-muted hover:text-text-primary hover:bg-bg-hover transition-colors ${focusRing}`}
            title={t("settings.sidebar.hideTitle")}
          >
            {t("settings.sidebar.hide")}
          </button>
          <button
            onClick={() => quitApp()}
            className={`flex-1 px-2 py-1.5 rounded-lg text-xs font-medium text-text-muted hover:text-error hover:bg-error-muted transition-colors ${focusRing}`}
            title={t("settings.sidebar.quitTitle")}
          >
            {t("settings.sidebar.quit")}
          </button>
        </div>
      </nav>

      {/* Content area */}
      <div className="flex-1 overflow-y-auto bg-bg-raised">
        <div className="max-w-2xl px-8 py-6">
          {/* Error banner */}
          {error && (
            <div className="mb-6 px-3 py-2 bg-error-muted border border-error rounded-lg text-error text-sm flex items-center justify-between">
              <span>{error}</span>
              <button
                onClick={() => setError(null)}
                className={`text-error hover:text-text-primary ml-3 text-xs ${focusRing}`}
              >
                {t("settings.error.dismiss")}
              </button>
            </div>
          )}

          {/* Dictionary migration notice */}
          {config.show_dictionary_migration_notice && (
            <div className="mb-6 px-4 py-3 bg-warning-muted border border-warning rounded-lg text-warning text-sm">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="font-medium">{t("settings.migration.title")}</p>
                  <p className="text-xs text-warning mt-1">
                    {t("settings.migration.description")}
                  </p>
                </div>
                <button
                  onClick={() =>
                    updateConfig({ show_dictionary_migration_notice: false })
                  }
                  className={`px-2 py-1 rounded bg-warning-muted hover:bg-bg-hover text-xs text-text-primary transition-colors ${focusRing}`}
                >
                  {t("settings.migration.dismiss")}
                </button>
              </div>
            </div>
          )}

          {/* Update banner */}
          {updateStatus === "downloading" && (
            <div className="mb-6 px-4 py-3 bg-accent/10 border border-accent rounded-lg text-accent text-sm flex items-center gap-3">
              <span className="animate-spin text-base">&#8635;</span>
              <span>{t("updater.banner.downloading", { version: updateVersion })}</span>
            </div>
          )}
          {updateStatus === "ready" && !updateDismissed && (
            <div className="mb-6 px-4 py-3 bg-success/10 border border-success rounded-lg text-sm">
              <div className="flex items-center justify-between gap-3">
                <span className="text-success font-medium">
                  {t("updater.banner.ready", { version: updateVersion })}
                </span>
                <div className="flex gap-2 shrink-0">
                  <button
                    onClick={() => setUpdateDismissed(true)}
                    className={`px-3 py-1.5 rounded-lg text-xs font-medium text-text-muted hover:text-text-primary hover:bg-bg-hover transition-colors ${focusRing}`}
                  >
                    {t("updater.banner.later")}
                  </button>
                  <button
                    onClick={installUpdate}
                    className={`px-3 py-1.5 rounded-lg text-xs font-medium bg-success text-white hover:opacity-90 transition-opacity ${focusRing}`}
                  >
                    {t("updater.banner.installRestart")}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Section title */}
          <h2 className="text-lg font-semibold text-text-primary mb-6">
            {t(SECTION_TITLE_KEYS[activeSection])}
          </h2>

          {/* Active section */}
          {activeSection === "general" && (
            <GeneralSection
              config={config}
              updateConfig={updateConfig}
              setConfig={setConfig}
              setError={setError}
            />
          )}
          {activeSection === "hotkeys" && (
            <HotkeySection config={config} updateConfig={updateConfig} />
          )}
          {activeSection === "transcription" && (
            <TranscriptionSection
              config={config}
              updateConfig={updateConfig}
              setError={setError}
            />
          )}
          {activeSection === "command" && (
            <CommandSection config={config} updateConfig={updateConfig} />
          )}
          {activeSection === "vocabulary" && (
            <VocabularySection
              config={config}
              setConfig={setConfig}
              setError={setError}
            />
          )}
          {activeSection === "profiles" && (
            <ProfilesSection config={config} updateConfig={updateConfig} />
          )}
          {activeSection === "history" && <HistorySection />}
          {activeSection === "about" && <AboutSection />}
        </div>
      </div>
    </div>
  );
}
