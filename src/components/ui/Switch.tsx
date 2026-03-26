import { useId } from "react";

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  /** Accessible label — rendered as invisible aria text when children are used instead */
  label?: string;
}

export function Switch({ checked, onChange, disabled, label }: SwitchProps) {
  const id = useId();

  return (
    <button
      id={id}
      role="switch"
      type="button"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`
        relative inline-flex h-5 w-9 shrink-0 items-center rounded-full
        border-2 border-transparent transition-colors duration-150
        focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-border-focus
        focus-visible:ring-offset-2 focus-visible:ring-offset-bg-base
        disabled:cursor-not-allowed disabled:opacity-50
        ${checked ? "bg-success" : "bg-bg-active"}
      `}
    >
      <span
        className={`
          pointer-events-none inline-block h-3.5 w-3.5 rounded-full
          bg-white shadow-sm transition-transform duration-150
          ${checked ? "translate-x-4" : "translate-x-0.5"}
        `}
      />
    </button>
  );
}
