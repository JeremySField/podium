//! Onboarding flow for Podium — Sheet container and 7-step card system.
//!
//! ## Architecture
//!
//! `OnboardingSheet` is a proper GPUI entity (`Entity<OnboardingSheet>`) that
//! owns its own state and drives its own navigation. This follows the Zed
//! pattern established in `onboarding.rs` and `handle_sign_in` — async actions
//! use `window.spawn(cx, async move |cx: &mut AsyncWindowContext| { ... })`
//! directly on the entity, giving back `&mut Window` after the await without
//! any injected callbacks.
//!
//! `PodiumApp` creates the entity, opens the Sheet, and subscribes to
//! `OnboardingEvent::ProjectCreated`. That subscription is the entire interface
//! between the two — no `Arc<dyn Fn>` callbacks, no state on `PodiumApp`.
//!
//! ## Flow
//!
//! 1. User clicks "Add New Project" → `PodiumApp::open_onboarding` called
//! 2. `PodiumApp` creates `Entity<OnboardingSheet>` and opens the Sheet
//! 3. `OnboardingSheet` renders itself; nav buttons use `cx.listener`
//! 4. Each nav action mutates `self.state` and calls `cx.notify()`
//! 5. The Sheet re-renders from the updated state — no `open_sheet_at` repeat
//! 6. Folder picker: `window.spawn` → `AsyncWindowContext` → `.update` back
//! 7. Step 7 confirm: emit `OnboardingEvent::ProjectCreated`, close Sheet
//! 8. Cancel: close Sheet — `PodiumApp` subscription cleans up the entity ref
//!
//! ## Steps
//!
//! | Step | Required | Content |
//! |------|----------|---------|
//! | 1 — Folder     | Yes | Native folder picker, .git/.podium detection |
//! | 2 — Identity   | Yes | Project name, validation |
//! | 3 — Git        | No  | SSH/HTTPS toggle, account input/dropdown, remote URL |
//! | 4 — Agents     | No  | Agent roster |
//! | 5 — KB Sources | No  | Provider tile grid — multi-select with per-source wing config |
//! | 6 — Services   | No  | External service connections |
//! | 7 — Confirm    | —   | Summary + Create Project button |
//!
//! Steps 1 and 2 are required. Steps 3–6 are skippable. Step 7 has no Skip.
//!
//! ## Step 4 — Dynamic agent list architecture
//!
//! Each agent card owns four GPUI entities: name `InputState`, purpose
//! `InputState`, provider `SelectState`, and model `SelectState`. These live
//! in `OnboardingSheet::agent_inputs: Vec<AgentInputState>` — a plain vec of
//! plain structs. Subscriptions for agent `i` are stored in
//! `agent_subscriptions[i]: Vec<Subscription>`.
//!
//! Add: push to both vecs. Remove at `i`: `remove(i)` on both — entity
//! handles and subscriptions drop atomically.
//!
//! Provider subscription uses `cx.subscribe_in` (confirmed from
//! `gpui/src/app/context.rs`) which delivers `&mut Window` to the callback,
//! allowing `set_items` and `set_selected_index` to be called directly from
//! the stored subscription. Name, purpose, and model subscriptions use plain
//! `cx.subscribe` — they need only `cx`.
//!
//! ## Step 5 — KB sources tile grid and wing configuration
//!
//! Sources are loaded once from `KbSourcesConfig::load()` in `new()` and stored
//! on `OnboardingSheet::kb_sources`. Selections are stored as
//! `OnboardingState::kb_connections: Vec<KbConnectionDraft>`, each carrying a
//! `source_id` and an optional `wing`.
//!
//! **Tile toggle:** click a tile to select/deselect. On selection of a MemPalace
//! source with a configured `endpoint`, Podium fires an async wing fetch via
//! `window.spawn`. The fetch result arrives in `sheet.update(cx, |this, cx| { ... })`
//! — that callback has `&mut Window` — so the wing `SelectState` entity is
//! created there (via `create_wing_select`), before `cx.notify()`. Render only
//! reads; it never creates entities. This is the same pattern used for the
//! provider/model coupling in Step 4 (session 013, `subscribe_in`).
//!
//! **Wing fetch** — MemPalace MCP-over-HTTP protocol (v3.5.0+):
//! ```
//! POST {endpoint}/mcp
//! Authorization: Bearer {token}   // read from keyring, scoped to source ID
//! Content-Type: application/json
//!
//! { "jsonrpc": "2.0", "id": 1, "method": "tools/call",
//!   "params": { "name": "mempalace_list_wings", "arguments": {} } }
//! ```
//! Response: `result.content[0].text` → JSON `{ "wings": { "name": count } }`.
//!
//! **Fallback:** if no endpoint is configured (local stdio MemPalace) or the
//! fetch fails, a plain text `InputState` is shown. Created in
//! `handle_toggle_kb_source` (has `&mut Window`), never in render.
//!
//! **Non-MemPalace sources** (Obsidian, Notion, Custom) have no wing field.
//!
//! ## ADRs
//!
//! - ADR-019: Sheet over Dialog for onboarding
//! - ADR-020: Progressive disclosure on every field
//! - ADR-023: KB sources global, connected per project

use std::collections::HashMap;

use gpui::{
    AnyElement, App, AppContext as _, AsyncWindowContext, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    StatefulInteractiveElement, Subscription, Window,
};
use gpui::prelude::FluentBuilder as _;
use gpui::{div, px};
use gpui_component::{
    ActiveTheme as _,
    Sizable as _,
    StyledExt as _,
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    searchable_list::SearchableVec,
    select::{Select, SelectEvent, SelectState},
    switch::Switch,
};

use crate::colors::PodiumColorsExt as _;
use crate::config::{KbConnection, KbSourcesConfig, ProjectEntry};
use crate::ssh_hosts::parse_ssh_hosts;

