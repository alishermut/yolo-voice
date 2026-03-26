# UI Design Systems Reference for Dark-Theme Desktop Apps (2025-2026)

Research compiled 2026-03-26. All values are implementable specifics extracted from
primary sources. This document covers shadcn/ui, Vercel Geist, Linear, Raycast,
Arc Browser, and Tailwind CSS 4 patterns.

---

## 1. shadcn/ui (Tailwind v4) -- Dark Theme Tokens

### Complete Dark Theme CSS Variables (OKLCH)

```css
.dark {
  --background: oklch(0.145 0 0);           /* ~#191919 - near black */
  --foreground: oklch(0.985 0 0);           /* ~#fafafa - near white */
  --card: oklch(0.205 0 0);                 /* ~#262626 - elevated surface */
  --card-foreground: oklch(0.985 0 0);
  --popover: oklch(0.205 0 0);
  --popover-foreground: oklch(0.985 0 0);
  --primary: oklch(0.922 0 0);              /* ~#e5e5e5 - primary action */
  --primary-foreground: oklch(0.205 0 0);
  --secondary: oklch(0.269 0 0);            /* ~#333333 */
  --secondary-foreground: oklch(0.985 0 0);
  --muted: oklch(0.269 0 0);               /* ~#333333 */
  --muted-foreground: oklch(0.708 0 0);    /* ~#a0a0a0 - subdued text */
  --accent: oklch(0.269 0 0);
  --accent-foreground: oklch(0.985 0 0);
  --destructive: oklch(0.704 0.191 22.216); /* red-400 */
  --border: oklch(1 0 0 / 10%);            /* white at 10% - subtle border */
  --input: oklch(1 0 0 / 15%);             /* white at 15% - input border */
  --ring: oklch(0.556 0 0);                /* ~#737373 - focus ring */

  /* Chart colors */
  --chart-1: oklch(0.488 0.243 264.376);   /* blue */
  --chart-2: oklch(0.696 0.17 162.48);     /* green */
  --chart-3: oklch(0.769 0.188 70.08);     /* amber */
  --chart-4: oklch(0.627 0.265 303.9);     /* purple */
  --chart-5: oklch(0.645 0.246 16.439);    /* red */

  /* Sidebar */
  --sidebar: oklch(0.205 0 0);
  --sidebar-foreground: oklch(0.985 0 0);
  --sidebar-primary: oklch(0.488 0.243 264.376);
  --sidebar-primary-foreground: oklch(0.985 0 0);
  --sidebar-accent: oklch(0.269 0 0);
  --sidebar-accent-foreground: oklch(0.985 0 0);
  --sidebar-border: oklch(1 0 0 / 10%);
  --sidebar-ring: oklch(0.556 0 0);
}
```

### OKLCH-to-Hex Approximations for the Dark Theme

| Token             | OKLCH Value          | Hex Approx  | Usage                    |
|-------------------|----------------------|-------------|--------------------------|
| background        | oklch(0.145 0 0)     | #191919     | Page background          |
| foreground        | oklch(0.985 0 0)     | #fafafa     | Primary text             |
| card              | oklch(0.205 0 0)     | #262626     | Card/elevated surfaces   |
| secondary         | oklch(0.269 0 0)     | #333333     | Secondary surfaces       |
| muted-foreground  | oklch(0.708 0 0)     | #a0a0a0     | Subdued/secondary text   |
| ring              | oklch(0.556 0 0)     | #737373     | Focus rings              |
| border            | oklch(1 0 0 / 10%)   | rgba(255,255,255,0.1) | Borders        |
| input             | oklch(1 0 0 / 15%)   | rgba(255,255,255,0.15)| Input borders   |

### Radius Scale (Relative to base --radius)

```css
--radius: 0.625rem;  /* default base = 10px */
--radius-sm: calc(var(--radius) * 0.6);   /* 6px */
--radius-md: calc(var(--radius) * 0.8);   /* 8px */
--radius-lg: var(--radius);                /* 10px */
--radius-xl: calc(var(--radius) * 1.4);   /* 14px */
--radius-2xl: calc(var(--radius) * 1.8);  /* 18px */
--radius-3xl: calc(var(--radius) * 2.2);  /* 22px */
--radius-4xl: calc(var(--radius) * 2.6);  /* 26px */
```

---

