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
- TitleBar visible at top with Podium name and project switcher (Combobox)
- Application menu accessible from TitleBar
- Tab bar visible: Files | Agents | Knowledge | Review | Terminal | Health
- Main content area visible (empty for Phase 1)
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

**Questions to resolve at session start:**
- Does gpui-component ship a dark theme that can be set without a themes directory?
- Is there a built-in dark theme name that can be loaded from the registry without copying theme files?
- What is the simplest way to set dark mode for Phase 1?

**Source:** https://longbridge.github.io/gpui-component/docs/theme
**Theme files location in repo:** https://github.com/longbridge/gpui-component/tree/main/themes

### 4. TitleBar Component — Windows-Specific Behavior

gpui-component has a `TitleBar` component. On Windows, native title bars behave differently than macOS. Need to confirm:
- Does TitleBar support custom content on Windows (Combobox project switcher placement)?
- Does it handle window dragging correctly on Windows?

**Source:** https://longbridge.github.io/gpui-component/docs/components/title-bar

### 5. Resizable Panel Layout

gpui-component has a `Resizable` component for panel layouts. Need to understand:
- How is a basic layout (main content area) structured?
- How are initial panel sizes set?

**Source:** https://longbridge.github.io/gpui-component/docs/components/resizable

### 6. Tabs Component — State Management

Tabs is a stateful component. Need to confirm:
- How is the active tab state held in the view struct?
- How does tab switching trigger content area re-render?

**Source:** https://longbridge.github.io/gpui-component/docs/components/tabs

---

## Locked Design Decisions for Phase 1

| Decision | Value |
|----------|-------|
| Project switcher component | Combobox (searchable, Zed-style) |
| Tab bar | Files \| Agents \| Knowledge \| Review \| Terminal \| Health |
| Settings location | Application menu — NOT a tab |
| Theme | Dark — always, no toggle |
| Visual style | Minimal, text-focused, no color noise |
| StatusBar | Bottom of window |
| TitleBar | Top of window with Podium name + project switcher |

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
| Combobox | https://longbridge.github.io/gpui-component/docs/components/combobox |
| LLM-optimized full docs | https://longbridge.github.io/gpui-component/llms-full.txt |
| Local reference copy | vault/phase_1/gpui-component-docs.md |
| gpui-component examples | https://github.com/longbridge/gpui-component/tree/main/examples |
| API reference | https://docs.rs/gpui-component |

---

## Podium Visual Structure — Phase 1 Target

```
┌─────────────────────────────────────────────────────┐
│  TitleBar — [≡ Menu] [Podium] [Project Switcher ▼]  │
├─────────────────────────────────────────────────────┤
│  Files | Agents | Knowledge | Review | Terminal | Health │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Main Content Area                                  │
│  (empty for Phase 1)                                │
│                                                     │
│                                                     │
├─────────────────────────────────────────────────────┤
│  StatusBar — branch / status indicators             │
└─────────────────────────────────────────────────────┘
```

---

## main.rs Rewrite Scope for Phase 1

The entire `main.rs` needs to be rewritten — it is currently a hello world placeholder. Phase 1 replaces it with:

1. `PodiumApp` struct — the root view
2. Assets registered on application with `.with_assets()`
3. `Root` wrapping `PodiumApp` in the window
4. Dark theme applied
5. TitleBar with Podium name and Combobox project switcher placeholder
6. Application menu placeholder
7. Tab bar — Files, Agents, Knowledge, Review, Terminal, Health
8. Main content area (empty div for now)
9. StatusBar at bottom
10. Dialog, sheet, notification layers in render method

---

## Pre-Session Checklist

Before writing any Phase 1 code, read these docs in order:

- [ ] Root view — https://longbridge.github.io/gpui-component/docs/root
- [ ] TitleBar — https://longbridge.github.io/gpui-component/docs/components/title-bar
- [ ] Tabs — https://longbridge.github.io/gpui-component/docs/components/tabs
- [ ] Combobox — https://longbridge.github.io/gpui-component/docs/components/combobox
- [ ] StatusBar — https://longbridge.github.io/gpui-component/docs/components/status-bar
- [ ] Theme — https://longbridge.github.io/gpui-component/docs/theme
- [ ] Confirm dark theme name from themes directory
- [ ] Check gpui-component examples for TitleBar + Tabs patterns

---

## Known Issues Entering Phase 1

- Current `main.rs` missing `Root` wrapper — must fix before any gpui-component features work correctly
- Current `main.rs` missing `.with_assets()` — must fix before icons work
- Zed right-click context menu not working on Windows — check for Zed update before session

---

*Phase 1 Primer — updated 2026-08-07*
*Load alongside _primer.md at Phase 1 session start*
