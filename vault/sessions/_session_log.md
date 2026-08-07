---
project: Podium
file: session_log
type: running session index — concise overview entries, newest at top
last_updated: 2026-08-06
---

<!-- Podium session_log -->

# Podium Session Log

This Podium session log is the running index of all sessions for the Podium project. Entries are concise — orientation and decisions only. Newest entries at top.

<!-- Podium session_log -->

---

## 2026-08-06 — Session 1 (Concept + Phase 0)

**Type:** Strategy / Architecture / Build
**Goal:** Define Podium from scratch and get Phase 0 working

### Part 1 — Concept (Rev 1.0 to Rev 2.0)
- Named the project: Podium
- Stack evolved from Tauri + React to GPUI + Rust
- Defined room model — Podium is the room, projects load into it
- Defined agent isolation — inbox/outbox + live HTTP chat endpoint per agent
- Defined project isolation protocol — sequential load/unload, hard cleanup
- Defined chat panel — Anthropic API + local agent HTTP endpoints
- ADR-001 through ADR-018 written
- Vault structure created, all four core docs written
- Archived original ShowFlyer Homepage dashboard to dashboard/archive/

### Part 2 — Phase 0 Build
- Rust 1.97.1 confirmed installed
- Visual Studio Build Tools confirmed installed
- Repo created: github.com/JeremySField/podium (public, Apache-2.0)
- Local folder: C:\Users\jerem\OneDrive\Desktop\Repos\podium
- create-gpui-app installed and scaffolded project
- Fixed API mismatch — Application::new() replaced with gpui_platform::application()
- Added gpui_platform dependency to Cargo.toml
- GPUI window confirmed working on Windows
- First commit pushed: 5244156

**Phase 0 status: COMPLETE**

**Next steps:**
- Copy vault folder into repo for versioning
- Phase 1: Dark background, panel layout, project selector
- Evaluate gpui-component library before building custom UI components
