import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { inputStyles, textareaStyles, buttonVariants, focusRing } from "./ui/styles";
import { Select } from "./ui/Select";
import type { Profile } from "../shared/types";
import {
  getProfiles,
  saveProfile,
  deleteProfile,
  resetProfileToDefault,
} from "../shared/platform";

// Display-friendly key names
const KEY_DISPLAY: Record<string, string> = {
  ControlLeft: "L-Ctrl",
  ControlRight: "R-Ctrl",
  ShiftLeft: "L-Shift",
  ShiftRight: "R-Shift",
  AltLeft: "L-Alt",
  AltRight: "R-Alt",
  MetaLeft: "L-Win",
  MetaRight: "R-Win",
  CapsLock: "CapsLock",
  Space: "Space",
  BackSpace: "Backspace",
  Return: "Enter",
};

// Map browser e.code to rdev key name (same as KeybindingInput)
const CODE_TO_RDEV: Record<string, string> = {
  ControlLeft: "ControlLeft",
  ControlRight: "ControlRight",
  ShiftLeft: "ShiftLeft",
  ShiftRight: "ShiftRight",
  AltLeft: "AltLeft",
  AltRight: "AltRight",
  MetaLeft: "MetaLeft",
  MetaRight: "MetaRight",
  Space: "Space",
  CapsLock: "CapsLock",
  Escape: "Escape",
  Tab: "Tab",
  Backspace: "BackSpace",
  Enter: "Return",
  ArrowUp: "UpArrow",
  ArrowDown: "DownArrow",
  ArrowLeft: "LeftArrow",
  ArrowRight: "RightArrow",
  Delete: "Delete",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  PrintScreen: "PrintScreen",
  ScrollLock: "ScrollLock",
  Pause: "Pause",
  NumLock: "NumLock",
  F1: "F1",
  F2: "F2",
  F3: "F3",
  F4: "F4",
  F5: "F5",
  F6: "F6",
  F7: "F7",
  F8: "F8",
  F9: "F9",
  F10: "F10",
  F11: "F11",
  F12: "F12",
};

function displayKey(key: string): string {
  return KEY_DISPLAY[key] || key;
}

interface ProfileEditorProps {
  activeProfileId: string;
  onProfileChange: (profileId: string) => void;
  dictationHotkey: string;
  commandHotkey: string;
}

