---
project: Podium
file: project_onboarding_spec
type: spec — all questions resolved, approved for build
last_updated: 2026-08-07
status: APPROVED — all decisions locked, ready for build
---

<!-- Podium project_onboarding_spec -->

# Podium — Project Onboarding & Project Switcher Spec
## Rev 1.1 — APPROVED

All decisions locked. This document is the authoritative reference for building the project switcher, onboarding flow, and related systems.

<!-- Podium project_onboarding_spec -->

---

## 1. What This Covers

- The project switcher UI in the TitleBar
- The "Add New Project" onboarding flow
- The `projects.toml` and `kb_sources.toml` data schemas
- The room filesystem structure created during onboarding
- Agent configuration model
- Knowledge base connector model
- Services configuration
- Credential storage

---

## 2. The Project Switcher

### 2.1 Location
TitleBar. Always visible regardless of active tab or panel state.

### 2.2 Component
Combobox — searchable dropdown. Type to filter projects. Matches Zed's project switcher pattern. Fast, keyboard-friendly.

### 2.3 Switcher Behavior
- Shows currently active project name
- Click opens searchable project list
- Search filters project list in real time
- Active project shown with checkmark
- Projects ordered by most recently used first
- Text only — no color indicators, no icons
- No project status indicators — clean and minimal
- "Add New Project" at bottom — opens onboarding Sheet
- "Project Settings" at bottom — opens active project settings via application menu

### 2.4 Application Menu
Top-level application menu — same pattern as Zed:
- Podium Settings (global — KB sources, provider API keys, theme, keymap)
- Project Settings (active project)
- Quit

### 2.5 Tab Bar
**Files | Agents | Knowledge | Review | Terminal | Health**

---

## 3. Knowledge Base Sources — Global Library

### 3.1 KB Sources Are Global to Podium
KB sources are a Podium-level resource, not scoped to any project. They are configured once in Podium Settings and are available to all projects. Managing the library — adding, editing, removing sources, rotating tokens — happens in Podium Settings, not in project configuration.

### 3.2 KB Source Usage
- **Podium Settings** — add, edit, remove KB sources from the global library
- **Project Settings** — select which KB sources this project uses (from the global library)
- **Agent configuration** — select which of the project's connected sources each agent can access

### 3.3 Knowledge Tab in Podium
Shows the KB sources connected to the loaded project and their agent assignments:
- Source name and provider
- Which agents are connected to which sources
- Sync status / last query per source
- Link to manage sources in Podium Settings

### 3.4 kb_sources.toml Schema
KB sources stored separately from projects — global Podium config:

```toml
# %APPDATA%\podium\kb_sources.toml

[[sources]]
id = "main-mempalace"
name = "Main MemPalace"
provider = "mempalace"
endpoint = "http://nas-ip:port"
# bearer token in Windows Credential Manager, scoped to source ID

[[sources]]
id = "project-obsidian"
name = "Project Obsidian Vault"
provider = "obsidian"
endpoint = "http://localhost:27123"
# token in Windows Credential Manager, scoped to source ID

[[sources]]
id = "notion-docs"
name = "Notion Documentation"
provider = "notion"
# API key in Windows Credential Manager, scoped to source ID
```

### 3.5 Supported KB Providers

**Beta:**
| Provider | Notes |
|----------|-------|
| MemPalace | Endpoint + bearer token |
| Obsidian | Via Local REST API plugin |
| Notion | API key + database ID |
| Custom | Any HTTP query endpoint |

**v1.0 — based on community demand from beta.**

### 3.6 Per KB Source Fields
- **Source name** — user defined
- **Provider** — dropdown
- **Endpoint URL**
- **Wing / namespace** — provider-specific (e.g. MemPalace wing — configured per project connection, not in the global source)
- **Auth token** — Windows Credential Manager, scoped to source ID
- **Read-only** — always enforced, not configurable

---

## 4. The Project Model

### 4.1 What a Project Is
A project is a registered workspace room with:
- Name and display identity
- Root filesystem path
- Room folder structure (`.podium/` inside project root)
- User-defined agent roster with per-agent model source and KB source assignments
- KB sources connected to this project (selected from global library)
- Service connections (per-project)
- Git configuration (per-project)

### 4.2 projects.toml Location
`%APPDATA%\podium\projects.toml` on Windows.

### 4.3 projects.toml Schema