// ---------------------------------------------------------------------------
// OnboardingEvent
// ---------------------------------------------------------------------------

pub enum OnboardingEvent {
    ProjectCreated(ProjectEntry),
}

// ---------------------------------------------------------------------------
// AgentDraft
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct AgentDraft {
    pub name: String,
    pub purpose: String,
    pub provider: String,
    pub model: String,
}

// ---------------------------------------------------------------------------
// KbConnectionDraft
// ---------------------------------------------------------------------------

/// In-progress KB source connection for Step 5.
///
/// Maps 1:1 to `KbConnection` on Step 7 confirm. `wing` is populated either
/// via a `SelectState` dropdown (remote MemPalace, wings fetched) or a plain
/// `InputState` (local MemPalace, no endpoint). Non-MemPalace sources never
/// set `wing`.
#[derive(Debug, Clone)]
pub struct KbConnectionDraft {
    pub source_id: String,
    pub wing: Option<String>,
}

// ---------------------------------------------------------------------------
// WingFetchState
// ---------------------------------------------------------------------------

/// Lifecycle of the async wing fetch for a single MemPalace source.
///
/// Transitions: absent → Loading (tile selected, fetch fired)
///                     → Loaded(wings) (fetch succeeded; Select entity created)
///                     → Failed(msg)   (fetch failed; Input entity created)
///
/// State is preserved on deselect/reselect — the fetch does not repeat.
#[derive(Debug, Clone)]
enum WingFetchState {
    Loading,
    Loaded(Vec<String>),
    Failed(String),
}

// ---------------------------------------------------------------------------
// OnboardingStep
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingStep {
    Folder = 0,
    Identity = 1,
    Git = 2,
    Agents = 3,
    KbSources = 4,
    Services = 5,
    Confirm = 6,
}

impl OnboardingStep {
    pub const TOTAL: usize = 7;

    pub fn display_number(self) -> usize {
        self as usize + 1
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Folder    => "Choose Folder",
            Self::Identity  => "Project Name",
            Self::Git       => "Git Configuration",
            Self::Agents    => "Agents",
            Self::KbSources => "Knowledge Sources",
            Self::Services  => "Services",
            Self::Confirm   => "Confirm & Create",
        }
    }

    pub fn required(self) -> bool {
        matches!(self, Self::Folder | Self::Identity | Self::Confirm)
    }

    pub fn next(self) -> Option<Self> {
        match self {
            Self::Folder    => Some(Self::Identity),
            Self::Identity  => Some(Self::Git),
            Self::Git       => Some(Self::Agents),
            Self::Agents    => Some(Self::KbSources),
            Self::KbSources => Some(Self::Services),
            Self::Services  => Some(Self::Confirm),
            Self::Confirm   => None,
        }
    }

    pub fn prev(self) -> Option<Self> {
        match self {
            Self::Folder    => None,
            Self::Identity  => Some(Self::Folder),
            Self::Git       => Some(Self::Identity),
            Self::Agents    => Some(Self::Git),
            Self::KbSources => Some(Self::Agents),
            Self::Services  => Some(Self::KbSources),
            Self::Confirm   => Some(Self::Services),
        }
    }
}

// ---------------------------------------------------------------------------
// OnboardingState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OnboardingState {
    pub step: OnboardingStep,

    // Step 1
    pub folder_path: Option<String>,
    pub git_detected: bool,
    pub podium_detected: bool,
    pub detected_remote: Option<String>,

    // Step 2
    pub project_name: String,
    pub project_name_error: Option<String>,

    // Step 3
    pub git_auth: String,
    pub git_account: String,
    pub git_remote: String,
    pub ssh_aliases: Vec<String>,

    // Step 4
    pub agents: Vec<AgentDraft>,

    // Step 5
    /// KB connections being configured. Maps to `Vec<KbConnection>` on confirm.
    pub kb_connections: Vec<KbConnectionDraft>,
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Folder,
            folder_path: None,
            git_detected: false,
            podium_detected: false,
            detected_remote: None,
            project_name: String::new(),
            project_name_error: None,
            git_auth: "https".to_string(),
            git_account: String::new(),
            git_remote: String::new(),
            ssh_aliases: Vec::new(),
            agents: Vec::new(),
            kb_connections: Vec::new(),
        }
    }

    pub fn is_kb_source_selected(&self, source_id: &str) -> bool {
        self.kb_connections.iter().any(|c| c.source_id == source_id)
    }

    pub fn kb_connection_mut(&mut self, source_id: &str) -> Option<&mut KbConnectionDraft> {
        self.kb_connections.iter_mut().find(|c| c.source_id == source_id)
    }

    pub fn advance(&mut self) {
        if let Some(next) = self.step.next() {
            self.step = next;
        }
    }

    pub fn go_back(&mut self) {
        if let Some(prev) = self.step.prev() {
            self.step = prev;
        }
    }
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AgentInputState
// ---------------------------------------------------------------------------

struct AgentInputState {
    name_input:      Entity<InputState>,
    purpose_input:   Entity<InputState>,
    provider_select: Entity<SelectState<SearchableVec<String>>>,
    model_select:    Entity<SelectState<SearchableVec<String>>>,
}

fn models_for_provider(provider: &str) -> SearchableVec<String> {
    match provider {
        "anthropic" => SearchableVec::new(vec![
            "claude-sonnet-4-6".to_string(),
            "claude-opus-4-6".to_string(),
            "claude-haiku-4-5".to_string(),
        ]),
        "openai" => SearchableVec::new(vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "o3".to_string(),
            "o4-mini".to_string(),
        ]),
        "google" => SearchableVec::new(vec![
            "gemini-2.5-pro".to_string(),
            "gemini-2.5-flash".to_string(),
        ]),
        "xai" => SearchableVec::new(vec![
            "grok-3".to_string(),
            "grok-3-mini".to_string(),
        ]),
        "ollama" => SearchableVec::new(vec![
            "llama3.3".to_string(),
            "mistral".to_string(),
            "deepseek-r1".to_string(),
        ]),
        _ => SearchableVec::new(vec![]),
    }
}

