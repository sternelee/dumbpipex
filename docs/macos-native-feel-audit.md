# macOS Native-Feel Audit — dumbpipex

## Scope

Code-backed audit of the current Tauri + SvelteKit app in this repo, focused on why it is likely to read as "web-y" on macOS and what to fix first. This is a source audit, not a screenshot/recording-based visual QA pass.

## 2026-06-05 refresh

This document predates later mobile and web-shell work. The current repo now
contains generated iOS/Android platform directories under `src-tauri/gen/`, and
`docs/mobile-gen-regeneration.md` records manual generated-platform tweaks. A
Svelte `MenuBar` component also exists.

Those updates do not change the core macOS conclusion: the app still lacks
native Tauri/macOS menu wiring, window/titlebar/material decisions, and
shell-level shortcut behavior. Treat the older "no mobile files found" wording
below as superseded by this refresh.

## Repo profile

- Frontend: SvelteKit SPA (`src/routes/+page.svelte`, `src/lib/*.svelte`)
- Shell: Tauri v2 (`src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`)
- Primary surface: remote terminal workspace with connection screen + PTY tabs
- Mobile status: generated iOS/Android projects exist under `src-tauri/gen/`,
  but mobile-native behavior remains mostly WebView/responsive UI plus manual
  platform tweaks.

## Executive summary

This app currently reads as **web-first inside a default Tauri window**, not as a Mac-native desktop app.

The biggest reasons are:

1. **Default shell/window setup** — no visible native window customization, menu, material, titlebar, or macOS-specific behavior.
2. **Strong web visual language** — gradient page background, rounded cards, CSS shadows, pill chips, and a branded web-app control style.
3. **Web interaction cues** — `cursor: pointer` on buttons, browser-style clipboard/open-link behavior, and no evidence of standard Mac command shortcuts.
4. **Mobile is platform-generated but still not product-native** — generated
   iOS/Android shells and manual platform tweaks exist, but the primary
   terminal interaction model is still mostly WebView/responsive UI.

If the goal is "passes as a real Mac app," the current UI should be treated as a **good functional prototype** rather than a finished native-feel shell.

---

## High-confidence findings

### 1) The window is configured like a default Tauri app

**Evidence**

- `src-tauri/tauri.conf.json:13-17` defines only a basic window with title + size:
  - `"windows"`
  - `"title": "dumbpipex"`
  - `"width": 800`
  - `"height": 600`
- No signs in config of:
  - custom/native titlebar strategy
  - transparency/material/vibrancy
  - macOS traffic-light placement tuning
  - hidden title / title bar style choices
  - native menu setup

**Impact**
This strongly suggests the app is running in a mostly stock Tauri shell. That is fine for utility apps, but it usually falls short of "indistinguishable from native" on macOS.

### 2) The native shell bootstrap has no macOS-native integration yet

**Evidence**

- `src-tauri/src/lib.rs:304-307`:
  - `tauri::Builder::default()`
  - `.plugin(tauri_plugin_opener::init())`
  - `.invoke_handler(...)`
- `src/lib/MenuBar.svelte` provides a web-rendered menu bar, not a native
  macOS app menu.
- No evidence in Rust bootstrap of:
  - native menu creation
  - window appearance customization
  - macOS material/vibrancy hooks
  - shortcut registration
  - dock/menu-bar specific behavior

**Impact**
The shell is currently acting as a transport host, not as a Mac app shell. That is the main architectural reason the app will still feel web-y even if the CSS improves.

### 3) The root surface uses a web-app aesthetic, not a desktop-app aesthetic

**Evidence**

- `src/routes/+page.svelte:635-636` uses:
  - `Inter, ui-sans-serif, system-ui, -apple-system...`
- `src/routes/+page.svelte:642` uses a decorative page-level `radial-gradient(...)`
- `src/lib/ConnectionHome.svelte:119-120` and `src/lib/SessionWorkspace.svelte:299-300` use large `border-radius` + `box-shadow`
- `src/lib/ConnectionHome.svelte:210` uses a prominent `linear-gradient(...)` primary button

