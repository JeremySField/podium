//! Podium configuration — projects.toml and kb_sources.toml.
//!
//! Handles reading and writing the two global Podium config files stored in
//! `%APPDATA%\podium\`. These files are the source of truth for all registered
//! projects and knowledge base sources. No database — ADR-005.
//!
//! ## File locations
//!
//! - `%APPDATA%\podium\projects.toml`  — registered projects, MRU ordered
//! - `%APPDATA%\podium\kb_sources.toml` — global KB source library
//!
//! ## Design notes
//!
//! - All structs derive `Serialize` + `Deserialize` so round-trip through toml
//!   is lossless.
//! - `GitConfig` uses a flat struct with an `auth` discriminator field
//!   (`"ssh"` or `"https"`) and `Option<String>` for mode-specific fields.
//!   This matches the TOML schema directly and avoids enum serialization
//!   complexity.
//! - Credentials (PATs, API keys) are never stored in these files — they live
//!   in Windows Credential Manager via the `keyring` crate (ADR-022).
//! - `last_opened` is stored as an ISO 8601 string (`chrono::DateTime<Utc>`)
//!   serialized via serde. MRU ordering in the project switcher is derived
//!   from this field at read time.
//! - Config directory is resolved from the `APPDATA` environment variable
//!   directly — Podium targets Windows and has no need for a cross-platform
//!   dirs crate.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Directory helpers
// ---------------------------------------------------------------------------

/// Returns the Podium config directory: `%APPDATA%\podium\` on Windows.
///
/// Reads the `APPDATA` environment variable directly. Falls back to the
/// current directory if `APPDATA` is not set (should not occur on Windows).
pub fn podium_config_dir() -> PathBuf {
    std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("podium")
}

/// Returns the path to `projects.toml`.
pub fn projects_toml_path() -> PathBuf {
    podium_config_dir().join("projects.toml")
}

/// Returns the path to `kb_sources.toml`.
pub fn kb_sources_toml_path() -> PathBuf {
    podium_config_dir().join("kb_sources.toml")
}

// ---------------------------------------------------------------------------
// projects.toml — top-level container
// ---------------------------------------------------------------------------

/// Root of `projects.toml`. Holds all registered projects.
///
/// Serializes as:
/// ```toml
/// [[projects]]
/// id = "showflyer"
/// ...
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectsConfig {
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
}

