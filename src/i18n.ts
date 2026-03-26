import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./locales/en.json";

// All locale bundles are imported statically — this is a desktop app,
// so lazy-loading buys us nothing and avoids async complexity.
const resources: Record<string, { translation: Record<string, string> }> = {
  en: { translation: en },
};

// Dynamically import locale files that exist.
// New languages are registered by adding a JSON file to src/locales/
// and an entry in this map.
const localeModules = import.meta.glob<{ default: Record<string, string> }>(
  "./locales/*.json",
  { eager: true },
);

for (const [path, mod] of Object.entries(localeModules)) {
  const code = path.replace("./locales/", "").replace(".json", "");
  if (code !== "en") {
    resources[code] = { translation: mod.default };
  }
}

i18n.use(initReactI18next).init({
  resources,
  lng: "en",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;

/** Languages available in the UI selector, shown in their native name. */
export const UI_LANGUAGES = [
  { code: "en", name: "English" },
  { code: "ru", name: "Русский" },
  { code: "uk", name: "Українська" },
  { code: "es", name: "Español" },
  { code: "pt", name: "Português" },
  { code: "fr", name: "Français" },
  { code: "de", name: "Deutsch" },
  { code: "it", name: "Italiano" },
  { code: "pl", name: "Polski" },
  { code: "nl", name: "Nederlands" },
  { code: "cs", name: "Čeština" },
  { code: "tr", name: "Türkçe" },
  { code: "zh-CN", name: "简体中文" },
  { code: "ja", name: "日本語" },
  { code: "ko", name: "한국어" },
] as const;
