# Podium

A GPU-accelerated native desktop workspace for developers and builders running multiple complex projects simultaneously.

Podium is built on [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) — the same rendering framework that powers [Zed](https://zed.dev). It is fast, native, and purpose-built for deep work.

---

## What Podium Is

Podium is a single workspace that replaces the collection of tools typically needed to manage active development work — code editor, terminal, git visibility, AI chat, agent management, knowledge base access, and project health monitoring — in one native application.

**One window. One project loaded at a time. Everything in it.**

Podium is not a dashboard that sits on top of other tools. It is the workspace itself.

---

## Core Concepts

### The Room Model
Podium is the room. It is fixed and persistent. Projects load into the room — the workspace doesn't change, the context does. Switching projects triggers a complete, clean context swap. Nothing from the previous project bleeds through.

### Project Isolation
Every project is fully isolated. Agent inboxes, outboxes, chat history, git context, knowledge base connections, and service credentials are all scoped to the loaded project. Load and unload are sequential — never concurrent.

### Agent Workspace
Podium is designed for AI-assisted workflows. Each project has a user-defined roster of agents, each configured with a model provider, knowledge base sources, and a purpose.

Each agent has two interfaces:
- **Live chat** — real-time, interactive conversation with the agent directly from Podium
- **Inbox/outbox** — asynchronous, file-based delivery of instructions, documents, and work packages

These are separate delivery mechanisms. Chat is real-time and interactive. Inbox/outbox is asynchronous and file-based — instructions and documents are dropped in and picked up when the agent runs. Both can be used by the human, by governance agents, or by automated pipelines. The separation is about how work is delivered, not who delivers it.

### Knowledge Base
Each project connects to a library of knowledge base sources. Each agent is explicitly assigned which sources it can access — isolating agent context precisely. Supported providers include MemPalace, Obsidian, Notion, and any custom HTTP endpoint.

---

## Panels

| Panel | Purpose |
|-------|---------|
| **Files** | Project file tree, git status, commit history |
| **Agents** | Agent cards — status, inbox/outbox counts, live chat |
| **Knowledge** | Connected KB sources, agent assignments, sync status |
| **Review** | Agent outputs awaiting human review |
| **Terminal** | Native terminal scoped to the loaded project |
| **Health** | External service status — Supabase, Railway, GitHub, and more |

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| UI Framework | [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) — GPU-accelerated native rendering |
| Language | Rust |
| UI Components | [gpui-component](https://github.com/longbridge/gpui-component) |
| Terminal | alacritty_terminal (MIT) — independent implementation |
| Editor | Native GPUI editor |
| Async | Tokio |
| Git | git2 |
| Filesystem | Notify |
| HTTP | Reqwest |
| Credentials | Windows Credential Manager (keyring) |
| Folder picker | rfd (Rusty File Dialogs) |

---

## Agent Providers

Podium supports any combination of model providers per agent:

- Anthropic (Claude)
- OpenAI (GPT)
- Google (Gemini)
- xAI (Grok)
- Any OpenAI-compatible local endpoint (Ollama, LM Studio, custom inference nodes)

---

## Service Integrations

**Beta:** Supabase, Railway, GitHub, Custom

**v1.0:** Vercel, Netlify, Fly.io, Render, AWS, Digital Ocean, PlanetScale, Neon, MongoDB Atlas, Firebase, Datadog, Sentry, Uptime Robot, Better Uptime, CircleCI, Jenkins, Slack, Discord

---

## Status

Podium is in active development. Phase 1 complete — dark theme, panel layout, dock system, and application shell are working.

| Phase | Deliverable | Status |
|-------|------------|--------|
| 0 | GPUI shell, gpui-component initialized | ✅ Complete |
| 1 | Dark theme, panel layout, project switcher | ✅ Complete |
| 2 | Project load/unload, onboarding, credentials | 🔄 Next |
| 3 | Terminal — alacritty_terminal | — |
| 4 | Git panel | — |
| 5 | Files panel | — |
| 6 | Editor | — |
| 7 | Agent panel — live filesystem watcher | — |
| 8 | Chat panel — Anthropic API + local agent endpoints | — |
| 9 | Knowledge panel | — |
| 10 | Polish | — |

---

## License

Apache-2.0 — see [LICENSE](LICENSE)