## 2. shadcn/ui Component Class Patterns (Tailwind v4)

### Input

```tsx
<input
  data-slot="input"
  className={cn(
    "h-9 w-full min-w-0 rounded-md border border-input bg-transparent px-3 py-1",
    "text-base shadow-xs transition-[color,box-shadow] outline-none",
    "selection:bg-primary selection:text-primary-foreground",
    "file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground",
    "placeholder:text-muted-foreground",
    "disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50",
    "md:text-sm",
    "dark:bg-input/30",
    // Focus state
    "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
    // Error state
    "aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40",
  )}
/>
```

Key details:
- Height: h-9 (36px)
- Border radius: rounded-md (0.375rem / 6px)
- Padding: px-3 py-1 (12px horizontal, 4px vertical)
- Focus: 3px ring at 50% opacity of ring color
- Dark mode: bg-input/30 (input color at 30% opacity)
- Transition: color + box-shadow
- Shadow: shadow-xs

### Button (Base + Variants)

```tsx
// Base classes
"inline-flex shrink-0 items-center justify-center gap-2 rounded-md",
"text-sm font-medium whitespace-nowrap",
"transition-all outline-none",
"focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
"disabled:pointer-events-none disabled:opacity-50",
"[&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"

// Size variants
default:   h-9 px-4 py-2        // 36px height
xs:        h-6                    // 24px height, smaller text + SVGs
sm:        h-8                    // 32px height
lg:        h-10                   // 40px height
icon:      size-9                 // 36x36px square
icon-xs:   size-6                 // 24x24px square
icon-sm:   size-8                 // 32x32px square
icon-lg:   size-10               // 40x40px square

// Visual variants
default:     bg-primary text-primary-foreground hover:bg-primary/90
destructive: bg-destructive text-white hover:bg-destructive/90
outline:     border border-input bg-background hover:bg-accent hover:text-accent-foreground
secondary:   bg-secondary text-secondary-foreground hover:bg-secondary/80
ghost:       hover:bg-accent hover:text-accent-foreground
link:        text-primary underline-offset-4 hover:underline
```

### Switch / Toggle

```tsx
// Root (track)
"peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full",
"border-2 border-transparent",
"transition-colors",
"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
"disabled:cursor-not-allowed disabled:opacity-50",
"data-[state=checked]:bg-primary data-[state=unchecked]:bg-input"

// Thumb (knob)
"pointer-events-none block h-5 w-5 rounded-full bg-background shadow-lg ring-0",
"transition-transform",
"data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0"
```

Key details:
- Track: 44px wide x 24px tall (w-11 h-6)
- Thumb: 20px x 20px (h-5 w-5)
- Both fully rounded (rounded-full)
- Checked: translate-x-5 (20px shift)
- Transition: colors on track, transform on thumb

### Card

```tsx
Card:            "flex flex-col gap-6 rounded-xl border bg-card py-6 text-card-foreground shadow-sm"
CardHeader:      "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-2 px-6
                  has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-6"
CardTitle:       "leading-none font-semibold"
CardDescription: "text-sm text-muted-foreground"
CardContent:     "px-6"
CardFooter:      "flex items-center px-6 [.border-t]:pt-6"
```

Key details:
- Border radius: rounded-xl (0.75rem / 12px)
- Internal gap: gap-6 (24px)
- Padding: py-6 px-6 (24px all around)
- Shadow: shadow-sm

### Select

```tsx
// Trigger
"flex w-fit items-center justify-between gap-2 rounded-md",
"border border-input bg-transparent px-3 py-2 text-sm whitespace-nowrap shadow-xs",
"transition-[color,box-shadow] outline-none",
"focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
"data-[size=default]:h-9 data-[size=sm]:h-8",
"dark:bg-input/30 dark:hover:bg-input/50"

// Content (dropdown)
"relative z-50 max-h-(--radix-select-content-available-height) min-w-[8rem]",
"overflow-x-hidden overflow-y-auto rounded-md border bg-popover text-popover-foreground shadow-md",
// Open/close animations
"data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
"data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
// Slide-in based on position
"data-[side=bottom]:slide-in-from-top-2 data-[side=top]:slide-in-from-bottom-2"

// Item
"relative flex w-full cursor-default items-center gap-2 rounded-sm py-1.5 pr-8 pl-2 text-sm",
"outline-hidden select-none",
"focus:bg-accent focus:text-accent-foreground",
"data-[disabled]:pointer-events-none data-[disabled]:opacity-50"

// Separator
"pointer-events-none -mx-1 my-1 h-px bg-border"

// Label
"px-2 py-1.5 text-xs text-muted-foreground"
```