```toml
[[projects]]
id = "showflyer"
name = "ShowFlyer"
path = "C:/Users/jerem/OneDrive/Desktop/Repos/showflyer"
last_opened = "2026-08-07T20:00:00Z"

[projects.git]
account = "showflyer-github"
remote = "git@github-showflyer:username/showflyer.git"

# KB sources connected to this project — references global source IDs
# wing/namespace config is per-project-connection since the same source
# may serve different wings for different projects
[[projects.kb_connections]]
source_id = "main-mempalace"
wing = "showflyer"

[[projects.kb_connections]]
source_id = "project-obsidian"

[[projects.agents]]
id = "research-agent"
name = "Research Agent"
purpose = "Finds and synthesizes external sources"
avatar = "path/to/avatar.png"     # optional
provider = "anthropic"
model = "claude-sonnet-4-6"
kb_sources = ["main-mempalace"]   # references global source IDs

[[projects.agents]]
id = "doc-agent"
name = "Doc Agent"
purpose = "Maintains documentation and session logs"
provider = "custom"
endpoint = "http://inference-node:8080"
model = "hermes-14b"
kb_sources = ["main-mempalace", "project-obsidian"]

[[projects.services]]
type = "supabase"
name = "ShowFlyer DB"
url = "https://xxx.supabase.co"
# anon key in Windows Credential Manager, scoped to project ID

[[projects.services]]
type = "railway"
name = "ShowFlyer Pipeline"
project_id = "xxx"
# API token in Windows Credential Manager, scoped to project ID
```

### 4.4 Project Name Rules
Letters, numbers, spaces, hyphens, underscores only. If it can be a folder name, it can be a project name. Special characters disallowed: `/ \ : * ? " < > | . @`

---

## 5. Room Folder Structure

`.podium/` lives inside the project root. Auto-added to project `.gitignore` on creation.

```
[project-root]/
  .podium/
    agents/
      [agent-id]/
        inbox/
        outbox/
        working/
      archive/          ← deleted agents archived here, never permanently deleted
    review/             ← cross-agent review queue for this project
    kb/                 ← local KB cache if needed
```

---

## 6. Onboarding Flow

### 6.1 Entry Points
- "Add New Project" in project switcher Combobox
- Application menu

### 6.2 First Launch Empty State
Empty state screen — no auto-open of onboarding:
- Centered text: "No projects yet. Add your first project."
- Single "Add New Project" button
- Nothing else

### 6.3 Flow UI Container
Sheet — slides in from the side. Podium visible behind it. Per ADR-019.

### 6.4 Step Navigation
- Linear step flow
- Back button on all steps except Step 1
- Cancel on all steps — discards all input, no project created
- Progress indicator shows current step and total
- Folder and name required. All other steps skippable.
- New project loads automatically after creation.

### 6.5 Progressive Disclosure — Per ADR-020
Every field has:
- Field label + input (always visible)
- One-line hint beneath (always visible, subtle)
- Expandable "?" for fuller explanation (collapsed by default)

Fast path is always the default. Explanation always one tap away.

---

## 7. Step 1 — Folder Selection

- Native OS folder picker via `rfd` crate (Rusty File Dialog)
- Folder name pre-fills project name in Step 2
- Detects `.git` directory — pre-fills remote URL from `.git/config`
- Detects existing `.podium/` folder — prompts user: **Skip room creation** or **Overwrite**

---

## 8. Step 2 — Project Identity

**Fields:**
- Project name — pre-filled from folder name, editable, required
- No color picker, no project icon

**Validation:**
- Unique across registered projects
- Cannot be empty
- Characters: letters, numbers, spaces, hyphens, underscores only

---

## 9. Step 3 — Git Configuration (Skippable)

**Fields:**
- Git account — SSH config alias, populated by parsing `~/.ssh/config` Host entries
- Remote URL — pre-filled from `.git/config` if detected, editable

**No SSH config detected:**
- Remote URL field shown pre-filled if available
- One-line hint explains SSH accounts with link to setup docs
- User can proceed with HTTPS or set up SSH and return
- No blocking, no error

**Skip behavior:** Git panel shows "No git account configured" with option to configure from project settings.

---

## 10. Step 4 — Agent Configuration (Skippable)

### 10.1 Agent Roster
Fully user-defined per project. No fixed roster. Users create agents from scratch.

