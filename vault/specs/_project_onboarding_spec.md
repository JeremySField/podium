---
project: Podium
file: project_onboarding_spec
type: working document — all questions must be resolved before build
last_updated: 2026-08-07
status: DRAFT — open questions unresolved, do not build from this document yet
---

<!-- Podium project_onboarding_spec -->

# Podium — Project Onboarding & Project Switcher Spec
## Working Document — Rev 0.1

This is a working document. Every open question marked [OPEN] must be answered and this document promoted to APPROVED before any onboarding or project switcher code is written.

<!-- Podium project_onboarding_spec -->

---

## 1. What This Covers

- The project switcher UI in the TitleBar
- The "Add New Project" onboarding flow
- The `projects.toml` data schema that persists project configuration
- The room filesystem structure created during onboarding
- Every design decision and open question

---

## 2. The Project Switcher

### 2.1 Location
TitleBar. Always visible regardless of active tab or panel state. This is a locked decision — the switcher never moves.

### 2.2 Visual Design
[OPEN] — Which gpui-component is used for the switcher?
- **Select** — simple dropdown, shows current project name, click opens list
- **Combobox** — searchable dropdown, useful for many projects
- **DropdownButton** — button with chevron, more visual control, closest to VS Code workspace selector

Recommendation: DropdownButton for Phase 1 — most control over appearance, clean single-line TitleBar presence. Combobox can replace it later if project count grows.

**Decision needed:** DropdownButton, Select, or Combobox?

### 2.3 Switcher Behavior
- Shows currently active project name
- Click opens project list
- List shows all registered projects
- Selecting a project triggers project load sequence (Phase 2)
- In Phase 1 — selection updates active project state only, no load/unload yet
- "Add New Project" option at bottom of list opens onboarding flow
- [OPEN] — Does the switcher show a project icon/color indicator alongside the name?
- [OPEN] — Does the switcher show any project status indicator (e.g. unsaved changes, agent activity)?

### 2.4 Project List Display
[OPEN] — How are projects ordered in the switcher list?
- Alphabetical
- Most recently used first
- Manual ordering (drag to reorder)

Recommendation: Most recently used first — matches how you actually work.

---

## 3. The Project Model

### 3.1 What a Project Is
A project is a registered workspace room. It has:
- A name and display identity
- A root filesystem path
- A room folder structure (agent inbox/outbox, working folders)
- Agent configuration (which agents, which model source per agent)
- MemPalace configuration (wing name, serve endpoint)
- Service configuration (Supabase, Railway, GitHub, etc.)
- Git configuration (which SSH key / GitHub account)

### 3.2 projects.toml Schema — Draft

```toml
[[projects]]
id = "showflyer"                    # unique slug, used for room folder name
name = "ShowFlyer"                  # display name in switcher
path = "C:/Users/jerem/OneDrive/Desktop/Repos/showflyer"  # project root
color = "#4F46E5"                   # [OPEN] display color in switcher?
last_opened = "2026-08-07T20:00:00Z"

[projects.git]
account = "showflyer-github"        # SSH config alias
remote = "git@github-showflyer:username/showflyer.git"

[projects.mempalace]
wing = "showflyer"
endpoint = "http://nas-ip:port/mcp"
read_only = true

[projects.agents]
# [OPEN] — full agent schema TBD — see Section 5

[projects.services]
# [OPEN] — full services schema TBD — see Section 6
```

