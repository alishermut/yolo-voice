import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";

interface KeybindingInputProps {
  value: string;
  onChange: (key: string) => void;
  /** When true, accumulates multiple keys into a chord (e.g. "ControlLeft+ShiftLeft"). */
  chord?: boolean;
}

// Map browser e.code values to rdev-compatible key names
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
  Insert: "Insert",
  PrintScreen: "PrintScreen",
  ScrollLock: "ScrollLock",
  Pause: "Pause",
  NumLock: "NumLock",
  // Numpad keys
  Numpad0: "Kp0",
  Numpad1: "Kp1",
  Numpad2: "Kp2",
  Numpad3: "Kp3",
  Numpad4: "Kp4",
  Numpad5: "Kp5",
  Numpad6: "Kp6",
  Numpad7: "Kp7",
  Numpad8: "Kp8",
  Numpad9: "Kp9",
  NumpadEnter: "KpReturn",
  NumpadAdd: "KpPlus",
  NumpadSubtract: "KpMinus",
  NumpadMultiply: "KpMultiply",
  NumpadDivide: "KpDivide",
  NumpadDecimal: "KpDelete",
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

// Display-friendly labels
const DISPLAY_NAMES: Record<string, string> = {
  ControlLeft: "Left Ctrl",
  ControlRight: "Right Ctrl",
  ShiftLeft: "Left Shift",
  ShiftRight: "Right Shift",
  AltLeft: "Left Alt",
  AltRight: "Right Alt",
  MetaLeft: "Left Win",
  MetaRight: "Right Win",
  CapsLock: "CapsLock",
  Space: "Space",
  BackSpace: "Backspace",
  Return: "Enter",
  Insert: "Insert",
  Kp0: "Numpad 0",
  Kp1: "Numpad 1",
  Kp2: "Numpad 2",
  Kp3: "Numpad 3",
  Kp4: "Numpad 4",
  Kp5: "Numpad 5",
  Kp6: "Numpad 6",
  Kp7: "Numpad 7",
  Kp8: "Numpad 8",
  Kp9: "Numpad 9",
  KpReturn: "Numpad Enter",
  KpPlus: "Numpad +",
  KpMinus: "Numpad -",
  KpMultiply: "Numpad *",
  KpDivide: "Numpad /",
  KpDelete: "Numpad .",
  Digit0: "0",
  Digit1: "1",
  Digit2: "2",
  Digit3: "3",
  Digit4: "4",
  Digit5: "5",
  Digit6: "6",
  Digit7: "7",
  Digit8: "8",
  Digit9: "9",
};

/** Convert a browser e.code to an rdev key name. */
function codeToRdev(code: string): string | null {
  if (CODE_TO_RDEV[code]) return CODE_TO_RDEV[code];
  if (code.startsWith("Key")) return code.replace("Key", "");
  if (code.startsWith("Digit")) return code;
  return null;
}

/** Display-friendly name for an rdev key string. */
function displayName(rdevKey: string): string {
  return DISPLAY_NAMES[rdevKey] || rdevKey;
}

/** Format a "+"-separated chord value for display. */
function formatChordDisplay(value: string): string {
  if (!value) return "";
  return value.split("+").map(displayName).join(" + ");
}

export function KeybindingInput({ value, onChange, chord }: KeybindingInputProps) {
  const { t } = useTranslation();
  const [listening, setListening] = useState(false);
  const [unsupportedHint, setUnsupportedHint] = useState(false);

  // --- Chord mode: accumulate held keys, register on full release ---
  const heldKeys = useRef<Set<string>>(new Set());
  const peakKeys = useRef<Set<string>>(new Set());
  const [chordPreview, setChordPreview] = useState("");

  const handleChordKeyDown = useCallback((e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const rdev = codeToRdev(e.code);
    if (!rdev) {
      setUnsupportedHint(true);
      return;
    }
    setUnsupportedHint(false);
    heldKeys.current.add(rdev);
    peakKeys.current.add(rdev);
    setChordPreview(Array.from(peakKeys.current).map(displayName).join(" + "));
  }, []);

  const handleChordKeyUp = useCallback((e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const rdev = codeToRdev(e.code);
    if (rdev) heldKeys.current.delete(rdev);

    // All keys released → register the chord
    if (heldKeys.current.size === 0 && peakKeys.current.size > 0) {
      const chord = Array.from(peakKeys.current).join("+");
      onChange(chord);
      peakKeys.current.clear();
      setChordPreview("");
      setUnsupportedHint(false);
      setListening(false);
    }
  }, [onChange]);

  // --- Single-key mode (original behavior) ---
  const handleSingleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const rdevKey = codeToRdev(e.code);
      if (rdevKey) {
        setUnsupportedHint(false);
        onChange(rdevKey);
        setListening(false);
      } else {
        setUnsupportedHint(true);
      }
    },
    [onChange],
  );

  // Attach/detach listeners
  useEffect(() => {
    if (!listening) return;

    if (chord) {
      // Reset accumulation state
      heldKeys.current.clear();
      peakKeys.current.clear();
      setChordPreview("");
      window.addEventListener("keydown", handleChordKeyDown);
      window.addEventListener("keyup", handleChordKeyUp);
      return () => {
        window.removeEventListener("keydown", handleChordKeyDown);
        window.removeEventListener("keyup", handleChordKeyUp);
      };
    } else {
      window.addEventListener("keydown", handleSingleKeyDown);
      return () => window.removeEventListener("keydown", handleSingleKeyDown);
    }
  }, [listening, chord, handleChordKeyDown, handleChordKeyUp, handleSingleKeyDown]);

  const displayValue = chord
    ? formatChordDisplay(value) || t("keybinding.placeholder")
    : displayName(value) || t("keybinding.placeholder");

  return (
    <div className="flex flex-col items-start gap-1">
      <button
        onClick={() => {
          setUnsupportedHint(false);
          setListening(!listening);
        }}
        className={`px-4 py-2 rounded-lg text-sm font-medium border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-border-focus focus-visible:ring-offset-2 focus-visible:ring-offset-bg-base ${
          listening
            ? "bg-warning-muted border-warning text-warning animate-pulse"
            : "bg-bg-raised border-border-default text-text-primary hover:border-border-hover"
        }`}
      >
        {listening ? (
          chordPreview || t("keybinding.listening")
        ) : (
          <span className="text-text-secondary">{displayValue}</span>
        )}
      </button>
      {unsupportedHint && (
        <span className="text-xs text-warning">{t("keybinding.unsupported")}</span>
      )}
    </div>
  );
}
