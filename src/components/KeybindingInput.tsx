import { useState, useEffect, useCallback } from "react";

interface KeybindingInputProps {
  value: string;
  onChange: (key: string) => void;
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

export function KeybindingInput({ value, onChange }: KeybindingInputProps) {
  const [listening, setListening] = useState(false);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Use e.code for precise key identification (left/right aware)
      const rdevKey = CODE_TO_RDEV[e.code];

      if (rdevKey) {
        onChange(rdevKey);
        setListening(false);
      } else if (e.code.startsWith("Key")) {
        // Letter keys: KeyA → A
        const letter = e.code.replace("Key", "");
        onChange(letter);
        setListening(false);
      } else if (e.code.startsWith("Digit")) {
        // Number keys
        onChange(e.code);
        setListening(false);
      }
    },
    [onChange],
  );

  useEffect(() => {
    if (listening) {
      window.addEventListener("keydown", handleKeyDown);
      return () => window.removeEventListener("keydown", handleKeyDown);
    }
  }, [listening, handleKeyDown]);

  const displayValue = DISPLAY_NAMES[value] || value || "Click to set";

  return (
    <button
      onClick={() => setListening(!listening)}
      className={`px-4 py-2 rounded-lg text-sm font-medium border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-border-focus focus-visible:ring-offset-2 focus-visible:ring-offset-bg-base ${
        listening
          ? "bg-warning-muted border-warning text-warning animate-pulse"
          : "bg-bg-raised border-border-default text-text-primary hover:border-border-hover"
      }`}
    >
      {listening ? (
        "Press any key..."
      ) : (
        <span className="text-text-secondary">{displayValue}</span>
      )}
    </button>
  );
}