**[OPEN] — Where does projects.toml live?**
Options:
- Podium config directory (OS-specific: `%APPDATA%\podium\` on Windows)
- Fixed path in user home directory (`~/.podium/`)
- Alongside the Podium executable
- User-configurable location

Recommendation: `%APPDATA%\podium\projects.toml` on Windows — standard location for app config, consistent with OS conventions.

---

## 4. Onboarding Flow — Step by Step

### 4.1 Entry Points
- "Add New Project" option in the project switcher dropdown
- [OPEN] — Is there also an entry point from a Settings panel?
- [OPEN] — On first launch with no projects registered, does Podium open the onboarding flow automatically?

### 4.2 Flow UI Container
[OPEN] — How is the onboarding flow presented?
- **Dialog** — modal overlay, blocks interaction with the rest of Podium
- **Sheet** — slides in from the side, Podium still visible behind it
- **Full panel replacement** — onboarding takes over the main content area
- **Dedicated window** — separate OS window

Recommendation: Dialog — clean, focused, standard for setup flows. gpui-component has a Dialog component.

### 4.3 Step Navigation
- Linear step flow — must complete each step before proceeding
- Back button available on all steps except Step 1
- Cancel available on all steps — discards all input, no project created
- Progress indicator shows current step and total steps
- [OPEN] — Can steps be skipped? (e.g. skip Services if not needed yet)

Recommendation: Services and agent configuration should be skippable — not every project uses every service. Required fields: folder, name. Everything else optional at onboarding, configurable later in project settings.

---

## 5. Step 1 — Folder Selection

### 5.1 Interaction
- Folder browser opens — native OS folder picker
- User selects project root folder
- [OPEN] — Does gpui-component provide a native folder picker, or does this require a Tauri/OS dialog call from Rust?
- [OPEN] — On Windows, what is the Rust API for opening a native folder picker dialog?

Candidate: `rfd` crate (Rusty File Dialog) — cross-platform native file/folder dialogs for Rust. Widely used, actively maintained. Would need to be added as a dependency.

### 5.2 After Folder Selection
- Podium reads the folder name and pre-fills project name (Step 2)
- Podium checks if folder contains a `.git` directory — if yes, pre-fills git remote from `.git/config`
- Podium checks if a room folder structure already exists in the folder
- [OPEN] — If a room structure already exists (re-importing a project), does onboarding skip room creation?

---

## 6. Step 2 — Project Identity

### 6.1 Fields
- **Project name** — pre-filled from folder name, editable text input
- **Display color** — [OPEN] color picker for switcher display? Or fixed palette to choose from?
- **[OPEN]** — Project icon? Or color only?

### 6.2 Validation
- Name must be unique across registered projects
- Name cannot be empty
- [OPEN] — Character restrictions on project name?

---

## 7. Step 3 — Git Configuration

### 7.1 Fields
- **GitHub account** — which SSH config alias to use for this project
- **[OPEN]** — How does Podium know which SSH aliases are available? Does it read `~/.ssh/config` and parse Host entries?
- **Remote URL** — pre-filled from `.git/config` if detected, editable

### 7.2 SSH Config Parsing
If Podium reads `~/.ssh/config` to populate available accounts:
- Parse all `Host` entries
- Present as a dropdown in the git configuration step
- [OPEN] — What if no SSH config exists? Fall back to HTTPS? Show a setup guide?

---

## 8. Step 4 — Agent Configuration

### 8.1 Agent Roster
[OPEN] — Is the agent roster fixed (same agents for every project) or configurable per project?

Options:
- **Fixed roster** — every project has the same set of agents (Orchestrator, Doc Agent, Research Agent, Builder, Reviewer, etc.). Agents not needed for a project are set to "disabled."
- **Configurable roster** — each project defines which agents it uses and can add custom agents

Recommendation: Fixed roster with enable/disable per agent. Simpler to build, easier to manage. Custom agents can be added later.

### 8.2 Agent Schema (per agent)
For each agent in the roster:

```toml
[projects.agents.doc_agent]
enabled = true
model_source = "local"             # "local" | "anthropic" | "disabled"
endpoint = "http://inference:8080" # if local
model = "hermes-14b"               # if local
# if anthropic:
# model = "claude-sonnet-4-6"
```

### 8.3 Model Source Options
- **Local LLM** — inference node HTTP endpoint + model name
- **Anthropic API** — uses global Anthropic API key, model selection (Haiku, Sonnet, Opus)
- **Disabled** — agent not used for this project

### 8.4 Open Questions — Agents
- [OPEN] — What is the fixed agent roster? Define all agent names and roles.
- [OPEN] — Where is the global Anthropic API key stored? OS keychain? Podium config file?
- [OPEN] — Where is the local inference endpoint configured — globally (same for all projects) or per project?
- [OPEN] — Can the inference endpoint differ per agent within the same project?

---

## 9. Step 5 — MemPalace Configuration

### 9.1 Fields
- **Wing name** — the MemPalace wing for this project (text input)
- **Serve endpoint** — URL of the `mempalace serve` instance (text input)
- **[OPEN]** — Is MemPalace configuration global (one instance shared across all projects) or per project?

Recommendation: Global MemPalace endpoint configured once in Podium settings, wing name configured per project. The serve endpoint is the same for all projects — only the wing differs.

### 9.2 Read-Only Enforcement
MemPalace is always read-only from Podium. This is not configurable. The `--read-only` flag is enforced at the serve endpoint level, not at the Podium level. No UI toggle for this.

### 9.3 Open Questions — MemPalace
- [OPEN] — Where is the global MemPalace endpoint configured? Podium settings panel?
- [OPEN] — Authentication — does the serve endpoint require a bearer token? If yes, where is it stored?

---

## 10. Step 6 — Services Configuration

### 10.1 Purpose
Services power the Health tab — external APIs that Podium polls to show project health status.

### 10.2 Service Types (initial set)
- **Supabase** — project URL + anon key
- **Railway** — project ID + API token
- **GitHub** — repo owner/name (uses SSH config from Step 3)
- **[OPEN]** — What other services need to be supported at launch?

### 10.3 Optional Step
Services are optional at onboarding. A project can be registered without services — the Health tab will show "no services configured" until they are added. Services can be added/edited later in project settings.

### 10.4 Open Questions — Services
- [OPEN] — Where are API tokens and keys stored? OS keychain strongly recommended over plaintext in projects.toml.
- [OPEN] — Is there a global Railway API token or per-project?
- [OPEN] — How does the Health tab know what to poll if services are added after onboarding?

---

## 11. Step 7 — Confirm & Create

### 11.1 Summary Screen
Shows all configured values for review before creation:
- Project name, folder, color
- Git account and remote
- Agents enabled and their model sources
- MemPalace wing
- Services configured

### 11.2 Creation Actions (on confirm)
1. Write project entry to `projects.toml`
2. Create room folder structure in the project root:
```
/[project-root]/
  .podium/
    /agents/
      /[agent-name]/
        /inbox/
        /outbox/
        /working/
    /review/
```
3. Add project to switcher list
4. [OPEN] — Does Podium automatically load the new project after creation, or return to the current active project?

### 11.3 Room Folder Location
[OPEN] — Where does the room folder structure live?
- **Inside the project root** — `.podium/` subfolder alongside the code. Clean, portable, travels with the repo. Risk: accidentally committed if `.gitignore` not set.
- **Outside the project root** — separate Podium data directory, e.g. `%APPDATA%\podium\rooms\[project-id]\`. Clean separation from code. Less portable.

Recommendation: `.podium/` inside the project root, with `.podium/` added to `.gitignore` automatically during room creation.

---

## 12. Project Settings (Post-Onboarding)

After a project is registered, all configuration must be editable. A project settings panel (accessible from the switcher or a settings icon) allows editing any field set during onboarding.

[OPEN] — Where is the project settings UI accessed from?
- Right-click on project name in switcher
- Settings icon next to project name in switcher
- Dedicated Settings tab in the main panel

---

## 13. Global Settings vs Project Settings

Some settings are global (apply to all projects), some are per-project.

| Setting | Scope |
|---------|-------|
| Anthropic API key | Global |
| MemPalace serve endpoint | Global |
| MemPalace bearer token | Global |
| Local inference endpoint | [OPEN] — Global or per-project? |
| Project name | Per-project |
| Project folder path | Per-project |
| Git account | Per-project |
| Agent roster and model sources | Per-project |
| MemPalace wing name | Per-project |
| Services (Supabase, Railway, etc.) | Per-project |

---

## 14. Sensitive Data Storage

API keys, tokens, and credentials must not be stored in plaintext in `projects.toml`.

[OPEN] — Credential storage strategy:
- **OS keychain** (Windows Credential Manager) — most secure, OS-managed, not portable
- **Encrypted file** — Podium manages encryption, portable but requires key management
- **Environment variables** — user manages, not stored by Podium at all
- **Plaintext with warning** — simple but insecure, not recommended

Recommendation: Windows Credential Manager via the `keyring` Rust crate. Standard approach for desktop apps on Windows. Credentials stored securely by the OS, retrieved by Podium at runtime. `projects.toml` contains no secrets.

---

## 15. Open Questions — Master List

All of the following must be answered before build begins:

### UI / UX
- [ ] Project switcher component: DropdownButton, Select, or Combobox?
- [ ] Does the switcher show project color/icon?
- [ ] Does the switcher show project status indicators?
- [ ] How are projects ordered in the switcher list?
- [ ] Onboarding flow container: Dialog, Sheet, full panel, or separate window?
- [ ] Can onboarding steps be skipped?
- [ ] On first launch with no projects, does onboarding open automatically?
- [ ] Where is project settings accessed from post-onboarding?

### Folder & Filesystem
- [ ] Where does projects.toml live?
- [ ] Where does the room folder structure live — inside or outside project root?
- [ ] Does Podium auto-add .podium/ to .gitignore on room creation?
- [ ] If room structure already exists on import, does onboarding skip creation?
- [ ] Does Podium load the new project automatically after creation?

### Git
- [ ] Does Podium parse ~/.ssh/config for available GitHub accounts?
- [ ] What if no SSH config exists — fallback behavior?

### Agents
- [ ] What is the fixed agent roster — all agent names and roles?
- [ ] Is the inference endpoint global or per-project?
- [ ] Can inference endpoint differ per agent within a project?

### MemPalace
- [ ] Is MemPalace endpoint global or per-project?
- [ ] Does the serve endpoint require bearer token authentication?
- [ ] Where is the global MemPalace endpoint configured?

### Services
- [ ] What services are supported at launch beyond Supabase, Railway, GitHub?
- [ ] Is Railway API token global or per-project?

### Credentials
- [ ] Credential storage strategy — OS keychain, encrypted file, env vars, or plaintext?
- [ ] Where is the global Anthropic API key stored and configured?

### Technical
- [ ] Folder picker — rfd crate or alternative?
- [ ] Character restrictions on project name?
- [ ] Project color — color picker or fixed palette?

---

*Rev 0.1 — 2026-08-07 — Working document, not approved for build*
*Status: DRAFT — all [OPEN] items must be resolved before promotion to APPROVED*
