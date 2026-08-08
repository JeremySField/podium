---
project: Podium
file: primer
type: permanent — load at every session start
last_updated: 2026-08-08
---

<!-- Podium primer -->

# Podium Primer

Load this primer at the start of every Podium session before any other vault file.

<!-- Podium primer -->

---

## What Podium Is

Podium is a purpose-built, GPU-accelerated native desktop workspace for a solo builder running multiple complex projects. It is the workspace itself — not a dashboard, not an observability layer, not a wrapper around other tools.

**Podium replaces:** Zed, claude.ai browser tab, the ShowFlyer Homepage dashboard, and any other tool currently filling workflow gaps.

**One app. One window. Everything in it.**

---

## Current Phase

**Status:** Phase 0 complete — Phase 1 not yet started
**Next step:** Phase 1 — dark theme, panel layout, project switcher
**Read phase_1/_phase_1_primer.md before starting Phase 1**

---

## The Room Model

Podium is the room. Fixed. Permanent. Projects load into the room — the room does not change, the context does. One project loaded at a time. All panels recontextualize on load. Hard cleanup on unload before any new project loads.

---

## Key Vault Files

| File | Load When |
|------|-----------|
| `primer/_primer.md` | Every session — this file |
| `specs/_concept.md` | Every session — full architecture Rev 2.0 |
| `decisions/_adr.md` | Any session touching architectural decisions |
| `sessions/_session_log.md` | Every session — current status and priorities |
| `phase_1/_phase_1_primer.md` | All Phase 1 sessions — load alongside this file |

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| UI framework | GPUI (Zed's rendering framework — Apache 2.0) |
| Language | Rust |
| UI components | gpui-component (Apache 2.0 — 12.3k stars) |
| Terminal | alacritty_terminal (MIT) — independent implementation (Zed terminal crates are GPL-3.0) |
| Editor | gpui-component Editor (200K lines, LSP, Tree-sitter) |
| HTTP | Reqwest |
| Filesystem watcher | Notify + Zed fs_watcher.rs pattern (Apache 2.0 lift) |
| Git | git2 |
| Async | Tokio |
| Credentials | keyring / Windows Credential Manager |
| Folder picker | rfd (Rusty File Dialogs) |

**No Tauri. No React. No webview. No Monaco. No xterm.js.**

---

## Zed Codebase — Direct Lifts (Apache 2.0)

From Session 3 inspection. All Apache 2.0, safe to incorporate:

| Zed Artifact | Use In Phase |
|---|---|
| `watch/src/watch.rs` + `error.rs` | Phase 7 agent panel reactive status |
| `credentials_provider/` trait + Windows impl | Phase 2 credential storage |
| `fs/src/fs_watcher.rs` | Phase 7 inbox/outbox watcher |
| `recent_projects/src/ssh_config.rs` | Phase 2 onboarding git step |

Zed `terminal/` and `terminal_view/` are GPL-3.0 — NOT incorporated. See ADR-026.

---

## gpui-component

Core UI library. Do not build custom components where this library covers the need. Initialized with `gpui_component::init(cx)` as first call in `app.run`.

LLM-optimized docs: `https://longbridge.github.io/gpui-component/llms-full.txt`
Local reference: `vault/phase_1/gpui-component-docs.md`

---

## Chat Panel

Two surfaces, one UI:
- Anthropic API — Claude, native HTTP
- Local agent HTTP endpoints — direct interactive chat per agent

Inbox/outbox is a separate delivery mechanism — asynchronous, file-based. Both inbox/outbox and chat can be used by the human or by automated pipelines.

---

## Agent Model

Each agent has two simultaneous interfaces:
- Filesystem inbox/outbox — asynchronous delivery of instructions and documents
- HTTP server — live interactive chat endpoint

Agents are isolated per project room. No cross-project access.

---

## Hardware

- Laptop: Ryzen 9 8940HX, 64GB RAM, RTX 5060 8GB (dedicated to Podium)
- NAS: Docker containers — MemPalace, agent services, pipeline services
- Inference node: DGX Spark x2 (planned) — local agent models

---

## Build Phases

| Phase | Deliverable | Status |
|-------|------------|--------|
| 0 | GPUI shell, gpui-component initialized | COMPLETE ✅ |
| 1 | Dark theme, panel layout, project switcher | NEXT |
| 2 | Project load/unload, onboarding, credentials | — |
| 3 | Terminal (alacritty_terminal, independent build) | — |
| 4 | Git panel | — |
| 5 | Files panel | — |
| 6 | Editor | — |
| 7 | Agent panel — live filesystem watcher | — |
| 8 | Chat panel — Anthropic API + local agent endpoints | — |
| 9 | Knowledge panel | — |
| 10 | Polish | — |

---

## Repository

**Repo:** github.com/JeremySField/podium
**Local path:** C:\Users\jerem\OneDrive\Desktop\Repos\podium
**Knowledgebase codebase:** C:\Users\jerem\OneDrive\Desktop\Claude Knowledgebase\podium\codebase
**Account:** Personal GitHub (public)
**License:** Apache-2.0

## Workflow

1. Claude edits files in knowledgebase codebase
2. Files copied from knowledgebase to repo
3. Committed and pushed per logical change
4. One commit per logical change — methodical versioning
