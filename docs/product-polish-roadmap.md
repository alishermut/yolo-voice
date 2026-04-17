# Product Polish Roadmap

This roadmap captures the product-facing improvements we want to bring into `YOLO Voice` after reviewing stronger product patterns from tools like Whispering.

The goal is not to copy another app feature-for-feature. The goal is to keep our dictation-focused technical strengths while improving clarity, onboarding, trust, and day-to-day usability.

## Product Direction

`YOLO Voice` should feel:

- Fast to understand
- Fast to start using
- Trustworthy about where data goes
- Powerful without overwhelming new users

We want the app to present a simple promise up front:

> Offline-first dictation that inserts text anywhere.

## Principles

- Preserve our engine strengths: `Parakeet`, `Distil-Whisper`, segmented mode, vocabulary, cleanup, and offline control stay core.
- Reduce visible complexity: beginner flows should not require understanding every engine or tuning option.
- Optimize for first success: the first-run experience should get the user speaking and seeing results quickly.
- Make trust visible: privacy, storage, and provider behavior should be obvious in the UI.
- Ship in layers: each phase should deliver a complete improvement on its own.

## Active Workstream

This roadmap is a focused sub-roadmap for product polish and workflow design.

It should be used for:

- Onboarding improvements
- Everyday dictation UX
- Hands-free workflows
- Transformation and command packaging
- Trust and local-first UX
- Simple vs advanced settings design

It should not be used to justify unrelated engine churn or broad product expansion.

## Phase 1: First-Run Success

### Goal

Help a new user get from install to successful dictation in under two minutes.

### Why this phase comes first

Our core capabilities are already strong, but the first-run flow still feels more like setup than success. We should bias the onboarding toward a fast win.

### Scope

- Simplify onboarding copy and decisions
- Recommend a sensible default engine automatically
- Add a guided `test dictation now` step
- Explain the hotkey and insertion flow in plain language
- Show a clear success state after the first test

### Out of scope

- New engine work
- Large settings refactors
- Advanced power-user features

### Done when

- A new user can choose a mic, accept the recommended engine, speak once, and see text inserted without visiting advanced settings

## Phase 2: Hands-Free Dictation

### Goal

Make daily dictation feel effortless and more natural.

### Why this phase matters

Hands-free interaction is one of the clearest product upgrades we can learn from. It improves delight and makes the app feel more intentional, not just technically capable.

### Scope

- Add a true voice-activated mode
- Improve start, listening, recording, processing, and inserted-state feedback
- Make the pill state changes more legible
- Reduce confusion around when recording begins and ends

### Out of scope

- General command workflows
- Large history or data model changes

### Done when

- A user can trigger dictation once, speak naturally, pause, and get text without manually managing the full recording cycle

## Phase 3: Transformations and Command Packaging

### Goal

Turn command mode into a clearer, more reusable product feature.

### Why this phase matters

Raw transcription is valuable, but text actions create leverage. We already have the ingredients; this phase packages them into something faster to discover and use repeatedly.

### Scope

- Add saved text transformations
- Add a quick action picker for common transformations
- Add one-shot shortcut flow for the default transformation
- Reframe command mode as part of a broader text-actions workflow

### Out of scope

- General-purpose assistant behavior
- Expanding into a separate app category

### Done when

- A user can dictate or select text and quickly run a saved rewrite action with minimal UI friction

## Phase 4: Trust and Local-First UX

### Goal

Make privacy and storage behavior explicit and easy to verify.

### Why this phase matters

Users should not have to infer whether data stays local, where it is stored, or when cloud providers are involved. Trust should be visible, not just technically true.

### Scope

- Show where transcripts, settings, models, and logs are stored
- Add `Open data folder` style actions where appropriate
- Improve privacy and provider messaging
- Add export affordances for history where useful
- Clarify what is local vs cloud in relevant settings

### Out of scope

- Full storage architecture rewrite
- New sync or multi-device features

### Done when

- A user can easily understand what is stored, where it lives, and which actions use cloud services

## Phase 5: Simple vs Advanced

### Goal

Preserve our depth while making the default surface dramatically easier to navigate.

### Why this phase matters

We have a strong power-user core. The risk is exposing too much of it too early. This phase keeps the depth but improves approachability.

### Scope

- Introduce a `Simple` and `Advanced` split where it improves clarity
- Add recommended presets such as `Fastest`, `Best Quality`, `Coding`, and `Hands-Free`
- Improve model tradeoff presentation
- Surface better compatibility and diagnostics hints

### Out of scope

- Removing advanced controls
- Hiding critical configuration behind unclear UX

### Done when

- New users can operate comfortably in the simple surface while experienced users still have access to the controls they expect

## Delivery Rules

- Only one phase should be actively in progress at a time
- Each phase should end in a shippable product improvement
- Each phase should have its own implementation plan before coding starts
- If a task does not clearly support the active phase, it should be deferred or justified explicitly

## Current Recommendation

Start with `Phase 1: First-Run Success`.

This has the best ratio of user impact to implementation risk, and it sets up the later hands-free and transformation work on a cleaner foundation.

## Candidate First Tasks For Phase 1

- Review the current onboarding flow and identify every decision the user must make before first dictation
- Define the recommended default engine logic
- Design a `test dictation now` step and success confirmation state
- Tighten onboarding copy around hotkey, insertion, and offline/cloud behavior
- Decide which settings must remain in onboarding and which should move out