### Badge

```tsx
// Base
"inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold",
"transition-colors",
"focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"

// Variants
default:     "border-transparent bg-primary text-primary-foreground hover:bg-primary/80"
secondary:   "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80"
destructive: "border-transparent bg-destructive text-destructive-foreground hover:bg-destructive/80"
outline:     "text-foreground"
```

### Separator

```tsx
"pointer-events-none -mx-1 my-1 h-px bg-border"
// or for full-width horizontal separator:
"shrink-0 bg-border h-[1px] w-full"
// vertical:
"shrink-0 bg-border w-[1px] h-full"
```

---

## 3. Vercel Geist Design System

### Dark Theme Color Palette

| Token                | Hex Value  | Usage                        |
|----------------------|------------|------------------------------|
| Background           | #000000    | Page background              |
| Foreground/Text      | #ffffff    | Primary text                 |
| Gray 100 (lightest)  | #fafafa    | Subtle background            |
| Gray 200             | #eaeaea    | Borders, dividers            |
| Gray 300             | #999999    | Tertiary text                |
| Gray 400             | #888888    | Placeholder text             |
| Gray 500             | #666666    | Secondary text               |
| Gray 600             | #444444    | Muted elements               |
| Gray 700             | #333333    | Elevated surfaces (dark)     |
| Gray 800             | #111111    | Card/panel background (dark) |
| Gray 900 (darkest)   | #000000    | Page background (dark)       |

Note: In dark mode, the scale inverts. Gray 100 becomes the darkest, Gray 900
becomes the lightest.

### Semantic Colors

| Name              | Hex Value  | Usage                          |
|-------------------|------------|--------------------------------|
| Link / Success    | #0070f3    | Links, primary actions         |
| Success Light     | #3291ff    | Hover states                   |
| Success Dark      | #0366d6    | Active/pressed states          |
| Error             | #ee0000    | Error states                   |
| Error Light       | #ff1a1a    | Error hover                    |
| Error Dark        | #cc0000    | Error active                   |
| Warning           | #f5a623    | Warning states                 |
| Warning Light     | #f7b955    | Warning hover                  |
| Alert             | #ff0080    | Alert/critical                 |
| Cyan              | #50e3c2    | Accents                        |
| Violet            | #7928ca    | Accents                        |
| Code/Magenta      | #f81ce5    | Code highlights, accents       |

### Typography Scale

| Class                  | Size     | Usage                    |
|------------------------|----------|--------------------------|
| text-heading-72        | 4.5rem   | Hero displays            |
| text-heading-64        | 4rem     | Page titles              |
| text-heading-56        | 3.5rem   | Major headings           |
| text-heading-48        | 3rem     | Section titles           |
| text-heading-40        | 2.5rem   | Sub-section titles       |
| text-heading-32        | 2rem     | Card titles              |
| text-heading-24        | 1.5rem   | Component headings       |
| text-heading-20        | 1.25rem  | Settings section headers |
| text-heading-16        | 1rem     | Small headings           |
| text-heading-14        | 0.875rem | Overlines                |
| text-copy-16           | 1rem     | Body text (default)      |
| text-copy-14           | 0.875rem | Body text (compact)      |
| text-copy-13           | 0.8125rem| Small body text          |
| text-label-14          | 0.875rem | Form labels              |
| text-label-13          | 0.8125rem| Small labels             |
| text-label-12          | 0.75rem  | Captions, tags           |
| text-button-16         | 1rem     | Large buttons            |
| text-button-14         | 0.875rem | Default buttons          |
| text-button-12         | 0.75rem  | Small buttons            |

### Font Families

- **Sans**: Geist Sans (fallback: Inter, -apple-system, system-ui)
- **Mono**: Geist Mono (fallback: ui-monospace, Menlo, Monaco, Consolas)
- **Font Weights**: Thin(100) through Black(900), Body=400, Heading=600, Bold=700

### Spacing Scale

