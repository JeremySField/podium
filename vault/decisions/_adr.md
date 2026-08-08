---
project: Podium
file: adr
type: append-only — decisions never removed, superseded decisions marked
last_updated: 2026-08-08
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

**Superseded because:** Podium evolved from a dashboard/observability layer to a full native workspace. Tauri uses a webview for the frontend. Not appropriate for a workspace where editor and terminal are first-class citizens. See ADR-011.

---

## ADR-002 — Rust backend
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Use Rust for the backend layer.

**Rationale:** Podium's needs — filesystem watching, shell process management, async I/O, HTTP polling — are exactly the problem domain Rust excels at.

---

## ADR-003 — Room model for filesystem structure
**Date:** 2026-08-06
**Status:** Accepted — refined by ADR-014

**Decision:** Adopt a room model for filesystem segregation. Each project is a room. Each agent within a room has its own inbox/outbox. No data crosses room boundaries.

**Refined by ADR-014:** Podium is the room, not the projects. Projects load into the room.

---

## ADR-004 — Agent isolation — no agent-to-agent communication
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Agents at the same level have no awareness of each other and never communicate directly. The only communication mechanism is the Primer — assembled by governance and placed in the agent's inbox.

**Supersedes:** MQTT communication layer concept from Command_Center_Concept_Rev1_0.md

---

## ADR-005 — No database in Podium
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Podium has no database. Its data is the filesystem, git state, and network APIs.

---

## ADR-006 — Podium does not write to external services
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Podium does not write to external services. The only write operations permitted are filing reviewed agent outputs within the local filesystem room structure.

---

## ADR-007 — Zed handles code editing
**Date:** 2026-08-06
**Status:** SUPERSEDED by ADR-011 and ADR-013

**Decision:** Podium opens files in Zed, does not embed an editor.

**Superseded because:** Podium is the workspace itself. Zed is replaced by Podium. See ADR-011 and ADR-013.

---

## ADR-008 — Folder inbox/outbox over MQTT
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Agent work instruction communication uses a folder inbox/outbox system rather than an MQTT message broker.

**Supersedes:** MQTT / Mosquitto broker concept from Command_Center_Concept_Rev1_0.md

---

## ADR-009 — xterm.js for embedded terminal
**Date:** 2026-08-06
**Status:** SUPERSEDED by ADR-012, then ADR-026

**Decision:** Use xterm.js for the embedded terminal.

**Superseded because:** Podium moved to GPUI. See ADR-012, then ADR-026.

---

## ADR-010 — React for frontend framework
**Date:** 2026-08-06
**Status:** SUPERSEDED by ADR-011

**Decision:** Use React as the frontend framework inside Tauri.

**Superseded because:** Podium moved off Tauri entirely to GPUI. See ADR-011.

---

