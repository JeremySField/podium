---
project: Podium
file: session_log
type: running session index — concise overview entries, newest at top
last_updated: 2026-08-07
---

<!-- Podium session_log -->

# Podium Session Log

This Podium session log is the running index of all sessions for the Podium project. Entries are concise — orientation and decisions only. Newest entries at top.

<!-- Podium session_log -->

---

## 2026-08-07 — Session 2 (Phase 0 Completion + Spec Work)

**Type:** Build / Setup / Architecture
**Goal:** Complete Phase 0, integrate gpui-component, define onboarding and project switcher spec

**Part 1 — Phase 0 Completion**
- Knowledgebase folder renamed from dashboard to podium
- gpui-component and gpui-component-assets added to Cargo.toml
- cargo build confirmed clean
- main.rs updated — gpui_component::init(cx) added
- cargo run confirmed — window opens cleanly
- All changes committed methodically

**Part 2 — Spec Work**
- Phase 1 primer written — vault/phase_1/_phase_1_primer.md
- gpui-component reference docs downloaded — vault/phase_1/gpui-component-docs.md
- Vault restructured — phase_1 folder added
- Project onboarding spec written and fully resolved — vault/specs/_project_onboarding_spec.md Rev 1.0 APPROVED
- ADR-019 (Sheet over Dialog) and ADR-020 (Progressive disclosure) added

**Key decisions this session:**
- Project switcher: Combobox (searchable, Zed-style)
- projects.toml: %APPDATA%\podium\
- Room folder: .podium/ inside project root, auto-gitignored
- Onboarding: Sheet, skippable steps except folder+name
- First launch: empty state with prompt, no auto-open
- Agent roster: fully user-defined, no fixed roster
- Agent providers: Anthropic, OpenAI, Google, xAI, Custom/Local
- API keys: global per provider, Windows Credential Manager
- KB: library of sources per project, assigned per agent
- KB providers: MemPalace, Obsidian, Notion, Custom (beta)
- All service credentials: per-project, Windows Credential Manager
- Services beta: Supabase, Railway, GitHub, Custom
- Services v1.0: full list documented in spec
- Settings: application menu (Zed pattern), not a tab
- Tab bar: Files | Agents | Knowledge | Review | Terminal | Health
- Progressive disclosure: speed default, clarity on demand
- Re-import detected .podium/: prompt Skip or Overwrite
- New project loads automatically after creation
- Deleted agent folders archived, never permanently deleted

**Phase 0 status: COMPLETE**

**Next steps:**
- Commit all spec and vault updates
- Phase 1: GPUI shell with dark theme, panel layout, project switcher
- Read phase_1/_phase_1_primer.md before Phase 1 session start

---

## 2026-08-06 — Session 1 (Concept + Phase 0 Start)

**Type:** Strategy / Architecture / Build
**Goal:** Define Podium from scratch and get Phase 0 working

**Accomplished:**
- Named the project: Podium
- Stack evolved from Tauri + React to GPUI + Rust
- Defined room model — Podium is the room, projects load into it
- Defined agent isolation — inbox/outbox + live HTTP chat endpoint per agent
- Defined project isolation protocol — sequential load/unload, hard cleanup
- Defined chat panel — Anthropic API + local agent HTTP endpoints
- ADR-001 through ADR-018 written
- Vault structure created, all four core docs written
- Archived original ShowFlyer Homepage dashboard to podium/archive/
- Repo created: github.com/JeremySField/podium (public, Apache-2.0)
- Rust 1.97.1 and Visual Studio Build Tools confirmed
- create-gpui-app scaffolded project
- Fixed API mismatch — Application::new() replaced with gpui_platform::application()
- GPUI window confirmed working on Windows
- First commits pushed

**Key decisions:** ADR-001 through ADR-018 — see _adr.md
