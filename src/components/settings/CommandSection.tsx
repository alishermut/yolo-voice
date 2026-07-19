import { useState } from "react";
import { useTranslation } from "react-i18next";
import { TextActionManager } from "../TextActionManager";
import { Select } from "../ui/Select";
import type {
  AppConfig,
  SettingsExperienceMode,
  StorageOverview,
} from "../../shared/types";
import { openStorageLocation, testCommandLlmConnection } from "../../shared/platform";
import {
  buttonVariants,
  descStyles,
  inputStyles,
  sectionHeader,
} from "../ui/styles";
import { TrustCard } from "./TrustCard";
import { ConfigTextInput } from "./ConfigTextInput";

interface CommandSectionProps {
  config: AppConfig;
  settingsMode: SettingsExperienceMode;
  storageOverview: StorageOverview | null;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
}

export function CommandSection({
  config,
  settingsMode,
  storageOverview,
  updateConfig,
}: CommandSectionProps) {
  const { t } = useTranslation();
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    msg: string;
  } | null>(null);
  const [trustMessage, setTrustMessage] = useState<{
    tone: "error" | "info" | "success";
    text: string;
  } | null>(null);
  const providerName = (() => {
    switch (config.command_provider) {
      case "ollama":
        return "Ollama";
      case "openai":
        return "OpenAI";
      case "claude":
        return "Claude";
      default:
        return "Groq";
    }
  })();
  const effectiveProviderLabel =
    config.command_provider === "ollama"
      ? t("trust.badge.localVia", {
          defaultValue: "Local via {{provider}}",
          provider: providerName,
        })
      : t("trust.badge.cloudVia", {
          defaultValue: "Cloud via {{provider}}",
          provider: providerName,
        });

  const handleOpenLocation = async (
    kind: "text_actions" | "config",
    label: string,
  ) => {
    try {
      setTrustMessage(null);
      await openStorageLocation(kind);
    } catch (error) {
      setTrustMessage({
        tone: "error",
        text: t("trust.message.openError", {
          defaultValue: "Couldn't open {{label}}: {{error}}",
          label,
          error: String(error),
        }),
        });
    }
  };
  const isAdvanced = settingsMode === "advanced";
  const showCloudApiKey = config.command_provider !== "ollama";

  return (
    <div className="space-y-8">
      <p className={descStyles}>{t("textActions.description")}</p>

      <div>
        <h3 className={sectionHeader}>{t("textActions.actions.heading")}</h3>
        <TextActionManager
          defaultActionId={config.default_text_action_id ?? "clean_up"}
          settingsMode={settingsMode}
          onDefaultActionChange={(default_text_action_id) =>
            updateConfig({ default_text_action_id })
          }
        />
      </div>

      <div>
        <h3 className={sectionHeader}>{t("textActions.api.heading")}</h3>
        <div className="space-y-4">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-text-primary w-36">
              {t("llm.providerLabel")}
            </span>
            <Select
              value={config.command_provider ?? "groq"}
              onChange={(command_provider) => updateConfig({ command_provider })}
              options={[
                { value: "groq", label: t("llm.provider.groq.name") },
                { value: "ollama", label: t("llm.provider.ollama.name") },
                { value: "openai", label: t("llm.provider.openai.name") },
                { value: "claude", label: t("llm.provider.claude.name") },
              ]}
              className="flex-1"
            />
          </div>
          {isAdvanced && (
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-primary w-36">
                {t("llm.modelLabel")}
              </span>
              <ConfigTextInput
                type="text"
                value={config.command_model ?? ""}
                onCommit={(command_model) => updateConfig({ command_model })}
                placeholder={t("llm.modelPlaceholder")}
                className={`flex-1 ${inputStyles}`}
              />
            </div>
          )}
          {showCloudApiKey && (
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-primary w-36">
                {t("textActions.api.keyLabel")}
              </span>
              <ConfigTextInput
                type="password"
                value={config.command_api_key ?? ""}
                onCommit={(command_api_key) =>
                  updateConfig({ command_api_key })
                }
                placeholder={t("textActions.api.keyPlaceholder")}
                className={`flex-1 ${inputStyles}`}
              />
            </div>
          )}
          {isAdvanced && (
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-text-primary w-36">
                {t("llm.baseUrlLabel")}
              </span>
              <ConfigTextInput
                type="text"
                value={config.command_base_url ?? ""}
                onCommit={(command_base_url) =>
                  updateConfig({ command_base_url })
                }
                placeholder={t("llm.baseUrlPlaceholder")}
                className={`flex-1 ${inputStyles}`}
              />
            </div>
          )}
          <p className={descStyles}>{t("textActions.api.description")}</p>
        </div>
      </div>

      <TrustCard
        title={t("textActions.trust.title", {
          defaultValue: "Storage & provider use",
        })}
        badgeLabel={effectiveProviderLabel}
        badgeTone={config.command_provider === "ollama" ? "local" : "cloud"}
        description={[
          config.command_provider === "ollama"
            ? t("textActions.trust.ollamaLine", {
                defaultValue:
                  "When you run a text action with Ollama, dictated text plus the selected action prompt stay on this device through your local Ollama setup.",
              })
            : t("textActions.trust.cloudLine", {
                defaultValue:
                  "When you run a text action, dictated text plus the selected action prompt are sent to {{provider}}.",
                provider: providerName,
              }),
          t("textActions.trust.configLine", {
            defaultValue:
              "API keys and base URL are stored locally in your app config.",
          }),
        ]}
        paths={[
          {
            label: t("trust.path.textActions", {
              defaultValue: "Text actions folder",
            }),
            value:
              storageOverview?.text_actions_dir ||
              t("trust.value.unavailable", { defaultValue: "Unavailable" }),
          },
          {
            label: t("trust.path.config", {
              defaultValue: "Config file",
            }),
            value:
              storageOverview?.config_path ||
              t("trust.value.unavailable", { defaultValue: "Unavailable" }),
          },
        ]}
        actions={[
          {
            label: t("trust.action.openTextActions", {
              defaultValue: "Open text actions folder",
            }),
            onClick: () =>
              handleOpenLocation(
                "text_actions",
                t("trust.action.openTextActions", {
                  defaultValue: "Open text actions folder",
                }),
              ),
          },
          {
            label: t("trust.action.openConfig", {
              defaultValue: "Open config folder",
            }),
            onClick: () =>
              handleOpenLocation(
                "config",
                t("trust.action.openConfig", {
                  defaultValue: "Open config folder",
                }),
              ),
          },
        ]}
        message={trustMessage}
      />

      {isAdvanced && (
        <div className="space-y-2">
          <button
            onClick={async () => {
              setTestResult(null);
              try {
                await testCommandLlmConnection(
                  config.command_provider ?? "groq",
                  config.command_model ?? "openai/gpt-oss-120b",
                  config.command_api_key ?? "",
                  config.command_base_url ?? "https://api.groq.com/openai",
                );
                setTestResult({
                  ok: true,
                  msg: t("textActions.testConnection.success"),
                });
              } catch (e) {
                setTestResult({ ok: false, msg: String(e) });
              }
            }}
            className={buttonVariants.secondary}
          >
            {t("textActions.testConnection.button")}
          </button>
          {testResult && (
            <p
              className={`text-xs ${testResult.ok ? "text-success" : "text-error"}`}
            >
              {testResult.msg}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
