---
project: Podium
file: concept
type: spec — authoritative, rewrite supersedes all prior versions
last_updated: 2026-08-06
status: ACTIVE — concept phase, pre-build
supersedes: _concept.md Rev 1.0 (2026-08-06), Command_Center_Concept_Rev1_0.md
---

<!-- Podium concept -->

# Podium — Command Center Concept
## Rev 2.0 | August 2026

<!-- Podium concept -->

---

## 1. What Podium Is

Podium is a purpose-built, GPU-accelerated native desktop workspace for a solo builder running multiple complex projects simultaneously. It is not a dashboard, not an observability layer, and not a wrapper around other tools.

**Podium is the workspace itself.**

Editor, terminal, git insight, agent chat, MemPalace access — all native, all in one application, running on dedicated GPU hardware. One app. One window. Everything in it.

**The one sentence version:** A native Rust workspace where one project loads completely into a persistent room, all tools recontextualize around it, and nothing bleeds across projects under any circumstances.

**What Podium replaces:**
- Zed (code editor + terminal)
- Claude.ai browser tab (agent chat)
- The ShowFlyer Homepage dashboard (project visibility)
- Any other tool currently filling gaps in the workflow

**What Podium is not:**
- An operating system
- A project management tool
- A communication platform
- An observability layer that opens other apps
- A multi-project simultaneous view

---

## 2. The Room Model

Podium is the room. Fixed. Permanent. Always the same space.

What changes is what is loaded into it. A project gets loaded to the room the same way any other utility does — the room itself never changes, the context does.

This maps to how a physical workspace functions — the desk, monitors, and tools are always there. You bring the project to the workspace, not the other way around. When work on a project is complete, you swap the project out. The room stays. The context changes.

**Reference pattern:** Claude Desktop implements this model for AI chat — each project/chat is an isolated sandbox with its own context, artifacts, and history loaded into the same interface. Podium applies the same isolation principle to a full development workspace.

**One project loaded at a time.** This is not a limitation — it is the design. Deep focus on one project. Intentional context switch when needed.

---

## 3. Project Isolation Protocol

The room model requires absolute isolation between projects. When only one room exists and projects are swapped in and out, cross-contamination is an architectural risk that must be eliminated at the application level.

**On project unload — hard cleanup (sequential, verified):**
- All filesystem watchers explicitly stopped and dereferenced
- All agent chat connections closed
- Terminal session ended
- Chat history scoped and persisted to the project before unload
- Git context cleared
- Editor buffers closed
- No project loads until unload is confirmed complete

**On project load — clean state:**
- Fresh filesystem watchers initialized for new project paths
- New agent connections opened only for new project's agents
- Chat history loaded only from new project's scope
- Terminal scoped to new project directory
- Git context initialized for new project
- Editor opens new project files

**The critical rule:** Project load and unload are sequential, never concurrent. A hard loading state is enforced at the application level between projects. No overlap permitted under any circumstances.

**Safeguards are architectural requirements, not optional features.** They are baked into how the room loads and unloads, not added on top.

---

## 4. Panels — What Lives in the Room

Every panel is always present in the room. Each panel recontextualizes completely when a project loads.

| Panel | What It Does |
|-------|-------------|
| **Editor** | Full code editor — native, GPU-accelerated, built on GPUI |
| **Terminal** | Native terminal built on Zed's terminal component — same performance characteristics |
| **Git** | Commit history, branch status, diff view — scoped to loaded project |
| **Agents** | Agent status cards for the loaded project's agents — idle, has work, output pending review |
| **Chat** | Unified chat panel — Anthropic API and local agent chat endpoints |
| **MemPalace** | Wing stats, drawer counts, sync status for the loaded project's wing |
| **Files** | Project file tree and structure |

Panel layout is configurable. All panels serve the loaded project exclusively.

---

## 5. The Chat Panel

The chat panel is a first-class citizen, not a peripheral feature. It provides two distinct chat surfaces in one unified interface:

