import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Settings } from "./pages/Settings";
import { About } from "./pages/About";
import { Onboarding } from "./pages/Onboarding";
import { useUpdater } from "./hooks/useUpdater";
import { getConfig, quitApp } from "./shared/platform";

type Page = "settings" | "about";

/** Spinning loader icon */
function Spinner({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`animate-spin h-3.5 w-3.5 ${className}`}
      viewBox="0 0 24 24"
      fill="none"
    >
      <circle
        className="opacity-25"
        cx="12" cy="12" r="10"
        stroke="currentColor" strokeWidth="4"
      />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
      />
    </svg>
  );
}

function App() {
  const [onboarded, setOnboarded] = useState<boolean | null>(null);
  const [page, setPage] = useState<Page>("settings");
  const { status: updateStatus, version: updateVersion, checkForUpdates, installUpdate } = useUpdater();

  useEffect(() => {
    getConfig()
      .then((config) => setOnboarded(config.onboarding_completed))
      .catch(() => setOnboarded(true));

    // Auto-check for updates on startup
    checkForUpdates();
  }, [checkForUpdates]);

  // Loading state
  if (onboarded === null) {
    return (
      <div className="min-h-screen bg-gray-950 text-gray-100 flex items-center justify-center">
        <div className="text-gray-400">Loading...</div>
      </div>
    );
  }

  // Onboarding
  if (!onboarded) {
    return <Onboarding onComplete={() => setOnboarded(true)} />;
  }

  // Derive update button appearance
  const updateButton = (() => {
    switch (updateStatus) {
      case "idle":
        return {
          label: "Check for updates",
          icon: null,
          className: "text-gray-500 hover:text-gray-300 hover:bg-gray-800",
          onClick: checkForUpdates,
          disabled: false,
        };
      case "checking":
        return {
          label: "Checking...",
          icon: <Spinner className="text-blue-400" />,
          className: "text-blue-400 bg-blue-900/20",
          onClick: undefined,
          disabled: true,
        };
      case "downloading":
        return {
          label: "Downloading update...",
          icon: <Spinner className="text-blue-400" />,
          className: "text-blue-400 bg-blue-900/20",
          onClick: undefined,
          disabled: true,
        };
      case "ready":
        return {
          label: `Restart to update (v${updateVersion})`,
          icon: (
            <span className="relative flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
              <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
            </span>
          ),
          className: "text-green-300 bg-green-900/30 hover:bg-green-900/50 border border-green-700/50",
          onClick: installUpdate,
          disabled: false,
        };
      case "up-to-date":
        return {
          label: "Up to date",
          icon: (
            <svg className="h-3.5 w-3.5 text-green-400" viewBox="0 0 20 20" fill="currentColor">
              <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
            </svg>
          ),
          className: "text-green-400",
          onClick: undefined,
          disabled: true,
        };
      case "error":
        return {
          label: "Update check failed",
          icon: null,
          className: "text-red-400/70",
          onClick: undefined,
          disabled: true,
        };
    }
  })();

  // Main app
  return (
    <div className="min-h-screen bg-gray-950 text-gray-100">
      {/* Navigation */}
      <div className="flex items-center justify-between border-b border-gray-800 px-6 py-3">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-bold">YOLO Voice</h1>

          {/* Update button — always visible in nav bar */}
          <button
            onClick={updateButton.onClick}
            disabled={updateButton.disabled}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium transition-all ${updateButton.className} ${updateButton.disabled ? "cursor-default" : "cursor-pointer"}`}
          >
            {updateButton.icon}
            {updateButton.label}
          </button>
        </div>

        <div className="flex items-center gap-3">
          <nav className="flex gap-1">
            <button
              onClick={() => setPage("settings")}
              className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                page === "settings"
                  ? "bg-gray-700 text-white"
                  : "text-gray-400 hover:text-gray-200"
              }`}
            >
              Settings
            </button>
            <button
              onClick={() => setPage("about")}
              className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                page === "about"
                  ? "bg-gray-700 text-white"
                  : "text-gray-400 hover:text-gray-200"
              }`}
            >
              About
            </button>
          </nav>

          <div className="w-px h-5 bg-gray-700" />

          <button
            onClick={() => getCurrentWindow().hide()}
            className="px-3 py-1.5 rounded-lg text-sm font-medium text-gray-400 hover:text-gray-200 transition-colors"
            title="Hide to tray"
          >
            Hide
          </button>
          <button
            onClick={() => quitApp()}
            className="px-3 py-1.5 rounded-lg text-sm font-medium text-red-400 hover:text-red-300 hover:bg-red-900/30 transition-colors"
            title="Quit YOLO Voice completely"
          >
            Quit
          </button>
        </div>
      </div>

      {/* Page content */}
      <div className="p-6">
        {page === "settings" && <Settings />}
        {page === "about" && <About />}
      </div>
    </div>
  );
}

export default App;