| Token   | Value  | Pixels |
|---------|--------|--------|
| space-0 | 0      | 0px    |
| space-1 | 4pt    | 4px    |
| space-2 | 8pt    | 8px    |
| space-3 | 16pt   | 16px   |
| space-4 | 24pt   | 24px   |
| space-5 | 32pt   | 32px   |
| space-6 | 48pt   | 48px   |

### Border Radius

| Token       | Value    |
|-------------|----------|
| Default     | 5px      |
| Large       | 10px     |
| Pill        | 9999px   |

### Shadows

| Token    | Value                                  |
|----------|----------------------------------------|
| Default  | 0 4px 4px 0 rgba(0,0,0,0.02)          |
| Small    | 0 5px 10px rgba(0,0,0,0.12)           |
| Medium   | 0 8px 30px rgba(0,0,0,0.12)           |
| Large    | 0 30px 60px rgba(0,0,0,0.12)          |

### Line Heights

- Body: 1.5
- Heading: 1.2

---

## 4. Linear App

### Dark Theme Colors

| Element              | Hex Value  | Usage                       |
|----------------------|------------|-----------------------------|
| Background           | #121212    | Main page background        |
| Text                 | #cccccc    | Primary text                |
| Alt Background       | #1b1c1d    | Sidebar/panel background    |
| Accent               | #848CD0    | Primary accent (purple-blue)|
| Input Background     | #171717    | Form input backgrounds      |

### Design System Approach

- Uses LCH color space for perceptually uniform theme generation
- Only 3 core variables needed: base color, accent color, contrast
- Contrast slider (30-100) for accessibility compliance
- Theme generation auto-creates borders, elevated surfaces from base

### Typography

- **Headings**: Inter Display (more expressive for titles)
- **Body**: Inter (standard body text)

### Key UI Patterns

**Settings**: Side navigation with "inverted L-shape" chrome (sidebar + top bar)
**Command Palette**: Triggered via Cmd/Ctrl+K, fuzzy finder with shortcut hints
**Keyboard Shortcuts**: Single-key shortcuts (c=create, a=assign, l=label, p=priority)
**Performance Target**: 100ms for interactions to feel instantaneous
**Toasts**: Used for command feedback after keyboard actions

---

## 5. Raycast

### Dark Theme UI Color Tokens

| Token                    | Value     | Usage                          |
|--------------------------|-----------|--------------------------------|
| Text (default)           | #e1e1e1   | Primary text                   |
| Text (light/secondary)   | #9b9b9b   | Secondary text                 |
| Background               | #191919   | Page background                |
| Border                   | #373737   | Standard borders               |
| Border (hover)           | #262626   | Hover state borders            |
| Card Background          | #262626   | Card surfaces                  |
| Card Hover               | #2f2f2f   | Card hover state               |
| Button Text              | #191919   | Button text on light bg        |
| Button Background        | #e1e1e1   | Primary button background      |
| Hover Background         | #282828   | Nav/list item hover            |

### Dark Theme Accent Colors (HSL)

| Color   | Text HSL              | Background HSL         | Pill Background HSL    |
|---------|-----------------------|------------------------|------------------------|
| Gray    | hsl(0, 0%, 61%)       | hsl(0, 0%, 14.5%)     | hsl(0, 0%, 35%)        |
| Blue    | hsl(217, 50%, 58%)    | hsl(201, 59%, 19%)    | hsl(214, 44%, 29%)     |
| Green   | hsl(146, 32%, 47%)    | hsl(149, 26%, 19%)    | hsl(147, 35%, 26%)     |
| Purple  | hsl(270, 55%, 62%)    | hsl(272, 24%, 23%)    | hsl(270, 36%, 29%)     |
| Red     | hsl(1, 69%, 60%)      | hsl(6, 32%, 24%)      | hsl(6, 39%, 31%)       |
| Orange  | hsl(25, 54%, 53%)     | hsl(25, 44%, 25%)     | hsl(34, 63%, 32%)      |
| Yellow  | hsl(38, 54%, 54%)     | hsl(38, 36%, 25%)     | hsl(37, 56%, 35%)      |
| Pink    | hsl(329, 57%, 58%)    | hsl(332, 28%, 24%)    | hsl(327, 36%, 30%)     |

### Settings Page Layout

