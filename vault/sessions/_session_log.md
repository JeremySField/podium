---
project: Podium
file: session_log
type: running session index — concise overview entries, newest at top
last_updated: 2026-08-08
---

<!-- Podium session_log -->

# Podium Session Log

This Podium session log is the running index of all sessions for the Podium project. Entries are concise — orientation and decisions only. Newest entries at top.

<!-- Podium session_log -->

---

## 2026-08-08 — Session 3b (PodiumPanel Trait — Design and Implementation)

**Type:** Development
**Goal:** Write `PodiumPanel` trait before Phase 1 build begins

**Accomplished:**
- All three Session 3 open questions resolved before writing any code
- ADR-027 recorded — dock owns panel position persistence, panels are storage-agnostic
- `src/panel.rs` created — `PanelPosition`, `PanelEvent`, `PodiumPanel` trait, fully documented
- `src/main.rs` updated — `mod panel;` added
- Trait shape: 7 required methods, 2 lifecycle hooks with default no-ops
- Repositionable positions retained — `position()`, `position_is_valid()`, `set_position()` all on trait
- Excluded from trait (intentional): `min_size`, `is_zoomed`, `starts_open`, `Window` param on sizing methods, pane/collab/proto fields, `hide_button_setting`, `enabled`, `is_agent_panel`
- One open compile question: `gpui_component::IconName` vs `gpui_component::icon::IconName` — surfaces on first `cargo build`

**Key decisions:**
- `name()` is a static method — type-level identity, not instance state
- `PanelPosition` derives `Copy` — passed by value everywhere, no clone noise
- `set_position()` takes `&mut Context<Self>` — panel calls `cx.notify()` after updating state
- `activation_priority()` documented with recommended values (100/200/300/400/500/600) with gaps
- Dock panics in debug on duplicate priority — same as Zed

**Next session:** `PanelHandle` object-safe wrapper → `PodiumDock` → `main.rs` rewrite → stub FilesPanel → Phase 1 build

**Full report:** `session_reports/session_003b_2026-08-08.md`

---

## 2026-08-08 — Session 3 (Zed Codebase Inspection)

**Type:** Research / Strategy
**Goal:** Full Zed crate inspection against Podium spec and ADRs

**Key findings:**
- 229 crates inspected and mapped to Podium phases
- Critical GPL discovery: terminal/ and terminal_view/ are GPL-3.0, not Apache 2.0
- 4 direct lifts identified: watch/, credentials_provider/, fs_watcher.rs, ssh_config.rs
- dock.rs Panel trait fully mapped — PodiumPanel trait design is clear before Phase 1
- All major AI provider clients already exist in Zed (anthropic/, open_ai/, google_ai/, ollama/, x_ai/) — potential lifts for Phase 8
- Zed Docker files noted for agent container pattern research

**Decisions made:**
- ADR-026: Terminal uses alacritty_terminal (MIT), not Zed terminal crates (GPL)
- ADR-012 superseded by ADR-026
- Terminal moves to Phase 3 minimum — independent implementation required

**Phase numbering updated:**
Phase 3 is now Terminal (was Git). Git moved to Phase 4. Files moved to Phase 5. Editor Phase 6. Agent Panel Phase 7. Chat Phase 8. Knowledge Phase 9. Polish Phase 10.

**Estimated build savings from Zed inspection: 20–35% overall**

**Next session:**
- Confirm Phase 1 scope
- Draft PodiumPanel trait signature for approval
- Begin Phase 1 build

---

## 2026-08-07 — Session 2 (Phase 0 Completion + Spec Work)

**Type:** Build / Setup / Architecture
**Goal:** Complete Phase 0, integrate gpui-component, define onboarding and project switcher spec

**Accomplished:**
- gpui-component integrated and confirmed working
- Project onboarding spec written — Rev 1.1 APPROVED
- ADR-019 through ADR-025 added
- README written
- Phase 1 primer written and updated
- Vault restructured with phase_1 folder
- Zed repo cloned locally and copied to knowledgebase

**Key decisions:** See session_002 report

**Phase 0 status: COMPLETE**

---

## 2026-08-06 — Session 1 (Concept + Phase 0 Start)

**Type:** Strategy / Architecture / Build
**Goal:** Define Podium from scratch and get Phase 0 working

**Accomplished:**
- Named the project: Podium
- Full architecture defined — room model, agent isolation, project isolation protocol
- Stack: GPUI + Rust + gpui-component
- ADR-001 through ADR-018 written
- Repo created: github.com/JeremySField/podium (public, Apache-2.0)
- GPUI window confirmed working on Windows
- First commits pushed

**Key decisions:** ADR-001 through ADR-018 — see _adr.md
