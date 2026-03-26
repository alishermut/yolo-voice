import { useState, useEffect } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Settings } from "./pages/Settings";
import { Onboarding } from "./pages/Onboarding";
import { UpdaterProvider } from "./contexts/UpdaterContext";
import { getConfig } from "./shared/platform";
import i18n from "./i18n";

function AppContent() {
  const [onboarded, setOnboarded] = useState<boolean | null>(null);

  useEffect(() => {
    getConfig()
      .then((config) => {
        setOnboarded(config.onboarding_completed);
        if (config.ui_language && config.ui_language !== i18n.language) {
          i18n.changeLanguage(config.ui_language);
        }
      })
      .catch(() => setOnboarded(true));
  }, []);

  // Listen for self-insert-text: when dictation target is our own app,
  // insert text into the currently focused input/textarea element.
  useEffect(() => {
    const unlisten = getCurrentWebviewWindow().listen<string>("self-insert-text", (event: { payload: string }) => {
      const el = document.activeElement;
      if (el && (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement)) {
        const start = el.selectionStart ?? el.value.length;
        const end = el.selectionEnd ?? el.value.length;
        const before = el.value.slice(0, start);
        const after = el.value.slice(end);
        const newValue = before + event.payload + after;
        const nativeSetter = Object.getOwnPropertyDescriptor(
          el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype,
          "value"
        )?.set;
        if (nativeSetter) {
          nativeSetter.call(el, newValue);
          el.dispatchEvent(new Event("input", { bubbles: true }));
        } else {
          el.value = newValue;
        }
        el.selectionStart = el.selectionEnd = start + event.payload.length;
      }
    });
    return () => { unlisten.then((fn: () => void) => fn()); };
  }, []);

  if (onboarded === null) {
    return (
      <div className="h-screen bg-bg-base text-text-primary flex items-center justify-center">
        <div className="text-text-secondary">Loading...</div>
      </div>
    );
  }

  if (!onboarded) {
    return <Onboarding onComplete={() => setOnboarded(true)} />;
  }

  return <Settings />;
}

function App() {
  return (
    <UpdaterProvider>
      <AppContent />
    </UpdaterProvider>
  );
}

export default App;