**Impact**
These are standard web/SaaS cues. On macOS, desktop apps usually rely more on system materials, flatter surfaces, tighter grouping, and less decorative depth.

### 4) Controls use browser-style pointer affordances

**Evidence**

- `src/lib/ConnectionHome.svelte:206` — `cursor: pointer`
- `src/lib/SessionWorkspace.svelte:374` — `cursor: pointer`

**Impact**
This is a small CSS detail with outsized perceptual cost. Native Mac controls generally do **not** announce clickability with a hand cursor across normal UI chrome.

### 5) The UI is built from card stacks and pill chips rather than native desktop structure

**Evidence**

- `ConnectionHome.svelte` and `SessionWorkspace.svelte` rely on:
  - large cards
  - pill status chips (`border-radius: 999px`)
  - segmented panel stacks
  - rounded tab-like controls

**Impact**
This reads more like a modern web dashboard than a Mac utility. For a terminal-adjacent productivity app, the native-feel bar is usually met by denser toolbars, subtler sections, and less "cardification."

### 6) Keyboard conventions are underdeveloped for macOS

**Evidence**

- Search found only one explicit keyboard handler in app UI:
  - `src/lib/SessionWorkspace.svelte:179` handles `Enter` in search
- No evidence found of:
  - `Cmd+,` for preferences
  - `Cmd+W` / `Cmd+N` / `Cmd+F`
  - standard app-level menu shortcuts
  - menu-command routing

**Impact**
Mac-native feel is often more about keyboard/menu behavior than visuals. Right now the app appears terminal-focused but not Mac-command fluent.

### 7) Browser APIs are used where shell-mediated behavior may be preferable

**Evidence**

- `src/lib/RemotePtyPane.svelte:79` uses `navigator.clipboard.writeText(...)`
- `src/lib/RemotePtyPane.svelte:182` uses `window.open(...)` for links

**Impact**
These are acceptable for a prototype, but they reinforce the feeling that the WebView is in charge. For a more native-feel shell, clipboard/open-link handling should be intentionally mediated through the host where needed.

### 8) Mobile support is generated and responsive, not yet native

**Evidence**

- `src-tauri/src/lib.rs:302` has `#[cfg_attr(mobile, tauri::mobile_entry_point)]`
- `src/lib/SessionWorkspace.svelte` includes `compactLayout` behavior and mobile shortcut rows
- `src/lib/RemotePtyPane.svelte:144-148` adds long-press copy for non-mouse pointers
- Generated native projects exist under `src-tauri/gen/android` and
  `src-tauri/gen/apple`
- `docs/mobile-gen-regeneration.md` records manual generated-platform tweaks

**Impact**
This codebase is _mobile-aware_ and has generated mobile shells, but it is not
yet _mobile-native_. Responsive adaptation plus generated project files alone
will not produce a good mobile terminal experience.

---

## Top 10 fixes by native-feel impact

### 1. Rework the window/titlebar before touching most CSS

**Do**

- Decide whether to keep standard decorations or build a deliberate macOS titlebar strategy.
- Add a real toolbar/titlebar model instead of a generic full-bleed web page.
- Make the app feel like a terminal utility window, not a website inside a window.

**Why first**
Window feel beats color palette in user perception.

### 2. Remove `cursor: pointer` from normal app chrome

**Files**

- `src/lib/ConnectionHome.svelte`
- `src/lib/SessionWorkspace.svelte`

**Why**
This is a fast, high-signal native-feel improvement.

### 3. Replace the page-level gradient/card aesthetic with a denser desktop surface

**Do**

- Remove the radial page glow from `src/routes/+page.svelte`
- Reduce large-radius cards and big shadows in `ConnectionHome.svelte` and `SessionWorkspace.svelte`
- Prefer flatter grouped sections / split-pane desktop composition

