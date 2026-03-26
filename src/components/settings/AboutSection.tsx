import { useState, useEffect } from "react";
import { getAppInfo } from "../../shared/platform";
import type { AppInfo } from "../../shared/types";
import { focusRing, infoBoxStyles } from "../ui/styles";

export function AboutSection() {
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    getAppInfo().then(setInfo).catch(console.error);
  }, []);

  return (
    <div className="space-y-6 max-w-md">
      <div className="space-y-1">
        <h3 className="text-xl font-bold text-text-primary">
          {info?.name ?? "YOLO Voice"}
        </h3>
        <p className="text-sm text-text-muted">
          Version {info?.version ?? "..."}
        </p>
      </div>

      <div className="p-4 bg-bg-raised border border-border-default rounded-lg space-y-3">
        <p className="text-sm text-text-secondary">
          A Windows-first, offline-focused voice dictation app. Speak naturally
          and your words appear wherever you type.
        </p>

        <div className="space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-text-muted">Transcription</span>
            <span className="text-text-secondary">Parakeet TDT (offline) / Groq / Deepgram</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">Post-processing</span>
            <span className="text-text-secondary">Groq / Ollama / OpenAI / Claude</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">Framework</span>
            <span className="text-text-secondary">Tauri 2.0 + React</span>
          </div>
        </div>
      </div>

      <div className="p-4 bg-bg-raised border border-border-default rounded-lg space-y-2">
        <h3 className="text-sm font-semibold text-text-primary">Keyboard Shortcuts</h3>
        <div className="text-sm space-y-1">
          <div className="flex justify-between">
            <span className="text-text-muted">Hold mode</span>
            <span className="text-text-secondary">Hold hotkey to record, release to stop</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">Toggle mode</span>
            <span className="text-text-secondary">Double-press to start, single press to stop</span>
          </div>
        </div>
      </div>

      {info?.log_path && (
        <div className={infoBoxStyles}>
          <h3 className="text-sm font-semibold text-text-primary">Diagnostics</h3>
          <div className="flex items-center gap-2">
            <span className="text-xs text-text-muted truncate flex-1" title={info.log_path}>
              Log: {info.log_path}
            </span>
            <button
              onClick={() => navigator.clipboard.writeText(info.log_path)}
              className={`px-2 py-1 text-xs bg-bg-hover hover:bg-bg-active text-text-secondary rounded transition-colors shrink-0 ${focusRing}`}
            >
              Copy path
            </button>
          </div>
        </div>
      )}

      <p className="text-xs text-text-disabled text-center">
        Built by Alish. Privacy-first: audio is processed locally by default.
      </p>
    </div>
  );
}
