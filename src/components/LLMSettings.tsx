import { useState } from "react";
import { useTranslation } from "react-i18next";
import { inputStyles, buttonVariants } from "./ui/styles";
import { testLlmConnection } from "../shared/platform";

interface LLMSettingsProps {
  provider: string;
  model: string;
  apiKey: string;
  baseUrl: string;
  onUpdate: (updates: {
    llm_provider?: string;
    llm_model?: string;
    llm_api_key?: string;
    llm_base_url?: string;
  }) => void;
  /** Custom test function. Defaults to testLlmConnection. */
  testFn?: (provider: string, model: string, apiKey: string, baseUrl: string) => Promise<string>;
  /** Radio group name to avoid collisions when multiple instances on same page. */
  radioGroupName?: string;
}

const PROVIDER_IDS = ["groq", "ollama", "openai", "claude"] as const;

const PROVIDER_KEYS: Record<string, { name: string; desc: string; needsKey: boolean }> = {
  groq: { name: "llm.provider.groq.name", desc: "llm.provider.groq.desc", needsKey: true },
  ollama: { name: "llm.provider.ollama.name", desc: "llm.provider.ollama.desc", needsKey: false },
  openai: { name: "llm.provider.openai.name", desc: "llm.provider.openai.desc", needsKey: true },
  claude: { name: "llm.provider.claude.name", desc: "llm.provider.claude.desc", needsKey: true },
};

const DEFAULT_MODELS: Record<string, string> = {
  groq: "openai/gpt-oss-120b",
  ollama: "llama3.1:8b",
  openai: "gpt-4o-mini",
  claude: "claude-sonnet-4-20250514",
};

export function LLMSettings({
  provider,
  model,
  apiKey,
  baseUrl,
  onUpdate,
  testFn,
  radioGroupName,
}: LLMSettingsProps) {
  const { t } = useTranslation();
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    msg: string;
  } | null>(null);

  const groupName = radioGroupName || "llm_provider";
  const doTest = testFn || testLlmConnection;

  const handleProviderChange = (newProvider: string) => {
    onUpdate({
      llm_provider: newProvider,
      llm_model: DEFAULT_MODELS[newProvider] || "",
      llm_base_url:
        newProvider === "ollama"
          ? "http://localhost:11434"
          : newProvider === "openai"
            ? "https://api.openai.com"
            : newProvider === "groq"
              ? "https://api.groq.com/openai"
              : "",
    });
  };

  const handleTestConnection = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const result = await doTest(provider, model, apiKey, baseUrl);
      setTestResult({ ok: true, msg: t("llm.testResponsePrefix", { result }) });
    } catch (e) {
      setTestResult({ ok: false, msg: String(e) });
    } finally {
      setTesting(false);
    }
  };

  const currentProvider = PROVIDER_KEYS[provider];

  return (
    <div className="space-y-4">
      {/* Provider selection */}
      <div className="space-y-4">
        <span className="text-sm text-text-primary">{t("llm.providerLabel")}</span>
        <div className="space-y-2">
          {PROVIDER_IDS.map((id) => {
            const p = PROVIDER_KEYS[id];
            return (
              <label
                key={id}
                className={`flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors ${
                  provider === id
                    ? "bg-accent-muted border-accent"
                    : "bg-bg-raised border-border-default hover:border-border-hover"
                }`}
              >
                <input
                  type="radio"
                  name={groupName}
                  value={id}
                  checked={provider === id}
                  onChange={() => handleProviderChange(id)}
                  className="accent-blue-500 mt-0.5"
                />
                <div>
                  <span className="text-sm font-medium text-text-primary">
                    {t(p.name)}
                  </span>
                  <p className="text-xs text-text-muted">{t(p.desc)}</p>
                </div>
              </label>
            );
          })}
        </div>
      </div>

      {/* Model name */}
      <div className="flex items-center gap-3">
        <span className="text-sm text-text-primary w-20">{t("llm.modelLabel")}</span>
        <input
          type="text"
          value={model}
          onChange={(e) => onUpdate({ llm_model: e.target.value })}
          placeholder={DEFAULT_MODELS[provider] || t("llm.modelPlaceholder")}
          className={`flex-1 ${inputStyles}`}
        />
      </div>

      {/* API Key (only for cloud providers) */}
      {currentProvider?.needsKey && (
        <div className="flex items-center gap-3">
          <span className="text-sm text-text-primary w-20">{t("llm.apiKeyLabel")}</span>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => onUpdate({ llm_api_key: e.target.value })}
            placeholder={t("llm.apiKeyPlaceholder")}
            className={`flex-1 ${inputStyles}`}
          />
        </div>
      )}

      {/* Base URL (for Ollama and OpenAI-compatible) */}
      {(provider === "ollama" || provider === "openai" || provider === "groq") && (
        <div className="flex items-center gap-3">
          <span className="text-sm text-text-primary w-20">{t("llm.baseUrlLabel")}</span>
          <input
            type="text"
            value={baseUrl}
            onChange={(e) => onUpdate({ llm_base_url: e.target.value })}
            placeholder={t("llm.baseUrlPlaceholder")}
            className={`flex-1 ${inputStyles}`}
          />
        </div>
      )}

      {/* Test connection */}
      <div className="flex items-center gap-3">
        <button
          onClick={handleTestConnection}
          disabled={testing}
          className={buttonVariants.secondary}
        >
          {testing ? t("llm.testing") : t("llm.testConnection")}
        </button>
        {testResult && (
          <span
            className={`text-xs ${testResult.ok ? "text-success" : "text-error"}`}
          >
            {testResult.ok ? t("llm.testSuccess") : testResult.msg}
          </span>
        )}
      </div>
    </div>
  );
}