### Anthropic API Chat
Direct conversational access to Claude via the Anthropic API. Native HTTP call from Rust — no browser, no claude.ai tab, no context switching to another application. Strategy, architecture, reasoning, decisions — all from within Podium.

### Local Agent Chat
Direct, interactive, conversational access to each agent running on the inference node/NAS. Each agent exposes a live HTTP chat endpoint alongside its inbox/outbox. The chat panel opens a direct line to a specific agent — you select the agent, conversation is real-time and persistent within the project scope.

**Critical distinction:**
- **Inbox/outbox** — work instructions, autonomous, asynchronous. Governance drops instructions in, agent processes, output goes to outbox. Not conversational. Not accessed through the chat panel.
- **Live chat endpoint** — interactive, conversational, real-time. Direct line to a specific agent. This is what the chat panel connects to.

Each agent runs as a persistent service with both interfaces simultaneously:
```
Agent Service
├── Filesystem watcher → inbox/outbox (autonomous work instructions)
└── HTTP server → chat endpoint (interactive conversation)
```

**Chat panel routing:**
- Anthropic API → Claude (frontier reasoning, strategy)
- Agent selector → specific local agent's HTTP endpoint
- Chat history persisted per agent per project, loaded on project load, scoped and saved on project unload

---

## 6. Agent Model

Each agent is an isolated worker with a clearly defined responsibility. Agents at the same level have no awareness of each other and never communicate directly.

**Agent interfaces:**
- Inbox/outbox for autonomous work instructions (filesystem-based)
- HTTP chat endpoint for direct interactive access from Podium

**Agent filesystem structure within a project:**
```
/[project-room]/
  /agents/
    /[agent-name]/
      /inbox/
      /outbox/
      /working/
```

Agents are isolated per project room. An agent assigned to ShowFlyer has no access to HousesUnder150K's room. Filesystem segregation is absolute.

**Podium's agent panel shows per agent:**
- Name and role
- Current status (Idle / Has Work / Output Pending Review)
- Inbox item count
- Outbox item count
- Last activity timestamp

Clicking an agent card opens the chat panel connected to that agent's live endpoint.

---

## 7. Tech Stack

