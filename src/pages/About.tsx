import { useState, useEffect } from "react";
import { useUpdater } from "../hooks/useUpdater";
import { getAppInfo } from "../shared/platform";
import type { AppInfo } from "../shared/types";

export function About() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const { status: updateStatus, version: updateVersion, error: updateError, checkForUpdates, installUpdate } = useUpdater();

  useEffect(() => {
    getAppInfo().then(setInfo).catch(console.error);
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

      <div className="bg-gray-800/50 rounded-lg p-4 space-y-3">
        <h3 className="text-sm font-semibold text-gray-200">Updates</h3>

        {updateStatus === "idle" && (
          <button
            onClick={checkForUpdates}
            className="w-full px-4 py-2 rounded-lg text-sm font-medium bg-blue-600 hover:bg-blue-500 text-white transition-colors"
          >
            Check for Updates
          </button>
        )}

        {updateStatus === "checking" && (
          <p className="text-sm text-gray-400 text-center">Checking for updates...</p>
        )}

        {updateStatus === "downloading" && (
          <p className="text-sm text-blue-300 text-center">
            Downloading v{updateVersion}...
          </p>
        )}

        {updateStatus === "ready" && (
          <div className="space-y-2">
            <p className="text-sm text-green-300 text-center">
              Update v{updateVersion} is ready!
            </p>
            <button
              onClick={installUpdate}
              className="w-full px-4 py-2 rounded-lg text-sm font-medium bg-green-600 hover:bg-green-500 text-white transition-colors"
            >
              Restart to Apply
            </button>
          </div>
        )}

        {updateStatus === "up-to-date" && (
          <p className="text-sm text-green-400 text-center">
            You're on the latest version!
          </p>
        )}

        {updateStatus === "error" && (
          <div className="space-y-2">
            <p className="text-sm text-red-400 text-center">
              {updateError || "Failed to check for updates"}
            </p>
            <button
              onClick={checkForUpdates}
              className="w-full px-4 py-2 rounded-lg text-sm font-medium bg-gray-700 hover:bg-gray-600 text-gray-200 transition-colors"
            >
              Try Again
            </button>
          </div>
        )}
      </div>

      {info?.log_path && (
        <div className="bg-gray-800/50 rounded-lg p-4 space-y-2">
          <h3 className="text-sm font-semibold text-gray-200">Diagnostics</h3>
          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-400 truncate flex-1" title={info.log_path}>
              Log: {info.log_path}
            </span>
            <button
              onClick={() => navigator.clipboard.writeText(info.log_path)}
              className="px-2 py-1 text-xs bg-gray-700 hover:bg-gray-600 text-gray-300 rounded transition-colors shrink-0"
            >
              Copy path
            </button>
          </div>
        </div>
      )}

      <p className="text-xs text-gray-600 text-center">
        Built by Alish. Privacy-first: audio is processed locally by default.
      </p>
    </div>
  );
}