- **5 main tabs**: General, Appearance, Extensions, Advanced, About
- **Sidebar navigation** with tab selection on left, detail panel on right
- **Controls used**: Toggle switches (binary), Dropdowns (theme/text size), Input fields
- **Keyboard shortcuts**: Configurable per-command in Extensions tab

---

## 6. Arc Browser

### Dark Theme Pattern

- Theme picker: stars (auto), sun (light), moon (dark)
- Sidebar-centric design -- all navigation lives in left sidebar
- Spaces with individual themes (color + gradient customization)
- Sidebar houses: pinned tabs, unpinned tabs, folders, library, notes
- Gradient backgrounds with 1-3 complementary colors + saturation control

### Settings Organization

- Settings accessed through sidebar right-click > space options
- Minimal settings surface -- most configuration is inline/contextual
- Each Space: own pinned section, unpinned section, theme, icon

---

## 7. Tailwind CSS 4 -- Complete Token Reference

### @theme Directive Syntax

```css
@import "tailwindcss";

@theme {
  /* Define design tokens that generate utility classes + CSS variables */
  --color-brand-500: oklch(0.72 0.11 178);
  --font-display: "Inter Display", sans-serif;
}

/* Usage: bg-brand-500, font-display */
/* Also available as: var(--color-brand-500) */
```

### Default Spacing Scale

Base unit: `--spacing: 0.25rem` (4px). All spacing is multiples of this.

| Class  | Value    | Pixels |
|--------|----------|--------|
| p-0    | 0        | 0px    |
| p-0.5  | 0.125rem | 2px    |
| p-1    | 0.25rem  | 4px    |
| p-1.5  | 0.375rem | 6px    |
| p-2    | 0.5rem   | 8px    |
| p-2.5  | 0.625rem | 10px   |
| p-3    | 0.75rem  | 12px   |
| p-3.5  | 0.875rem | 14px   |
| p-4    | 1rem     | 16px   |
| p-5    | 1.25rem  | 20px   |
| p-6    | 1.5rem   | 24px   |
| p-7    | 1.75rem  | 28px   |
| p-8    | 2rem     | 32px   |
| p-9    | 2.25rem  | 36px   |
| p-10   | 2.5rem   | 40px   |
| p-12   | 3rem     | 48px   |
| p-14   | 3.5rem   | 56px   |
| p-16   | 4rem     | 64px   |
| p-20   | 5rem     | 80px   |
| p-24   | 6rem     | 96px   |

### Font Size Scale

```css
@theme {
  --text-xs:   0.75rem;   /* 12px, line-height: calc(1/0.75) */
  --text-sm:   0.875rem;  /* 14px, line-height: calc(1.25/0.875) */
  --text-base: 1rem;      /* 16px, line-height: calc(1.5/1) */
  --text-lg:   1.125rem;  /* 18px, line-height: calc(1.75/1.125) */
  --text-xl:   1.25rem;   /* 20px, line-height: calc(1.75/1.25) */
  --text-2xl:  1.5rem;    /* 24px, line-height: calc(2/1.5) */
  --text-3xl:  1.875rem;  /* 30px, line-height: calc(2.25/1.875) */
  --text-4xl:  2.25rem;   /* 36px, line-height: calc(2.5/2.25) */
  --text-5xl:  3rem;      /* 48px, line-height: 1 */
  --text-6xl:  3.75rem;   /* 60px, line-height: 1 */
  --text-7xl:  4.5rem;    /* 72px, line-height: 1 */
  --text-8xl:  6rem;      /* 96px, line-height: 1 */
  --text-9xl:  8rem;      /* 128px, line-height: 1 */
}
```

### Font Weight Scale

```css
@theme {
  --font-weight-thin:       100;
  --font-weight-extralight: 200;
  --font-weight-light:      300;
  --font-weight-normal:     400;
  --font-weight-medium:     500;
  --font-weight-semibold:   600;
  --font-weight-bold:       700;
  --font-weight-extrabold:  800;
  --font-weight-black:      900;
}
```

### Letter Spacing

```css
@theme {
  --tracking-tighter: -0.05em;
  --tracking-tight:   -0.025em;
  --tracking-normal:  0em;
  --tracking-wide:    0.025em;
  --tracking-wider:   0.05em;
  --tracking-widest:  0.1em;
}
```

### Line Height

