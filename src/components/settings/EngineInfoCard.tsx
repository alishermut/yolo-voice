import { useState } from "react";
import { useTranslation } from "react-i18next";

const PARAKEET_LANGUAGES: Record<string, string> = {
  en: "English", bg: "Bulgarian", hr: "Croatian", cs: "Czech", da: "Danish",
  nl: "Dutch", et: "Estonian", fi: "Finnish", fr: "French", de: "German",
  el: "Greek", hu: "Hungarian", it: "Italian", lv: "Latvian", lt: "Lithuanian",
  mt: "Maltese", pl: "Polish", pt: "Portuguese", ro: "Romanian", ru: "Russian",
  sk: "Slovak", sl: "Slovenian", es: "Spanish", sv: "Swedish", uk: "Ukrainian",
};

const DISTIL_PRIMARY_LANGUAGES = ["English"];

function LanguageChip({ name }: { name: string }) {
  return (
    <span className="inline-block text-[10px] px-1.5 py-0.5 rounded bg-bg-active text-text-secondary">
      {name}
    </span>
  );
}

function SpeedRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline gap-2">
      <span className="text-text-muted text-[10px] w-10 shrink-0">{label}</span>
      <span className="text-text-secondary text-[10px]">{value}</span>
    </div>
  );
}

export function OfflineInfoCard({ engine }: { engine: "parakeet" | "distil_whisper" }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const isDistil = engine === "distil_whisper";
  const langs = isDistil ? DISTIL_PRIMARY_LANGUAGES : Object.values(PARAKEET_LANGUAGES);

  return (
    <div>
      <button
        onClick={() => setOpen(!open)}
        className="text-[10px] text-text-muted hover:text-accent transition-colors flex items-center gap-1"
      >
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm-.75 3.5a.75.75 0 111.5 0 .75.75 0 01-1.5 0zM7 7h2v4.5H7V7z" />
        </svg>
        {open ? t("engine.toggleHide") : t("engine.toggleShow")}
      </button>

      {open && (
        <div className="mt-2 p-3 bg-bg-raised border border-border-default rounded-lg space-y-3">
          <div>
            <p className="text-[10px] font-medium text-text-primary mb-1.5">
              {isDistil
                ? t("engine.offline.distilLanguageCount", {
                    defaultValue: "Primary language: English",
                  })
                : t("engine.offline.languageCount")}
            </p>
            <div className="flex flex-wrap gap-1">
              {langs.map((l) => <LanguageChip key={l} name={l} />)}
            </div>
            {isDistil && (
              <p className="text-[10px] text-text-muted mt-2">
                {t("engine.offline.distilLanguageNote", {
                  defaultValue:
                    "This exact Distil-Whisper model is tuned for English dictation. It comes from the Whisper family, but it should be presented as English-focused in the app.",
                })}
              </p>
            )}
          </div>

          <div className="border-t border-border-default pt-2">
            <p className="text-[10px] font-medium text-text-primary mb-1">
              {t("engine.offline.speedHeading")}
            </p>
            {isDistil ? (
              <>
                <SpeedRow
                  label={t("engine.offline.speedGpuLabel")}
                  value={t("engine.offline.distilSpeedGpu", {
                    defaultValue: "Best on GPU. Strong long-form English dictation performance with much lower latency than heavier quality-first models.",
                  })}
                />
                <SpeedRow
                  label={t("engine.offline.speedCpuLabel")}
                  value={t("engine.offline.distilSpeedCpu", {
                    defaultValue: "Works on CPU, but expect significantly slower whole-clip processing.",
                  })}
                />
              </>
            ) : (
              <>
                <SpeedRow label={t("engine.offline.speedGpuLabel")} value={t("engine.offline.speedGpu")} />
                <SpeedRow label={t("engine.offline.speedCpuLabel")} value={t("engine.offline.speedCpu")} />
              </>
            )}
          </div>

              <p className="text-[10px] text-text-muted">
            {isDistil
              ? t("engine.offline.distilNote", {
                  defaultValue:
                    "Uses whole-clip transcription with external speech compaction first. Slower than Parakeet, but usually stronger on long-form English dictation.",
                })
              : t("engine.offline.privacyNote")}
          </p>
        </div>
      )}
    </div>
  );
}

export function CloudInfoCard({ provider }: { provider: string }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  const isGroq = provider === "groq";
  const name = isGroq ? t("engine.cloud.nameGroq") : t("engine.cloud.nameDeepgram");
  const langCount = isGroq ? t("engine.cloud.languageCountGroq") : t("engine.cloud.languageCountDeepgram");

  const allLangs = isGroq
    ? [...Object.values(PARAKEET_LANGUAGES), "Arabic", "Chinese", "Hindi", "Indonesian",
       "Japanese", "Korean", "Malay", "Norwegian", "Thai", "Turkish", "Vietnamese"].sort()
    : [...Object.values(PARAKEET_LANGUAGES), "Arabic", "Chinese", "Hindi", "Indonesian",
       "Japanese", "Korean", "Malay", "Norwegian", "Thai", "Turkish", "Vietnamese"].sort();

  return (
    <div>
      <button
        onClick={() => setOpen(!open)}
        className="text-[10px] text-text-muted hover:text-accent transition-colors flex items-center gap-1"
      >
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm-.75 3.5a.75.75 0 111.5 0 .75.75 0 01-1.5 0zM7 7h2v4.5H7V7z" />
        </svg>
        {open ? t("engine.toggleHide") : t("engine.toggleShow")}
      </button>

      {open && (
        <div className="mt-2 p-3 bg-bg-raised border border-border-default rounded-lg space-y-3">
          <div>
            <p className="text-[10px] font-medium text-text-primary mb-1">
              {name}
            </p>
          </div>

          <div>
            <p className="text-[10px] font-medium text-text-primary mb-1.5">
              {langCount} languages
            </p>
            <div className="flex flex-wrap gap-1">
              {allLangs.map((l) => <LanguageChip key={l} name={l} />)}
            </div>
          </div>

          <div className="border-t border-border-default pt-2">
            <p className="text-[10px] font-medium text-text-primary mb-1">
              {t("engine.cloud.speedHeading")}
            </p>
            <SpeedRow label="" value={t("engine.cloud.speedValue")} />
          </div>

          <p className="text-[10px] text-text-muted">
            {isGroq ? t("engine.cloud.privacyNoteGroq") : t("engine.cloud.privacyNoteDeepgram")}
          </p>
        </div>
      )}
    </div>
  );
}
