import { useState } from "react";
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
import { Switch } from "../ui/Switch";

interface CommandSectionProps {
  config: AppConfig;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
}

export function CommandSection({ config, updateConfig }: CommandSectionProps) {
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    msg: string;
  } | null>(null);

  return (
    <div className="space-y-8">
      <p className={descStyles}>
        Hold the command hotkey, speak your request, release. Powered by Groq.
      </p>

      {/* Command hotkey */}
      <div>
        <h3 className={sectionHeader}>Command Hotkey</h3>
        <div className="space-y-1">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-text-primary w-36">
              Command hotkey
            </span>
            <KeybindingInput
              value={config.command_hotkey ?? ""}
              onChange={(command_hotkey) => updateConfig({ command_hotkey })}
            />
          </div>
          {config.hotkey &&
            config.command_hotkey &&
            config.hotkey === config.command_hotkey && (
              <p className="text-xs text-warning ml-40">
                &#9888; Same as dictation hotkey
              </p>
            )}
        </div>
      </div>

      {/* Groq API key */}
      <div>
        <h3 className={sectionHeader}>API Configuration</h3>
        <div className="space-y-4">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-text-primary w-36">
              Groq API Key
            </span>
            <input
              type="password"
              value={config.command_api_key ?? ""}
              onChange={(e) =>
                updateConfig({ command_api_key: e.target.value })
              }
              placeholder="gsk_..."
              className={`flex-1 ${inputStyles}`}
            />
          </div>
          <p className={descStyles}>
            Commands use{" "}
            <span className="text-text-secondary">openai/gpt-oss-120b</span> for
            text. Vision uses{" "}
            <span className="text-text-secondary">
              meta-llama/llama-4-scout-17b-16e-instruct
            </span>
            .
          </p>
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
                config.command_api_key ?? "",
                config.command_base_url ?? "https://api.groq.com/openai",
              );
              setTestResult({ ok: true, msg: "Connected!" });
            } catch (e) {
              setTestResult({ ok: false, msg: String(e) });
            }
          }}
          className={buttonVariants.secondary}
        >
          Test Connection
        </button>
        {testResult && (
          <p
            className={`text-xs ${testResult.ok ? "text-success" : "text-error"}`}
          >
            {testResult.msg}
          </p>
        )}
      </div>

      {/* Vision toggle */}
      <div>
        <h3 className={sectionHeader}>Vision</h3>
        <div className="flex items-center justify-between">
          <div>
            <span className="text-sm font-medium text-text-primary">
              Enable screenshot context
            </span>
            <p className={descStyles}>
              Commands referencing the screen will capture a screenshot
              automatically. Never saved to disk.
            </p>
          </div>
          <Switch
            checked={config.cloud_vision_enabled ?? false}
            onChange={(checked) =>
              updateConfig({ cloud_vision_enabled: checked })
            }
            label="Enable screenshot context"
          />
        </div>
      </div>

      {/* System prompt */}
      <div>
        <h3 className={sectionHeader}>System Prompt</h3>
        <p className={`${descStyles} mb-2`}>
          Instructions that tell the LLM how to handle your voice commands.
        </p>
        <textarea
          value={config.command_system_prompt ?? ""}
          onChange={(e) =>
            updateConfig({ command_system_prompt: e.target.value })
          }
          rows={3}
          className={textareaStyles}
          placeholder="You are a voice command assistant..."
        />
      </div>
    </div>
  );
}