```css
@theme {
  --leading-tight:   1.25;
  --leading-snug:    1.375;
  --leading-normal:  1.5;
  --leading-relaxed: 1.625;
  --leading-loose:   2;
}
```

### Border Radius Scale

```css
@theme {
  --radius-xs:  0.125rem;  /* 2px */
  --radius-sm:  0.25rem;   /* 4px */
  --radius-md:  0.375rem;  /* 6px */
  --radius-lg:  0.5rem;    /* 8px */
  --radius-xl:  0.75rem;   /* 12px */
  --radius-2xl: 1rem;      /* 16px */
  --radius-3xl: 1.5rem;    /* 24px */
  --radius-4xl: 2rem;      /* 32px */
}
```

### Shadow Scale

```css
@theme {
  --shadow-2xs: 0 1px rgb(0 0 0 / 0.05);
  --shadow-xs:  0 1px 2px 0 rgb(0 0 0 / 0.05);
  --shadow-sm:  0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1);
  --shadow-md:  0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1);
  --shadow-lg:  0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1);
  --shadow-xl:  0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1);
  --shadow-2xl: 0 25px 50px -12px rgb(0 0 0 / 0.25);
}
```

### Easing Functions

```css
@theme {
  --ease-in:     cubic-bezier(0.4, 0, 1, 1);
  --ease-out:    cubic-bezier(0, 0, 0.2, 1);
  --ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
}
```

### Default Font Families

```css
@theme {
  --font-sans:  ui-sans-serif, system-ui, sans-serif, "Apple Color Emoji",
                "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji";
  --font-serif: ui-serif, Georgia, Cambria, "Times New Roman", Times, serif;
  --font-mono:  ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas,
                "Liberation Mono", "Courier New", monospace;
}
```

### Blur Scale

```css
@theme {
  --blur-xs:  4px;
  --blur-sm:  8px;
  --blur-md:  12px;
  --blur-lg:  16px;
  --blur-xl:  24px;
  --blur-2xl: 40px;
  --blur-3xl: 64px;
}
```

### Animations

```css
@theme {
  --animate-spin:   spin 1s linear infinite;
  --animate-ping:   ping 1s cubic-bezier(0, 0, 0.2, 1) infinite;
  --animate-pulse:  pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  --animate-bounce: bounce 1s infinite;
}
```

### Customization Patterns

```css
/* Extend defaults */
@theme {
  --font-display: "Inter Display", sans-serif;
}

/* Replace entire namespace */
@theme {
  --color-*: initial;
  --color-white: #fff;
  --color-brand: #3f3cbb;
  --color-surface: #121063;
}

/* Reference other variables (use inline keyword) */
@theme inline {
  --font-sans: var(--font-inter);
}

/* Static tokens (won't generate utilities) */
@theme static {
  --color-primary: var(--color-red-500);
}
```

---

## 8. Recommended Dark-Theme Token Architecture (Tailwind v4)

### Three-Tier System

```css
/* === TIER 1: Base Primitives === */
@theme {
  --color-gray-50:  oklch(0.985 0 0);
  --color-gray-100: oklch(0.965 0 0);
  --color-gray-200: oklch(0.922 0 0);
  --color-gray-300: oklch(0.869 0 0);
  --color-gray-400: oklch(0.708 0 0);
  --color-gray-500: oklch(0.556 0 0);
  --color-gray-600: oklch(0.439 0 0);
  --color-gray-700: oklch(0.371 0 0);
  --color-gray-800: oklch(0.269 0 0);
  --color-gray-900: oklch(0.205 0 0);
  --color-gray-950: oklch(0.145 0 0);
}

/* === TIER 2: Semantic Tokens (switch per theme) === */
:root {
  /* Light mode defaults */
  --color-background:     var(--color-white);
  --color-foreground:     var(--color-gray-900);
  --color-muted:          var(--color-gray-500);
  --color-border:         var(--color-gray-200);
  --color-surface:        var(--color-white);
  --color-surface-raised: var(--color-gray-50);
}

.dark {
  --color-background:     var(--color-gray-950);
  --color-foreground:     var(--color-gray-50);
  --color-muted:          var(--color-gray-400);
  --color-border:         var(--color-gray-800);
  --color-surface:        var(--color-gray-900);
  --color-surface-raised: var(--color-gray-800);
}

/* === TIER 3: Component Tokens === */
@theme {
  --card-radius:   var(--radius-lg);
  --card-padding:  var(--spacing-6);
  --input-radius:  var(--radius-md);
  --input-height:  2.25rem;
  --button-radius: var(--radius-md);
}
```

