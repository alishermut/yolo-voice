import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AppInfo {
  version: string;
  name: string;
  launch_on_startup: boolean;
}

export function About() {
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    invoke<AppInfo>("get_app_info").then(setInfo).catch(console.error);
  }, []);

  return (
    <div className="max-w-md mx-auto space-y-6">
      <div className="text-center space-y-3">
        <h2 className="text-2xl font-bold text-gray-100">
          {info?.name ?? "YOLO Voice"}
        </h2>
        <p className="text-sm text-gray-400">
          Version {info?.version ?? "..."}
        </p>
      </div>

      <div className="bg-gray-800/50 rounded-lg p-4 space-y-3">
        <p className="text-sm text-gray-300">
          A Windows-first, offline-focused voice dictation app. Speak naturally
          and your words appear wherever you type.
        </p>

        <div className="space-y-2 text-sm text-gray-400">
          <div className="flex justify-between">
            <span>Transcription</span>
            <span className="text-gray-300">faster-whisper (offline) / Groq / Deepgram</span>
          </div>
          <div className="flex justify-between">
            <span>Post-processing</span>
            <span className="text-gray-300">Ollama / OpenAI / Claude</span>
          </div>
          <div className="flex justify-between">
            <span>Framework</span>
            <span className="text-gray-300">Tauri 2.0 + React</span>
          </div>
        </div>
      </div>

      <div className="bg-gray-800/50 rounded-lg p-4 space-y-2">
        <h3 className="text-sm font-semibold text-gray-200">Keyboard Shortcuts</h3>
        <div className="text-sm text-gray-400 space-y-1">
          <div className="flex justify-between">
            <span>Hold mode</span>
            <span className="text-gray-300">Hold hotkey to record, release to stop</span>
          </div>
          <div className="flex justify-between">
            <span>Toggle mode</span>
            <span className="text-gray-300">Double-press to start, single press to stop</span>
          </div>
        </div>
      </div>

      <p className="text-xs text-gray-600 text-center">
        Built by Alish. Privacy-first: audio is processed locally by default.
      </p>
    </div>
  );
}
