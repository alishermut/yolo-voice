import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getAppInfo } from "../../shared/platform";
import type { AppInfo } from "../../shared/types";
import { focusRing } from "../ui/styles";
import { useUpdaterContext } from "../../contexts/UpdaterContext";

export function AboutSection() {
  const { t } = useTranslation();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const { status, version, error, checkForUpdates, installUpdate, dismissError } = useUpdaterContext();

  useEffect(() => {
    getAppInfo().then(setInfo).catch(console.error);
  }, []);

  return (
    <div className="space-y-6 max-w-md">
      <div className="space-y-1">
        <h3 className="text-xl font-bold text-text-primary">
          {info?.name ?? t("about.appName")}
        </h3>
        <p className="text-sm text-text-muted">
          {t("about.version", { version: info?.version ?? "..." })}
        </p>
      </div>

      <div className="p-4 bg-bg-raised border border-border-default rounded-lg space-y-3">
        <p className="text-sm text-text-secondary">
          {t("about.description")}
        </p>

        <div className="space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-text-muted">{t("about.transcriptionLabel")}</span>
            <span className="text-text-secondary">{t("about.transcriptionValue")}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">{t("about.postProcessingLabel")}</span>
            <span className="text-text-secondary">{t("about.postProcessingValue")}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">{t("about.frameworkLabel")}</span>
            <span className="text-text-secondary">{t("about.frameworkValue")}</span>
          </div>
        </div>
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

      <div className="p-4 bg-bg-raised border border-border-default rounded-lg space-y-2">
        <h3 className="text-sm font-semibold text-text-primary">{t("about.keyboardShortcuts.heading")}</h3>
        <div className="text-sm space-y-1">
          <div className="flex justify-between">
            <span className="text-text-muted">{t("about.keyboardShortcuts.holdLabel")}</span>
            <span className="text-text-secondary">{t("about.keyboardShortcuts.holdValue")}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">{t("about.keyboardShortcuts.toggleLabel")}</span>
            <span className="text-text-secondary">{t("about.keyboardShortcuts.toggleValue")}</span>
          </div>
        </div>
      </div>

      <p className="text-xs text-text-disabled text-center">
        {t("about.footer")}
      </p>
    </div>
  );
}