### Motion Tokens

```css
@theme {
  --duration-instant: 0ms;
  --duration-fast:    100ms;
  --duration-normal:  200ms;
  --duration-slow:    300ms;
  --duration-slower:  500ms;

  --ease-in:     cubic-bezier(0.4, 0, 1, 1);
  --ease-out:    cubic-bezier(0, 0, 0.2, 1);
  --ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);

  --stagger-base: 50ms;
  --stagger-fast: 30ms;
  --stagger-slow: 100ms;
}
```

### Dark Mode Configuration

```css
/* Tailwind v4: CSS-first, no JS config needed */
/* Default: uses prefers-color-scheme automatically */

/* For manual toggle via class: */
@custom-variant dark (&:where(.dark, .dark *));

/* For data attribute: */
@custom-variant dark (&:where([data-theme="dark"], [data-theme="dark"] *));
```

---

## 9. Cross-Cutting Best Practices

### Toggle/Switch vs. Checkbox

Use toggle switches when:
- The setting takes **immediate effect** (no save button needed)
- The choice is binary on/off
- The setting is a mode or preference

Use checkboxes when:
- The user must confirm/submit a batch of changes
- Multiple options can be selected
- The setting is opt-in to a list of features

Toggle implementation requirements:
- role="switch" + aria-checked for accessibility
- Keyboard: Space to toggle, Tab to navigate
- Minimum WCAG 4.5:1 contrast ratio
- Use shape/position indicators, not just color
- Label should be to the LEFT of the toggle (for LTR languages)

### Input Field Patterns (Focus Rings)

shadcn/ui v4 pattern (recommended):
```
focus-visible:border-ring
focus-visible:ring-[3px]
focus-visible:ring-ring/50
```

Key principles:
- 3px ring width at 50% opacity of ring color
- Border changes to ring color on focus
- Transition on box-shadow for smooth focus animation
- Dark mode: bg-input/30 for subtle fill
- Error state: aria-invalid:border-destructive with destructive ring

### Section Dividers and Grouping

Patterns from modern apps:
- **Separator component**: 1px height, bg-border color, full-width
- **Card-based sections**: Each settings group in its own card with padding
- **Header + description**: Section title (font-semibold) + muted description (text-sm text-muted-foreground)
- **Visual hierarchy**: gap-6 (24px) between sections, gap-4 (16px) within sections
- **Labels above controls**: text-label-14 (0.875rem) with consistent spacing

### Keyboard Navigation for Settings

- Tab order follows visual layout (top-to-bottom, left-to-right)
- tabindex="0" for custom interactive elements
- tabindex="-1" for programmatic-only focus
- Never use positive tabindex values
- Space bar toggles switches and checkboxes
- Enter activates buttons and links
- Arrow keys navigate within radio groups and select menus
- Escape closes modals and popovers
- Focus trap inside dialogs (focus cycles within modal)
- Focus returns to trigger element when dialog closes

### Settings Page Layout Recommendations

**Sidebar Navigation Pattern** (best for 5+ categories):
- Sidebar width: 240-300px expanded, 48-64px collapsed
- 5-7 primary navigation items maximum
- Primary nav text: 15-16px
- Sub-items: 13-14px, indented 16-24px
- Active state: highlight background + bold/colored text
- Current section indicator: left border accent or background fill

**Section Organization**:
- Group related settings in cards or bordered sections
- Section header: font-semibold text-lg (18px)
- Section description: text-sm text-muted-foreground (14px)
- Settings item: label left, control right, flex justify-between
- Spacing between items: gap-4 (16px)
- Spacing between sections: gap-8 (32px)

### Toast/Notification Patterns (Sonner)

- Default position: bottom-right
- Auto-dismiss: 4s (pauses on hover, pauses when document hidden)
- Animation: transform 400ms ease (translateY(100%) to translateY(0))
- Dark theme: automatically matches system preference
- Stacking: Multiple toasts stack vertically with gap
- Positions available: top-left, top-center, top-right, bottom-left, bottom-center, bottom-right

### Command Palette Patterns

