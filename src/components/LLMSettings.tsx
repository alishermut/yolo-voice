import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
}

const PROVIDERS = [
  {
    id: "ollama",
    name: "Ollama (Local)",
    desc: "Free, runs locally. Requires Ollama installed.",
    needsKey: false,
  },
  {
    id: "openai",
    name: "OpenAI",
    desc: "GPT-4o-mini or GPT-4o. Requires API key.",
    needsKey: true,
  },
  {
    id: "claude",
    name: "Claude (Anthropic)",
    desc: "Claude Sonnet or Haiku. Requires API key.",
    needsKey: true,
  },
];

const DEFAULT_MODELS: Record<string, string> = {
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
}: LLMSettingsProps) {
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    msg: string;
  } | null>(null);

  const handleProviderChange = (newProvider: string) => {
    onUpdate({
      llm_provider: newProvider,
      llm_model: DEFAULT_MODELS[newProvider] || "",
      llm_base_url:
        newProvider === "ollama"
          ? "http://localhost:11434"
          : newProvider === "openai"
            ? "https://api.openai.com"
            : "",
    });
  };

  const handleTestConnection = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const result = await invoke<string>("test_llm_connection", {
        provider,
        model,
        apiKey,
        baseUrl,
      });
      setTestResult({ ok: true, msg: `Response: "${result}"` });
    } catch (e) {
      setTestResult({ ok: false, msg: String(e) });
    } finally {
      setTesting(false);
    }
  };

  const currentProvider = PROVIDERS.find((p) => p.id === provider);

  return (
    <div className="space-y-4">
      {/* Provider selection */}
      <div className="space-y-2">
        <span className="text-sm text-gray-400">LLM Provider</span>
        <div className="space-y-2">
          {PROVIDERS.map((p) => (
            <label
              key={p.id}
              className={`flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors ${
                provider === p.id
                  ? "bg-blue-600/10 border-blue-500/50"
                  : "bg-gray-800/50 border-gray-700 hover:border-gray-600"
              }`}
            >
              <input
                type="radio"
                name="llm_provider"
                value={p.id}
                checked={provider === p.id}
                onChange={() => handleProviderChange(p.id)}
                className="accent-blue-500 mt-0.5"
              />
              <div>
                <span className="text-sm font-medium text-gray-200">
                  {p.name}
                </span>
                <p className="text-xs text-gray-500">{p.desc}</p>
              </div>
            </label>
          ))}
        </div>
      </div>

      {/* Model name */}
      <div className="flex items-center gap-3">
        <span className="text-sm text-gray-400 w-20">Model</span>
        <input
          type="text"
          value={model}
          onChange={(e) => onUpdate({ llm_model: e.target.value })}
          placeholder={DEFAULT_MODELS[provider] || "model-name"}
          className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
        />
      </div>

      {/* API Key (only for cloud providers) */}
      {currentProvider?.needsKey && (
        <div className="flex items-center gap-3">
          <span className="text-sm text-gray-400 w-20">API Key</span>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => onUpdate({ llm_api_key: e.target.value })}
            placeholder="sk-..."
            className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
          />
        </div>
      )}

      {/* Base URL (for Ollama and OpenAI-compatible) */}
      {(provider === "ollama" || provider === "openai") && (
        <div className="flex items-center gap-3">
          <span className="text-sm text-gray-400 w-20">Base URL</span>
          <input
            type="text"
            value={baseUrl}
            onChange={(e) => onUpdate({ llm_base_url: e.target.value })}
            placeholder="http://localhost:11434"
            className="flex-1 bg-gray-800 border border-gray-700 text-gray-200 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
          />
        </div>
      )}

      {/* Test connection */}
      <div className="flex items-center gap-3">
        <button
          onClick={handleTestConnection}
          disabled={testing}
          className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
        >
          {testing ? "Testing..." : "Test Connection"}
        </button>
        {testResult && (
          <span
            className={`text-xs ${testResult.ok ? "text-green-400" : "text-red-400"}`}
          >
            {testResult.ok ? "Connected" : testResult.msg}
          </span>
        )}
      </div>
    </div>
  );
}