## ADR-011 — GPUI as the UI framework — full native stack
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Use GPUI (Zed's rendering framework) as Podium's UI layer. Rust all the way through. No Tauri, no React, no webview.

**Rationale:** GPUI provides GPU-accelerated native rendering — the same foundation that makes Zed fast. Apache 2.0 licensed.

**Supersedes:** ADR-001 (Tauri), ADR-010 (React)

---

## ADR-012 — Zed terminal component as Podium's terminal foundation
**Date:** 2026-08-06
**Status:** SUPERSEDED by ADR-026

**Decision:** Use Zed's open source terminal component as the foundation for Podium's terminal panel.

**Superseded because:** Zed's `terminal/` and `terminal_view/` crates are GPL-3.0-or-later. Podium is Apache-2.0 and intended for public GitHub distribution. Incorporating GPL code would force Podium to relicense under GPL-3.0, breaking the Apache-2.0 commitment. See ADR-026.

---

## ADR-013 — Editor is native GPUI, not Monaco
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Podium's editor is built natively on GPUI, referencing Zed's editor architecture. Monaco is not used.

**Rationale:** Monaco is web-based. GPUI renders natively with GPU acceleration. gpui-component Editor is the foundation.

**Supersedes:** ADR-007 (Zed as external editor)

---

## ADR-014 — Podium is the room — projects load into it
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Podium is a single persistent room. Projects load into the room. One project loaded at a time. All panels recontextualize completely on project load.

**Refines:** ADR-003

---

## ADR-015 — Project isolation protocol is an architectural requirement
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** Project load and unload are sequential, never concurrent. Hard cleanup on unload before any new project loads. Loading state enforced at application level.

---

## ADR-016 — Chat panel has two distinct surfaces
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** The chat panel provides two surfaces — Anthropic API chat and local agent HTTP endpoint chat. Both are native HTTP calls from Rust. Chat history persisted per agent per project.

---

## ADR-017 — Inbox/outbox and live chat are separate delivery mechanisms
**Date:** 2026-08-06
**Status:** Accepted — clarified 2026-08-07

**Decision:** Each agent has two interfaces — filesystem inbox/outbox (asynchronous, file-based) and HTTP chat endpoint (real-time, conversational). Separate delivery mechanisms, never mixed.

**Clarification:** Both can be used by the human, by governance agents, or by automated pipelines. The distinction is how work is delivered, not who delivers it.

---

## ADR-018 — Dedicated GPU for Podium
**Date:** 2026-08-06
**Status:** Accepted

**Decision:** The laptop's RTX 5060 8GB is dedicated entirely to Podium's GPUI rendering.

---

## ADR-019 — Sheet over Dialog for onboarding flow
**Date:** 2026-08-07
**Status:** Accepted

**Decision:** The project onboarding flow uses a Sheet (slide-in panel) rather than a Dialog (modal overlay).

**Rationale:** Sheet keeps Podium visible behind the onboarding flow. More approachable for non-developer users. Supports progressive disclosure.

---

## ADR-020 — Progressive disclosure as the core UX design principle
**Date:** 2026-08-07
**Status:** Accepted

**Decision:** Speed is the default. Clarity is available on demand. Every configuration surface implements progressive disclosure.

**Pattern:** Field label + input (always visible) / one-line hint (always visible) / expandable "?" (collapsed by default).

**Applies to:** All onboarding steps, all settings panels, all configuration surfaces.

---

## ADR-021 — Agent roster is fully user-defined
**Date:** 2026-08-07
**Status:** Accepted

**Decision:** No fixed agent roster. Users define agents from scratch per project — name, purpose, avatar, model provider, KB sources. Any provider supported.

---

## ADR-022 — API keys stored globally per provider in Windows Credential Manager
**Date:** 2026-08-07
**Status:** Accepted

**Decision:** Provider API keys stored once globally per provider in Windows Credential Manager. Shared across all agents using that provider. Never in projects.toml. Service credentials per-project scoped to project ID.

---

## ADR-023 — KB sources are global to Podium, connected per project, assigned per agent
**Date:** 2026-08-07
**Status:** Accepted

**Decision:** KB sources are a Podium-level global resource in `kb_sources.toml`. Configured once in Podium Settings. Projects select which sources to connect. Agents select which of the project's connected sources they can access.

**Three levels:** Podium Settings (library) → Project Settings (connected sources) → Agent config (assigned sources)

---

## ADR-024 — Settings accessed via application menu, not a tab
**Date:** 2026-08-07
**Status:** Accepted

**Decision:** Settings accessed via top-level application menu — same pattern as Zed. Tab bar: Files | Agents | Knowledge | Review | Terminal | Health.

---

## ADR-025 — All service credentials are per-project
**Date:** 2026-08-07
**Status:** Accepted

**Decision:** All external service credentials are per-project. Stored in Windows Credential Manager scoped to project ID. Applies universally to all services.

---

## ADR-026 — Terminal implemented using alacritty_terminal, not Zed terminal crates
**Date:** 2026-08-08
**Status:** Accepted

**Decision:** Podium's terminal panel is built using the `alacritty_terminal` crate (MIT licensed) directly, with Zed's `terminal_view/terminal_element.rs` used as structural reference only — not copied. Zed's `terminal/` and `terminal_view/` crates are not incorporated into Podium.

**Rationale:** Zed's `terminal/` and `terminal_view/` crates are GPL-3.0-or-later. Podium is Apache-2.0 and intended for public release on GitHub. GPL-3.0 is copyleft — incorporating it into Podium would require relicensing the entire Podium codebase under GPL-3.0, breaking the Apache-2.0 commitment to the community and constraining future users of Podium's code. The `alacritty_terminal` crate provides the same PTY and terminal emulation capability under a compatible license. Zed's terminal_element.rs is used as architectural reference to understand the GPUI rendering pattern — no code is copied.

**Impact on build phases:** Terminal moves to Phase 3 minimum. It is not a quick lift — it requires an independent implementation. Phases 1 and 2 are unaffected.

**Supersedes:** ADR-012 (Zed terminal component)

---

## ADR-027 — Dock owns panel position persistence, panels are storage-agnostic
**Date:** 2026-08-08
**Status:** Accepted

**Decision:** Panel position persistence is owned by the dock layer, not by individual panels. The dock maintains a record of each panel's current position (left, bottom, right) and persists it to `projects.toml` or a Podium-level config. On load, the dock reads the persisted positions and calls `set_position()` on each panel. Panels respond to `set_position()` but have no knowledge of config file format or storage mechanism.

**Rationale:** Panels should not need to know how or where their position is stored. Centralizing persistence in the dock keeps the `PodiumPanel` trait storage-agnostic and makes the config structure predictable — one place manages all panel layout state. This is consistent with the Zed pattern where `SettingsStore` drives position changes and the dock responds, rather than panels reaching into storage themselves.

**Applies to:** `PodiumPanel` trait design and dock implementation (Phase 1).