fn provider_items() -> SearchableVec<String> {
    SearchableVec::new(vec![
        "anthropic".to_string(),
        "openai".to_string(),
        "google".to_string(),
        "xai".to_string(),
        "ollama".to_string(),
        "custom".to_string(),
    ])
}

// ---------------------------------------------------------------------------
// KB source provider accent colors
// ---------------------------------------------------------------------------

fn provider_accent(provider: &str, cx: &App) -> gpui::Hsla {
    use gpui_component::ActiveTheme as _;
    match provider {
        "mempalace" => cx.theme().primary,
        "obsidian"  => cx.theme().magenta,
        "notion"    => cx.theme().blue,
        _           => cx.theme().muted_foreground,
    }
}

fn provider_abbrev(provider: &str) -> &'static str {
    match provider {
        "mempalace" => "MP",
        "obsidian"  => "Ob",
        "notion"    => "N",
        _           => "?",
    }
}

// ---------------------------------------------------------------------------
// MemPalace wing fetch — MCP JSON-RPC over HTTP
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct WingsPayload {
    wings: HashMap<String, u64>,
}

/// POST to `{endpoint}/mcp` with the `mempalace_list_wings` tool call.
/// Returns sorted display strings: `"showflyer (509 drawers)"`.
async fn fetch_wings_from_mempalace(
    endpoint: String,
    token: Option<String>,
) -> Result<Vec<String>, String> {
    let url = format!("{}/mcp", endpoint.trim_end_matches('/'));

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "mempalace_list_wings", "arguments": {} }
    });

    let client = reqwest::Client::new();
    let mut request = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body);

    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("Network error: {}", error))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|error| format!("Parse error: {}", error))?;

    let text = json
        .pointer("/result/content/0/text")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Unexpected response shape".to_string())?;

    let payload: WingsPayload = serde_json::from_str(text)
        .map_err(|error| format!("Wings parse error: {}", error))?;

    let mut wings: Vec<(String, u64)> = payload.wings.into_iter().collect();
    wings.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(wings
        .into_iter()
        .map(|(name, count)| format!("{} ({} drawers)", name, count))
        .collect())
}

/// Strip the " (N drawers)" suffix from a wing display string.
fn wing_name_from_display(display: &str) -> String {
    display
        .find(" (")
        .map(|index| &display[..index])
        .unwrap_or(display)
        .to_string()
}

// ---------------------------------------------------------------------------
// OnboardingSheet
// ---------------------------------------------------------------------------

pub struct OnboardingSheet {
    state: OnboardingState,
    focus_handle: FocusHandle,

    // Step 2
    name_input: Entity<InputState>,

    // Step 3
    https_input:  Entity<InputState>,
    ssh_select:   Entity<SelectState<SearchableVec<String>>>,
    remote_input: Entity<InputState>,

    // Step 4
    agent_inputs: Vec<AgentInputState>,
    agent_subscriptions: Vec<Vec<Subscription>>,

    // Step 5 — KB Sources
    kb_sources: KbSourcesConfig,

    /// Wing fetch lifecycle per MemPalace source ID.
    wing_fetch_states: HashMap<String, WingFetchState>,

    /// Wing Select entities — created in `create_wing_select` when a fetch
    /// succeeds, inside the `sheet.update` callback which has `&mut Window`.
    /// Keyed by source ID. Render only reads these; never creates them.
    wing_selects: HashMap<String, Entity<SelectState<SearchableVec<String>>>>,

    /// Subscriptions for wing selects. Keyed by source ID.
    wing_select_subscriptions: HashMap<String, Subscription>,

    /// Fallback text inputs for wings when no endpoint / fetch failed.
    /// Created in `create_wing_input` (has `&mut Window` from toggle handler).
    /// Keyed by source ID. Render only reads; never creates.
    wing_inputs: HashMap<String, Entity<InputState>>,

    /// Subscriptions for wing text inputs. Keyed by source ID.
    wing_input_subscriptions: HashMap<String, Subscription>,

    _subscriptions: Vec<Subscription>,
}

