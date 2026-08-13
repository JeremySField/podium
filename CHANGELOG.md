# Changelog

All notable changes to Podium are recorded here.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).


---

## [0.12.0] — 2026-08-10 — Step 12: KB Sources Tile Grid

**Commit:** `feat(onboarding): Step 12 — KB sources tile grid with MemPalace wing fetch`

### Added
- `KbConnectionDraft` struct — replaces bare `Vec<String>` for KB source selections
- `WingFetchState` enum: `Loading | Loaded(Vec<String>) | Failed(String)`
- `fetch_wings_from_mempalace()` — async MCP JSON-RPC POST to `{endpoint}/mcp`
- `create_wing_select()` — wing SelectState entity, created in `&mut Window` context only
- `create_wing_input()` — fallback InputState for local MemPalace, created in handler
- `render_step_kb_sources()` — pure read render, tile grid with 2-column layout
- `render_wing_expansion()` — wing field expansion for selected MemPalace sources
- `reqwest 0.12` and `serde_json 1.0` dependencies

### Changed
- `OnboardingState::kb_source_ids: Vec<String>` → `kb_connections: Vec<KbConnectionDraft>`
- `handle_confirm` maps `KbConnectionDraft` → `KbConnection`
- `render_step_confirm` shows KB source count from `kb_connections`

### Fixed
- `InteractiveElement` import required for `.id()` on `div()`
- `StyledExt as _` import required for `.font_bold()`
- `StatefulInteractiveElement` import required for `.on_click()` after `.id()`
- `ElementId` tuple order: `(&str, usize)` — str first, confirmed from compiler error

---

## [0.11.0] — 2026-08-09 — Step 11: Agent Config Card

**Commit:** `feat(onboarding): Step 11 — agent config card with dynamic roster`

### Added
- `AgentDraft` struct on `OnboardingState`
- `AgentInputState` plain struct — four GPUI entity handles per agent card
- `agent_inputs: Vec<AgentInputState>` and `agent_subscriptions: Vec<Vec<Subscription>>` on `OnboardingSheet`
- `handle_add_agent()` — creates all four entities with subscriptions in one handler
- `handle_remove_agent()` — atomic drop of entities and subscriptions at index
- Six providers with curated model lists: anthropic, openai, google, xai, ollama, custom
- Provider/model coupling via `cx.subscribe_in` — delivers `&mut Window` to stored subscription

### Changed
- `render_step_agents()` — replaces stub with full dynamic agent roster builder
- Model select hidden until provider chosen; custom shows hint instead of model dropdown
- Agent count shown in Step 7 confirm summary

---

## [0.10.0] — 2026-08-09 — Theme: Gruvbox Dark

**Commit:** `fix(theme): apply_config direct — Theme::change bypasses load_themes_from_str`

### Added
- `assets/themes/gruvbox-dark.json` — full ThemeSet format with Gruvbox Dark palette
- Orange primary, warm background, correct contrast ratios throughout

### Fixed
- Sheet white background — ThemeTokens now derived from dark palette
- Near-zero contrast between layers — neutral-950 base corrected
- Primary buttons not orange — button.primary tokens set in JSON
- `Theme::change(Dark)` bypasses `load_themes_from_str` — fixed by using `apply_config` directly

---

## [0.9.0] — 2026-08-09 — Steps 9–10: Project Name + Git Config

### Added
- Step 9: project name `InputState` with alphanumeric validation
- Step 9: `danger_foreground` error color on validation failure
- Step 9: pre-fill from folder name detected in Step 1
- Step 10: git auth `Switch` toggle (SSH / HTTPS)
- Step 10: HTTPS username `InputState`
- Step 10: SSH alias `SelectState` populated from `~/.ssh/config`
- Step 10: remote URL `InputState` pre-filled from `.git/config` if detected

---

## [0.8.0] — 2026-08-09 — Steps 7–8: OnboardingSheet Entity + Folder Picker

### Added
- `OnboardingSheet` as proper GPUI entity — refactored from `Arc<dyn Fn>` callbacks
- Native folder picker via `rfd = "0.17"`
- `.git/` and `.podium/` detection on folder selection
- Project name pre-fill from folder name
- `window.spawn` pattern for async operations in GPUI

### Changed
- Onboarding flow: Sheet container replaces dialog approach
- `open_sheet_at` replaces not stacks — confirmed behavior

---

## [0.7.0] — 2026-08-08 — Phase 2 Foundation + License Remediation

**Commits:**
- `feat(shell): phase 2 start — zed lift watch.rs`
- `fix(license): replace GPL-3.0 ssh_config.rs with clean-room ssh_hosts.rs`
- `chore(manifest): add license, description, repository fields to Cargo.toml`

### Added
- `watch.rs` and `watch_error.rs` — lifted from Zed Apache 2.0 crates with attribution
- `config.rs` — `ProjectsConfig`, `ProjectEntry`, `KbConnection`, `AgentEntry`, `KbSourcesConfig`
- `state.rs` — `PodiumState`, `DockState`
- `ssh_hosts.rs` — clean-room SSH config parser
- Phase 2 dependencies: `toml`, `serde`, `parking_lot`, `rfd`, `keyring`, `chrono`, `uuid`, `dirs`
- First launch init — creates `%APPDATA%\podium\` config directory
- Empty state screen with "Add New Project" button

### Removed
- `ssh_config.rs` — GPL-3.0 verbatim lift, replaced with clean-room implementation

---

## [0.6.0] — 2026-08-09 — Phase 1 Complete: Shell + Panel Layout

**Commit:** `feat(shell): phase 1 complete — Podium shell`

### Added
- Gruvbox Dark theme (initial version)
- Tab bar: Files | Agents | Knowledge | Review | Terminal | Health
- Panel layout with dock system
- Project switcher shell
- `PodiumPanel` trait
- `PodiumDock` with left/right/bottom positions
- `colors.rs` — `PodiumColors` semantic token layer

---

## [0.1.0] — 2026-08-06 — Project Start

### Added
- Project named: Podium
- Architecture defined: room model, agent isolation, project isolation protocol
- Stack selected: GPUI + Rust + gpui-component
- Repository created: github.com/JeremySField/podium
- Apache-2.0 license
- ADR-001 through ADR-018 written
