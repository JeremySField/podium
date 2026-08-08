---
project: Podium
file: project_onboarding_spec
type: spec — all questions resolved, approved for build
last_updated: 2026-08-07
status: APPROVED — all decisions locked, ready for build
---

<!-- Podium project_onboarding_spec -->

# Podium — Project Onboarding & Project Switcher Spec
## Rev 1.0 — APPROVED

All decisions locked. This document is the authoritative reference for building the project switcher, onboarding flow, and related systems.

<!-- Podium project_onboarding_spec -->

---

## 1. What This Covers

- The project switcher UI in the TitleBar
- The "Add New Project" onboarding flow
- The `projects.toml` data schema
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
- Podium Settings (global)
- Project Settings (active project)
- Theme
- Keymap
- Quit

### 2.5 Tab Bar
Settings removed from tab bar. Tab bar is:
**Files | Agents | Knowledge | Review | Terminal | Health**

---

## 3. The Project Model

### 3.1 What a Project Is
A project is a registered workspace room with:
- Name and display identity
- Root filesystem path
- Room folder structure (`.podium/` inside project root)
- User-defined agent roster with per-agent model source and KB source assignments
- Knowledge base source library (per-project, assigned per agent)
- Service connections (per-project)
- Git configuration (per-project)

### 3.2 projects.toml Location
`%APPDATA%\podium\projects.toml` on Windows.

### 3.3 projects.toml Schema

```toml
[[projects]]
id = "showflyer"
name = "ShowFlyer"
path = "C:/Users/jerem/OneDrive/Desktop/Repos/showflyer"
last_opened = "2026-08-07T20:00:00Z"

[projects.git]
account = "showflyer-github"
remote = "git@github-showflyer:username/showflyer.git"

[[projects.agents]]
id = "research-agent"
name = "Research Agent"
purpose = "Finds and synthesizes external sources"
avatar = "path/to/avatar.png"     # optional
provider = "anthropic"
model = "claude-sonnet-4-6"
kb_sources = ["showflyer-mempalace"]

[[projects.agents]]
id = "doc-agent"
name = "Doc Agent"
purpose = "Maintains documentation and session logs"
provider = "custom"
endpoint = "http://inference-node:8080"
model = "hermes-14b"
kb_sources = ["showflyer-mempalace", "showflyer-docs"]

[[projects.kb_sources]]
id = "showflyer-mempalace"
name = "ShowFlyer MemPalace"
provider = "mempalace"
endpoint = "http://nas-ip:port"
wing = "showflyer"
# bearer token in Windows Credential Manager

[[projects.kb_sources]]
id = "showflyer-docs"
name = "ShowFlyer Obsidian Vault"
provider = "obsidian"
endpoint = "http://localhost:27123"
# token in Windows Credential Manager

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

### 3.4 Project Name Rules
Letters, numbers, spaces, hyphens, underscores only. If it can be a folder name, it can be a project name. Special characters disallowed: `/ \ : * ? " < > | . @`

---

## 4. Room Folder Structure

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

## 5. Onboarding Flow

### 5.1 Entry Points
- "Add New Project" in project switcher Combobox
- Application menu

### 5.2 First Launch Empty State
Empty state screen — no auto-open of onboarding:
- Centered text: "No projects yet. Add your first project."
- Single "Add New Project" button
- Nothing else

### 5.3 Flow UI Container
Sheet — slides in from the side. Podium visible behind it. Per ADR-019.

### 5.4 Step Navigation
- Linear step flow
- Back button on all steps except Step 1
- Cancel on all steps — discards all input, no project created
- Progress indicator shows current step and total
- Folder and name required. All other steps skippable.
- New project loads automatically after creation.

### 5.5 Progressive Disclosure — Per ADR-020
Every field has:
- Field label + input (always visible)
- One-line hint beneath (always visible, subtle)
- Expandable "?" for fuller explanation (collapsed by default)

Fast path is always the default. Explanation always one tap away.

---

## 6. Step 1 — Folder Selection

- Native OS folder picker via `rfd` crate (Rusty File Dialog)
- Folder name pre-fills project name in Step 2
- Detects `.git` directory — pre-fills remote URL from `.git/config`
- Detects existing `.podium/` folder — prompts user: **Skip room creation** or **Overwrite**

---

## 7. Step 2 — Project Identity

**Fields:**
- Project name — pre-filled from folder name, editable, required
- No color picker, no project icon

**Validation:**
- Unique across registered projects
- Cannot be empty
- Characters: letters, numbers, spaces, hyphens, underscores only

---

## 8. Step 3 — Git Configuration (Skippable)

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

## 9. Step 4 — Agent Configuration (Skippable)

### 9.1 Agent Roster
Fully user-defined per project. No fixed roster. Users create agents from scratch, name them, assign purpose, configure model source.

