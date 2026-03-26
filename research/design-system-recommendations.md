# YOLO Voice — Design System Integration Recommendations

**Date**: 2026-03-26
**Based on**: shadcn/ui v4, Vercel Geist, Linear, Raycast, Tailwind CSS 4
**Current stack**: Tauri 2 + React 19 + TypeScript + Tailwind CSS 4 (no UI library)

---

## Table of Contents

1. [Current State Audit Summary](#1-current-state-audit-summary)
2. [Recommended Design Tokens](#2-recommended-design-tokens)
3. [Component Upgrades](#3-component-upgrades)
4. [Typography System](#4-typography-system)
5. [Spacing & Layout Standardization](#5-spacing--layout-standardization)
6. [Interaction Patterns](#6-interaction-patterns)
7. [Accessibility Improvements](#7-accessibility-improvements)
8. [Implementation Priority & Steps](#8-implementation-priority--steps)

---

## 1. Current State Audit Summary

### What's working well
- Dark theme foundation (gray-950 base) is solid
- Sidebar navigation (just implemented) matches industry standard
- Component extraction into settings sections is clean
- Toast system exists with correct animation pattern
- Pill component has excellent state-based animations

### Inconsistencies found

| Issue | Occurrences | Impact |
|-------|-------------|--------|
| **3 different section spacings** | `space-y-8`, `space-y-6`, `space-y-4` across sections | Visual rhythm breaks |
| **4 different text-gray shades for labels** | gray-200, gray-300, gray-400, gray-500 | No clear hierarchy |
| **No focus-visible rings** | 0 instances of `focus-visible:ring` | Accessibility gap |
| **Checkboxes instead of toggles** | 6 checkboxes for instant-effect settings | Doesn't match convention |
| **Inconsistent button padding** | `px-3 py-2`, `px-4 py-2`, `p-2` | Uneven visual weight |
| **Semi-transparent color chaos** | 15+ unique `/50`, `/30`, `/20`, `/10` opacity variants | Unpredictable layering |
| **No disabled state standard** | Some use `disabled:opacity-50`, others nothing | Broken affordances |

---

## 2. Recommended Design Tokens

Replace the current `@theme` block in `src/styles.css` with a comprehensive token system modeled on shadcn/ui + Raycast:

```css
@import "tailwindcss";

@theme {
  /* ─── Background Layers ─── */
  --color-bg-base:    oklch(0.130 0 0);   /* #171717 — page background */
  --color-bg-raised:  oklch(0.160 0 0);   /* #1e1e1e — sidebar, cards */
  --color-bg-overlay: oklch(0.195 0 0);   /* #272727 — popovers, modals */
  --color-bg-hover:   oklch(0.225 0 0);   /* #303030 — hover states */
  --color-bg-active:  oklch(0.260 0 0);   /* #393939 — pressed/active */

  /* ─── Borders ─── */
  --color-border-default: oklch(0.280 0 0);   /* #3a3a3a — standard border */
  --color-border-hover:   oklch(0.350 0 0);   /* #4a4a4a — hover border */
  --color-border-focus:   oklch(0.530 0.140 250); /* blue-ish focus ring */

  /* ─── Text ─── */
  --color-text-primary:   oklch(0.960 0 0);   /* #f0f0f0 — headings, labels */
  --color-text-secondary: oklch(0.680 0 0);   /* #9a9a9a — descriptions */
  --color-text-muted:     oklch(0.500 0 0);   /* #6b6b6b — hints, placeholders */
  --color-text-disabled:  oklch(0.380 0 0);   /* #4e4e4e — disabled elements */

  /* ─── Accent (Blue) ─── */
  --color-accent:       oklch(0.600 0.180 250);  /* ~#3b82f6 — primary actions */
  --color-accent-hover: oklch(0.550 0.200 250);  /* ~#2563eb — hover */
  --color-accent-muted: oklch(0.300 0.060 250);  /* ~#1e3a5f — subtle bg */

  /* ─── Semantic ─── */
  --color-success:      oklch(0.650 0.170 160);  /* ~#22c55e */
  --color-success-muted: oklch(0.250 0.050 160); /* subtle green bg */
  --color-warning:      oklch(0.750 0.150 80);   /* ~#eab308 */
  --color-warning-muted: oklch(0.280 0.050 80);  /* subtle yellow bg */
  --color-error:        oklch(0.580 0.220 25);   /* ~#ef4444 */
  --color-error-muted:  oklch(0.250 0.060 25);   /* subtle red bg */
  --color-purple:       oklch(0.580 0.200 295);  /* ~#a855f7 — command mode */
  --color-purple-muted: oklch(0.250 0.060 295);  /* subtle purple bg */

  /* ─── Radius ─── */
  --radius-sm: 0.25rem;   /* 4px — badges, small elements */
  --radius-md: 0.375rem;  /* 6px — inputs, buttons */
  --radius-lg: 0.5rem;    /* 8px — cards, containers */
  --radius-xl: 0.75rem;   /* 12px — modals, large cards */
  --radius-full: 9999px;  /* pill shapes */

  /* ─── Component Tokens ─── */
  --input-height: 2.25rem;    /* 36px (h-9) */
  --input-height-sm: 2rem;    /* 32px (h-8) */
  --button-height: 2.25rem;   /* 36px */
  --button-height-sm: 2rem;   /* 32px */
  --sidebar-width: 12rem;     /* 192px */

  /* ─── Transitions ─── */
  --duration-fast: 100ms;
  --duration-normal: 150ms;
  --duration-slow: 300ms;
}
```

### Migration from current tokens

| Current | Replace with | Why |
|---------|-------------|-----|
| `bg-gray-950` | `bg-bg-base` | Semantic naming |
| `bg-gray-800` | `bg-bg-raised` | Clearer purpose |
| `bg-gray-800/50` | `bg-bg-raised` at full opacity | Eliminates transparency guessing |
| `border-gray-700` | `border-border-default` | Consistent borders |
| `text-gray-100` / `text-gray-200` | `text-text-primary` | Single primary text color |
| `text-gray-400` / `text-gray-500` | `text-text-secondary` or `text-text-muted` | Two-tier secondary text |
| `text-blue-500` / `focus:border-blue-500` | `text-accent` / `focus:border-border-focus` | Semantic accent |

---

## 3. Component Upgrades

### 3.1 Toggle Switch (replace checkboxes)

YOLO Voice uses 6 native checkboxes for instant-effect settings (text cleanup, diagnostics, vision, launch on startup, start minimized, dictionary migration). Modern apps use toggle switches for these.

**Implementation** — Create `src/components/ui/Switch.tsx`:

```tsx
interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  label: string;
  description?: string;
}

export function Switch({ checked, onChange, disabled, label, description }: SwitchProps) {
  return (
    <label className="flex items-center justify-between gap-4 cursor-pointer group">
      <div className="flex-1 min-w-0">
        <span className="text-sm font-medium text-text-primary">{label}</span>
        {description && (
          <p className="text-xs text-text-muted mt-0.5">{description}</p>
        )}
      </div>
      <button
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={`
          relative inline-flex h-6 w-11 shrink-0 items-center rounded-full
          border-2 border-transparent transition-colors duration-150
          focus-visible:outline-none focus-visible:ring-2
          focus-visible:ring-border-focus focus-visible:ring-offset-2
          focus-visible:ring-offset-bg-base
          disabled:cursor-not-allowed disabled:opacity-50
          ${checked ? "bg-accent" : "bg-bg-active"}
        `}
      >
        <span
          className={`
            pointer-events-none block h-5 w-5 rounded-full bg-white shadow-md
            ring-0 transition-transform duration-150
            ${checked ? "translate-x-5" : "translate-x-0"}
          `}
        />
      </button>
    </label>
  );
}
```

**Where to use**: Replace all 6 checkbox instances in GeneralSection and TranscriptionSection.

### 3.2 Input Field (standardized with focus ring)

Current pattern has no focus ring, just `focus:border-blue-500`. Modern standard is border + ring.

**Implementation** — standard input class string:

```tsx
// src/components/ui/styles.ts
export const inputStyles = [
  "h-9 w-full rounded-md",
  "border border-border-default bg-bg-raised",
  "px-3 py-1 text-sm text-text-primary",
  "placeholder:text-text-muted",
  "transition-[color,box-shadow] duration-150",
  "focus-visible:border-border-focus",
  "focus-visible:ring-[3px] focus-visible:ring-border-focus/30",
  "focus-visible:outline-none",
  "disabled:cursor-not-allowed disabled:opacity-50",
].join(" ");

export const selectStyles = inputStyles;  // Same base for selects

export const textareaStyles = [
  "w-full rounded-md",
  "border border-border-default bg-bg-raised",
  "px-3 py-2 text-sm text-text-primary",
  "placeholder:text-text-muted",
  "transition-[color,box-shadow] duration-150",
  "focus-visible:border-border-focus",
  "focus-visible:ring-[3px] focus-visible:ring-border-focus/30",
  "focus-visible:outline-none",
  "disabled:cursor-not-allowed disabled:opacity-50",
  "resize-y",
].join(" ");
```

### 3.3 Button Variants (standardized)

Create a shared button system with 4 variants matching shadcn/ui:

```tsx
// src/components/ui/styles.ts (continued)

const buttonBase = [
  "inline-flex items-center justify-center gap-2",
  "rounded-md text-sm font-medium whitespace-nowrap",
  "transition-colors duration-150",
  "focus-visible:outline-none focus-visible:ring-2",
  "focus-visible:ring-border-focus focus-visible:ring-offset-2",
  "focus-visible:ring-offset-bg-base",
  "disabled:pointer-events-none disabled:opacity-50",
].join(" ");

export const buttonVariants = {
  primary:   `${buttonBase} h-9 px-4 py-2 bg-accent text-white hover:bg-accent-hover`,
  secondary: `${buttonBase} h-9 px-4 py-2 bg-bg-raised border border-border-default text-text-primary hover:bg-bg-hover hover:border-border-hover`,
  ghost:     `${buttonBase} h-9 px-4 py-2 text-text-secondary hover:bg-bg-hover hover:text-text-primary`,
  danger:    `${buttonBase} h-9 px-4 py-2 bg-bg-raised border border-border-default text-text-primary hover:bg-error-muted hover:border-error hover:text-error`,
  icon:      `${buttonBase} h-9 w-9 bg-bg-raised border border-border-default text-text-secondary hover:bg-bg-hover hover:text-text-primary`,
};
```

### 3.4 Section Header (standardized)

Current: `text-xs font-semibold uppercase tracking-wider text-gray-500 mb-4` — this is good but should use a token:

```tsx
export const sectionHeader = "text-xs font-semibold uppercase tracking-wider text-text-muted mb-4";
```

### 3.5 Settings Item Row

Current settings items mix various flex layouts. Standardize:

```tsx
// Label left, control right
export function SettingRow({ label, description, children }: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="flex-1 min-w-0">
        <span className="text-sm font-medium text-text-primary">{label}</span>
        {description && (
          <p className="text-xs text-text-muted mt-0.5">{description}</p>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
```

### 3.6 Info Box / Alert (standardized)

Current: 15+ unique color combinations. Standardize to 4 semantic variants:

```tsx
const alertVariants = {
  info:    "bg-accent-muted/50 border-accent/30 text-text-secondary",
  success: "bg-success-muted/50 border-success/30 text-success",
  warning: "bg-warning-muted/50 border-warning/30 text-warning",
  error:   "bg-error-muted/50 border-error/30 text-error",
};

export function Alert({ variant = "info", children }: {
  variant?: keyof typeof alertVariants;
  children: React.ReactNode;
}) {
  return (
    <div className={`p-3 rounded-lg border text-sm ${alertVariants[variant]}`}>
      {children}
    </div>
  );
}
```

### 3.7 Separator

Current: `border-t border-gray-700/50 pt-4` and `w-px h-5 bg-gray-700` used inconsistently.

```tsx
export function Separator({ className }: { className?: string }) {
  return <div className={`h-px w-full bg-border-default ${className ?? ""}`} />;
}
```

---

## 4. Typography System

### Current problems
- `text-gray-200` and `text-gray-100` both used for "primary" text
- `text-gray-400` and `text-gray-500` both used for "muted" text
- No consistent weight pairing with sizes
- Section headers use `text-lg` in page title but `text-xs uppercase` in subsections — correct hierarchy but undocumented

### Recommended scale

| Role | Classes | Used for |
|------|---------|----------|
| **Page title** | `text-lg font-semibold text-text-primary` | Settings page heading |
| **Section header** | `text-xs font-semibold uppercase tracking-wider text-text-muted` | Subsection dividers |
| **Item label** | `text-sm font-medium text-text-primary` | Setting names |
| **Item description** | `text-xs text-text-muted` | Help text under labels |
| **Body text** | `text-sm text-text-secondary` | General content |
| **Caption** | `text-xs text-text-muted` | Footnotes, counters |
| **Mono/key** | `text-xs font-mono text-text-secondary` | Keyboard shortcuts, API keys |
| **Badge** | `text-xs font-semibold` | Status pills |

---

## 5. Spacing & Layout Standardization

### Current inconsistencies

| Component | Section gap | Item gap | Issue |
|-----------|-----------|----------|-------|
| GeneralSection | `space-y-8` | `space-y-3` | Good |
| HotkeySection | `space-y-8` | `space-y-3` | Good |
| CommandSection | `space-y-6` | `space-y-4` | Different from above |
| TranscriptionSection | `space-y-8` | `space-y-4` | Mixed |
| ProfilesSection | — | — | Delegates to ProfileEditor |
| VocabularySection | `space-y-2` | — | Tighter than others |

### Standardized spacing rules

```
Page padding:              px-8 py-6 (current — keep)
Section-to-section gap:    space-y-8 (32px) — standardize everywhere
Subsection gap:            space-y-6 (24px)
Item-to-item gap:          space-y-4 (16px) — standardize everywhere
Label-to-control gap:      gap-3 (12px) — inline flex items
Section header margin:     mb-4 (16px) — after uppercase headers
```

---

## 6. Interaction Patterns

### 6.1 Focus management

**Add to all interactive elements:**
```
focus-visible:outline-none
focus-visible:ring-2 focus-visible:ring-border-focus focus-visible:ring-offset-2 focus-visible:ring-offset-bg-base
```

**Sidebar buttons** should highlight with focus-visible ring when tabbed to.

### 6.2 Toast improvements

Current: top-right, 3s auto-dismiss, slide from right.
Recommended (matches Sonner/Linear):
- **Position**: bottom-right (less intrusive for settings UI)
- **Duration**: 4s (pauses on hover)
- **Animation**: slide up + fade in (not slide from right)
- **Stacking**: Multiple toasts stack with 8px gap

### 6.3 Loading states

Add skeleton placeholders for sections that load async data:

```tsx
export function SettingSkeleton() {
  return (
    <div className="animate-pulse space-y-4">
      <div className="h-4 w-32 bg-bg-hover rounded" />
      <div className="h-9 w-full bg-bg-raised rounded-md" />
      <div className="h-9 w-full bg-bg-raised rounded-md" />
    </div>
  );
}
```

### 6.4 Keyboard shortcuts for settings navigation

Add `1-6` number keys to switch sections when no input is focused:

```tsx
useEffect(() => {
  const handler = (e: KeyboardEvent) => {
    if (document.activeElement?.tagName === "INPUT" ||
        document.activeElement?.tagName === "TEXTAREA" ||
        document.activeElement?.tagName === "SELECT") return;

    const sectionKeys: Record<string, SettingsSection> = {
      "1": "general", "2": "hotkeys", "3": "transcription",
      "4": "command", "5": "vocabulary", "6": "profiles",
    };
    if (sectionKeys[e.key]) {
      setActiveSection(sectionKeys[e.key]);
    }
  };
  window.addEventListener("keydown", handler);
  return () => window.removeEventListener("keydown", handler);
}, []);
```

---

## 7. Accessibility Improvements

| Issue | Current | Fix |
|-------|---------|-----|
| No focus rings | `focus:border-blue-500` only | Add `focus-visible:ring-2` everywhere |
| Emoji icon buttons | `🔊` with no label | Add `aria-label="Preview sound"` |
| No keyboard nav in sidebar | Click only | Add `tabIndex={0}` + Enter handler |
| Color-only status | Green/red dots | Add text labels alongside dots |
| `window.confirm()` | Native browser dialog | Consider accessible modal (future) |
| No skip-to-content | None | Add for keyboard users (optional for desktop app) |

### Minimum implementation

1. Add `focus-visible:ring-2 focus-visible:ring-border-focus` to all buttons and inputs
2. Add `aria-label` to all icon-only buttons
3. Replace emoji 🔊 buttons with SVG icon + aria-label
4. Add `role="switch"` + `aria-checked` to all toggle switches

---

## 8. Implementation Priority & Steps

### Priority 1: Design Tokens (foundation — do first)

**File**: `src/styles.css`
**Effort**: 15 minutes
**Steps**:
1. Replace current `@theme` block with the full token set from Section 2
2. No component changes needed yet — old Tailwind classes still work
3. New components can start using semantic token classes immediately

### Priority 2: Shared UI Styles

**File**: Create `src/components/ui/styles.ts`
**Effort**: 10 minutes
**Steps**:
1. Create the shared styles file with `inputStyles`, `buttonVariants`, `sectionHeader`, alert variants
2. Components can import and use these incrementally

### Priority 3: Switch Component

**File**: Create `src/components/ui/Switch.tsx`
**Effort**: 15 minutes
**Steps**:
1. Build the Switch component (code in Section 3.1)
2. Replace checkboxes in GeneralSection (2 instances: launch on startup, start minimized)
3. Replace checkboxes in TranscriptionSection (2 instances: text cleanup, diagnostics enabled)
4. Replace checkbox in CommandSection (1 instance: vision toggle)

### Priority 4: Focus Rings

**Files**: All components
**Effort**: 20 minutes
**Steps**:
1. Add focus-visible ring classes to all `<button>` elements across:
   - Settings.tsx sidebar buttons
   - All section components
   - KeybindingInput, MicSelector, ModelSelector
   - Toast dismiss buttons
   - App.tsx nav buttons
2. Add focus-visible ring classes to all `<input>`, `<select>`, `<textarea>` elements
3. Replace `focus:outline-none focus:border-blue-500` with the standardized pattern

### Priority 5: Spacing Standardization

**Files**: All settings section components
**Effort**: 15 minutes
**Steps**:
1. Set all top-level section wrappers to `space-y-8`
2. Set all subsection groups to `space-y-4`
3. Set all section header margins to `mb-4`
4. Set all label-control gaps to `gap-3`

### Priority 6: Text Color Consolidation

**Files**: All components
**Effort**: 20 minutes (find-and-replace)
**Steps**:
1. Replace `text-gray-100` and `text-gray-200` with `text-text-primary`
2. Replace `text-gray-300` with `text-text-primary` (it's used for labels)
3. Replace `text-gray-400` with `text-text-secondary`
4. Replace `text-gray-500` and `text-gray-600` with `text-text-muted`
5. Leave semantic colors (green, red, blue, purple, amber) as-is — they serve specific status purposes

### Priority 7: Button & Input Standardization

**Files**: All components
**Effort**: 30 minutes
**Steps**:
1. Import `inputStyles` from shared styles
2. Replace all inline input class strings with the shared constant
3. Import `buttonVariants` from shared styles
4. Replace inline button class strings with appropriate variant

### Future Priorities (not in initial pass)

| Feature | Effort | Value |
|---------|--------|-------|
| SettingRow component | Low | Consistent label-control layout |
| Alert component | Low | Replace 15+ unique alert color combos |
| Separator component | Low | Replace inconsistent dividers |
| Skeleton loading states | Medium | Better perceived performance |
| Keyboard shortcuts for section nav | Low | Power user feature |
| Toast position change (bottom-right) | Low | Less intrusive |
| Accessible modal (replace window.confirm) | Medium | Better UX + accessibility |
| Command palette (Ctrl+K) | High | Power user search across settings |

---

## Appendix: Design System Sources

| System | What we took | URL |
|--------|-------------|-----|
| shadcn/ui v4 | Token architecture, input/button/switch patterns, focus ring pattern | ui.shadcn.com |
| Vercel Geist | Color scale philosophy, typography naming, spacing scale | vercel.com/geist |
| Linear | LCH color approach, settings sidebar pattern, keyboard shortcuts, performance targets | linear.app |
| Raycast | Dark theme accent families, settings tab layout, toggle patterns | manual.raycast.com |
| Tailwind CSS 4 | @theme directive syntax, default scales, animation tokens | tailwindcss.com/docs |
