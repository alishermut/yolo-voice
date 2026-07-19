import { useEffect, useRef, useState, type InputHTMLAttributes } from "react";

interface ConfigTextInputProps
  extends Omit<
    InputHTMLAttributes<HTMLInputElement>,
    "value" | "onChange" | "onBlur"
  > {
  value: string;
  /** Persist when the user blurs or after the debounce window. */
  onCommit: (value: string) => void;
  /** Debounce window while typing (ms). Defaults to 450. */
  debounceMs?: number;
}

/**
 * Local-state text input that avoids firing config saves on every keystroke.
 * Commits on blur, and also after a quiet debounce window.
 */
export function ConfigTextInput({
  value,
  onCommit,
  debounceMs = 450,
  ...rest
}: ConfigTextInputProps) {
  const [local, setLocal] = useState(value);
  const localRef = useRef(local);
  const committedRef = useRef(value);
  const onCommitRef = useRef(onCommit);

  localRef.current = local;
  onCommitRef.current = onCommit;

  // Sync from parent when an external save/normalization changes the value,
  // but don't clobber in-progress typing that hasn't been committed yet.
  useEffect(() => {
    if (value !== committedRef.current && value !== localRef.current) {
      setLocal(value);
      committedRef.current = value;
    } else if (value === localRef.current) {
      committedRef.current = value;
    }
  }, [value]);

  useEffect(() => {
    if (local === committedRef.current) return;
    const timer = window.setTimeout(() => {
      if (localRef.current !== committedRef.current) {
        committedRef.current = localRef.current;
        onCommitRef.current(localRef.current);
      }
    }, debounceMs);
    return () => window.clearTimeout(timer);
  }, [local, debounceMs]);

  return (
    <input
      {...rest}
      value={local}
      onChange={(e) => setLocal(e.target.value)}
      onBlur={() => {
        if (localRef.current !== committedRef.current) {
          committedRef.current = localRef.current;
          onCommitRef.current(localRef.current);
        }
      }}
    />
  );
}
