---
project: Podium
file: adr
type: append-only — decisions never removed, superseded decisions marked
last_updated: 2026-08-06
---

<!-- Podium adr -->

# Podium — Architecture Decision Record

This Podium ADR is the append-only record of all architectural decisions made for the Podium project. Decisions are never removed. Superseded decisions are marked with a reference to the decision that replaced them.

<!-- Podium adr -->

---

## ADR-001 — Tauri over Electron for application shell
**Date:** 2026-08-06
**Status:** SUPERSEDED by ADR-011

**Decision:** Use Tauri as the native desktop application framework instead of Electron.

**Superseded because:** Podium evolved from a dashboard/observability layer to a full native workspace. Tauri uses a webview for the frontend — Monaco inside a webview is JavaScript rendering text. Not appropriate for a workspace where editor and terminal are first-class citizens. See ADR-011.

---

## ADR-002 — Rust backend
**Date:** 2026-08-06
**Status:** Accepted — scope expanded by ADR-011

**Decision:** Use Rust for the backend layer.

**Rationale:** Podium's needs — filesystem watching, shell process management, async I/O, HTTP polling — are exactly the problem domain Rust excels at. No prior Rust experience; will be built iteratively with Claude Code.

---

## ADR-003 — Room model for filesystem structure
**Date:** 2026-08-06
**Status:** Accepted — refined by ADR-014

**Decision:** Adopt a room model for filesystem segregation. Each project is a room. Each agent within a room has its own inbox/outbox. No data crosses room boundaries.

**Refined by ADR-014:** The room model was clarified — Podium is the room, not the projects. Projects load into the room.

---

## ADR-004 — Agent isolation — no agent-to-agent communication
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Agents at the same level have no awareness of each other and never communicate directly. The only communication mechanism is the Primer — assembled by governance and placed in the agent's inbox.

**Rationale:** Eliminates need for a message broker. Agents are stateless workers handed a complete brief. Coordination intelligence lives at the governance layer and with the human director.

**Supersedes:** MQTT communication layer concept from Command_Center_Concept_Rev1_0.md

---

## ADR-005 — No database in Podium
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Podium has no database. Its data is the filesystem, git state, and network APIs.

**Rationale:** Podium reads data that already exists in structured form. A database adds infrastructure overhead for a problem that doesn't exist.

---

## ADR-006 — Podium does not write to external services
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Podium does not write to external services (Supabase, Railway, GitHub). The only write operations permitted are filing reviewed agent outputs within the local filesystem room structure.

**Rationale:** Keeps Podium's responsibility surface minimal and safe. All writes to external services happen through the appropriate tools.

---

## ADR-007 — Zed handles code editing
**Date:** 2026-08-06
**Status:** SUPERSEDED by ADR-011 and ADR-013

**Decision:** Podium opens files in Zed, does not embed an editor.

**Superseded because:** Podium is the workspace itself. Zed is replaced by Podium, not complemented by it. The editor is native inside Podium built on GPUI.

---

## ADR-008 — Folder inbox/outbox over MQTT
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Agent work instruction communication uses a folder inbox/outbox system rather than an MQTT message broker.

**Rationale:** All compute on one workstation with OCuLink to inference node makes network-protocol overhead unjustified. Filesystem is already the interface. Folder watching via Notify achieves the same event-driven behavior without a broker to maintain.

**Supersedes:** MQTT / Mosquitto broker concept from Command_Center_Concept_Rev1_0.md

---

## ADR-009 — xterm.js for embedded terminal
**Date:** 2026-08-06
**Status:** SUPERSEDED by ADR-012

**Decision:** Use xterm.js for the embedded terminal.

**Superseded because:** Podium is built on GPUI, not a webview. xterm.js is a web-based terminal. Zed's native terminal component is the correct foundation. See ADR-012.

---

## ADR-010 — React for frontend framework
**Date:** 2026-08-06
**Status:** SUPERSEDED by ADR-011

**Decision:** Use React as the frontend framework inside Tauri.

**Superseded because:** Podium moved off Tauri entirely to GPUI. React is a web framework and has no place in a pure GPUI application. See ADR-011.