impl ProjectsConfig {
    /// Read `projects.toml` from disk. Returns an empty config if the file
    /// does not exist yet (first launch before any project is registered).
    pub fn load() -> Result<Self> {
        let path = projects_toml_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))
    }

    /// Write `projects.toml` to disk, creating the config directory if needed.
    pub fn save(&self) -> Result<()> {
        let path = projects_toml_path();
        std::fs::create_dir_all(path.parent().unwrap())
            .context("failed to create podium config directory")?;
        let content = toml::to_string_pretty(self)
            .context("failed to serialize projects.toml")?;
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write {}", path.display()))
    }

    /// Return projects sorted by `last_opened` descending (most recent first).
    ///
    /// Projects with no `last_opened` value sort to the end.
    pub fn projects_mru(&self) -> Vec<&ProjectEntry> {
        let mut sorted: Vec<&ProjectEntry> = self.projects.iter().collect();
        sorted.sort_by(|a, b| {
            match (&b.last_opened, &a.last_opened) {
                (Some(b_time), Some(a_time)) => b_time.cmp(a_time),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        sorted
    }

    /// Find a project by id.
    pub fn find_project(&self, id: &str) -> Option<&ProjectEntry> {
        self.projects.iter().find(|p| p.id == id)
    }

    /// Find a project by id, returning a mutable reference.
    pub fn find_project_mut(&mut self, id: &str) -> Option<&mut ProjectEntry> {
        self.projects.iter_mut().find(|p| p.id == id)
    }

    /// Add a new project entry. Does not write to disk — call `save()` after.
    pub fn add_project(&mut self, project: ProjectEntry) {
        self.projects.push(project);
    }

    /// Update `last_opened` for the project with `id` to now.
    /// Does not write to disk — call `save()` after.
    pub fn touch_project(&mut self, id: &str) {
        if let Some(project) = self.find_project_mut(id) {
            project.last_opened = Some(Utc::now());
        }
    }
}

// ---------------------------------------------------------------------------
// ProjectEntry
// ---------------------------------------------------------------------------

/// A single registered project in `projects.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    /// Unique project identifier — used as the key for credential storage,
    /// dock state, and all per-project lookups. Kebab-case, lowercase.
    /// Generated as a UUID v4 slug during onboarding.
    pub id: String,

    /// Display name — shown in the project switcher and title bar.
    /// Letters, numbers, spaces, hyphens, underscores only (onboarding spec).
    pub name: String,

    /// Absolute path to the project root on disk.
    pub path: String,

    /// Timestamp of last project load. Used for MRU ordering in the switcher.
    /// `None` for projects that have never been loaded after registration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_opened: Option<DateTime<Utc>>,

    /// Git authentication configuration for this project.
    /// `None` if the user skipped Step 3 during onboarding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitConfig>,

    /// KB sources connected to this project (selected from global library).
    /// Empty if the user skipped Step 5 during onboarding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kb_connections: Vec<KbConnection>,

    /// Agent roster for this project. Fully user-defined (ADR-021).
    /// Empty if the user skipped Step 4 during onboarding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<AgentEntry>,

    /// External service connections for the Health tab.
    /// Empty if the user skipped Step 6 during onboarding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<ServiceEntry>,
}

// ---------------------------------------------------------------------------
// GitConfig
// ---------------------------------------------------------------------------

/// Git authentication configuration for a project.
///
/// Uses a flat struct with an `auth` discriminator rather than a serde enum
/// to match the TOML schema directly:
///
/// ```toml
/// [projects.git]
/// auth = "https"
/// account = "JeremySField"
/// remote = "https://github.com/JeremySField/podium.git"
/// ```
///
/// For SSH auth, `account` is the SSH config alias (e.g. `github-personal`).
/// For HTTPS auth, `account` is the GitHub username.
/// The PAT for HTTPS is stored in Windows Credential Manager scoped to the
/// project id — never in this file (ADR-022).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Authentication method: `"ssh"` or `"https"`.
    pub auth: String,

    /// SSH: the Host alias from `~/.ssh/config`.
    /// HTTPS: the GitHub username.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,

    /// Remote URL for this project's git repository.
    /// Pre-filled from `.git/config` during onboarding if detected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
}

impl GitConfig {
    /// True if this config uses HTTPS authentication.
    pub fn is_https(&self) -> bool {
        self.auth == "https"
    }

    /// True if this config uses SSH authentication.
    pub fn is_ssh(&self) -> bool {
        self.auth == "ssh"
    }
}

// ---------------------------------------------------------------------------
// KbConnection
// ---------------------------------------------------------------------------

/// A KB source connected to a project, referencing a source in `kb_sources.toml`.
///
/// ```toml
/// [[projects.kb_connections]]
/// source_id = "main-mempalace"
/// wing = "showflyer"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbConnection {
    /// ID of the source in `kb_sources.toml`.
    pub source_id: String,

    /// Provider-specific namespace for this project's data within the source.
    /// For MemPalace: the wing name. Optional for providers that don't use
    /// namespacing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wing: Option<String>,
}

// ---------------------------------------------------------------------------
// AgentEntry
// ---------------------------------------------------------------------------