### 9.2 Per Agent Fields
- **Name** — user defined
- **Purpose** — free text, shown on agent card
- **Avatar** — optional image upload
- **Provider** — Anthropic, OpenAI, Google, xAI, Custom/Local
- **Model** — dropdown by provider, or free text for Custom/Local
- **Endpoint URL** — Custom/Local only
- **Knowledge Base Sources** — multi-select from configured KB sources

### 9.3 Supported Providers
| Provider | Notes |
|----------|-------|
| Anthropic | Claude — Haiku, Sonnet, Opus |
| OpenAI | GPT models |
| Google | Gemini models |
| xAI | Grok models |
| Custom / Local | Any OpenAI-compatible endpoint — Ollama, LM Studio, inference node, etc. |

### 9.4 API Key Storage
Stored globally per provider in Windows Credential Manager. Entered once per provider, shared across all agents using that provider. Never in projects.toml.

### 9.5 Agent Folder Creation
On agent creation Podium creates `.podium/agents/[agent-id]/inbox/`, `outbox/`, `working/`.
On agent deletion — moved to `.podium/agents/archive/[agent-id]/`. Never permanently deleted.

**Skip behavior:** Agents tab shows "No agents configured" with "Add Agent" button.

---

## 10. Step 5 — Knowledge Base Configuration (Skippable)

### 10.1 KB Model
A library of configured sources per project. Multiple sources configurable. Each agent selects which sources it has access to — isolating agent context explicitly. An agent only knows what it is given.

### 10.2 Supported Providers

**Beta:**
| Provider | Notes |
|----------|-------|
| MemPalace | Endpoint + wing name + bearer token |
| Obsidian | Via Local REST API plugin |
| Notion | API key + database ID |
| Custom | Any HTTP query endpoint |

**v1.0 — based on community demand from beta.**

### 10.3 Per KB Source Fields
- **Source name** — user defined
- **Provider** — dropdown
- **Endpoint URL**
- **Wing / namespace** — provider-specific (e.g. MemPalace wing)
- **Auth token** — Windows Credential Manager, scoped to source ID
- **Read-only** — always enforced, not configurable

### 10.4 Knowledge Tab in Podium
- All configured KB sources for the loaded project
- Which agents are connected to which sources
- Sync status / last query per source
- "Add Source" button

**Skip behavior:** Knowledge tab shows "No knowledge base sources configured" with "Add Source" button.

---

## 11. Step 6 — Services Configuration (Skippable)

### 11.1 Purpose
Powers the Health tab — external APIs Podium polls to show project health and status.

### 11.2 All service credentials are per-project, stored in Windows Credential Manager scoped to project ID.

### 11.3 Supported Services

**Beta:**
| Service | Fields |
|---------|--------|
| Supabase | Project URL + anon key |
| Railway | Project ID + API token |
| GitHub | Repo owner/name, uses SSH config |
| Custom | Name + polling URL + expected response |

**v1.0:**

Infrastructure/Deploy: Vercel, Netlify, Fly.io, Render, AWS (CloudWatch/EC2), Digital Ocean

Database: PlanetScale, Neon, MongoDB Atlas, Firebase

Monitoring: Datadog, Sentry, Uptime Robot, Better Uptime

CI/CD: CircleCI, Jenkins (GitHub Actions via GitHub)

Notifications: Slack webhook, Discord webhook

**Skip behavior:** Health tab shows "No services configured" with "Add Service" button.

---

## 12. Step 7 — Confirm & Create

### 12.1 Summary Screen
Review of all configured values:
- Project name and folder path
- Git account and remote (if configured)
- Agents list with providers (if configured)
- KB sources (if configured)
- Services (if configured)

### 12.2 Creation Actions
1. Write project entry to `%APPDATA%\podium\projects.toml`
2. Create `.podium/` folder structure in project root
3. Add `.podium/` to project `.gitignore`
4. Store credentials in Windows Credential Manager
5. Add project to switcher list (most recently used — appears at top)
6. Load new project automatically

---

## 13. Global vs Project Settings Reference

| Setting | Scope | Storage |
|---------|-------|---------|
| Provider API keys | Global per provider | Windows Credential Manager |
| Theme | Global | Podium config |
| Keymap | Global | Podium config |
| Project name | Per-project | projects.toml |
| Project folder path | Per-project | projects.toml |
| Git account | Per-project | projects.toml |
| Git remote URL | Per-project | projects.toml |
| Agent roster | Per-project | projects.toml |
| Agent model source | Per-agent | projects.toml |
| Agent KB sources | Per-agent | projects.toml |
| KB source library | Per-project | projects.toml |
| KB auth tokens | Per-source | Windows Credential Manager |
| Service connections | Per-project | projects.toml |
| Service API tokens | Per-project | Windows Credential Manager |

---

*Rev 1.0 — 2026-08-07 — APPROVED — all decisions locked*
*Supersedes Rev 0.1 and Rev 0.2*