---

## ADR-011 — GPUI as the UI framework — full native stack
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Use GPUI (Zed's rendering framework) as Podium's UI layer. Rust all the way through. No Tauri, no React, no webview.

**Rationale:** Podium evolved from a dashboard to a full native workspace where editor and terminal are first-class citizens. GPUI provides GPU-accelerated native rendering — the same foundation that makes Zed fast. Apache 2.0 licensed. Tauri + React was chosen for a dashboard; it is the wrong stack for a full workspace. The performance characteristics of Zed — which motivated moving off VS Code — require native rendering, not a webview.

**Supersedes:** ADR-001 (Tauri), ADR-010 (React)

---

## ADR-012 — Zed terminal component as Podium's terminal foundation
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Use Zed's open source terminal component as the foundation for Podium's terminal panel.

**Rationale:** Zed's terminal is native, GPU-accelerated, battle-tested, and open source. The performance is proven — directly experienced when switching from VS Code to Zed. Building on what already exists and works is smarter than rebuilding it. Zed is MIT/Apache licensed.

**Supersedes:** ADR-009 (xterm.js)

---

## ADR-013 — Editor is native GPUI, not Monaco
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Podium's editor is built natively on GPUI, referencing Zed's editor architecture. Monaco is not used.

**Rationale:** Monaco is a web-based editor running in a webview. Even inside Tauri it is JavaScript rendering text. GPUI renders natively with GPU acceleration. For a workspace where the editor is a first-class citizen and performance is a core requirement, native rendering is non-negotiable. Zed's editor architecture is open source and serves as the reference implementation.

**Supersedes:** ADR-007 (Zed as external editor)

---

## ADR-014 — Podium is the room — projects load into it
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Podium is a single persistent room. Projects are loaded into the room, not the other way around. One project loaded at a time. All panels recontextualize completely on project load.

**Rationale:** Cleaner architecture than maintaining simultaneous state for multiple projects. Mirrors how a physical workspace functions — the workspace is fixed, the work changes. Claude Desktop implements the same isolation model for AI chat and validates that the pattern works in daily use. Deep focus on one project at a time matches actual working patterns.

**Refines:** ADR-003 (room model)

---

## ADR-015 — Project isolation protocol is an architectural requirement
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Project load and unload are sequential, never concurrent. Hard cleanup on unload — all watchers stopped, all connections closed, all context cleared — before any new project loads. A hard loading state enforced at the application level between projects.

**Rationale:** When one room exists and projects swap in and out, cross-contamination is an architectural risk. Stale context, wrong project attribution, open connections to the previous project's agents — all preventable with strict isolation protocol. Safeguards are baked into the application, not added as optional features.

---

## ADR-016 — Chat panel has two distinct surfaces
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** The chat panel provides two surfaces in one unified UI — Anthropic API chat and local agent HTTP endpoint chat. Both are native HTTP calls from Rust. Chat history is persisted per agent per project, loaded on project load, saved on project unload.

**Rationale:** Podium replaces the claude.ai browser tab for AI chat. It also provides direct interactive access to local agents on the inference node/NAS. Both surfaces belong in the same panel with a unified interface but distinct backends.

---

## ADR-017 — Inbox/outbox and live chat are separate agent interfaces
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Each agent runs two interfaces simultaneously — filesystem inbox/outbox for autonomous work instructions, and an HTTP server for live interactive chat. These are distinct, never mixed.

**Rationale:** Inbox/outbox is for autonomous governance-driven work — Primers dropped in, outputs returned, no human in the loop during execution. Live chat is for direct interactive conversation with a specific agent from within Podium. Conflating them would compromise the autonomous work pipeline and the interactive chat experience. Each serves a different purpose and must remain separate.

---

## ADR-018 — Dedicated GPU for Podium
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** The laptop's RTX 5060 8GB is dedicated entirely to Podium's GPUI rendering. No other workloads compete for this VRAM.

**Rationale:** GPUI uses the GPU for rendering. Dedicated hardware means consistent, fast rendering with no competition from other processes. The inference node (DGX Spark) handles all model inference — the laptop GPU is free to serve Podium exclusively.