/// A configured agent in a project's roster.
///
/// ```toml
/// [[projects.agents]]
/// id = "research-agent"
/// name = "Research Agent"
/// purpose = "Finds and synthesizes external sources"
/// provider = "anthropic"
/// model = "claude-sonnet-4-6"
/// kb_sources = ["main-mempalace"]
/// ```
///
/// Provider API keys are stored globally in Windows Credential Manager,
/// keyed by provider name — never per-agent or per-project (ADR-022).
/// Custom/local endpoint URLs are stored here since they are not credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEntry {
    /// Unique agent identifier within this project. Kebab-case.
    pub id: String,

    /// Display name shown on the agent card.
    pub name: String,

    /// Free-text description of what this agent does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,

    /// Model provider: `"anthropic"`, `"openai"`, `"google"`, `"xai"`,
    /// or `"custom"`.
    pub provider: String,

    /// Model identifier — provider-specific string or free text for custom.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// HTTP endpoint for custom/local providers. Not used for cloud providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,

    /// IDs of KB sources (from this project's `kb_connections`) this agent
    /// can access.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kb_sources: Vec<String>,
}

// ---------------------------------------------------------------------------
// ServiceEntry
// ---------------------------------------------------------------------------

/// An external service connection for the Health tab.
///
/// ```toml
/// [[projects.services]]
/// type = "supabase"
/// name = "ShowFlyer DB"
/// url = "https://xxx.supabase.co"
/// ```
///
/// Service credentials (anon keys, API tokens) are stored in Windows
/// Credential Manager scoped to the project id — never in this file (ADR-022,
/// ADR-025).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    /// Service type: `"supabase"`, `"railway"`, `"github"`, `"custom"`, etc.
    #[serde(rename = "type")]
    pub service_type: String,

    /// Display name shown in the Health tab.
    pub name: String,

    /// Primary URL for this service (project URL, API endpoint, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Provider-specific project or resource identifier.
    /// Railway: project_id. GitHub: repo owner/name. Custom: unused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

// ---------------------------------------------------------------------------
// kb_sources.toml — top-level container
// ---------------------------------------------------------------------------

/// Root of `kb_sources.toml`. Holds the global KB source library.
///
/// KB sources are a Podium-level resource, not scoped to any project (ADR-023).
/// They are configured once here and connected per-project in `projects.toml`.
///
/// ```toml
/// [[sources]]
/// id = "main-mempalace"
/// name = "Main MemPalace"
/// provider = "mempalace"
/// endpoint = "http://nas-ip:port"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KbSourcesConfig {
    #[serde(default)]
    pub sources: Vec<KbSourceEntry>,
}

impl KbSourcesConfig {
    /// Read `kb_sources.toml` from disk. Returns an empty config if the file
    /// does not exist yet.
    pub fn load() -> Result<Self> {
        let path = kb_sources_toml_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))
    }

    /// Write `kb_sources.toml` to disk, creating the config directory if needed.
    pub fn save(&self) -> Result<()> {
        let path = kb_sources_toml_path();
        std::fs::create_dir_all(path.parent().unwrap())
            .context("failed to create podium config directory")?;
        let content = toml::to_string_pretty(self)
            .context("failed to serialize kb_sources.toml")?;
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write {}", path.display()))
    }

    /// Find a source by id.
    pub fn find_source(&self, id: &str) -> Option<&KbSourceEntry> {
        self.sources.iter().find(|s| s.id == id)
    }
}

// ---------------------------------------------------------------------------
// KbSourceEntry
// ---------------------------------------------------------------------------

/// A single knowledge base source in the global library.
///
/// Auth tokens for KB sources are stored in Windows Credential Manager
/// scoped to the source id — never in this file (ADR-022).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbSourceEntry {
    /// Unique source identifier. Kebab-case.
    pub id: String,

    /// Display name shown in the KB panel and onboarding Step 5.
    pub name: String,

    /// Provider type: `"mempalace"`, `"obsidian"`, `"notion"`, `"custom"`.
    pub provider: String,

    /// HTTP endpoint for query requests.
    /// Required for MemPalace, Obsidian, and Custom providers.
    /// Not used for Notion (uses official API client).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}
