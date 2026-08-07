---
project: Podium
file: phase_1_primer
type: phase primer — load at Phase 1 session start alongside _primer.md
last_updated: 2026-08-07
status: READY — Phase 1 not yet started
---

<!-- Podium phase_1_primer -->

# Podium — Phase 1 Primer
## Dark Theme + Panel Layout + Project Switcher

<!-- Podium phase_1_primer -->

---

## Phase 1 Goal

Replace the Hello World green screen with the actual Podium shell — dark themed, panel layout visible, project switcher present. No real data. No wired functionality. Visual structure only.

**Phase 1 is complete when:**
- Window opens with dark theme applied
- TitleBar visible at top with Podium name and project switcher placeholder
- Panel layout visible — at minimum a left sidebar area and main content area
- Tab bar visible with placeholder tabs (Files, Agents, MemPalace, Review, Terminal, Health)
- StatusBar visible at bottom
- Everything static — no data, no interactions required yet
- `cargo run` opens cleanly, no errors

---

## Critical Prerequisites — Must Resolve Before Writing Code

### 1. Root View is Required

The current `main.rs` does NOT use `Root` as the first-level window child. This is a hard requirement for gpui-component to function correctly. Without it, dialogs, sheets, notifications, and theming will not work.

**Current main.rs opens the window like this:**
```rust
cx.open_window(WindowOptions::default(), |_, cx| {
    cx.new(|_cx| HelloWorld { ... })
})
```

**Must be changed to:**
```rust
cx.spawn(async move |cx| {
    cx.open_window(WindowOptions::default(), |window, cx| {
        let view = cx.new(|_| PodiumApp);
        cx.new(|cx| Root::new(view, window, cx))
    })
    .expect("Failed to open window");
})
.detach();
```

**Source:** https://longbridge.github.io/gpui-component/docs/root

### 2. Assets Must Be Registered

The current `main.rs` calls `gpui_platform::application()` without registering assets. gpui-component-assets must be registered for icons and default assets to work.

**Current:**
```rust
gpui_platform::application().run(...)
```

**Must be changed to:**
```rust
gpui_platform::application()
    .with_assets(gpui_component_assets::Assets)
    .run(...)
```

**Source:** https://longbridge.github.io/gpui-component/docs/getting-started

### 3. Dark Theme Must Be Set Explicitly

gpui-component has 20+ built-in themes. Dark mode is not automatic — it must be set. The theme system uses `ThemeRegistry` to load themes from a `./themes` directory, or themes can be set programmatically.

**Questions to resolve:**
- Does gpui-component ship a dark theme that can be set without a themes directory?
- Is there a built-in dark theme name that can be loaded from the registry without copying theme files?
- What is the simplest way to set dark mode for Phase 1?

**Source:** https://longbridge.github.io/gpui-component/docs/theme
**Theme files location in repo:** https://github.com/longbridge/gpui-component/tree/main/themes

### 4. TitleBar Component — Windows-Specific Behavior

gpui-component has a `TitleBar` component. On Windows, native title bars behave differently than macOS. Need to confirm:
- Does TitleBar support custom content on Windows (project switcher placement)?
- Does it handle window dragging correctly on Windows?
- Is a custom title bar needed or does the component handle this?

**Source:** https://longbridge.github.io/gpui-component/docs/components/title-bar

### 5. Resizable Panel Layout

gpui-component has a `Resizable` component for panel layouts. Need to understand:
- How is a basic two-panel layout (sidebar left, main content right) structured?
- How are initial panel sizes set?
- Is there a minimum panel size constraint?

**Source:** https://longbridge.github.io/gpui-component/docs/components/resizable

### 6. Tabs Component — State Management

Tabs is a stateful component. Need to confirm:
- How is the active tab state held in the view struct?
- How does tab switching trigger content area re-render?
- Can tab labels be set with icons?

**Source:** https://longbridge.github.io/gpui-component/docs/components/tabs

---

## Information Sources for Phase 1