### 10.2 Per Agent Fields
- **Name** — user defined
- **Purpose** — free text, shown on agent card
- **Avatar** — optional image upload
- **Provider** — Anthropic, OpenAI, Google, xAI, Custom/Local
- **Model** — dropdown by provider, or free text for Custom/Local
- **Endpoint URL** — Custom/Local only
- **Knowledge Base Sources** — multi-select from KB sources connected to this project

### 10.3 Supported Providers
| Provider | Notes |
|----------|-------|
| Anthropic | Claude — Haiku, Sonnet, Opus |
| OpenAI | GPT models |
| Google | Gemini models |
| xAI | Grok models |
| Custom / Local | Any OpenAI-compatible endpoint |

### 10.4 API Key Storage
Stored globally per provider in Windows Credential Manager. Entered once per provider. Never in projects.toml.

### 10.5 Agent Folder Creation
On agent creation: `.podium/agents/[agent-id]/inbox/`, `outbox/`, `working/` created.
On agent deletion: moved to `.podium/agents/archive/[agent-id]/`. Never permanently deleted.

**Skip behavior:** Agents tab shows "No agents configured" with "Add Agent" button.

---

## 11. Step 5 — Knowledge Base Sources (Skippable)

### 11.1 What Happens Here
The user selects which KB sources from the global library to connect to this project. This is a selection step, not a creation step. KB sources are created and managed in Podium Settings.

If no KB sources have been configured yet in Podium Settings, this step shows a prompt: "No knowledge base sources configured yet. Add sources in Podium Settings."

### 11.2 Fields
- Multi-select from global KB source library
- For MemPalace sources — wing/namespace field (project-specific, since the same MemPalace instance may serve different wings for different projects)

**Skip behavior:** Knowledge tab shows "No KB sources connected" with link to Podium Settings.

---

## 12. Step 6 — Services Configuration (Skippable)

### 12.1 Purpose
Powers the Health tab — external APIs Podium polls for project health status.

### 12.2 All service credentials are per-project, stored in Windows Credential Manager scoped to project ID.

### 12.3 Supported Services

**Beta:**
| Service | Fields |
|---------|--------|
| Supabase | Project URL + anon key |
| Railway | Project ID + API token |
| GitHub | Repo owner/name, uses SSH config |
| Custom | Name + polling URL + expected response |

**v1.0:**
Infrastructure/Deploy: Vercel, Netlify, Fly.io, Render, AWS, Digital Ocean
Database: PlanetScale, Neon, MongoDB Atlas, Firebase
Monitoring: Datadog, Sentry, Uptime Robot, Better Uptime
CI/CD: CircleCI, Jenkins (GitHub Actions via GitHub)
Notifications: Slack webhook, Discord webhook

**Skip behavior:** Health tab shows "No services configured" with "Add Service" button.

---

## 13. Step 7 — Confirm & Create

### 13.1 Summary Screen
- Project name and folder path
- Git account and remote (if configured)
- Agents list with providers (if configured)
- KB sources connected (if configured)
- Services (if configured)

### 13.2 Creation Actions
1. Write project entry to `%APPDATA%\podium\projects.toml`
2. Create `.podium/` folder structure in project root
3. Add `.podium/` to project `.gitignore`
4. Store service credentials in Windows Credential Manager
5. Add project to switcher list
6. Load new project automatically

---

## 14. Global vs Project Settings Reference

| Setting | Scope | Storage |
|---------|-------|---------|
| Provider API keys | Global per provider | Windows Credential Manager |
| KB source library | Global — Podium level | kb_sources.toml |
| KB source auth tokens | Global per source | Windows Credential Manager |
| Theme | Global | Podium config |
| Keymap | Global | Podium config |
| Project name | Per-project | projects.toml |
| Project folder path | Per-project | projects.toml |
| Git account | Per-project | projects.toml |
| Git remote URL | Per-project | projects.toml |
| KB sources connected | Per-project | projects.toml |
| Wing/namespace per source | Per-project connection | projects.toml |
| Agent roster | Per-project | projects.toml |
| Agent model source | Per-agent | projects.toml |
| Agent KB source access | Per-agent | projects.toml |
| Service connections | Per-project | projects.toml |
| Service API tokens | Per-project | Windows Credential Manager |

---

*Rev 1.1 — 2026-08-07 — APPROVED*
*KB sources correctly modeled as global Podium-level resource*
*Supersedes Rev 1.0*
