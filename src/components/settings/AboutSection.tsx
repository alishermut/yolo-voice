import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getAppInfo } from "../../shared/platform";
import type { AppConfig, AppInfo } from "../../shared/types";
import { focusRing } from "../ui/styles";
import { useUpdaterContext } from "../../contexts/UpdaterContext";
import { SupportDiagnosticsCard } from "./SupportDiagnosticsCard";

const releaseNoteFiles = import.meta.glob("../../../docs/releases/*.md", {
  eager: true,
  import: "default",
  query: "?raw",
}) as Record<string, string>;

type ReleaseNotesSection = {
  heading: string;
  paragraphs: string[];
  bullets: string[];
};

function parseReleaseNotes(markdown: string) {
  const lines = markdown.split(/\r?\n/);
  let title = "";
  const sections: ReleaseNotesSection[] = [];
  let currentSection: ReleaseNotesSection | null = null;

  const ensureSection = () => {
    if (!currentSection) {
      currentSection = { heading: "", paragraphs: [], bullets: [] };
      sections.push(currentSection);
    }
    return currentSection;
  };

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line) {
      continue;
    }

    if (line.startsWith("# ")) {
      title = line.slice(2).trim();
      continue;
    }

    if (line.startsWith("## ")) {
      currentSection = {
        heading: line.slice(3).trim(),
        paragraphs: [],
        bullets: [],
      };
      sections.push(currentSection);
      continue;
    }

    if (line.startsWith("- ")) {
      ensureSection().bullets.push(line.slice(2).trim());
      continue;
    }

    ensureSection().paragraphs.push(line);
  }

  return { title, sections };
}

function getReleaseNotes(version?: string) {
  if (!version) {
    return null;
  }

  const normalizedVersion = version.startsWith("v") ? version : `v${version}`;
  const match = Object.entries(releaseNoteFiles).find(([path]) =>
    path.endsWith(`${normalizedVersion}.md`),
  );

  return match ? parseReleaseNotes(match[1]) : null;
}

interface AboutSectionProps {
  config: AppConfig;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
}

export function AboutSection({ config, updateConfig }: AboutSectionProps) {
  const { t } = useTranslation();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const { status, version, error, checkForUpdates, installUpdate, dismissError } = useUpdaterContext();
  const releaseNotes = getReleaseNotes(info?.version);

  useEffect(() => {
    getAppInfo().then(setInfo).catch(console.error);
  }, []);

  return (
    <div className="space-y-6 max-w-2xl">
      <div className="space-y-1">
        <h3 className="text-xl font-bold text-text-primary">
          {info?.name ?? t("about.appName")}
        </h3>
        <p className="text-sm text-text-muted">
          {t("about.version", { version: info?.version ?? "..." })}
        </p>
      </div>

      {/* Updates */}
      <div className="p-4 bg-bg-raised border border-border-default rounded-lg space-y-3">
        <h3 className="text-sm font-semibold text-text-primary">{t("updater.heading")}</h3>

        {status === "idle" && (
          <button
            onClick={checkForUpdates}
            className={`w-full px-4 py-2 rounded-lg text-sm font-medium bg-accent hover:opacity-90 text-white transition-opacity ${focusRing}`}
          >
            {t("updater.checkButton")}
          </button>
        )}

        {status === "checking" && (
          <p className="text-sm text-text-muted text-center">{t("updater.checking")}</p>
        )}

        {status === "downloading" && (
          <p className="text-sm text-accent text-center">
            {t("updater.downloading", { version })}
          </p>
        )}

        {status === "ready" && (
          <div className="space-y-2">
            <p className="text-sm text-success text-center font-medium">
              {t("updater.ready", { version })}
            </p>
            <button
              onClick={installUpdate}
              className={`w-full px-4 py-2 rounded-lg text-sm font-medium bg-success hover:opacity-90 text-white transition-opacity ${focusRing}`}
            >
              {t("updater.banner.installRestart")}
            </button>
          </div>
        )}

        {status === "up-to-date" && (
          <p className="text-sm text-success text-center">
            {t("updater.upToDate")}
          </p>
        )}

        {status === "error" && (
          <div className="space-y-2">
            <p className="text-sm text-error text-center">
              {error || t("updater.error.fallback")}
            </p>
            <div className="flex gap-2">
              <button
                onClick={dismissError}
                className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium bg-bg-hover hover:bg-bg-active text-text-secondary transition-colors ${focusRing}`}
              >
                {t("updater.error.dismiss")}
              </button>
              <button
                onClick={() => { dismissError(); checkForUpdates(); }}
                className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium bg-accent hover:opacity-90 text-white transition-opacity ${focusRing}`}
              >
                {t("updater.error.tryAgain")}
              </button>
            </div>
          </div>
        )}
      </div>

      <div className="p-4 bg-bg-raised border border-border-default rounded-lg space-y-4">
        <div className="space-y-1">
          <h3 className="text-sm font-semibold text-text-primary">
            {t("about.releaseNotes.heading", { defaultValue: "Latest release notes" })}
          </h3>
          <p className="text-xs text-text-muted">
            {releaseNotes?.title ||
              t("about.releaseNotes.currentVersion", {
                defaultValue: "Release notes for v{{version}}",
                version: info?.version ?? "...",
              })}
          </p>
        </div>

        {releaseNotes ? (
          <div className="space-y-4 text-sm">
            {releaseNotes.sections.map((section, index) => (
              <div key={`${section.heading || "section"}-${index}`} className="space-y-2">
                {section.heading && (
                  <h4 className="font-medium text-text-primary">{section.heading}</h4>
                )}
                {section.paragraphs.map((paragraph, paragraphIndex) => (
                  <p
                    key={`${section.heading || "section"}-paragraph-${paragraphIndex}`}
                    className="text-text-secondary"
                  >
                    {paragraph}
                  </p>
                ))}
                {section.bullets.length > 0 && (
                  <ul className="list-disc pl-5 space-y-1 text-text-secondary">
                    {section.bullets.map((bullet, bulletIndex) => (
                      <li key={`${section.heading || "section"}-bullet-${bulletIndex}`}>
                        {bullet}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            ))}
          </div>
        ) : (
          <p className="text-sm text-text-muted">
            {t("about.releaseNotes.unavailable", {
              defaultValue: "Release notes for this version are not bundled yet.",
            })}
          </p>
        )}
      </div>

      <SupportDiagnosticsCard
        config={config}
        updateConfig={updateConfig}
      />

      <p className="text-xs text-text-disabled text-center">
        {t("about.footer")}
      </p>
    </div>
  );
}
