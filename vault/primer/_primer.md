---
project: Podium
file: primer
type: permanent — load at every session start
last_updated: 2026-08-07
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
**Read gpui-component docs before writing any Phase 1 UI code**

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

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| UI framework | GPUI (Zed's rendering framework — Apache 2.0) |
| Language | Rust |
| UI components | gpui-component (Apache 2.0 — 12.3k stars) |
| Terminal | Zed terminal component |
| Editor | gpui-component Editor (200K lines, LSP, Tree-sitter) |
| HTTP | Reqwest |
| Filesystem watcher | Notify |
| Git | git2 |
| Async | Tokio |

**No Tauri. No React. No webview. No Monaco. No xterm.js.**

---

## gpui-component

Core UI library. Covers Phases 1-6. Do not build custom components where this library covers the need. Initialized with `gpui_component::init(cx)` as first call in `app.run`.

LLM-optimized docs: `https://longbridge.github.io/gpui-component/llms-full.txt`

Key components for Podium: Tabs, Resizable, Sidebar, TitleBar, StatusBar, Tree, List, Editor, Input, Scrollable, Badge, Spinner, Notification, TextView, Chart, Settings.

---

## Panels

Editor, Terminal, Git, Agents, Chat, MemPalace, Files — all native, all recontextualize on project load.

---

## Chat Panel

Two surfaces, one UI:
- Anthropic API — Claude, native HTTP
- Local agent HTTP endpoints — direct interactive chat per agent

Inbox/outbox is NOT chat. Inbox/outbox is autonomous work instructions. They are separate interfaces.

---

## Agent Model

Each agent has two simultaneous interfaces:
- Filesystem inbox/outbox — autonomous work instructions
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
| 0 | GPUI shell, gpui-component initialized | COMPLETE |
| 1 | Dark theme, panel layout, project switcher | NEXT |
| 2 | Project load/unload with isolation protocol | — |
| 3 | Git panel | — |
| 4 | Files panel | — |
| 5 | Editor | — |
| 6 | Agent panel — live filesystem watcher | — |
| 7 | Chat panel — Anthropic API + local agent endpoints | — |
| 8 | MemPalace panel | — |
| 9 | Polish | — |

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
