# YOLO Voice — Competitive UI/UX Analysis & Design Recommendations

**Date**: 2026-03-25
**Scope**: Wispr Flow, Aqua Voice, Willow Voice Assistant
**Purpose**: Inform YOLO Voice's next UI/UX iteration

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Comparative Analysis](#2-comparative-analysis)
3. [Design Recommendation Report](#3-design-recommendation-report)
4. [Implementation Plan](#4-implementation-plan)

---

## 1. Executive Summary

Three voice assistant products were analyzed for UI/UX patterns relevant to YOLO Voice:

- **Wispr Flow** (wisprflow.ai) — YC-backed, cloud-based dictation for Mac/Windows/iOS/Android. The market leader with polished UX, per-app style adaptation, and an "invisible by default" philosophy. ~$12-15/mo.
- **Aqua Voice** (aquavoice.com) — Minimalist desktop dictation with a signature floating recording pill, context-aware formatting, and sub-500ms latency. Strongest design execution.
- **Willow Voice** (heywillow.io) — Open-source, self-hosted voice assistant on ESP32 hardware. Web-based configuration UI (Next.js + MUI). Different category but valuable for settings UX patterns.

**Key insight**: The top-performing apps in this space share a common philosophy — **the UI should disappear**. The best voice dictation UX is one where the user barely notices the app exists. YOLO Voice's current settings-heavy, full-window approach is functional but doesn't follow this trend.

---

## 2. Comparative Analysis

### 2.1 Visual Design

| Dimension | Wispr Flow | Aqua Voice | Willow | YOLO Voice (Current) |
|-----------|-----------|------------|--------|---------------------|
| **Primary Color** | Purple/indigo | Blue/aqua | Deep eggplant purple (#583759) | Blue-600 (#2563EB) |
| **Background** | Light/neutral (Hub) | Off-white, minimal | White (web UI) | Dark (gray-950) |
| **Theme** | Light mode, OS-native | Light mode, ultra-clean | Light mode (MUI) | Dark mode only |
| **Typography** | System fonts (SF Pro/Segoe UI) | Sans-serif mix | Raleway + Roboto Mono | Tailwind defaults |
| **Iconography** | Feather-style | Minimal, geometric | MUI icons | Almost no icons |
| **Polish Level** | High | Very high | Medium (developer-focused) | Functional but basic |

### 2.2 Recording Indicator / Floating UI

| Feature | Wispr Flow | Aqua Voice | Willow | YOLO Voice |
|---------|-----------|------------|--------|------------|
| **Floating element** | Flow Bar (bottom of screen) + optional bubble | Dark pill with blue dot + waveform (~120x40px) | Hardware LCD states | Pill component (inline in app) |
| **Position** | Bottom of screen, persistent | Near active text field | N/A (hardware) | Inside app window only |
| **States** | Ready / Listening / Processing | Ready / Recording / Transcribing / Done | 6 states (idle through error) | Recording / Transcribing / Done |
| **Dismiss** | Right-click menu, settings toggle | Minimal, auto-appears | N/A | N/A |

**Takeaway**: Both market leaders use a floating, always-accessible indicator that lives *outside* the main app window. YOLO Voice's Pill component is well-animated but confined to the app window.

### 2.3 Settings Organization

| Approach | Wispr Flow | Aqua Voice | Willow | YOLO Voice |
|----------|-----------|------------|--------|------------|
| **Pattern** | Left sidebar with sections (Hub) | Category tabs (8+ sections) | Accordion panels (3 tiers) | Linear scroll (10+ sections) |
| **Grouping** | Home, Styles, Dictionary, Snippets, Notes, Settings | Keybindings, Dictionary, Custom Instructions, Replacements, Languages, History, System | Connectivity > General > Advanced | Flat list: Mic, Hotkey, Command, Engine, Language, Sounds, Vocab, Profiles, Startup |
| **Progressive disclosure** | Partial (onboarding badges) | Role-based presets on first launch | Yes (sections unlock progressively) | None — all visible at once |
| **Navigation** | Sidebar click + keyboard shortcuts | Tab/category switching | Accordion expand/collapse | Scroll only |

**Takeaway**: YOLO Voice's flat, scroll-based settings page becomes unwieldy as features grow. All competitors use some form of sectioned/tabbed navigation.

### 2.4 Interaction Patterns

| Pattern | Wispr Flow | Aqua Voice | YOLO Voice |
|---------|-----------|------------|------------|
| **Activation** | Push-to-talk (hold) + hands-free (double-tap) | Instant (press-release) + streaming (real-time) + hands-free (double-tap) | Hold-to-record + double-tap toggle |
| **Hotkey config** | Press actual keys, up to 4 shortcuts per action, mouse button support | Configurable activation key | Press actual keys (KeybindingInput) |
| **Per-app adaptation** | Auto-detects active app, adjusts tone | Auto-detects active app context | Manual profile selection |
| **Voice commands** | Command Mode (highlight text + speak edit) | Natural language commands | Command mode (separate hotkey) |
| **Dictionary** | Auto-learning with sparkle indicators | 5 free / 800 pro entries with manual add | Manual vocabulary editor |
| **Text expansion** | Snippets (voice-triggered macros) | Replacements | Replacement rules |

### 2.5 Unique/Innovative Features

**Wispr Flow:**
- Per-app writing style adaptation (email = formal, Slack = casual)
- Auto-dictionary learning from active window context
- Course correction ("actually", "wait" mid-sentence)
- Flow Bar as persistent system-level element
- 10+ step progressive onboarding

**Aqua Voice:**
- Floating recording pill near active text field
- In-browser sandbox for trying before install
- Role-based onboarding presets (Developer, Sales, Healthcare, etc.)
- File tagging for code editors (@mentions from spoken filenames)
- Deep Context screen reading for technical accuracy
- Casual Messaging toggle for chat apps
- Sub-500ms transcription latency

**Willow:**
- Fleet-wide configuration (save + apply to all devices)
- Progressive settings disclosure (unlock tiers as you configure)
- Real-time device monitoring dashboard
- WebSerial firmware flashing from browser
- Hardware-level tuning (mic gain, VAD timeout, record buffer)

---

## 3. Design Recommendation Report

### 3.1 Best Design Practices Observed

1. **Invisible-by-default philosophy**: The app's primary interface is a hotkey + minimal floating indicator, not a window. The settings window is secondary.
2. **Floating recording indicator**: A small, non-intrusive pill/bar that appears near the user's focus point, not inside the app window.
3. **Sectioned settings with progressive disclosure**: Settings grouped into logical categories with tabs or sidebar navigation, not a single scrolling page.
4. **Context-aware behavior**: Automatically adapting output style based on the active application reduces manual profile switching.
5. **Role-based onboarding**: First-run experience that asks "What do you do?" and pre-configures vocabulary, style, and shortcuts.
6. **Single accent color system**: One primary accent color (blue, purple, or aqua) with semantic status colors. Keeps the UI cohesive.
7. **Auto-learning dictionary**: Vocabulary that grows from context rather than only manual entry.

### 3.2 UI Components & Layouts to Adopt

#### A. Redesign the Settings Page — Sidebar Navigation

Replace the flat scroll with a left sidebar + content area:

```
+------------------+----------------------------------------+
| [Logo]           |                                        |
|                  |  Section Title                         |
| > General        |  --------------------------------      |
|   Hotkeys        |  Setting 1          [toggle]           |
|   Audio          |  Setting 2          [dropdown]         |
|   Transcription  |  Setting 3          [input]            |
|   Vocabulary     |                                        |
|   Profiles       |  Setting 4          [slider]           |
|   Command Mode   |                                        |
| > Advanced       |                                        |
|                  |                                        |
| [About]          |  [Save]                                |
+------------------+----------------------------------------+
```

**Sections:**
1. **General** — Microphone, language, startup behavior, sounds
2. **Hotkeys** — Dictation key, command mode key (with conflict detection)
3. **Transcription** — Engine selection (local/cloud), model config, diagnostics
4. **Vocabulary** — Terms, rules, industry packs (current VocabularyEditor)
5. **Profiles** — Dictation styles, LLM settings (current ProfileEditor + LLMSettings)
6. **Command Mode** — Hotkey, API key, vision toggle, system prompt
7. **Advanced** — Debug options, log management, experimental features

#### B. Floating Recording Pill (System-Level)

Adapt the current Pill component to render as a system-level floating window:
- Small pill (~120x40px) with waveform animation
- Positioned near system tray or bottom-right
- Shows mode (green=dictation, purple=command), waveform during recording, spinner during transcription
- Tauri supports creating secondary borderless windows — use this for the pill

#### C. Improved Onboarding Flow

Add a first-run wizard:
1. **Welcome** — Brief product intro
2. **Microphone** — Select + test with visual waveform
3. **Hotkey** — Configure with key capture
4. **Engine** — Choose local vs cloud
5. **Profile** — Optional: select a role preset (Developer, Writer, Medical, etc.)
6. **Done** — Ready to use

#### D. Status Indicators & Feedback

- Add toast notifications for save confirmations (top-right, auto-dismiss 2s)
- Add success/error badges on settings sections with issues
- Use subtle animations for state transitions (already strong in Pill component)

### 3.3 Color Palette Recommendation

Retain the dark theme (it's a strong differentiator for a power-user tool) but refine it:

| Role | Current | Recommended | Rationale |
|------|---------|-------------|-----------|
| **Background** | gray-950 | gray-950 | Keep — works well |
| **Surface** | gray-800 | gray-900 | Slightly darker for better contrast layers |
| **Border** | gray-700 | gray-800 (rest), gray-600 (hover) | More subtle borders |
| **Primary accent** | blue-600 | blue-500 | Slightly brighter for better visibility |
| **Dictation mode** | green-400/500 | green-400 | Keep |
| **Command mode** | purple-500 | purple-400 | Slightly brighter |
| **Warning** | amber-700 | amber-500 | More visible |
| **Error** | red-400/700 | red-500 | Standardize |
| **Text primary** | gray-100 | gray-100 | Keep |
| **Text secondary** | gray-400/500 | gray-400 | Standardize to one |

### 3.4 Typography Recommendation

| Element | Current | Recommended |
|---------|---------|-------------|
| **Font family** | Tailwind default (system) | Inter (install) or keep system |
| **Page title** | text-xl | text-xl font-semibold |
| **Section headers** | text-lg inconsistent weight | text-base font-semibold uppercase tracking-wide text-gray-400 |
| **Labels** | text-sm | text-sm font-medium text-gray-300 |
| **Body** | text-sm | text-sm text-gray-300 |
| **Badges/tags** | text-xs | text-xs font-medium uppercase tracking-wider |

The ALL-CAPS + tracking-wide pattern for section labels (seen in Aqua Voice) provides clear visual hierarchy without large font sizes.

### 3.5 Prioritized Feature List

| Priority | Feature | Rationale | Effort |
|----------|---------|-----------|--------|
| **P0** | Sidebar settings navigation | Current scroll page doesn't scale; every competitor uses sections | Medium |
| **P0** | Floating recording pill (system window) | Core UX gap — users need to see recording state without switching windows | Medium-High |
| **P1** | Toast notification system | Save confirmations, error feedback currently lacks polish | Low |
| **P1** | Typography & spacing refinement | Consistent hierarchy, section labels, better density | Low |
| **P1** | Color palette standardization | Too many gray/color variants; consolidate tokens | Low |
| **P2** | First-run onboarding wizard | Reduces setup friction, matches competitor baseline | Medium |
| **P2** | Context-aware app detection | Auto-switch profiles based on active window (like Wispr/Aqua) | High |
| **P3** | Auto-learning dictionary | Detect vocabulary from active window context | High |
| **P3** | Role-based presets | Pre-configured vocabulary + style for common roles | Low-Medium |
| **P3** | In-app keyboard shortcuts | Navigate settings, quick actions without mouse | Low |

---

## 4. Implementation Plan

### 4.1 Phase 1 — Settings Redesign & Polish (Week 1-2)

**Goal**: Transform the flat settings page into a professional, navigable interface.

**Tasks:**
- [ ] Create `SettingsSidebar` component with section navigation
- [ ] Group existing settings into 7 sections (General, Hotkeys, Transcription, Vocabulary, Profiles, Command Mode, Advanced)
- [ ] Add section routing with active state highlighting
- [ ] Standardize typography tokens (section headers, labels, body text)
- [ ] Consolidate color usage (create a shared color constants file or Tailwind theme extension)
- [ ] Add toast notification component (lightweight, no library needed)
- [ ] Polish spacing and alignment across all settings sections

**Resources needed:**
- Wireframe: Sidebar + content layout (can sketch in code, no Figma needed)
- No new dependencies required

**Risks:**
- Large refactor of Settings.tsx — currently a monolithic file
- Mitigation: Extract each section into its own component first, then add sidebar

### 4.2 Phase 2 — Floating Recording Pill (Week 2-3)

**Goal**: Move the recording indicator to a system-level floating window.

**Tasks:**
- [ ] Create a Tauri secondary window (borderless, always-on-top, small size)
- [ ] Port Pill component to render in the secondary window
- [ ] Add window positioning logic (near system tray / bottom-right)
- [ ] Handle window show/hide based on recording state
- [ ] Ensure main window and pill window communicate via Tauri events
- [ ] Add drag-to-reposition support
- [ ] Add right-click context menu (mic select, mode toggle, settings)

**Resources needed:**
- Tauri multi-window documentation
- Test across different screen sizes and DPI settings

**Risks:**
- Tauri multi-window has platform-specific quirks (transparency, click-through)
- Mitigation: Start with a simple opaque pill, iterate on transparency/click-through later

### 4.3 Phase 3 — Onboarding Wizard (Week 3-4)

**Goal**: Guide new users through setup on first launch.

**Tasks:**
- [ ] Create `OnboardingWizard` component with step navigation
- [ ] Step 1: Welcome screen with product overview
- [ ] Step 2: Microphone selection + test (reuse existing component)
- [ ] Step 3: Hotkey configuration (reuse KeybindingInput)
- [ ] Step 4: Engine selection (local vs cloud, reuse LLMSettings if cloud)
- [ ] Step 5: Optional role preset selection
- [ ] Step 6: Complete — show "ready to use" with hotkey reminder
- [ ] Persist onboarding completion flag
- [ ] Add "Re-run setup" option in settings

**Resources needed:**
- Step illustration assets (optional — can use emoji/text initially)
- No new dependencies

**Risks:**
- Low risk — mostly composition of existing components
- Mitigation: Keep it simple, iterate based on feedback

### 4.4 Phase 4 — Context-Aware Features (Week 4-6)

**Goal**: Auto-detect active application and adapt behavior.

**Tasks:**
- [ ] Add active window detection in Rust backend (Windows API: `GetForegroundWindow` + process name)
- [ ] Create app-to-profile mapping configuration
- [ ] Auto-switch dictation profile based on active app
- [ ] Add "Casual Messaging" toggle for chat apps (auto-lowercase, informal)
- [ ] Add active-app indicator in floating pill
- [ ] Consider auto-dictionary learning from clipboard/window context (stretch goal)

**Resources needed:**
- Windows API bindings for active window detection
- Profile mapping UI in settings

**Risks:**
- Windows API for window detection can be unreliable with UWP/Electron apps
- Mitigation: Fall back to executable name matching, allow manual overrides

### 4.5 Milestone Timeline

```
Week 1   |████████████████████| Phase 1a: Settings component extraction
Week 2   |████████████████████| Phase 1b: Sidebar nav + typography polish
Week 3   |████████████████████| Phase 2a: Floating pill window (Tauri)
Week 4   |████████████████████| Phase 2b: Pill polish + Phase 3 onboarding
Week 5   |████████████████████| Phase 3 complete + Phase 4a: Window detection
Week 6   |████████████████████| Phase 4b: Context-aware switching + testing
```

### 4.6 Potential Challenges & Mitigations

| Challenge | Impact | Mitigation |
|-----------|--------|------------|
| Settings.tsx monolith refactor | High — breakage risk | Extract sections incrementally, test each extraction |
| Tauri secondary window transparency | Medium — platform-specific | Start opaque, add transparency as enhancement |
| Multi-window event synchronization | Medium — state sync bugs | Use Tauri's built-in event system, single source of truth in backend |
| Active window detection reliability | Medium — false positives | Use executable name + window title matching, manual fallback |
| Dark theme onboarding readability | Low — visual concern | Use sufficient contrast, test with screenshots |

---

## Appendix A: Source Links

### Wispr Flow
- Official: wisprflow.ai
- Docs: docs.wisprflow.ai
- Roadmap/Changelog: roadmap.wisprflow.ai/changelog

### Aqua Voice
- Official: aquavoice.com
- User Guide: aquavoice.com/guide
- Changelog: aquavoice.com/changelog
- Sandbox Demo: app.aquavoice.com/sandbox

### Willow
- Official: heywillow.io
- GitHub: github.com/HeyWillow
- WAS UI Source: github.com/HeyWillow/willow-application-server-ui

---

## Appendix B: YOLO Voice Current State Summary

**Stack**: Tauri (Rust backend) + React 18 + TypeScript + Tailwind CSS
**Theme**: Dark mode only (gray-950 base)
**Settings**: 10+ sections in a single scrolling page
**Recording UI**: Pill component (inline in app window)
**Hotkey**: KeybindingInput with key capture and conflict detection
**Profiles**: ProfileEditor + LLMSettings for dictation refinement
**Vocabulary**: VocabularyEditor with terms, rules, industry packs
**No external UI library** — all components hand-built with Tailwind