**Why**
Current styling says "web dashboard." The product category says "desktop terminal/workspace."

### 4. Stop mixing `Inter` into main app chrome

**Do**

- Use the system UI font for chrome, labels, buttons, tabs, headings
- Keep custom type only if there is a strong branding reason, and even then use sparingly

**Why**
Mac users are very sensitive to typography mismatch.

### 5. Add real macOS menu + shortcuts

**Do**

- Implement app menu items for About / Settings / Quit
- Add standard shortcuts like `Cmd+,`, `Cmd+W`, `Cmd+F`
- Route commands through the shell/app layer, not just ad hoc DOM listeners

**Why**
This is one of the fastest ways to make the app feel like a Mac app instead of a webpage.

### 6. Redesign the workspace header/toolbar as a native utility toolbar

**Do**

- Compress the current large header
- Move status, session controls, and PTY actions into a tighter toolbar model
- Reduce marketing-style eyebrow + hero-copy treatment inside the main app window

**Why**
The connection screen and workspace header currently feel like web landing/product UI.

### 7. Treat tabs and chips like native segmented controls, not rounded web pills

**Do**

- Reduce pill usage (`status-pill`, `mode-chip`, theme chips, tabs)
- Consider segmented-control styling and denser tab treatment

**Why**
Pill-heavy UI is a strong web/SaaS cue.

### 8. Move important shell behaviors out of the WebView

**Do**

- Review clipboard, link opening, dialogs, notifications, settings window, and file interactions
- Decide which should remain browser APIs and which should be shell-mediated

**Why**
Native feel improves when the WebView becomes a rendering surface, not the entire app runtime model.

### 9. Create a separate mobile product surface, not just a compact desktop layout

**Do**

- Keep shared core/session logic
- Re-think terminal controls, keyboard affordances, layout, and session actions for touch
- Avoid simply shrinking the desktop workspace onto mobile

**Why**
Mobile-native and Mac-native are different interaction models.

### 10. Add a "native-feel QA pass" to definition of done

**Do**
For every UI PR, check:

- Does this add pointer-cursor chrome?
- Does this add more card/shadow/radius styling?
- Does this ignore macOS keyboard/menu behavior?
- Does this push shell responsibilities into the WebView?

**Why**
Without a guardrail, the app will drift back toward web defaults.

---

## Recommended implementation phases

### Phase 1 — quick wins (1-2 days)

- Remove `cursor: pointer` from standard buttons/chrome
- Swap chrome typography to system font only
- Reduce card shadows/radii
- Remove root radial-gradient treatment
- Tighten spacing and density in header/toolbar/tab areas

### Phase 2 — meaningful desktop polish (2-5 days)

- Add native app menu + keyboard shortcuts
- Rework header into a compact utility toolbar
- Simplify chips/pills/tabs into more desktop-like controls
- Audit dialogs/clipboard/link opening paths

### Phase 3 — shell-level work (larger lift)

- Introduce explicit macOS window/titlebar/material decisions
- Separate shell responsibilities from WebView responsibilities
- Decide whether Tauri remains sufficient for the last 20% of native-feel requirements

---

## Mobile recommendation

If mobile matters, share:

- connection/session protocol
- PTY/session model
- reconnection logic
- Rust/core networking/domain logic

Do **not** force-share:

- desktop toolbar layout
- desktop tabs/chips/header composition
- hover assumptions
- keyboard-first affordances

For mobile, design a touch-first session surface instead of porting the desktop workspace verbatim.

---

## Architecture call

### Current state

Good fit for:

- functional Tauri desktop utility
- internal or early-stage product
- cross-platform prototype

### Not yet good fit for

- "this should pass as a real Mac app"
- high-polish desktop product where shell quality is part of the product value

### Rule of thumb

If, after Phases 1-2, the remaining issues are mostly:

- titlebar
- menu behavior
- window/material behavior
- shell-level shortcuts
- settings/dialog behavior

...then the limiting factor is no longer Svelte styling. It is the shell architecture.