impl OnboardingSheet {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ssh_aliases: Vec<String> = dirs::home_dir()
            .map(|home| home.join(".ssh").join("config"))
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|content| parse_ssh_hosts(&content).into_iter().collect())
            .unwrap_or_default();

        let kb_sources = KbSourcesConfig::load().unwrap_or_default();

        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. ShowFlyer")
                .validate(|text, _cx| {
                    text.is_empty()
                        || text.chars().all(|c| {
                            c.is_alphanumeric() || c == ' ' || c == '-' || c == '_'
                        })
                })
        });
        let name_subscription = cx.subscribe(&name_input, |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.state.project_name = input.read(cx).value().to_string();
                this.state.project_name_error = None;
                cx.notify();
            }
        });

        let https_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("GitHub username")
        });
        let https_subscription = cx.subscribe(&https_input, |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.state.git_account = input.read(cx).value().to_string();
                cx.notify();
            }
        });

        let ssh_select = cx.new(|cx| {
            SelectState::new(SearchableVec::new(ssh_aliases.clone()), None, window, cx)
        });
        let ssh_subscription = cx.subscribe(&ssh_select, |this, _select, event, cx| {
            let SelectEvent::Confirm(value) = event;
            this.state.git_account = value.clone().unwrap_or_default();
            cx.notify();
        });

        let remote_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("e.g. git@github.com:user/repo.git")
        });
        let remote_subscription = cx.subscribe(&remote_input, |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.state.git_remote = input.read(cx).value().to_string();
                cx.notify();
            }
        });

        let mut state = OnboardingState::new();
        state.ssh_aliases = ssh_aliases;

        Self {
            state,
            focus_handle: cx.focus_handle(),
            name_input,
            https_input,
            ssh_select,
            remote_input,
            agent_inputs: Vec::new(),
            agent_subscriptions: Vec::new(),
            kb_sources,
            wing_fetch_states: HashMap::new(),
            wing_selects: HashMap::new(),
            wing_select_subscriptions: HashMap::new(),
            wing_inputs: HashMap::new(),
            wing_input_subscriptions: HashMap::new(),
            _subscriptions: vec![
                name_subscription,
                https_subscription,
                ssh_subscription,
                remote_subscription,
            ],
        }
    }

    // --- Navigation ---------------------------------------------------------

    fn handle_next(&mut self, _: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.step == OnboardingStep::Identity
            && self.state.project_name.trim().is_empty()
        {
            self.state.project_name_error = Some("Project name is required.".to_string());
            cx.notify();
            return;
        }

        self.state.advance();
        cx.notify();

        if self.state.step == OnboardingStep::Identity {
            let name = self.state.project_name.clone();
            self.name_input.update(cx, |input, cx| input.set_value(name, window, cx));
        }
        if self.state.step == OnboardingStep::Git {
            self.sync_git_inputs(window, cx);
        }
    }

    fn handle_back(&mut self, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.state.go_back();
        cx.notify();
    }

    fn handle_skip(&mut self, _: &gpui::ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.state.advance();
        cx.notify();
    }

    fn sync_git_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.git_remote.is_empty() {
            if let Some(detected) = self.state.detected_remote.clone() {
                self.state.git_remote = detected;
            }
        }
        let remote = self.state.git_remote.clone();
        self.remote_input.update(cx, |input, cx| input.set_value(remote, window, cx));
        let account = self.state.git_account.clone();
        self.https_input.update(cx, |input, cx| input.set_value(account, window, cx));
    }

    // --- Step 4: agent management -------------------------------------------

    fn handle_add_agent(&mut self, _: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.state.agents.len();
        self.state.agents.push(AgentDraft::default());

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("e.g. Research Agent"));
        let name_sub = cx.subscribe(&name_input, move |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                if let Some(draft) = this.state.agents.get_mut(index) {
                    draft.name = input.read(cx).value().to_string();
                }
                cx.notify();
            }
        });

        let purpose_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("e.g. Finds and synthesizes external sources")
        });
        let purpose_sub = cx.subscribe(&purpose_input, move |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                if let Some(draft) = this.state.agents.get_mut(index) {
                    draft.purpose = input.read(cx).value().to_string();
                }
                cx.notify();
            }
        });

        let provider_select = cx.new(|cx| SelectState::new(provider_items(), None, window, cx));
        let model_select = cx.new(|cx| SelectState::new(SearchableVec::new(vec![]), None, window, cx));

        let model_select_handle = model_select.clone();
        let provider_sub = cx.subscribe_in(
            &provider_select,
            window,
            move |this, _select, event, window, cx| {
                let SelectEvent::Confirm(value) = event;
                let provider = value.clone().unwrap_or_default();
                if let Some(draft) = this.state.agents.get_mut(index) {
                    draft.provider = provider.clone();
                    draft.model = String::new();
                }
                model_select_handle.update(cx, |select, cx| {
                    select.set_items(models_for_provider(&provider), window, cx);
                    select.set_selected_index(None, window, cx);
                });
                cx.notify();
            },
        );

        let model_sub = cx.subscribe(&model_select, move |this, _select, event, cx| {
            let SelectEvent::Confirm(value) = event;
            if let Some(draft) = this.state.agents.get_mut(index) {
                draft.model = value.clone().unwrap_or_default();
            }
            cx.notify();
        });

        self.agent_inputs.push(AgentInputState {
            name_input,
            purpose_input,
            provider_select,
            model_select,
        });
        self.agent_subscriptions.push(vec![name_sub, purpose_sub, provider_sub, model_sub]);
        cx.notify();
    }

    fn handle_remove_agent(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.state.agents.len() {
            self.state.agents.remove(index);
            self.agent_inputs.remove(index);
            self.agent_subscriptions.remove(index);
            cx.notify();
        }
    }

    // --- Step 5: KB source tile toggle and wing fetch -----------------------

    fn handle_toggle_kb_source(
        &mut self,
        source_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.state.is_kb_source_selected(&source_id) {
            self.state.kb_connections.retain(|c| c.source_id != source_id);
            cx.notify();
            return;
        }

        self.state.kb_connections.push(KbConnectionDraft {
            source_id: source_id.clone(),
            wing: None,
        });

        let source = self.kb_sources.find_source(&source_id).cloned();
        let is_mempalace = source.as_ref().map(|s| s.provider == "mempalace").unwrap_or(false);
        let has_endpoint = source.as_ref().and_then(|s| s.endpoint.as_ref()).is_some();

        if is_mempalace && has_endpoint {
            // Only fetch once — preserve result on deselect/reselect
            if !self.wing_fetch_states.contains_key(&source_id) {
                let source = source.expect("confirmed Some above");
                let endpoint = source.endpoint.expect("confirmed Some above");

                let token: Option<String> = keyring::Entry::new("podium", &source_id)
                    .ok()
                    .and_then(|entry| entry.get_password().ok());

                self.wing_fetch_states.insert(source_id.clone(), WingFetchState::Loading);

                let entity = cx.weak_entity();
                let source_id_spawn = source_id.clone();

                window.spawn(cx, async move |cx: &mut AsyncWindowContext| {
                    let result = fetch_wings_from_mempalace(endpoint, token).await;

                    cx.update(|window, cx| {
                        entity.upgrade().map(|sheet| {
                            sheet.update(cx, |this, cx| {
                                match result {
                                    Ok(wings) => {
                                        // Create the Select entity here — we have &mut Window.
                                        // Same pattern as subscribe_in in handle_add_agent:
                                        // entity creation happens where Window is available,
                                        // render only reads.
                                        this.create_wing_select(
                                            &source_id_spawn,
                                            &wings.clone(),
                                            window,
                                            cx,
                                        );
                                        this.wing_fetch_states.insert(
                                            source_id_spawn.clone(),
                                            WingFetchState::Loaded(wings),
                                        );
                                    }
                                    Err(message) => {
                                        this.create_wing_input(&source_id_spawn, window, cx);
                                        this.wing_fetch_states.insert(
                                            source_id_spawn.clone(),
                                            WingFetchState::Failed(message),
                                        );
                                    }
                                }
                                cx.notify();
                            })
                        });
                    }).ok();
                }).detach();
            }
        } else if is_mempalace && !has_endpoint {
            self.create_wing_input(&source_id, window, cx);
        }

        cx.notify();
    }

    /// Create and register a wing `SelectState` for `source_id`.
    /// Called only from contexts with `&mut Window` — never from render.
    fn create_wing_select(
        &mut self,
        source_id: &str,
        wings: &[String],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.wing_selects.contains_key(source_id) {
            return;
        }

        let items = SearchableVec::new(wings.to_vec());
        let select = cx.new(|cx| SelectState::new(items, None, window, cx));

        let source_id_owned = source_id.to_string();
        let subscription = cx.subscribe(&select, move |this, _select, event, cx| {
            let SelectEvent::Confirm(value) = event;
            let display = value.clone().unwrap_or_default();
            let wing = if display.is_empty() {
                None
            } else {
                Some(wing_name_from_display(&display))
            };
            if let Some(draft) = this.state.kb_connection_mut(&source_id_owned) {
                draft.wing = wing;
            }
            cx.notify();
        });

        self.wing_selects.insert(source_id.to_string(), select);
        self.wing_select_subscriptions.insert(source_id.to_string(), subscription);
    }

    /// Create and register a fallback wing `InputState` for `source_id`.
    /// Called only from contexts with `&mut Window` — never from render.
    fn create_wing_input(
        &mut self,
        source_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.wing_inputs.contains_key(source_id) {
            return;
        }

        let input = cx.new(|cx| InputState::new(window, cx).placeholder("e.g. showflyer"));

        let source_id_owned = source_id.to_string();
        let subscription = cx.subscribe(&input, move |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                let value = input.read(cx).value().to_string();
                let wing = if value.is_empty() { None } else { Some(value) };
                if let Some(draft) = this.state.kb_connection_mut(&source_id_owned) {
                    draft.wing = wing;
                }
                cx.notify();
            }
        });

        self.wing_inputs.insert(source_id.to_string(), input);
        self.wing_input_subscriptions.insert(source_id.to_string(), subscription);
    }

    // --- Folder picker ------------------------------------------------------

    fn handle_browse(&mut self, _: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.weak_entity();
        window.spawn(cx, async move |cx: &mut AsyncWindowContext| {
            let folder = rfd::AsyncFileDialog::new().pick_folder().await;
            let Some(folder) = folder else { return; };

            let path = folder.path().to_string_lossy().to_string();
            let folder_path = std::path::Path::new(&path);
            let git_detected = folder_path.join(".git").is_dir();
            let podium_detected = folder_path.join(".podium").is_dir();
            let detected_remote = if git_detected {
                parse_git_remote_url(&folder_path.join(".git").join("config"))
            } else {
                None
            };
            let folder_name = folder_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();

            cx.update(|_window, cx| {
                entity.upgrade().map(|sheet| {
                    sheet.update(cx, |this, cx| {
                        this.state.folder_path = Some(path);
                        this.state.git_detected = git_detected;
                        this.state.podium_detected = podium_detected;
                        this.state.detected_remote = detected_remote;
                        if this.state.project_name.is_empty() {
                            this.state.project_name = folder_name;
                        }
                        cx.notify();
                    })
                });
            }).ok();
        }).detach();
    }

    fn handle_cancel(&mut self, _: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.close_sheet(cx);
    }

    fn handle_confirm(&mut self, _: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let kb_connections = self.state.kb_connections
            .iter()
            .map(|draft| KbConnection {
                source_id: draft.source_id.clone(),
                wing: draft.wing.clone(),
            })
            .collect();

        let entry = ProjectEntry {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.state.project_name.clone(),
            path: self.state.folder_path.clone().unwrap_or_default(),
            last_opened: None,
            git: None,
            kb_connections,
            agents: Vec::new(),
            services: Vec::new(),
        };
        cx.emit(OnboardingEvent::ProjectCreated(entry));
        window.close_sheet(cx);
    }

    // --- Step card dispatcher -----------------------------------------------

    fn render_step_card(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.state.step {
            OnboardingStep::Folder    => self.render_step_folder(cx).into_any_element(),
            OnboardingStep::Identity  => self.render_step_identity(cx).into_any_element(),
            OnboardingStep::Git       => self.render_step_git(cx).into_any_element(),
            OnboardingStep::Agents    => self.render_step_agents(cx).into_any_element(),
            OnboardingStep::KbSources => self.render_step_kb_sources(cx).into_any_element(),
            OnboardingStep::Services  => self.render_step_services(cx).into_any_element(),
            OnboardingStep::Confirm   => self.render_step_confirm(cx).into_any_element(),
        }
    }

    // --- Step 1 — Folder picker ---------------------------------------------

    fn render_step_folder(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let path_display = self.state.folder_path.as_deref().unwrap_or("No folder selected");
        let has_path = self.state.folder_path.is_some();

        div()
            .flex().flex_col().gap_3()
            .child(div().text_sm().text_color(cx.theme().muted_foreground)
                .child("Choose the folder that contains your project."))
            .child(div().p_3().rounded_md().border_1().border_color(cx.theme().border).text_sm()
                .when(!has_path, |this| this.text_color(cx.theme().muted_foreground))
                .when(has_path, |this| this.text_color(cx.theme().foreground))
                .child(path_display.to_string()))
            .when(has_path, |this| {
                this.child(div().flex().flex_row().gap_2()
                    .when(self.state.git_detected, |this| {
                        this.child(div().px_2().py(px(2.)).rounded(px(4.)).text_xs()
                            .bg(cx.theme().secondary).text_color(cx.theme().secondary_foreground)
                            .child("git repo detected"))
                    })
                    .when(self.state.podium_detected, |this| {
                        this.child(div().px_2().py(px(2.)).rounded(px(4.)).text_xs()
                            .bg(cx.theme().secondary).text_color(cx.theme().secondary_foreground)
                            .child(".podium detected"))
                    }))
            })
            .child(Button::new("pick-folder").label("Browse…").outline().small()
                .on_click(cx.listener(Self::handle_browse)))
            .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                .child("The folder you choose becomes the project root. It does not need to be empty."))
    }

    // --- Step 2 — Project name ----------------------------------------------

    fn render_step_identity(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_error = self.state.project_name_error.is_some();

        div()
            .flex().flex_col().gap_3()
            .child(div().text_sm().text_color(cx.theme().muted_foreground)
                .child("Give your project a name."))
            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Project name"))
            .child(Input::new(&self.name_input).when(has_error, |this| this.appearance(true)).small())
            .when(has_error, |this| {
                this.child(div().text_xs().text_color(cx.theme().danger_foreground)
                    .child(self.state.project_name_error.clone().unwrap_or_default()))
            })
            .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                .child("Letters, numbers, spaces, hyphens, and underscores only."))
    }

    // --- Step 3 — Git config ------------------------------------------------

    fn render_step_git(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let use_ssh = self.state.git_auth == "ssh";
        let has_ssh_aliases = !self.state.ssh_aliases.is_empty();

        div()
            .flex().flex_col().gap_4()
            .child(div().text_sm().text_color(cx.theme().muted_foreground)
                .child("Configure git authentication for this project."))
            .child(div().flex().flex_col().gap_2()
                .child(div().text_xs().text_color(cx.theme().muted_foreground)
                    .child("Authentication method"))
                .child(Switch::new("git-auth-toggle")
                    .checked(use_ssh)
                    .label(if use_ssh { "SSH" } else { "HTTPS" })
                    .on_click(cx.listener(|this, checked, _window, cx| {
                        this.state.git_auth = if *checked { "ssh".to_string() } else { "https".to_string() };
                        this.state.git_account = String::new();
                        cx.notify();
                    })))
                .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                    .child("HTTPS uses a Personal Access Token. SSH uses a key from ~/.ssh/config.")))
            .when(!use_ssh, |this| {
                this.child(div().flex().flex_col().gap_2()
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("GitHub account"))
                    .child(Input::new(&self.https_input).small())
                    .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                        .child("Your GitHub username or organization name. Used to route the correct PAT.")))
            })
            .when(use_ssh, |this| {
                this.child(div().flex().flex_col().gap_2()
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("SSH alias"))
                    .child(Select::new(&self.ssh_select).placeholder("Select an SSH alias…")
                        .disabled(!has_ssh_aliases).small())
                    .when(!has_ssh_aliases, |this| {
                        this.child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child("No SSH aliases found in ~/.ssh/config. Add a Host entry for each git account, then reopen onboarding."))
                    })
                    .when(has_ssh_aliases, |this| {
                        this.child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child("Choose the Host alias from ~/.ssh/config that corresponds to this project's git account."))
                    }))
            })
            .child(div().flex().flex_col().gap_2()
                .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Remote URL"))
                .child(Input::new(&self.remote_input).small())
                .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                    .child("Pre-filled from .git/config if detected. Leave blank to set later.")))
    }

    // --- Step 4 — Agents ----------------------------------------------------

    fn render_step_agents(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let agent_count = self.agent_inputs.len();

        div()
            .flex().flex_col().gap_3()
            .child(div().text_sm().text_color(cx.theme().muted_foreground)
                .child("Add AI agents to this project."))
            .children((0..agent_count).map(|index| {
                let widgets = &self.agent_inputs[index];
                let has_provider = !self.state.agents[index].provider.is_empty();
                let provider_has_models = has_provider && self.state.agents[index].provider != "custom";

                div().flex().flex_col().gap_2().p_3().rounded_md().border_1().border_color(cx.theme().border)
                    .child(div().flex().flex_row().items_center().justify_between()
                        .child(div().text_xs().text_color(cx.theme().muted_foreground)
                            .child(format!("Agent {}", index + 1)))
                        .child(Button::new(("remove-agent", index)).label("Remove").ghost().small()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.handle_remove_agent(index, cx);
                            }))))
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Name"))
                    .child(Input::new(&widgets.name_input).small())
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Purpose"))
                    .child(Input::new(&widgets.purpose_input).small())
                    .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                        .child("What does this agent do? Used to route work to the right agent."))
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Provider"))
                    .child(Select::new(&widgets.provider_select).placeholder("Select provider…").small())
                    .when(has_provider && provider_has_models, |this| {
                        this.child(div().text_xs().text_color(cx.theme().muted_foreground).child("Model"))
                            .child(Select::new(&widgets.model_select).placeholder("Select model…").small())
                    })
                    .when(has_provider && !provider_has_models, |this| {
                        this.child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child("Custom endpoint — model name configured after project creation."))
                    })
            }))
            .child(Button::new("add-agent").label("+ Add Agent").outline().small()
                .on_click(cx.listener(Self::handle_add_agent)))
            .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                .child("Agents can be added or removed at any time after project creation."))
    }

    // --- Step 5 — KB sources tile grid (Phase 2 Step 12) -------------------
    //
    // Render is pure read. All entity creation (wing Select and Input) happens
    // in handle_toggle_kb_source and the window.spawn callback — contexts that
    // have &mut Window. Render looks up already-created entities and displays
    // them. This is the same discipline as the provider/model Select pattern
    // in Step 4: creation in handler (subscribe_in), reading in render.

    fn render_step_kb_sources(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sources = &self.kb_sources.sources;
        let is_empty_library = sources.is_empty();

        div()
            .flex().flex_col().gap_3()
            .child(div().text_sm().text_color(cx.theme().muted_foreground)
                .child("Connect knowledge sources to this project."))
            .when(is_empty_library, |this| {
                this.child(div().p_4().rounded_md().border_1().border_color(cx.theme().border)
                    .flex().flex_col().gap_2()
                    .child(div().text_sm().text_color(cx.theme().muted_foreground)
                        .child("No knowledge sources configured yet."))
                    .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                        .child("Add sources to your global library from the Knowledge panel after the project is created.")))
            })
            .when(!is_empty_library, |this| {
                this.child(
                    div().flex().flex_col().gap_2()
                        .children(sources.chunks(2).enumerate().map(|(row_index, row)| {
                            div().flex().flex_row().gap_2()
                                .children(row.iter().enumerate().map(|(col_index, source)| {
                                    let source_id = source.id.clone();
                                    let is_selected = self.state.is_kb_source_selected(&source_id);
                                    let accent = provider_accent(&source.provider, cx);
                                    let abbrev = provider_abbrev(&source.provider);
                                    // ElementId accepts (&str, usize) — str first, index second
                                    let tile_id = ("kb-tile", row_index * 2 + col_index);
                                    let is_mempalace = source.provider == "mempalace";

                                    // Wing expansion — only for selected MemPalace sources.
                                    // Reads already-created entities; never creates them.
                                    let wing_expansion: Option<AnyElement> = if is_selected && is_mempalace {
                                        Some(self.render_wing_expansion(&source_id, cx))
                                    } else {
                                        None
                                    };

                                    div()
                                        .id(tile_id)
                                        .flex().flex_col().flex_1()
                                        .rounded_md().border_1().cursor_pointer()
                                        .when(!is_selected, |this| {
                                            this.border_color(cx.theme().border)
                                                .bg(cx.theme().background)
                                                .hover(|this| {
                                                    this.border_color(cx.theme().muted_foreground.opacity(0.5))
                                                        .bg(cx.theme().secondary.opacity(0.4))
                                                })
                                        })
                                        .when(is_selected, |this| {
                                            this.border_color(cx.theme().primary)
                                                .bg(cx.theme().primary.opacity(0.08))
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.handle_toggle_kb_source(source_id.clone(), window, cx);
                                        }))
                                        // Accent block
                                        .child(div()
                                            .h(px(44.)).w_full().rounded_t_md().bg(accent)
                                            .flex().items_center().justify_center()
                                            .child(div().text_sm().font_bold().text_color(gpui::white())
                                                .child(abbrev)))
                                        // Source name + provider
                                        .child(div().px_3().py_2().flex().flex_col().gap(px(2.))
                                            .child(div().text_sm().text_color(cx.theme().foreground)
                                                .child(source.name.clone()))
                                            .child(div().text_xs().text_color(cx.theme().muted_foreground)
                                                .child(source.provider.clone())))
                                        // Connected badge
                                        .when(is_selected, |this| {
                                            this.child(div().px_3().pb_2().flex().justify_end()
                                                .child(div().px(px(6.)).py(px(2.)).rounded(px(4.))
                                                    .bg(cx.theme().primary).text_xs()
                                                    .text_color(cx.theme().primary_foreground)
                                                    .child("✓ Connected")))
                                        })
                                        // Wing expansion
                                        .when_some(wing_expansion, |this, expansion| {
                                            this.child(expansion)
                                        })
                                }))
                        }))
                )
            })
            .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                .child("Sources can be connected or disconnected from the Knowledge panel at any time."))
    }

    /// Render the wing field expansion for a selected MemPalace source.
    ///
    /// Pure read — only looks up entities that were created by
    /// `handle_toggle_kb_source` / `create_wing_select` / `create_wing_input`.
    fn render_wing_expansion(&self, source_id: &str, cx: &mut Context<Self>) -> AnyElement {
        match self.wing_fetch_states.get(source_id) {
            Some(WingFetchState::Loading) => {
                div().px_3().pb_3().flex().flex_col().gap_1()
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Wing"))
                    .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                        .child("Fetching wings from MemPalace…"))
                    .into_any_element()
            }
            Some(WingFetchState::Loaded(_)) => {
                if let Some(select) = self.wing_selects.get(source_id) {
                    div().px_3().pb_3().flex().flex_col().gap_1()
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Wing"))
                        .child(Select::new(select).placeholder("Select wing…").small())
                        .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child("The MemPalace wing for this project's content."))
                        .into_any_element()
                } else {
                    div().px_3().pb_3()
                        .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child("Fetching wings from MemPalace…"))
                        .into_any_element()
                }
            }
            Some(WingFetchState::Failed(error)) => {
                let hint = format!("Could not fetch wings: {}. Enter name manually.", error);
                if let Some(input) = self.wing_inputs.get(source_id) {
                    div().px_3().pb_3().flex().flex_col().gap_1()
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Wing"))
                        .child(Input::new(input).small())
                        .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child(hint))
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            }
            None => {
                // No endpoint — local MemPalace, text input fallback
                if let Some(input) = self.wing_inputs.get(source_id) {
                    div().px_3().pb_3().flex().flex_col().gap_1()
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Wing"))
                        .child(Input::new(input).small())
                        .child(div().text_xs().text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child("Local MemPalace — enter wing name manually (e.g. showflyer)."))
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            }
        }
    }

    // --- Step 6 — Services stub (Phase 2 Step 13) ---------------------------

    fn render_step_services(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_3()
            .child(div().text_sm().text_color(cx.theme().muted_foreground)
                .child("Add external service connections for the Health tab. (Optional)"))
    }

    // --- Step 7 — Confirm & create ------------------------------------------

    fn render_step_confirm(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex().flex_col().gap_3()
            .child(div().text_sm().text_color(cx.theme().muted_foreground)
                .child("Review your project settings before creating."))
            .when(!self.state.project_name.is_empty(), |this| {
                this.child(div().flex().flex_col().gap_1()
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Project name"))
                    .child(div().text_sm().text_color(cx.theme().foreground)
                        .child(self.state.project_name.clone())))
            })
            .when(self.state.folder_path.is_some(), |this| {
                this.child(div().flex().flex_col().gap_1()
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Folder"))
                    .child(div().text_sm().text_color(cx.theme().foreground)
                        .child(self.state.folder_path.clone().unwrap_or_default())))
            })
            .when(!self.state.git_remote.is_empty() || !self.state.git_account.is_empty(), |this| {
                this.child(div().flex().flex_col().gap_1()
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Git"))
                    .child(div().text_sm().text_color(cx.theme().foreground)
                        .child(format!(
                            "{} — {}",
                            self.state.git_auth,
                            if self.state.git_account.is_empty() {
                                "no account set".to_string()
                            } else {
                                self.state.git_account.clone()
                            }
                        ))))
            })
            .when(!self.state.agents.is_empty(), |this| {
                this.child(div().flex().flex_col().gap_1()
                    .child(div().text_xs().text_color(cx.theme().muted_foreground).child("Agents"))
                    .child(div().text_sm().text_color(cx.theme().foreground)
                        .child(format!(
                            "{} agent{}",
                            self.state.agents.len(),
                            if self.state.agents.len() == 1 { "" } else { "s" }
                        ))))
            })
            .when(!self.state.kb_connections.is_empty(), |this| {
                this.child(div().flex().flex_col().gap_1()
                    .child(div().text_xs().text_color(cx.theme().muted_foreground)
                        .child("Knowledge sources"))
                    .child(div().text_sm().text_color(cx.theme().foreground)
                        .child(format!(
                            "{} source{} connected",
                            self.state.kb_connections.len(),
                            if self.state.kb_connections.len() == 1 { "" } else { "s" }
                        ))))
            })
    }

    // --- Navigation button row ----------------------------------------------

    fn render_nav_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let step = self.state.step;
        let is_first = step == OnboardingStep::Folder;
        let is_last = step == OnboardingStep::Confirm;
        let is_skippable = !step.required();

        div().flex().flex_row().items_center().gap_2()
            .child(Button::new("cancel").label("Cancel").ghost().small()
                .on_click(cx.listener(Self::handle_cancel)))
            .child(div().flex_1())
            .when(!is_first, |this| {
                this.child(Button::new("back").label("← Back").outline().small()
                    .on_click(cx.listener(Self::handle_back)))
            })
            .when(is_skippable, |this| {
                this.child(Button::new("skip").label("Skip").ghost().small()
                    .on_click(cx.listener(Self::handle_skip)))
            })
            .when(!is_last, |this| {
                this.child(Button::new("next").label("Next →").primary().small()
                    .on_click(cx.listener(Self::handle_next)))
            })
            .when(is_last, |this| {
                this.child(Button::new("confirm").label("Create Project").primary().small()
                    .on_click(cx.listener(Self::handle_confirm)))
            })
    }
}

