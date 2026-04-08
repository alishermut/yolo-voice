import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AppConfig, TranscriptDiagnosticsStatus } from "../../shared/types";
import {
  clearTranscriptDiagnostics,
  exportSupportDiagnostics,
  getTranscriptDiagnosticsStatus,
} from "../../shared/platform";
import { focusRing } from "../ui/styles";
import { Switch } from "../ui/Switch";

interface SupportDiagnosticsCardProps {
  config: AppConfig;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
}

export function SupportDiagnosticsCard({
  config,
  updateConfig,
}: SupportDiagnosticsCardProps) {
  const { t } = useTranslation();
  const [diagnosticsStatus, setDiagnosticsStatus] =
    useState<TranscriptDiagnosticsStatus | null>(null);
  const [diagnosticsMessage, setDiagnosticsMessage] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const [clearingDiagnostics, setClearingDiagnostics] = useState(false);

  useEffect(() => {
    getTranscriptDiagnosticsStatus()
      .then(setDiagnosticsStatus)
      .catch(() => setDiagnosticsStatus(null));
  }, [config.transcript_diagnostics_enabled]);

  const handleDiagnosticsToggle = async (checked: boolean) => {
    setDiagnosticsMessage(null);
    await updateConfig({ transcript_diagnostics_enabled: checked });
    try {
      const status = await getTranscriptDiagnosticsStatus();
      setDiagnosticsStatus(status);
    } catch {
      setDiagnosticsStatus(null);
    }
  };

  const handleExportDiagnostics = async () => {
    setExporting(true);
    setDiagnosticsMessage(null);
    try {
      const result = await exportSupportDiagnostics();
      const status = await getTranscriptDiagnosticsStatus().catch(() => null);
      if (status) {
        setDiagnosticsStatus(status);
      }
      setDiagnosticsMessage(
        t("transcription.diagnostics.exportSuccess", {
          defaultValue: "Support bundle saved to {{path}}",
          path: result.archive_path,
        }),
      );
    } catch (error) {
      setDiagnosticsMessage(
        t("transcription.diagnostics.exportError", {
          defaultValue: "Couldn't export diagnostics: {{error}}",
          error: String(error),
        }),
      );
    } finally {
      setExporting(false);
    }
  };

  const handleClearDiagnostics = async () => {
    setClearingDiagnostics(true);
    setDiagnosticsMessage(null);
    try {
      const status = await clearTranscriptDiagnostics();
      setDiagnosticsStatus(status);
      setDiagnosticsMessage(
        t("transcription.diagnostics.clearSuccess", {
          defaultValue: "Support diagnostics were cleared.",
        }),
      );
    } catch (error) {
      setDiagnosticsMessage(
        t("transcription.diagnostics.clearError", {
          defaultValue: "Couldn't clear diagnostics: {{error}}",
          error: String(error),
        }),
      );
    } finally {
      setClearingDiagnostics(false);
    }
  };

  return (
    <div className="p-4 bg-bg-raised border border-border-default rounded-lg space-y-4">
      <div className="space-y-1">
        <h3 className="text-sm font-semibold text-text-primary">
          {t("transcription.diagnostics.heading", {
            defaultValue: "Support diagnostics",
          })}
        </h3>
        <p className="text-xs text-text-muted">
          {t("transcription.diagnostics.description", {
            defaultValue:
              "Enable step-by-step debug logging, reproduce the problem once, then export one support bundle to share manually. Audio and API keys are excluded.",
          })}
        </p>
      </div>

      <div className="flex items-center justify-between gap-4">
        <div>
          <span className="text-sm font-medium text-text-primary">
            {t("transcription.diagnostics.enableLabel", {
              defaultValue: "Enable support diagnostics",
            })}
          </span>
          <p className="text-xs text-text-muted">
            {t("transcription.diagnostics.enableDescription", {
              defaultValue:
                "Captures recording, transcribe, model-load, and insert lifecycle events for this session.",
            })}
          </p>
        </div>
        <Switch
          checked={config.transcript_diagnostics_enabled}
          onChange={handleDiagnosticsToggle}
          label={t("transcription.diagnostics.enableLabel", {
            defaultValue: "Enable support diagnostics",
          })}
        />
      </div>

      <div className="text-xs text-text-muted space-y-1">
        <p>
          {t("transcription.diagnostics.sampleCount", {
            defaultValue: "Saved transcript samples: {{count}} / {{max}}",
            count: diagnosticsStatus?.sample_count ?? 0,
            max: diagnosticsStatus?.max_samples ?? 0,
          })}
        </p>
        {diagnosticsStatus?.db_path && (
          <p className="break-all">
            {t("transcription.diagnostics.storagePath", {
              defaultValue: "Local diagnostics database: {{path}}",
              path: diagnosticsStatus.db_path,
            })}
          </p>
        )}
      </div>

      <div className="flex flex-wrap gap-2">
        <button
          onClick={handleExportDiagnostics}
          disabled={exporting}
          className={`px-3 py-2 rounded-lg text-sm font-medium bg-accent hover:opacity-90 text-white transition-opacity disabled:opacity-60 ${focusRing}`}
        >
          {exporting
            ? t("transcription.diagnostics.exporting", {
                defaultValue: "Exporting...",
              })
            : t("transcription.diagnostics.exportButton", {
                defaultValue: "Export debug logs",
              })}
        </button>
        <button
          onClick={handleClearDiagnostics}
          disabled={clearingDiagnostics}
          className={`px-3 py-2 rounded-lg text-sm font-medium bg-bg-hover hover:bg-bg-active text-text-secondary transition-colors disabled:opacity-60 ${focusRing}`}
        >
          {clearingDiagnostics
            ? t("transcription.diagnostics.clearing", {
                defaultValue: "Clearing...",
              })
            : t("transcription.diagnostics.clearButton", {
                defaultValue: "Clear diagnostics",
              })}
        </button>
      </div>

      {diagnosticsMessage && (
        <div className="px-3 py-2 bg-bg-base border border-border-default rounded-lg text-sm text-text-secondary break-all">
          {diagnosticsMessage}
        </div>
      )}
    </div>
  );
}