The stack changed significantly from Rev 1.0. The original Tauri + React stack was chosen for a dashboard / observability layer. Podium is a full native workspace. The stack reflects that.

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| **UI framework** | GPUI (Zed's rendering framework) | GPU-accelerated, native, Apache 2.0 licensed — same foundation as Zed |
| **Language** | Rust | Required by GPUI; right tool for filesystem, process, async I/O |
| **Terminal** | Zed terminal component | Battle-tested, native performance, open source — proven by Zed itself |
| **Editor** | GPUI-based, Zed architecture reference | Native GPU-accelerated editor, not web-based Monaco |
| **HTTP client** | Reqwest | Anthropic API calls, local agent endpoint calls, external service polling |
| **Filesystem watcher** | Notify | Watches agent inbox/outbox folders, triggers agent card updates |
| **Git** | git2 (Rust crate) | Native git integration for the git panel |
| **Async runtime** | Tokio | Async backend operations |

**Why not Tauri + React:**
Tauri uses a webview for the frontend. Monaco inside a webview is JavaScript rendering text — capable, but not native. GPUI renders everything natively with GPU acceleration. For a workspace where the editor and terminal are first-class citizens, native rendering is the right choice. Tauri was right for a dashboard. It is not right for a full workspace.

**Why not Electron:**
Same reason Zed over VS Code — Electron bundles a full browser. The memory overhead is unacceptable for a workspace that should be light and fast.

**Dedicated GPU:**
The laptop's RTX 5060 8GB is dedicated entirely to Podium. No competition for VRAM. GPUI uses the GPU for rendering — dedicated hardware means consistent, fast rendering at all times.

---

## 8. Hardware Context

**Current hardware (build starts here):**
- Laptop: Ryzen 9 8940HX, 64GB RAM, RTX 5060 8GB
- RTX 5060 dedicated entirely to Podium
- NAS: Docker containers for MemPalace, agent services, pipeline services
- Inference node: DGX Spark (x2 planned) — 256GB unified memory total — hosts local agent models

**How Podium connects to the stack:**
- **NAS** → MemPalace API (HTTP), agent filesystem rooms (network mount or HTTP), pipeline service health
- **Inference node** → local agent HTTP chat endpoints
- **GitHub** → git2 crate, SSH per-project account
- **Anthropic API** → Reqwest, direct HTTP

Podium runs on the laptop. Everything else is network-accessible infrastructure. The laptop GPU handles Podium's rendering exclusively.

---

## 9. Build Approach

Built iteratively in sessions with Claude. No speculative features. Each phase produces something working before the next begins. Zero Rust experience at project start — Claude Code handles implementation, direction comes from the human.

**Phase 0 — GPUI Shell**
GPUI window opens. Project selector renders. Panel layout visible. No real data. Goal: confirm GPUI is set up correctly and the visual model feels right before wiring anything.

**Phase 1 — Terminal**
Zed terminal component integrated. Terminal opens scoped to a hardcoded project path. Goal: confirm the terminal works natively inside GPUI.

**Phase 2 — Project Load/Unload**
Project selector functional. Load and unload cycle implemented with hard cleanup. Isolation protocol enforced. Terminal rescopes on project switch.

**Phase 3 — Git Panel**
git2 integrated. Commit history, branch, status visible for loaded project.

**Phase 4 — Files Panel**
Project file tree. Open file in editor.

**Phase 5 — Editor**
GPUI-based editor. Syntax highlighting. Basic editing. File save.

**Phase 6 — Agent Panel**
Filesystem watcher live. Agent cards update in real time from inbox/outbox contents.

**Phase 7 — Chat Panel**
Anthropic API chat working natively. Local agent HTTP endpoint chat working. History persisted per agent per project.

**Phase 8 — MemPalace Panel**
MemPalace API queried. Wing stats, sync status displayed.

**Phase 9 — Polish**
Performance, visual design, edge cases, startup behavior, project isolation stress testing.

---

## 10. Locked Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | GPUI over Tauri + React | Native GPU-accelerated rendering — right stack for a full workspace, not a dashboard |
| 2 | Rust all the way through | Required by GPUI; right tool for filesystem, process, async I/O |
| 3 | Zed terminal component | Battle-tested native terminal — don't rebuild what already exists and works |
| 4 | One project loaded at a time | Deep focus model — intentional context switch, not ambient multi-project juggling |
| 5 | Podium is the room | The workspace is fixed; the project context loads into it — not the other way around |
| 6 | Project isolation is architectural | Load/unload safeguards are requirements baked into the application, not optional features |
| 7 | Sequential load/unload — no overlap | Hard loading state between projects enforced at application level |
| 8 | Chat panel has two surfaces | Anthropic API and local agent HTTP endpoints — unified UI, distinct backends |
| 9 | Inbox/outbox is not chat | Autonomous work instructions are filesystem-based and separate from the live chat endpoint |
| 10 | Each agent has both interfaces | Filesystem inbox/outbox for autonomous work + HTTP endpoint for interactive chat — simultaneously |
| 11 | Dedicated GPU | RTX 5060 8GB dedicated entirely to Podium — no VRAM competition |
| 12 | No database in Podium | Podium's data is the filesystem and network APIs — no local database needed |
| 13 | Build iteratively | Phase 0 working before Phase 1 begins — nothing speculative |

---

## 11. What This Is Not Yet

Out of scope for initial build, revisited later:

- Historian / audit trail — separate future service
- Remote access — local only initially
- Multi-user — single user, single machine
- Mobile — desktop only
- Broadcast mode to multiple agents simultaneously
- Night session synthesis integration

---

*Rev 2.0 — 2026-08-06*
*Supersedes Rev 1.0 (2026-08-06) and Command_Center_Concept_Rev1_0.md*
*Living document — will evolve as build progresses*