- Trigger: Cmd/Ctrl+K (universal convention)
- Overlay: Semi-transparent backdrop (bg-black/50)
- Panel: Centered, max-width ~640px, rounded-xl
- Search input at top, results list below
- Keyboard nav: Arrow keys for list, Enter to select, Escape to close
- Show shortcut hints next to each action item
- Fuzzy matching for search

---

## 10. Consolidated Recommended Values for a Dark Desktop App

### Color Palette (Hex Approximations)

```
Background Layer 0 (deepest):  #0a0a0a  (near-black, page bg)
Background Layer 1:            #141414  (main content area)
Background Layer 2:            #1a1a1a  (cards, panels)
Background Layer 3:            #262626  (elevated elements, popovers)
Background Layer 4:            #333333  (hover states, active items)

Border Default:                rgba(255,255,255,0.08)  (very subtle)
Border Hover:                  rgba(255,255,255,0.12)
Border Focus:                  rgba(255,255,255,0.20)

Text Primary:                  #fafafa  (near-white)
Text Secondary:                #a0a0a0  (muted gray)
Text Tertiary:                 #666666  (disabled, hints)
Text Placeholder:              #525252  (input placeholders)

Accent Blue:                   #0070f3  (links, primary actions)
Accent Green:                  #50e3c2  (success states)
Accent Red:                    #ee0000  (error/destructive)
Accent Yellow:                 #f5a623  (warnings)
Accent Purple:                 #7928ca  (special states)
```

### Typography Recommendations

```
Font Family:      Inter (body), Inter Display or Geist Sans (headings)
Mono:             Geist Mono, JetBrains Mono, or system monospace

Settings Section Title:  16px / 1rem,  font-weight: 600, line-height: 1.2
Settings Item Label:     14px / 0.875rem, font-weight: 500, line-height: 1.5
Settings Description:    14px / 0.875rem, font-weight: 400, color: muted-foreground
Body Text:               14px / 0.875rem, font-weight: 400, line-height: 1.5
Small/Caption:           12px / 0.75rem, font-weight: 400
Keyboard Shortcut:       12px / 0.75rem, font-weight: 500, monospace

Letter Spacing:
  Headings: -0.025em (tracking-tight)
  Body: 0em (tracking-normal)
  Small/Labels: 0.025em (tracking-wide) -- optional
```

### Spacing for Settings Pages

```
Page padding:           24px (p-6)
Section gap:            32px (gap-8)
Item gap within section: 16px (gap-4)
Card padding:           24px (p-6)
Input height:           36px (h-9)
Small input height:     32px (h-8)
Button height:          36px (h-9)
Small button height:    32px (h-8)
Icon size:              16px (size-4)
Sidebar width:          240px expanded
```

### Border Radius

```
Buttons/Inputs:  6px  (rounded-md)
Cards:           12px (rounded-xl)
Badges/Pills:    9999px (rounded-full)
Popovers:        8px  (rounded-lg)
Thumbnails:      8px  (rounded-lg)
```

### Transitions

```
Color changes:     150ms ease-in-out
Transform/motion:  200ms ease-out
Opening popovers:  150ms (fade + scale 95% -> 100%)
Closing popovers:  100ms (fade + scale 100% -> 95%)
Toggle slide:      transform 200ms ease
Toast enter:       transform 400ms ease
Toast exit:        opacity 150ms ease
Hover states:      150ms ease
Focus ring:        box-shadow 150ms ease
```

---

## Sources

- https://ui.shadcn.com/docs/theming
- https://ui.shadcn.com/docs/tailwind-v4
- https://vercel.com/geist/colors
- https://vercel.com/geist/typography
- https://theme-ui-preset-geist.vercel.app/
- https://linear.app/now/how-we-redesigned-the-linear-ui
- https://linear.style/
- https://manual.raycast.com/custom-themes
- https://manual.raycast.com/preferences
- https://developers.raycast.com/api-reference/user-interface/colors
- https://tailwindcss.com/docs/theme
- https://tailwindcss.com/docs/dark-mode
- https://www.maviklabs.com/blog/design-tokens-tailwind-v4-2026
- https://www.setproduct.com/blog/settings-ui-design
- https://github.com/shadcn-ui/ui (component source files)
- https://sonner.emilkowal.ski/styling
- https://manupa.dev/blog/anatomy-of-shadcn-ui
