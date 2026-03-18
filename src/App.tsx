import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Settings } from "./pages/Settings";
import { About } from "./pages/About";
import { Onboarding } from "./pages/Onboarding";

type Page = "settings" | "about";

function App() {
  const [onboarded, setOnboarded] = useState<boolean | null>(null);
  const [page, setPage] = useState<Page>("settings");

  useEffect(() => {
    invoke<{ onboarding_completed: boolean }>("get_config")
      .then((config) => setOnboarded(config.onboarding_completed))
      .catch(() => setOnboarded(true)); // On error, skip onboarding
  }, []);

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

  // Main app
  return (
    <div className="min-h-screen bg-gray-950 text-gray-100">
      {/* Navigation */}
      <div className="flex items-center justify-between border-b border-gray-800 px-6 py-3">
        <h1 className="text-xl font-bold">YOLO Voice</h1>
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
            onClick={() => invoke("quit_app")}
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
