import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getAppInfo } from "../../shared/platform";
import type { AppInfo } from "../../shared/types";
import { focusRing, infoBoxStyles } from "../ui/styles";

export function AboutSection() {
  const { t } = useTranslation();
  const [info, setInfo] = useState<AppInfo | null>(null);

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

      {info?.log_path && (
        <div className={infoBoxStyles}>
          <h3 className="text-sm font-semibold text-text-primary">{t("about.diagnostics.heading")}</h3>
          <div className="flex items-center gap-2">
            <span className="text-xs text-text-muted truncate flex-1" title={info.log_path}>
              {t("about.diagnostics.logPrefix", { path: info.log_path })}
            </span>
            <button
              onClick={() => navigator.clipboard.writeText(info.log_path)}
              className={`px-2 py-1 text-xs bg-bg-hover hover:bg-bg-active text-text-secondary rounded transition-colors shrink-0 ${focusRing}`}
            >
              {t("about.diagnostics.copyPath")}
            </button>
          </div>
        </div>
      )}

      <p className="text-xs text-text-disabled text-center">
        {t("about.footer")}
      </p>
    </div>
  );
}