// ---------------------------------------------------------------------------
// GPUI trait impls
// ---------------------------------------------------------------------------

impl EventEmitter<OnboardingEvent> for OnboardingSheet {}

impl Focusable for OnboardingSheet {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OnboardingSheet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.podium_colors();
        let step = self.state.step;
        let step_content = self.render_step_card(cx);
        let nav_buttons = self.render_nav_buttons(cx);

        div()
            .flex().flex_col().h_full().p_4().gap_4()
            .child(div().flex().items_center().justify_between()
                .child(div().text_xs().text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "Step {} of {} — {}",
                        step.display_number(),
                        OnboardingStep::TOTAL,
                        step.label(),
                    )))
                .child(div().flex().items_center().gap_1()
                    .children((0..OnboardingStep::TOTAL).map(|i| {
                        let is_current = i == step as usize;
                        let is_done = i < step as usize;
                        div().w(px(6.)).h(px(6.)).rounded_full()
                            .when(is_current, |this| this.bg(colors.tab_active_indicator))
                            .when(is_done, |this| this.bg(colors.tab_active_indicator.opacity(0.4)))
                            .when(!is_current && !is_done, |this| {
                                this.bg(cx.theme().muted_foreground.opacity(0.3))
                            })
                    }))))
            .child(div().flex_1().overflow_hidden().child(step_content))
            .child(nav_buttons)
    }
}

// ---------------------------------------------------------------------------
// Git config parsing
// ---------------------------------------------------------------------------

fn parse_git_remote_url(git_config_path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(git_config_path).ok()?;
    let mut in_origin_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == r#"[remote "origin"]"# {
            in_origin_section = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_origin_section = false;
            continue;
        }
        if in_origin_section {
            if let Some(rest) = trimmed.strip_prefix("url =") {
                let url = rest.trim().to_string();
                if !url.is_empty() {
                    return Some(url);
                }
            }
        }
    }

    None
}
