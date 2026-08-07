---
project: Podium
file: primer
type: permanent — load at every session start
last_updated: 2026-08-06
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

**Status:** Concept complete — build not yet started
**Phase:** Pre-Phase 0
**Next step:** Create repo, begin Phase 0 — GPUI shell and project selector

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
| Terminal | Zed terminal component |
| Editor | Native GPUI — Zed architecture reference |
| HTTP | Reqwest |
| Filesystem watcher | Notify |
| Git | git2 |
| Async | Tokio |

**No Tauri. No React. No webview. No Monaco. No xterm.js.**

---

## Panels

Editor, Terminal, Git, Agents, Chat, MemPalace, Files — all native, all recontextualize on project load.

## Chat Panel

Two surfaces, one UI:
- Anthropic API — Claude, native HTTP
- Local agent HTTP endpoints — direct interactive chat per agent

Inbox/outbox is NOT chat. Inbox/outbox is autonomous work instructions. They are separate interfaces.

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

| Phase | Deliverable |
|-------|------------|
| 0 | GPUI shell, project selector, panel layout |
| 1 | Terminal — Zed component integrated |
| 2 | Project load/unload with isolation protocol |
| 3 | Git panel |
| 4 | Files panel |
| 5 | Editor |
| 6 | Agent panel — live filesystem watcher |
| 7 | Chat panel — Anthropic API + local agent endpoints |
| 8 | MemPalace panel |
| 9 | Polish |

---

## Repository

**Repo:** Not yet created
**Account:** Personal GitHub (public)
**Knowledgebase:** `dashboard/` folder
**Archive:** `dashboard/archive/` — original ShowFlyer Homepage dashboard
