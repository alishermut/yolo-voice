import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { KeybindingInput } from "../KeybindingInput";
import type { AppConfig } from "../../shared/types";
import { testCommandLlmConnection } from "../../shared/platform";
import {
  inputStyles,
  textareaStyles,
  buttonVariants,
  sectionHeader,
  descStyles,
} from "../ui/styles";

interface CommandSectionProps {
  config: AppConfig;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
  clearSecret: (slot: "command_api_key") => Promise<void>;
}

export function CommandSection({
  config,
  updateConfig,
  clearSecret,
}: CommandSectionProps) {
  const { t } = useTranslation();
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    msg: string;
  } | null>(null);
  const [commandApiKeyInput, setCommandApiKeyInput] = useState(
    config.command_api_key ?? "",
  );

  useEffect(() => {
    setCommandApiKeyInput(config.command_api_key ?? "");
  }, [config.command_api_key]);

  return (
    <div className="space-y-8">
      <p className={descStyles}>
        {t("command.description")}
      </p>

      {/* Command hotkey */}
      <div>
        <h3 className={sectionHeader}>{t("command.hotkey.heading")}</h3>
        <div className="space-y-1">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-text-primary w-36">
              {t("command.hotkey.label")}
            </span>
            <KeybindingInput
              value={config.command_hotkey ?? ""}
              onChange={(command_hotkey) => updateConfig({ command_hotkey })}
              chord
            />
          </div>
          {config.hotkey &&
            config.command_hotkey &&
            config.hotkey === config.command_hotkey && (
              <p className="text-xs text-warning ml-40">
                &#9888; {t("command.hotkey.conflictWarning")}
              </p>
            )}
        </div>
      </div>

      {/* Groq API key */}
      <div>
        <h3 className={sectionHeader}>{t("command.api.heading")}</h3>
        <div className="space-y-4">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-text-primary w-36">
              {t("command.api.keyLabel")}
            </span>
            <input
              type="password"
              value={commandApiKeyInput}
              onChange={(e) => {
                const value = e.target.value;
                setCommandApiKeyInput(value);
                updateConfig({ command_api_key: value });
              }}
              placeholder={
                config.has_command_api_key && !commandApiKeyInput
                  ? t("command.api.keyStored", {
                      defaultValue: "Stored securely. Enter a new key to replace it.",
                    })
                  : t("command.api.keyPlaceholder")
              }
              className={`flex-1 ${inputStyles}`}
            />
            {config.has_command_api_key && (
              <button
                type="button"
                onClick={async () => {
                  await clearSecret("command_api_key");
                  setCommandApiKeyInput("");
                }}
                className={buttonVariants.danger}
              >
                {t("command.api.clearButton", { defaultValue: "Clear stored key" })}
              </button>
            )}
          </div>
          <p className={descStyles}>
            {t("command.api.description")}
          </p>
          {config.has_command_api_key && !commandApiKeyInput && (
            <p className="text-xs text-success">
              {t("command.api.storedNotice", {
                defaultValue: "A command API key is already stored in your OS keychain.",
              })}
            </p>
          )}
        </div>
      </div>

      {/* Test connection */}
      <div className="space-y-2">
        <button
          onClick={async () => {
            setTestResult(null);
            try {
              await testCommandLlmConnection(
                config.command_provider ?? "groq",
                config.command_model ?? "openai/gpt-oss-120b",
                commandApiKeyInput,
                config.command_base_url ?? "https://api.groq.com/openai",
              );
              setTestResult({ ok: true, msg: t("command.testConnection.success") });
            } catch (e) {
              setTestResult({ ok: false, msg: String(e) });
            }
          }}
          className={buttonVariants.secondary}
        >
          {t("command.testConnection")}
        </button>
        {testResult && (
          <p
            className={`text-xs ${testResult.ok ? "text-success" : "text-error"}`}
          >
            {testResult.msg}
          </p>
        )}
      </div>

      {/* System prompt */}
      <div>
        <h3 className={sectionHeader}>{t("command.systemPrompt.heading")}</h3>
        <p className={`${descStyles} mb-2`}>
          {t("command.systemPrompt.description")}
        </p>
        <textarea
          value={config.command_system_prompt ?? ""}
          onChange={(e) =>
            updateConfig({ command_system_prompt: e.target.value })
          }
          rows={3}
          className={textareaStyles}
          placeholder={t("command.systemPrompt.placeholder")}
        />
      </div>
    </div>
  );
}
