---
project: Podium
file: commit_procedure
type: reference — load when committing or reviewing commit history
last_updated: 2026-08-08
---

<!-- Podium commit_procedure -->

# Podium — Commit Procedure

This document defines the commit message standard and workflow for the Podium project. Follow this on every commit.

<!-- Podium commit_procedure -->

---

## Commit Message Format

```
<type>(<scope>): <short description>
```

**All lowercase. Present tense. No period at the end.**

---

## Types

| Type | When to Use |
|------|-------------|
| `feat` | Adding new functionality |
| `fix` | Fixing a bug or incorrect behavior |
| `docs` | Documentation only — vault docs, README, session reports |
| `chore` | Maintenance — Cargo.toml, Cargo.lock, gitignore, tooling |
| `refactor` | Code restructure with no behavior change |
| `style` | Formatting, whitespace, no logic change |
| `test` | Adding or updating tests |
| `build` | Build system changes — dependencies, compiler flags |

---

## Scopes

Scope is optional but recommended for clarity. Use the area of the codebase being changed.

| Scope | Covers |
|-------|--------|
| `panel` | panel.rs, panel traits, PanelEvent, PanelPosition |
| `dock` | dock.rs, PodiumDock |
| `shell` | main.rs, application bootstrap, Root wrapper |
| `theme` | color constants, dark theme |
| `titlebar` | TitleBar, application menu |
| `tabs` | tab bar, tab switching |
| `switcher` | project switcher Combobox |
| `statusbar` | StatusBar |
| `onboarding` | onboarding Sheet and steps |
| `projects` | projects.toml, project load/unload |
| `agents` | agent panel, agent cards, inbox/outbox watcher |
| `chat` | chat panel, API clients |
| `git` | git panel, git2 integration |
| `files` | files panel, file tree |
| `knowledge` | knowledge panel, KB source integration |
| `health` | health panel, service polling |
| `terminal` | terminal panel, alacritty_terminal |
| `credentials` | Windows Credential Manager integration |
| `vault` | all vault docs — specs, ADRs, primers, session log |
| `deps` | dependency additions or updates |

---

## Examples

```
feat(panel): add PodiumPanel trait and PanelEvent enum
feat(dock): implement PodiumDock with activation priority ordering
feat(shell): rewrite main.rs with Root wrapper and asset registration
feat(theme): add dark theme color constants from Zed reference
feat(titlebar): add TitleBar with Combobox project switcher stub
feat(tabs): wire tab bar with active tab state
fix(panel): correct IconName import path to gpui_component::icon::IconName
docs(vault): add Phase 1 primer with IconName import note
docs(vault): update onboarding spec to Rev 1.1 — KB sources global
chore(deps): add gpui-component and anyhow to Cargo.toml
build(deps): update Cargo.lock after gpui-component resolution
refactor(shell): extract PodiumApp struct from main
docs(vault): add ADR-026 terminal license decision
docs(vault): add ADR-027 dock owns panel persistence
```

---

## Workflow — One Logical Change Per Commit

**Never batch unrelated changes into one commit.**

Each commit should answer: *"What one thing changed and why?"*

**Order of operations:**
1. Edit files in knowledgebase codebase
2. Copy changed files to repo
3. `git add <specific files>` — never `git add .` unless every changed file belongs to the same logical change
4. `git commit -m "<type>(<scope>): <description>"`
5. `git push origin main`

**If multiple things changed, make multiple commits.**

---

## Vault Doc Commits

Vault doc changes always use `docs(vault):` regardless of which doc was changed.

```
docs(vault): add session 003b report — PodiumPanel trait design
docs(vault): update Phase 1 primer — build order and IconName note
docs(vault): update session log — Phase 1 next
```

---

## What Makes a Good Commit Message

**Good:**
```
feat(panel): add activation_priority method with documented values
fix(dock): prevent duplicate priority panic in debug builds
docs(vault): approve project onboarding spec Rev 1.1
```

**Bad:**
```
updates
fixed stuff
WIP
changes to panel
```

The description should be specific enough that reading the history tells the full story without opening each commit.

---

*Podium Commit Procedure — 2026-08-08*
*Reference document — keep in vault/reference/*