export function ProfileEditor({
  activeProfileId,
  onProfileChange,
  dictationHotkey,
  commandHotkey,
}: ProfileEditorProps) {
  const { t } = useTranslation();
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [editingProfile, setEditingProfile] = useState<Profile | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Track which profile is listening for a shortcut key
  const [listeningId, setListeningId] = useState<string | null>(null);
  // Conflict warning per profile
  const [conflicts, setConflicts] = useState<Record<string, string>>({});

  const TONE_OPTIONS = [
    { value: "neutral", label: t("profiles.editor.toneNeutral") },
    { value: "professional", label: t("profiles.editor.toneProfessional") },
    { value: "friendly", label: t("profiles.editor.toneFriendly") },
    { value: "excited", label: t("profiles.editor.toneExcited") },
    { value: "casual", label: t("profiles.editor.toneCasual") },
    { value: "formal", label: t("profiles.editor.toneFormal") },
    { value: "concise", label: t("profiles.editor.toneConcise") },
    { value: "empathetic", label: t("profiles.editor.toneEmpathetic") },
  ];

  useEffect(() => {
    loadProfiles();
  }, []);

  const loadProfiles = async () => {
    try {
      const result = await getProfiles();
      setProfiles(result);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleSave = async () => {
    if (!editingProfile) return;
    setError(null);
    try {
      await saveProfile(editingProfile);
      await loadProfiles();
      setEditingProfile(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDelete = async (id: string) => {
    setError(null);
    try {
      await deleteProfile(id);
      await loadProfiles();
      if (activeProfileId === id) {
        onProfileChange("general");
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const handleReset = async (id: string) => {
    setError(null);
    try {
      await resetProfileToDefault(id);
      await loadProfiles();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleCreate = async () => {
    const newProfile: Profile = {
      id: `custom-${Date.now()}`,
      name: "New Style",
      builtin: false,
      system_prompt:
        "You are a transcription post-processor.\n\nRules:\n- Fix grammar and punctuation\n- Output ONLY the corrected text, nothing else",
      terminology_hints: [],
      tone: "neutral",
      shortcut_key: "",
    };
    setError(null);
    try {
      await saveProfile(newProfile);
      await loadProfiles();
      setEditingProfile(newProfile);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleEdit = (profile: Profile) => {
    setEditingProfile({ ...profile });
  };

  // Check for shortcut key conflicts
  const checkConflict = useCallback(
    (key: string, profileId: string): string | null => {
      if (!key) return null;
      if (key === dictationHotkey) return "Dictation hotkey";
      if (key === commandHotkey) return "Command hotkey";
      const other = profiles.find(
        (p) => p.id !== profileId && p.shortcut_key === key,
      );
      if (other) return `Style "${other.name}"`;
      return null;
    },
    [profiles, dictationHotkey, commandHotkey],
  );

  // Handle shortcut key assignment via keyboard listener
  useEffect(() => {
    if (!listeningId) return;

    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      let rdevKey = CODE_TO_RDEV[e.code];
      if (!rdevKey && e.code.startsWith("Key")) {
        rdevKey = e.code.replace("Key", "");
      }
      if (!rdevKey) return;

      // Check conflict
      const conflict = checkConflict(rdevKey, listeningId);
      if (conflict) {
        setConflicts((prev) => ({
          ...prev,
          [listeningId]: t("profiles.list.conflictWarning", { conflict }),
        }));
      } else {
        setConflicts((prev) => {
          const next = { ...prev };
          delete next[listeningId];
          return next;
        });
      }

      // Save the shortcut
      const profile = profiles.find((p) => p.id === listeningId);
      if (profile) {
        const updated = { ...profile, shortcut_key: rdevKey };
        saveProfile(updated).then(() => loadProfiles());
      }
      setListeningId(null);
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [listeningId, profiles, checkConflict]);

  // -- Editor modal -----------------------------------------------------------
  if (editingProfile) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-text-primary">{t("profiles.editor.editTitle")}</h3>
          <button
            onClick={() => setEditingProfile(null)}
            className={`text-text-secondary hover:text-text-primary text-sm rounded ${focusRing}`}
          >
            &larr; {t("profiles.editor.backToList")}
          </button>
        </div>

        {error && (
          <div className="px-3 py-2 bg-error-muted border border-error rounded-lg text-error text-xs">
            {error}
          </div>
        )}

        <div className="space-y-3">
          <div>
            <label className="text-xs text-text-secondary block mb-1">{t("profiles.editor.nameLabel")}</label>
            <input
              type="text"
              value={editingProfile.name}
              onChange={(e) =>
                setEditingProfile({ ...editingProfile, name: e.target.value })
              }
              className={inputStyles}
            />
          </div>

          <div>
            <label className="text-xs text-text-secondary block mb-1">
              {t("profiles.editor.systemPromptLabel")}
            </label>
            <textarea
              value={editingProfile.system_prompt}
              onChange={(e) =>
                setEditingProfile({
                  ...editingProfile,
                  system_prompt: e.target.value,
                })
              }
              rows={6}
              className={textareaStyles}
            />
          </div>

          <div>
            <label className="text-xs text-text-secondary block mb-1">{t("profiles.editor.toneLabel")}</label>
            <Select
              value={editingProfile.tone}
              onChange={(v) =>
                setEditingProfile({ ...editingProfile, tone: v })
              }
              options={TONE_OPTIONS.map((opt) => ({
                value: opt.value,
                label: opt.label,
              }))}
            />
          </div>
        </div>

        <button
          onClick={handleSave}
          className={buttonVariants.primary}
        >
          {t("profiles.editor.save")}
        </button>
      </div>
    );
  }

  // -- Profile list -----------------------------------------------------------
  return (
    <div className="space-y-2">
      {error && (
        <div className="px-3 py-2 bg-error-muted border border-error rounded-lg text-error text-xs">
          {error}
        </div>
      )}

      {profiles.map((profile) => (
        <div key={profile.id} className="space-y-1">
          <div className="flex items-center gap-2 p-3 rounded-lg border bg-bg-raised border-border-default transition-colors">
            {/* Name */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium text-text-primary truncate">
                  {profile.name}
                </span>
                {profile.builtin && (
                  <span className="text-xs text-text-muted shrink-0">
                    {t("profiles.list.builtIn")}
                  </span>
                )}
              </div>
            </div>

            {/* Inline shortcut key button */}
            <button
              onClick={() =>
                setListeningId(
                  listeningId === profile.id ? null : profile.id,
                )
              }
              className={`px-3 py-1 rounded text-xs font-mono border transition-colors shrink-0 ${focusRing} ${
                listeningId === profile.id
                  ? "bg-warning-muted border-warning text-warning animate-pulse"
                  : profile.shortcut_key
                    ? "bg-purple-muted border-purple text-purple"
                    : "bg-bg-hover border-border-default text-text-secondary"
              }`}
              title={t("profiles.list.setShortcutTitle")}
            >
              {listeningId === profile.id
                ? t("profiles.list.pressKey")
                : profile.shortcut_key
                  ? displayKey(profile.shortcut_key)
                  : t("profiles.list.key")}
            </button>

            {/* Clear shortcut */}
            {profile.shortcut_key && listeningId !== profile.id && (
              <button
                onClick={async () => {
                  const updated = { ...profile, shortcut_key: "" };
                  await saveProfile(updated);
                  await loadProfiles();
                  setConflicts((prev) => {
                    const next = { ...prev };
                    delete next[profile.id];
                    return next;
                  });
                }}
                className={`text-text-muted hover:text-text-primary text-xs shrink-0 rounded ${focusRing}`}
                title={t("profiles.list.clearShortcutTitle")}
              >
                &#x2715;
              </button>
            )}

            {/* Edit */}
            <button
              onClick={() => handleEdit(profile)}
              className={`px-2 py-1 text-xs text-text-secondary hover:text-text-primary transition-colors shrink-0 rounded ${focusRing}`}
            >
              {t("profiles.list.edit")}
            </button>

            {/* Reset (built-in) or Delete (custom) */}
            {profile.builtin ? (
              <button
                onClick={() => handleReset(profile.id)}
                className={`px-2 py-1 text-xs text-text-secondary hover:text-accent transition-colors shrink-0 rounded ${focusRing}`}
                title={t("profiles.list.resetTitle")}
              >
                {t("profiles.list.reset")}
              </button>
            ) : (
              <button
                onClick={() => handleDelete(profile.id)}
                className={`px-2 py-1 text-xs text-text-muted hover:text-error transition-colors shrink-0 rounded ${focusRing}`}
              >
                {t("profiles.list.delete")}
              </button>
            )}
          </div>

          {/* Conflict warning */}
          {conflicts[profile.id] && (
            <p className="text-xs text-warning pl-3">
              {conflicts[profile.id]}
            </p>
          )}
        </div>
      ))}

      <button
        onClick={handleCreate}
        className={buttonVariants.secondary}
      >
        {t("profiles.list.newStyle")}
      </button>
    </div>
  );
}