| Topic | Source |
|-------|--------|
| Root view (required) | https://longbridge.github.io/gpui-component/docs/root |
| Getting started / assets | https://longbridge.github.io/gpui-component/docs/getting-started |
| Theme system | https://longbridge.github.io/gpui-component/docs/theme |
| Theme files | https://github.com/longbridge/gpui-component/tree/main/themes |
| TitleBar | https://longbridge.github.io/gpui-component/docs/components/title-bar |
| Resizable panels | https://longbridge.github.io/gpui-component/docs/components/resizable |
| Tabs | https://longbridge.github.io/gpui-component/docs/components/tabs |
| Sidebar | https://longbridge.github.io/gpui-component/docs/components/sidebar |
| StatusBar | https://longbridge.github.io/gpui-component/docs/components/status-bar |
| LLM-optimized full docs | https://longbridge.github.io/gpui-component/llms-full.txt |
| gpui-component examples | https://github.com/longbridge/gpui-component/tree/main/examples |
| API reference | https://docs.rs/gpui-component |

---

## Podium Visual Structure — Phase 1 Target

```
┌─────────────────────────────────────────────────────┐
│  TitleBar — [Podium] [Project Switcher ▼]           │
├─────────────────────────────────────────────────────┤
│  [Files] [Agents] [MemPalace] [Review] [Terminal] [Health] │  ← Tabs
├──────────┬──────────────────────────────────────────┤
│          │                                          │
│ Sidebar  │  Main Content Area                       │
│ (future) │  (empty for Phase 1)                     │
│          │                                          │
│          │                                          │
│          │                                          │
├─────────────────────────────────────────────────────┤
│  StatusBar — branch / status indicators             │
└─────────────────────────────────────────────────────┘
```

---

## Questions Requiring Design Decisions Before Code

**Q1: Where does the project switcher live?**
Options:
- In the TitleBar (most compact, like VS Code's workspace selector)
- As a top element above the tab bar
- In a left sidebar panel

Recommendation: TitleBar. Keeps the tab bar clean and matches the concept doc — project switcher always visible regardless of active tab.

**Q2: Where do the tabs live — above or below the TitleBar?**
Standard is below the TitleBar, above the content area. Confirm this is the intended layout before building.

**Q3: Does the sidebar have content in Phase 1?**
The sidebar is referenced in the concept doc but no panel requires a left sidebar in Phase 1. Recommendation: omit the sidebar for Phase 1, add in a later phase when Files or Agents panel needs it. Keep Phase 1 scope minimal.

**Q4: What dark theme name to use?**
Need to check available built-in theme names in the gpui-component themes directory before the session. Likely candidates: any theme with "dark" in the name, or "One Dark", "Nord", etc.

---

## main.rs Rewrite Scope for Phase 1

The entire `main.rs` needs to be rewritten — it's currently a hello world placeholder. Phase 1 replaces it with:

1. `PodiumApp` struct — the root view
2. `Root` wrapping `PodiumApp` in the window
3. Assets registered on application
4. Dark theme applied
5. TitleBar rendered
6. Tabs rendered below TitleBar
7. Content area (empty div for now)
8. StatusBar at bottom
9. Dialog, sheet, notification layers in render method

---

## Pre-Session Checklist

Before writing any Phase 1 code, read these docs in order:

- [ ] Root view — https://longbridge.github.io/gpui-component/docs/root
- [ ] TitleBar — https://longbridge.github.io/gpui-component/docs/components/title-bar
- [ ] Tabs — https://longbridge.github.io/gpui-component/docs/components/tabs
- [ ] Resizable — https://longbridge.github.io/gpui-component/docs/components/resizable
- [ ] StatusBar — https://longbridge.github.io/gpui-component/docs/components/status-bar
- [ ] Theme — https://longbridge.github.io/gpui-component/docs/theme
- [ ] Confirm dark theme name from themes directory
- [ ] Answer the 4 design questions above

---

## Known Issues Entering Phase 1

- Current `main.rs` missing `Root` wrapper — must fix before any gpui-component features will work correctly
- Current `main.rs` missing `.with_assets()` — must fix before icons will work
- Zed right-click context menu not working on Windows — check for Zed update before session

---

*Phase 1 Primer — 2026-08-07 — Load alongside _primer.md at Phase 1 session start*
