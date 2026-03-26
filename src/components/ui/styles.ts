/** Shared design-token–based style constants for settings UI */

export const focusRing =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-border-focus focus-visible:ring-offset-2 focus-visible:ring-offset-bg-base";

export const inputStyles =
  `bg-bg-raised border border-border-default text-text-primary rounded-lg px-3 py-2 text-sm transition-colors ${focusRing}`;

export const selectStyles =
  `bg-bg-raised border border-border-default text-text-primary rounded-lg px-3 py-2 text-sm transition-colors ${focusRing}`;

export const textareaStyles =
  `w-full bg-bg-raised border border-border-default text-text-primary rounded-lg px-3 py-2 text-sm resize-y transition-colors ${focusRing}`;

export const buttonVariants = {
  primary:
    `px-4 py-2 bg-accent hover:bg-accent-hover text-white rounded-lg text-sm font-medium transition-colors ${focusRing}`,
  secondary:
    `px-4 py-2 bg-bg-hover hover:bg-bg-active text-text-primary rounded-lg text-sm font-medium transition-colors ${focusRing}`,
  danger:
    `px-3 py-2 rounded-lg bg-bg-raised border border-border-default text-sm text-text-primary hover:border-error hover:text-error disabled:opacity-50 disabled:cursor-not-allowed transition-colors ${focusRing}`,
  icon: `p-2 bg-bg-raised border border-border-default rounded-lg text-text-secondary hover:border-accent hover:text-accent transition-colors ${focusRing}`,
} as const;

export const sectionHeader =
  "text-xs font-semibold uppercase tracking-wider text-text-muted mb-4";

export const labelStyles = "text-sm font-medium text-text-primary";

export const descStyles = "text-xs text-text-muted";

export const infoBoxStyles =
  "p-3 bg-bg-raised border border-border-default rounded-lg text-xs text-text-secondary space-y-2";

export const cardStyles =
  "p-3 rounded-lg border bg-bg-raised border-border-default hover:border-border-hover transition-colors";

export const cardActiveStyles =
  "p-3 rounded-lg border bg-accent-muted border-accent transition-colors";

export const linkStyles =
  "text-accent hover:text-accent-hover transition-colors";

export const badgeStyles =
  "text-xs font-medium px-2 py-0.5 rounded-full";

export const dividerStyles =
  "border-t border-border-default";
