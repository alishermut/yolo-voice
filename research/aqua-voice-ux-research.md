# Aqua Voice UX Research Findings

**Research Date**: 2026-03-25
**Research Method**: Secondary research (web analysis, visual inspection, review aggregation)
**Sources**: Official website, Product Hunt, 9to5Mac, Hacker News, review sites, web sandbox demo

---

## 1. PRODUCT OVERVIEW

Aqua Voice is an AI-native voice dictation application for Mac and Windows, built by a YC W24 company. It uses a proprietary transcription model called "Avalon" and positions itself as "5x faster than typing and twice as accurate." The product is cloud-based (not offline), requires an account, and operates as a system-wide overlay that works inside any text field.

**Tagline**: "We've typed for 150 years. It's time to speak."

---

## 2. UI/UX DESIGN: VISUAL IDENTITY

### 2.1 Color Scheme

| Role                | Color                          | Usage                                |
|---------------------|--------------------------------|--------------------------------------|
| Primary Brand       | Bright blue / aqua gradient    | CTA buttons, recording indicator, accent |
| Background (Light)  | Off-white (#f8f8f8-ish)        | Main website, light sections         |
| Background (Dark)   | Near-black (#1a1a1a-ish)       | "Coding & Prompting" section         |
| Text Primary        | Dark charcoal / slate gray     | Headlines, body text                 |
| Text Secondary      | Medium gray                    | Subtitles, descriptions              |
| Success/Check       | Blue circle checkmarks         | Pricing feature lists                |
| Recording Active    | Bright blue circle + glow      | Floating bar recording state         |
| Card Backgrounds    | White with subtle gray borders | Pricing cards, testimonial cards     |

The brand identity centers on a **blue-to-cyan aqua gradient** that appears on:
- The "Go Pro" CTA button (strongest gradient, blue-to-light-blue)
- The "Download for Windows" footer CTA button
- The recording indicator orb (bright blue circle)
- The floating bar waveform visualization
- The accuracy comparison bar chart (blue bar for Aqua)
- The hero orb/sphere graphic in the footer (a 3D blue sphere with water reflection)

The overall palette is extremely restrained: white/off-white backgrounds, dark gray text, and blue accents. No other colors are used prominently. The design language communicates "clean, technical, trustworthy."

### 2.2 Typography

- **Headlines**: Large serif or modern sans-serif font, thin/light weight (the hero text "We've typed for 150 years. It's time to speak." is set in a large, elegant thin-weight font, approximately 48-64px)
- **Subheadings**: Medium-weight sans-serif, smaller (e.g., "Coding & Prompting" label above section headlines)
- **Body text**: Regular-weight sans-serif, gray color, approximately 14-16px
- **Feature labels**: ALL-CAPS monospace tracking for section labels like "PRIVATE HISTORY", "BETTER ACCURACY", "GET STARTED WITH", "EVERYTHING IN STARTER"
- **Code/Editor text**: Monospace font used in the standalone Aqua editor, transcript history, and code demos
- **Metrics/Stats**: Very large, bold display font for numbers like "6h 23m", "230wpm", "4h 56m", "47s"
- **Navigation**: Regular sans-serif, standard weight

### 2.3 Iconography

- **Logo**: The word "AQUA" in uppercase, bold sans-serif. The full brand mark includes a blue orb/sphere icon to the left.
- **Role icons**: Small monoline icons next to role selector pills (code brackets for Developer, building for Consultant, chart for Sales, heart for Healthcare, graduation cap for Student, megaphone for Marketer)
- **Value proposition icons**: Three hand-drawn/outline style icons in the features section (a settings/toggle icon for "Works with all your apps", wavy lines for "Your thoughts set the pace", an eye for "Your screen is its dictionary")
- **Pricing checkmarks**: Blue filled circle checkmarks for feature lists
- **App integration icons**: Small app logos (Cursor, VS Code, Windsurf, ChatGPT, Claude, Bear, Google Docs, Notion, Slack, Gmail, Outlook, WhatsApp, Figma, etc.)
- **Navigation**: Minimal - no icons in the nav bar, just text links
- **Share icon**: A standard share/upload icon in the standalone editor toolbar

### 2.4 Brand Orb Graphic

The signature visual is a **luminous blue sphere/orb** that appears:
- As the app icon/logo (128x128 pixel Orb image)
- In the footer CTA section as a large 3D rendered blue sphere with water-like reflections beneath it
- As the blue circle in the floating recording bar

This orb is the single most distinctive visual element of the brand and reinforces the "Aqua" water theme.

---

## 3. LAYOUT AND WINDOW DESIGN

### 3.1 Desktop App Architecture

Aqua Voice operates as a **system-level overlay** rather than a traditional windowed application. The core UI elements are:

**A. The Floating Bar (Primary Interaction Element)**
- A small, dark, pill-shaped capsule that appears at the bottom of the active window or screen
- Approximately 120-150px wide, 36-40px tall
- Dark background (near-black, semi-transparent)
- Contains two elements:
  1. A **bright blue circle** on the left side (approximately 16-20px diameter) - the recording state indicator
  2. **Waveform bars** to the right of the circle - animated vertical bars of varying height that visualize audio input in real-time
  3. Small dots trailing after the waveform
- When recording is active, the waveform animates and the blue circle glows
- Can be hidden via Settings > System > "Show Floating Bar" toggle
- Positioning fixed in v0.11.8 to not jump around in fullscreen mode

**B. The Standalone Editor ("New Note" Window)**
- A clean, minimal text editor window
- macOS-style traffic light buttons (red, yellow, green) in the top-left corner
- "New Note" centered title in the title bar
- **Top toolbar**: Hamburger menu (three horizontal lines) on the left, character/word count badge centered ("8,032 characters  1,401 words" in a rounded pill), share/export icon on the right
- **Content area**: White background, monospace font for document text
- Supports chapters, headings, and long-form writing
- Glass-morphism effect (translucent/frosted background with slight blur) visible in the sandbox demo

**C. System Integration**
- Menu bar / system tray presence (inferred from background operation model)
- No dock icon option (can be hidden via settings on macOS)
- Activates via global hotkey without bringing a visible window to focus
- Text is typed directly into the currently focused text field of any application

### 3.2 Web Sandbox Interface

The browser-based demo simulates the desktop experience:
- Full-viewport macOS desktop wallpaper background (vivid orange/magenta Ventura-style gradient)
- Floating "New Note" editor centered on screen with glass-morphism
- "AQUA Voice" logo with blue orb in top-left corner
- "Step 1 of 2" progress indicator with "Avalon" badge
- Role selector pills at bottom center: Developer, Consultant, Sales, Healthcare, Student, Marketer
- Language selector dropdown bottom-left (defaults to "English" with globe icon)
- "Streaming" toggle switch bottom-right
- "Continue" button bottom-right of the editor panel

### 3.3 Website Layout Structure

The website follows a **vertical, single-column, section-based layout**:

1. **Sticky header nav**: "AQUA" logo left, nav links center-right (Pricing, User Guide, Blog, Changelog, API), "Download" blue pill button far right
2. **Hero section**: Large centered tagline, faded background orb shapes, animated typing demo
3. **Speed comparison section**: Two side-by-side panels (Aqua 230 WPM vs Keyboard 40 WPM) with live-typing animation, technical code content with syntax highlighting
4. **Three value proposition cards**: Inline, icon + title + description format
5. **Dark section** ("Coding & Prompting"): Dark background with code editor mockups showing integration with VS Code, ChatGPT, Claude Code
6. **Horizontally scrollable feature carousel**: Multiple panels (Prompting with technical accuracy, Syntax Highlighting, Prompt at the speed of thought) with app integration logos
7. **Metrics section**: Large stat numbers on dark background (6h 23m saved, 230wpm)
8. **Productivity section**: Slack, email, and messaging mockups in light cards
9. **Streaming mode section**: Document editing mockups (Google Docs-style, standalone editor)
10. **Feature grid** ("Built for the way you work"): Cards for Writing style, Language support, Private history, Accuracy comparison
11. **Social proof**: Logo bar (Amazon, Notion, Perplexity) + testimonial card grid (2 rows of ~5 cards each)
12. **Pricing**: Three-tier card layout (Starter, Pro, Team)
13. **Footer CTA**: Large blue orb graphic, "Let your voice do the writing" tagline, download button
14. **Footer**: Sitemap links in columns, social media links (X, Discord), system status indicator

---

## 4. SETTINGS ORGANIZATION

Settings are accessed through the app's main interface and are organized into the following categories/tabs:

### 4.1 Settings Hierarchy

```
Settings
  |
  +-- Keybindings
  |     |-- Activation key (default: Fn on Mac, Alt on Windows)
  |     |-- Paste transcript shortcut (customizable)
  |     |-- Hands-free activation key (reassignable)
  |     |-- Pencil icon to edit each keybinding
  |     |-- "Show Recommended" to restore defaults
  |
  +-- Dictionary
  |     |-- Custom vocabulary entries (5 free, 800 pro)
  |     |-- Each entry preserves casing
  |     |-- Supports names, brands, technical terms, packages
  |     |-- Syncs across devices (toggleable)
  |
  +-- Custom Instructions
  |     |-- Natural language formatting rules
  |     |-- Per-app customization (e.g., "Use lowercase in iMessage")
  |     |-- Punctuation, spacing, capitalization rules
  |     |-- Syncs across devices (toggleable)
  |
  +-- Replacements (v0.10.8+)
  |     |-- Shortcut phrase -> expansion text mappings
  |     |-- For repeated prompts, emails, links
  |
  +-- File Tagging (v0.10+)
  |     |-- Toggle for code editor file referencing
  |     |-- Works with Cursor, Windsurf, Cline, Antigravity
  |     |-- Converts spoken filenames to @filename.ext tags
  |
  +-- Languages
  |     |-- Language picker / selector
  |     |-- Auto-Detect mode
  |     |-- 49 languages supported
  |     |-- Also accessible from context menu
  |     |-- Syncs across devices (toggleable)
  |
  +-- History
  |     |-- Stored locally (transcripts + audio)
  |     |-- Timestamped entries with duration
  |     |-- Thumbs up / thumbs down feedback
  |     |-- Notes for bad responses
  |     |-- Activity graph visualization (v0.9.10+)
  |
  +-- System
  |     |-- Show Floating Bar (toggle)
  |     |-- Avoid Clipboard History (toggle)
  |     |-- Casual Messaging mode (toggle, v0.10.4+)
  |     |-- Hide dock icon (macOS)
  |     |-- Cross-device sync enable/disable
  |
  +-- Aqua Voice (Advanced)
  |     |-- Transcription model selection (Avalon on Pro)
  |     |-- Deep Context toggle (off by default)
  |     |-- Mode: Instant vs Streaming
  |
  +-- Account / Billing
        |-- Plan management
        |-- Team creation/transfer
        |-- Monthly vs annual billing toggle
```

### 4.2 Settings Design Patterns

- Settings appear to use a **tabbed or sidebar navigation** within a settings panel
- **Pencil icon** for inline editing of keybindings
- **Toggle switches** for on/off features (Show Floating Bar, Deep Context, Streaming, Casual Messaging, Avoid Clipboard History)
- **Text input fields** for dictionary entries and custom instructions
- **Language picker** dropdown with search
- **Stats card** on the home view showing activity graph and productivity metrics
- Cross-device sync status is visible and toggleable per setting category

---

## 5. INTERACTION PATTERNS

### 5.1 Core Dictation Flow

**Instant Mode (default for short inputs)**:
1. User presses activation hotkey (Fn on Mac, Alt on Windows)
2. Floating bar appears with blue circle indicator
3. Waveform animates as user speaks
4. User releases hotkey
5. Text processes in ~450ms
6. Text is pasted into the active text field

**Streaming Mode (for long content)**:
1. User activates streaming mode via settings or voice command
2. Text appears in real-time as user speaks (~850ms latency)
3. Text is streamed continuously into the active field or editor
4. Suitable for long documents, emails, prose

**Hands-Free Mode**:
1. User double-taps the activation key
2. Dictation stays active without holding a key
3. Continues until manually stopped

### 5.2 Hotkey System

| Platform | Default Activation | Alternative    |
|----------|-------------------|----------------|
| Mac      | Fn key            | Customizable   |
| Windows  | Alt key           | Customizable (e.g., Ctrl+Space, Alt+V) |

- Multiple hotkeys can be bound simultaneously
- Paste last transcript has a dedicated customizable shortcut
- Keybindings are edited via a pencil icon interaction
- "Show Recommended" button restores defaults

### 5.3 Voice Commands

Users can issue natural language commands without memorizing syntax:
- "Put that into bullet points"
- "Switch to Streaming Mode"
- "Make that lowercase"
- Formatting commands are contextually interpreted
- No rigid command grammar required

### 5.4 Context-Aware Adaptation

The app detects which application has focus and adapts:
- **Slack/iMessage**: More casual formatting, lowercase option
- **Gmail/Outlook**: Professional email formatting
- **VS Code/Cursor**: Code syntax awareness, variable naming
- **Google Docs/Notion**: Document formatting
- Automatic punctuation and capitalization adjustment per context

### 5.5 Deep Context (Screen Reading)

- Opt-in feature (disabled by default for privacy)
- Reads on-screen content to improve accuracy
- Converts spoken code references to formatted syntax (e.g., "canonical title" becomes `canonical_title`)
- Adds code syntax highlighting to transcripts
- Processes locally, nothing stored on servers

### 5.6 File Tagging Interaction

In supported code editors (Cursor, Windsurf):
- Say "at main" or "tag user profile" while dictating
- Aqua converts to `@main.ts` or `@userProfile.tsx`
- Triggers editor's autocomplete for file references
- Recognizes "(filename) (extension)" and "(filename) dot (extension)" patterns

### 5.7 Replacements System

- User defines shortcut phrase and expansion text
- When shortcut is spoken, it is automatically expanded
- Useful for repeated email signatures, URLs, standard phrases

---

## 6. COMPONENT PATTERNS

### 6.1 Buttons

| Type        | Style                                          | Usage              |
|-------------|------------------------------------------------|---------------------|
| Primary CTA | Blue gradient fill, white text, rounded-full    | "Go Pro", "Download for Windows" |
| Secondary   | White/light fill, dark text, rounded-full, border | "Write faster with Aqua", "Start transcribing", "Get Started" |
| Nav CTA     | Blue pill, white text                           | "Download" in header |
| Ghost       | Transparent, dark text                          | Nav links           |
| Icon Button | Square/circle, icon only                        | Share, hamburger menu |

- Buttons use full rounded corners (pill shape)
- Primary buttons feature the blue gradient
- Secondary buttons have subtle borders and white/light fill
- The "Download for Windows" button includes a Windows icon on the left

### 6.2 Cards

**Pricing Cards**:
- White background, subtle rounded corners, light border
- Header: Plan name + price (large font) + billing period
- Small decorative orb illustration in top-right corner (different for each tier)
- Feature list with blue checkmarks
- CTA button at bottom (gradient for Pro, outline for Starter/Team)

**Testimonial Cards**:
- White background, rounded corners
- Avatar + name + handle + X/Twitter logo
- Quote text in italic/regular style
- Grid layout (2 rows, ~5 columns, horizontally scrollable)

**Feature Cards**:
- Two-column grid within dark or light sections
- Title + description + "Designed for" app icon row
- Some cards include inline mockups/demos

### 6.3 Toggle Switches

- Used for: Show Floating Bar, Streaming mode, Deep Context, Casual Messaging, Avoid Clipboard History, Cross-device sync
- Standard toggle switch design (pill shape, circular knob)
- The Streaming toggle is visible in the web sandbox bottom-right

### 6.4 The Floating Recording Bar

This is the most distinctive UI component:
- **Shape**: Rounded pill / capsule (dark background, ~120-150px wide, ~36-40px tall)
- **Left element**: Solid bright blue circle (~16-20px), indicating active recording state
- **Right element**: Waveform visualization - approximately 8-12 vertical bars of varying height that animate with audio input, followed by trailing dots
- **Position**: Anchored to the bottom-center of the active text input or the screen
- **Behavior**: Appears when recording starts, disappears when recording stops
- **Variants**: Appears identically whether in Slack, code editors, Claude Code, or the standalone editor

### 6.5 Progress/Step Indicators

- "Step 1 of 2" text with progress bar segments (filled dark bar for current, gray for remaining)
- Used in the onboarding/sandbox flow

### 6.6 Role Selector Pills

- Horizontal row of pill-shaped buttons with icons
- Semi-transparent/frosted background
- Categories: Developer, Consultant, Sales, Healthcare, Student, Marketer
- Each has a small monoline icon prefix

### 6.7 Stats/Metrics Display

- Large display-weight numbers (e.g., "6h 23m", "230wpm", "4h 56m", "47s")
- Smaller label text beneath or beside the number
- Used on dark backgrounds for maximum contrast
- Activity graph visualization in the stats card (line chart)

---

## 7. UNIQUE AND INNOVATIVE UX FEATURES

### 7.1 Context-Aware Formatting Without Setup
The app automatically detects which application is in focus and adjusts formatting, punctuation, and style without any user configuration. This is described as "like MCP for every app without requiring setup." No competitor offers this level of automatic adaptation.

### 7.2 Natural Language Voice Commands
Users can say things like "put that into bullet points" without learning specific command syntax. The system interprets intent rather than requiring rigid commands.

### 7.3 File Tagging for Code Editors
Speaking filenames naturally and having them converted to @mentions that trigger editor autocomplete is unique to Aqua Voice.

### 7.4 The Floating Bar as Minimal Chrome
The entire recording interface is condensed into a tiny dark pill. No large windows, no complex controls. This is a strong design decision that minimizes interruption to the user's workflow.

### 7.5 Role-Based Presets in Onboarding
The sandbox offers role presets (Developer, Consultant, Sales, Healthcare, Student, Marketer) that presumably configure dictation behavior for different professional contexts. This personalizes the experience from first contact.

### 7.6 In-Browser Sandbox
Users can try Aqua Voice directly in the browser before downloading, reducing friction to first experience.

### 7.7 Transcript History with Feedback Loop
The history tab stores transcripts locally with thumbs up/down ratings and notes, creating a feedback mechanism that could improve the model over time.

### 7.8 Replacements as Text Expansion
The replacements feature acts like TextExpander within the dictation flow, allowing users to define shortcut phrases that auto-expand when spoken.

### 7.9 Deep Context Screen Reading
Optional screen reading improves accuracy for technical terms by understanding the current visual context. This is particularly innovative for code dictation where variable names and function names are visible on screen.

### 7.10 Casual Messaging Toggle
A specific setting for automatically lowercasing text when messaging in casual apps - a nuanced understanding of how users communicate differently across contexts.

---

## 8. COMPETITIVE POSITIONING (UI/UX Comparison)

| Feature               | Aqua Voice                       | Wispr Flow              | Superwhisper           |
|-----------------------|----------------------------------|------------------------|------------------------|
| Primary UI Element    | Floating dark pill bar           | In-line integration    | Modes-based system     |
| Recording Indicator   | Blue orb + waveform              | N/A (inline)           | Custom indicators      |
| Settings Complexity   | Moderate (8+ categories)         | Simpler                | Complex (customizable modes) |
| Onboarding            | In-browser sandbox + role presets | Standard               | Standard               |
| Platform              | Mac + Windows                    | Mac + Windows + iOS    | Mac only               |
| Processing            | Cloud (Avalon model)             | Cloud                  | Local (Whisper)        |
| Speed                 | 450ms (Instant), 850ms (Stream)  | 31% slower than Aqua   | 59% slower than Aqua   |
| Context Awareness     | Automatic per-app + screen reading | Per-app adaptation    | Modes-based            |
| Custom Dictionary     | Up to 800 entries                | Available              | Available              |

---

## 9. DESIGN PRINCIPLES OBSERVED

1. **Radical Minimalism**: The core interaction surface is a tiny dark pill. Everything else stays out of the way.
2. **Context Over Configuration**: The app adapts automatically rather than requiring users to set up per-app profiles.
3. **Speed as a Feature**: Sub-second latency is treated as a primary UX differentiator, not just a technical metric.
4. **Developer-First Design**: Code syntax highlighting, file tagging, and Claude Code integration show the primary audience is technical users.
5. **Progressive Disclosure**: The free tier is simple (1000 words, 5 dictionary entries). Complexity (800 dictionary entries, custom instructions, replacements) unlocks with Pro.
6. **Water/Aqua Theming**: The blue orb, fluid animations, and "aqua" naming create a cohesive brand metaphor.
7. **Privacy by Default**: Deep Context is opt-in, data is not stored on servers, history is local.
8. **Try Before You Buy**: The browser sandbox eliminates the download barrier for first-time users.

---

## 10. AREAS OF INTEREST FOR YOLO VOICE

Based on this research, the following Aqua Voice patterns may be relevant:

1. **The floating recording bar concept**: A minimal, dark pill with waveform visualization is an elegant way to show recording state without dominating the screen. Consider a similar pattern for YOLO Voice's recording indicator.

2. **Settings organization**: Aqua's categorization (Keybindings, Dictionary, Instructions, Replacements, File Tagging, Languages, History, System) provides a clean mental model. YOLO Voice could adopt similar groupings.

3. **Monospace styling for technical content**: The use of monospace fonts for code-related transcripts and the standalone editor creates visual distinction between casual and technical use.

4. **Stats/metrics display**: Showing users their productivity gains (words per minute, time saved) provides tangible value feedback.

5. **The all-caps monospace label pattern**: Using ALL-CAPS MONOSPACE for section labels and categories creates visual hierarchy without additional weight.

6. **Blue accent color system**: A single strong accent color (blue) used consistently across the recording indicator, CTAs, and branding creates a memorable, cohesive identity.

7. **Context-aware behavior is the killer feature**: Aqua's automatic per-app adaptation is its strongest UX differentiator. For an offline-first app like YOLO Voice, similar context detection (even if simpler) would add significant value.

---

## Sources

- [Aqua Voice Official Website](https://aquavoice.com)
- [Aqua Voice Web Sandbox Demo](https://app.aquavoice.com/sandbox)
- [Aqua Voice User Guide](https://aquavoice.com/guide)
- [Aqua Voice Changelog](https://aquavoice.com/changelog)
- [Aqua Voice FAQ](https://aquavoice.com/info/faq)
- [Aqua Voice on Product Hunt](https://www.producthunt.com/products/aqua)
- [9to5Mac Review](https://9to5mac.com/2025/08/15/aqua-voice-shows-just-how-good-mac-dictation-could-be-if-apple-just-tried/)
- [Aestumanda Review](https://www.aestumanda.com/reviews/2025/08/aqua-voice-delivers-what-apples-dictation-still-lacks/)
- [VoiceTypingTools Review](https://www.voicetypingtools.com/tools/aqua-voice)
- [Productivity Academy Article](https://productivity.academy/news/speech-text-aqua/)
- [Hacker News Discussion (Aqua Voice 2)](https://news.ycombinator.com/item?id=43634005)
- [Y Combinator Profile](https://www.ycombinator.com/companies/aqua-voice)
- [Spokenly Comparison](https://spokenly.app/comparison/aqua-voice)
- [TechCompanyNews Article](https://www.techcompanynews.com/aqua-voice-lets-you-replace-typing-with-ultra-fast-dictation-on-any-desktop-app/)
- [KDJingPai Configuration Guide](https://www.kdjingpai.com/en/ruheanzhuanghepeian-81/)
- [Aqua Voice File Tagging Guide](https://aquavoice.com/guide/file-tagging)
