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
  AltLeft: "Alt",
  AltRight: "Alt",
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

// Display-friendly labels
const DISPLAY_NAMES: Record<string, string> = {
  ControlLeft: "Left Ctrl",
  ControlRight: "Right Ctrl",
  ShiftLeft: "Left Shift",
  ShiftRight: "Right Shift",
  Alt: "Alt",
  MetaLeft: "Left Win",
  MetaRight: "Right Win",
  CapsLock: "CapsLock",
  Space: "Space",
  BackSpace: "Backspace",
  Return: "Enter",
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
      className={`px-4 py-2 rounded-lg text-sm font-medium border transition-colors ${
        listening
          ? "bg-yellow-600/20 border-yellow-500 text-yellow-300 animate-pulse"
          : "bg-gray-800 border-gray-700 text-gray-200 hover:border-gray-500"
      }`}
    >
      {listening ? "Press a key..." : displayValue}
    </button>
  );
}
