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
//! | 5 — KB Sources | No  | KB source connections |
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
//! ## ADRs
//!
//! - ADR-019: Sheet over Dialog for onboarding
//! - ADR-020: Progressive disclosure on every field

use gpui::{
    AnyElement, App, AppContext as _, AsyncWindowContext, Context, Entity, EventEmitter,
    FocusHandle, Focusable, IntoElement, ParentElement, Render, Styled, Subscription, Window,
};
use gpui::prelude::FluentBuilder as _;
use gpui::{div, px};
use gpui_component::{
    ActiveTheme as _,
    Sizable as _,
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    searchable_list::SearchableVec,
    select::{Select, SelectEvent, SelectState},
    switch::Switch,
};

use crate::colors::PodiumColorsExt as _;
use crate::config::ProjectEntry;
use crate::ssh_hosts::parse_ssh_hosts;

// ---------------------------------------------------------------------------
// OnboardingEvent — the single outbound signal from OnboardingSheet
// ---------------------------------------------------------------------------

/// Events emitted by `OnboardingSheet` to `PodiumApp`.
///
/// `ProjectCreated` carries the completed `ProjectEntry` ready to be written
/// to `projects.toml` and loaded. It is emitted exactly once, on Step 7
/// confirm, immediately before the Sheet is closed.
pub enum OnboardingEvent {
    /// Step 7 confirmed — project entry is ready for persistence and load.
    ProjectCreated(ProjectEntry),
}

// ---------------------------------------------------------------------------
// AgentDraft — in-progress agent entry during onboarding
// ---------------------------------------------------------------------------

/// A partially-configured agent being built in Step 4.
///
/// Distinct from `AgentEntry` in `config.rs` — the onboarding form is
/// incomplete until Step 7 confirm. `AgentDraft` carries only what the user
/// fills in during Step 4. Remaining fields (id, kb_sources, endpoint) are
/// filled or defaulted at project creation time.
#[derive(Debug, Clone, Default)]
pub struct AgentDraft {
    /// Display name for this agent.
    pub name: String,
    /// Free-text description of what this agent does.
    pub purpose: String,
    /// Provider identifier: `"anthropic"`, `"openai"`, `"google"`, `"xai"`,
    /// `"ollama"`, or `"custom"`.
    pub provider: String,
    /// Model identifier — provider-specific string.
    pub model: String,
}

// ---------------------------------------------------------------------------
// OnboardingStep
// ---------------------------------------------------------------------------

/// The seven onboarding steps.
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

    /// One-based step number for display ("1 of 7").
    pub fn display_number(self) -> usize {
        self as usize + 1
    }

    /// Human-readable step label.
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

    /// True if this step cannot be skipped.
    pub fn required(self) -> bool {
        matches!(self, Self::Folder | Self::Identity | Self::Confirm)
    }

    /// The step that follows this one, or `None` if this is the last.
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

    /// The step that precedes this one, or `None` if this is the first.
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
// OnboardingState — owned by OnboardingSheet
// ---------------------------------------------------------------------------

/// All in-progress form data for the onboarding flow.
///
/// Owned directly by `OnboardingSheet`. Mutated in place on each navigation
/// action — no cloning required, no external state on `PodiumApp`.
#[derive(Debug, Clone)]
pub struct OnboardingState {
    pub step: OnboardingStep,

    // Step 1 — Folder
    /// Absolute path chosen by the user via the folder picker.
    pub folder_path: Option<String>,
    /// True if a `.git/` directory was detected inside `folder_path`.
    pub git_detected: bool,
    /// True if a `.podium/` directory was detected inside `folder_path`.
    pub podium_detected: bool,
    /// Remote URL parsed from `.git/config` if detected.
    pub detected_remote: Option<String>,

    // Step 2 — Identity
    /// Project display name — kept in sync with the `name_input` InputState.
    pub project_name: String,
    /// Validation error message for the project name field, if any.
    pub project_name_error: Option<String>,

    // Step 3 — Git (skippable)
    /// Authentication method: `"https"` (default) or `"ssh"`.
    pub git_auth: String,
    /// HTTPS: GitHub username. SSH: SSH config alias selected from dropdown.
    pub git_account: String,
    /// Remote URL — pre-filled from detection or editable.
    pub git_remote: String,
    /// SSH aliases parsed from `~/.ssh/config` at sheet creation time.
    /// Empty if the file is absent or contains no non-provider host entries.
    pub ssh_aliases: Vec<String>,

    // Step 4 — Agents (skippable)
    /// Agent drafts being built in Step 4. Empty if the user skips.
    /// Index matches `OnboardingSheet::agent_inputs`.
    pub agents: Vec<AgentDraft>,
}

impl OnboardingState {
    /// Create a fresh `OnboardingState` at Step 1 with all fields at defaults.
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
        }
    }

    /// Advance to the next step. No-op on the last step.
    pub fn advance(&mut self) {
        if let Some(next) = self.step.next() {
            self.step = next;
        }
    }

    /// Go back to the previous step. No-op on the first step.
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
// AgentInputState — widget state for one agent card
// ---------------------------------------------------------------------------

/// Widget entities for a single agent entry in Step 4.
///
/// A plain Rust struct — not a GPUI entity. Owns four GPUI entity handles.
/// Dropping this struct releases all four handles; GPUI garbage-collects
/// the entities on the next cycle.
///
/// Subscriptions for these entities are stored separately in
/// `OnboardingSheet::agent_subscriptions[i]`. Removing agent `i` calls
/// `remove(i)` on both `agent_inputs` and `agent_subscriptions`, atomically
/// dropping the entities and deregistering the subscriptions.
struct AgentInputState {
    name_input:      Entity<InputState>,
    purpose_input:   Entity<InputState>,
    provider_select: Entity<SelectState<SearchableVec<String>>>,
    model_select:    Entity<SelectState<SearchableVec<String>>>,
}

/// Returns the curated model list for a given provider identifier.
///
/// Called when creating a new agent card (to seed the model select) and when
/// the provider changes (to replace the model list via `set_items`).
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
        // Custom endpoint — no curated list; user types the model name.
        _ => SearchableVec::new(vec![]),
    }
}

/// Provider list shown in the provider dropdown for every agent card.
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
// OnboardingSheet — GPUI entity
// ---------------------------------------------------------------------------

/// The onboarding Sheet view — a proper GPUI entity that owns its state and
/// drives all navigation internally via `cx.listener`.
///
/// ## Input entity ownership
///
/// All GPUI widget entities are owned by `OnboardingSheet`. Static inputs
/// (Steps 2 and 3) are created in `new()`. Dynamic inputs (Step 4 agents)
/// are created in `handle_add_agent`.
///
/// All subscriptions follow Zed Standard Rule 8 — `Vec<Subscription>` field
/// pattern. Static subscriptions live in `_subscriptions`. Per-agent
/// subscriptions live in `agent_subscriptions[i]`.
///
/// ## Sync pattern
///
/// - **User edits → state** (subscription): entity emits → subscription
///   fires → writes `state` field → `cx.notify()`
/// - **State → widget** (imperative push): `set_value` / `set_selected_index`
///   called when navigating into a pre-filled step. These do NOT re-emit
///   change events, so no subscription loop occurs.
///
/// ## Provider / model coupling
///
/// The provider subscription uses `cx.subscribe_in` (source-confirmed API
/// from `gpui/src/app/context.rs`) which provides `&mut Window` in the
/// callback. This allows `set_items` and `set_selected_index` to be called
/// directly from the stored subscription, keeping all four agent subscriptions
/// in the same place (`handle_add_agent`) with no render-time splits.
pub struct OnboardingSheet {
    state: OnboardingState,
    focus_handle: FocusHandle,

    // Step 2 — Identity
    name_input: Entity<InputState>,

    // Step 3 — Git
    https_input:  Entity<InputState>,
    ssh_select:   Entity<SelectState<SearchableVec<String>>>,
    remote_input: Entity<InputState>,

    // Step 4 — Agents (dynamic)
    /// Widget entities for each agent card. Index matches `state.agents`.
    agent_inputs: Vec<AgentInputState>,
    /// Subscriptions for each agent's inputs. `agent_subscriptions[i]` holds
    /// all four subscriptions for `agent_inputs[i]`. Dropping index `i`
    /// deregisters those subscriptions atomically with the entity drop.
    agent_subscriptions: Vec<Vec<Subscription>>,

    /// Subscriptions for the static Step 2 and Step 3 entities.
    _subscriptions: Vec<Subscription>,
}

impl OnboardingSheet {
    /// Create a new `OnboardingSheet` entity at Step 1.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // --- Parse SSH aliases from ~/.ssh/config ---------------------------
        let ssh_aliases: Vec<String> = dirs::home_dir()
            .map(|home| home.join(".ssh").join("config"))
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|content| parse_ssh_hosts(&content).into_iter().collect())
            .unwrap_or_default();

        // --- Step 2: project name input -------------------------------------

        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. ShowFlyer")
                .validate(|text, _cx| {
                    text.is_empty()
                        || text
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
                })
        });

        let name_subscription = cx.subscribe(
            &name_input,
            |this, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let new_name = input.read(cx).value().to_string();
                    this.state.project_name = new_name;
                    this.state.project_name_error = None;
                    cx.notify();
                }
            },
        );

        // --- Step 3: HTTPS username input -----------------------------------

        let https_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("GitHub username")
        });

        let https_subscription = cx.subscribe(
            &https_input,
            |this, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.state.git_account = input.read(cx).value().to_string();
                    cx.notify();
                }
            },
        );

        // --- Step 3: SSH alias select ---------------------------------------

        let ssh_items = SearchableVec::new(ssh_aliases.clone());
        let ssh_select = cx.new(|cx| SelectState::new(ssh_items, None, window, cx));

        let ssh_subscription = cx.subscribe(
            &ssh_select,
            |this, _select, event, cx| {
                let SelectEvent::Confirm(value) = event;
                this.state.git_account = value.clone().unwrap_or_default();
                cx.notify();
            },
        );

        // --- Step 3: remote URL input ---------------------------------------

        let remote_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. git@github.com:user/repo.git")
        });

        let remote_subscription = cx.subscribe(
            &remote_input,
            |this, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.state.git_remote = input.read(cx).value().to_string();
                    cx.notify();
                }
            },
        );

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
            _subscriptions: vec![
                name_subscription,
                https_subscription,
                ssh_subscription,
                remote_subscription,
            ],
        }
    }

    // --- Navigation handlers ------------------------------------------------

    fn handle_next(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.state.step == OnboardingStep::Identity {
            if self.state.project_name.trim().is_empty() {
                self.state.project_name_error =
                    Some("Project name is required.".to_string());
                cx.notify();
                return;
            }
        }

        self.state.advance();
        cx.notify();

        if self.state.step == OnboardingStep::Identity {
            self.sync_name_input_to_state(window, cx);
        }
        if self.state.step == OnboardingStep::Git {
            self.sync_git_inputs_to_state(window, cx);
        }
    }

    fn handle_back(
        &mut self,
        _: &gpui::ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.go_back();
        cx.notify();
    }

    fn handle_skip(
        &mut self,
        _: &gpui::ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.advance();
        cx.notify();
    }

    fn sync_name_input_to_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current_name = self.state.project_name.clone();
        self.name_input.update(cx, |input, cx| {
            input.set_value(current_name, window, cx);
        });
    }

    fn sync_git_inputs_to_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.git_remote.is_empty() {
            if let Some(detected) = self.state.detected_remote.clone() {
                self.state.git_remote = detected;
            }
        }

        let remote_value = self.state.git_remote.clone();
        self.remote_input.update(cx, |input, cx| {
            input.set_value(remote_value, window, cx);
        });

        let account_value = self.state.git_account.clone();
        self.https_input.update(cx, |input, cx| {
            input.set_value(account_value, window, cx);
        });
    }

    // --- Step 4: agent add / remove -----------------------------------------

    fn handle_add_agent(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let index = self.state.agents.len();
        self.state.agents.push(AgentDraft::default());

        let name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("e.g. Research Agent")
        });
        let name_sub = cx.subscribe(&name_input, move |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                if let Some(draft) = this.state.agents.get_mut(index) {
                    draft.name = input.read(cx).value().to_string();
                }
                cx.notify();
            }
        });

        let purpose_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. Finds and synthesizes external sources")
        });
        let purpose_sub = cx.subscribe(&purpose_input, move |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                if let Some(draft) = this.state.agents.get_mut(index) {
                    draft.purpose = input.read(cx).value().to_string();
                }
                cx.notify();
            }
        });

        let provider_select = cx.new(|cx| {
            SelectState::new(provider_items(), None, window, cx)
        });

        let model_select = cx.new(|cx| {
            SelectState::new(SearchableVec::new(vec![]), None, window, cx)
        });

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

                let new_models = models_for_provider(&provider);
                model_select_handle.update(cx, |select, cx| {
                    select.set_items(new_models, window, cx);
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
        self.agent_subscriptions.push(vec![
            name_sub,
            purpose_sub,
            provider_sub,
            model_sub,
        ]);

        cx.notify();
    }

    fn handle_remove_agent(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.state.agents.len() {
            return;
        }
        self.state.agents.remove(index);
        self.agent_inputs.remove(index);
        self.agent_subscriptions.remove(index);
        cx.notify();
    }

    // --- Async folder picker ------------------------------------------------

    fn handle_browse(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
            })
            .ok();
        })
        .detach();
    }

    fn handle_cancel(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.close_sheet(cx);
    }

    fn handle_confirm(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entry = ProjectEntry {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.state.project_name.clone(),
            path: self.state.folder_path.clone().unwrap_or_default(),
            last_opened: None,
            git: None,
            kb_connections: Vec::new(),
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

    // --- Step 1 — Folder picker (Phase 2 Step 8) ----------------------------

    fn render_step_folder(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let path_display = self.state.folder_path
            .as_deref()
            .unwrap_or("No folder selected");
        let has_path = self.state.folder_path.is_some();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Choose the folder that contains your project."),
            )
            .child(
                div()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .text_sm()
                    .when(!has_path, |this| this.text_color(cx.theme().muted_foreground))
                    .when(has_path, |this| this.text_color(cx.theme().foreground))
                    .child(path_display.to_string()),
            )
            .when(has_path, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .when(self.state.git_detected, |this| {
                            this.child(
                                div()
                                    .px_2()
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .text_xs()
                                    .bg(cx.theme().secondary)
                                    .text_color(cx.theme().secondary_foreground)
                                    .child("git repo detected"),
                            )
                        })
                        .when(self.state.podium_detected, |this| {
                            this.child(
                                div()
                                    .px_2()
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .text_xs()
                                    .bg(cx.theme().secondary)
                                    .text_color(cx.theme().secondary_foreground)
                                    .child(".podium detected"),
                            )
                        }),
                )
            })
            .child(
                Button::new("pick-folder")
                    .label("Browse…")
                    .outline()
                    .small()
                    .on_click(cx.listener(Self::handle_browse)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground.opacity(0.6))
                    .child("The folder you choose becomes the project root. It does not need to be empty."),
            )
    }

    // --- Step 2 — Project name (Phase 2 Step 9) -----------------------------

    fn render_step_identity(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_error = self.state.project_name_error.is_some();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Give your project a name."),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Project name"),
            )
            .child(
                Input::new(&self.name_input)
                    .when(has_error, |this| this.appearance(true))
                    .small(),
            )
            .when(has_error, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger_foreground)
                        .child(self.state.project_name_error.clone().unwrap_or_default()),
                )
            })
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground.opacity(0.6))
                    .child("Letters, numbers, spaces, hyphens, and underscores only."),
            )
    }

    // --- Step 3 — Git config (Phase 2 Step 10) ------------------------------

    fn render_step_git(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let use_ssh = self.state.git_auth == "ssh";
        let has_ssh_aliases = !self.state.ssh_aliases.is_empty();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Configure git authentication for this project."),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Authentication method"),
                    )
                    .child(
                        Switch::new("git-auth-toggle")
                            .checked(use_ssh)
                            .label(if use_ssh { "SSH" } else { "HTTPS" })
                            .on_click(cx.listener(|this, checked, _window, cx| {
                                this.state.git_auth = if *checked {
                                    "ssh".to_string()
                                } else {
                                    "https".to_string()
                                };
                                this.state.git_account = String::new();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child("HTTPS uses a Personal Access Token. SSH uses a key from ~/.ssh/config."),
                    ),
            )
            .when(!use_ssh, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("GitHub account"),
                        )
                        .child(Input::new(&self.https_input).small())
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground.opacity(0.6))
                                .child("Your GitHub username or organization name. Used to route the correct PAT."),
                        ),
                )
            })
            .when(use_ssh, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("SSH alias"),
                        )
                        .child(
                            Select::new(&self.ssh_select)
                                .placeholder("Select an SSH alias…")
                                .disabled(!has_ssh_aliases)
                                .small(),
                        )
                        .when(!has_ssh_aliases, |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground.opacity(0.6))
                                    .child("No SSH aliases found in ~/.ssh/config. Add a Host entry for each git account, then reopen onboarding."),
                            )
                        })
                        .when(has_ssh_aliases, |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground.opacity(0.6))
                                    .child("Choose the Host alias from ~/.ssh/config that corresponds to this project's git account."),
                            )
                        }),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Remote URL"),
                    )
                    .child(Input::new(&self.remote_input).small())
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child("Pre-filled from .git/config if detected. Leave blank to set later."),
                    ),
            )
    }

    // --- Step 4 — Agent config (Phase 2 Step 11) ----------------------------

    fn render_step_agents(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let agent_count = self.agent_inputs.len();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Add AI agents to this project."),
            )
            .children((0..agent_count).map(|index| {
                let widgets = &self.agent_inputs[index];
                let has_provider = !self.state.agents[index].provider.is_empty();
                let provider_has_models =
                    !self.state.agents[index].provider.is_empty()
                    && self.state.agents[index].provider != "custom";

                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Agent {}", index + 1)),
                            )
                            .child(
                                Button::new(("remove-agent", index))
                                    .label("Remove")
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.handle_remove_agent(index, cx);
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Name"),
                    )
                    .child(Input::new(&widgets.name_input).small())
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Purpose"),
                    )
                    .child(Input::new(&widgets.purpose_input).small())
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground.opacity(0.6))
                            .child("What does this agent do? Used to route work to the right agent."),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Provider"),
                    )
                    .child(
                        Select::new(&widgets.provider_select)
                            .placeholder("Select provider…")
                            .small(),
                    )
                    .when(has_provider && provider_has_models, |this| {
                        this
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Model"),
                            )
                            .child(
                                Select::new(&widgets.model_select)
                                    .placeholder("Select model…")
                                    .small(),
                            )
                    })
                    .when(has_provider && !provider_has_models, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground.opacity(0.6))
                                .child("Custom endpoint — model name configured after project creation."),
                        )
                    })
            }))
            .child(
                Button::new("add-agent")
                    .label("+ Add Agent")
                    .outline()
                    .small()
                    .on_click(cx.listener(Self::handle_add_agent)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground.opacity(0.6))
                    .child("Agents can be added or removed at any time after project creation."),
            )
    }

    // --- Step card stubs — replaced in build order steps 12–14 -------------

    fn render_step_kb_sources(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Connect knowledge base sources to this project. (Optional)"),
            )
    }

    fn render_step_services(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Add external service connections for the Health tab. (Optional)"),
            )
    }

    fn render_step_confirm(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Review your project settings before creating."),
            )
            .when(!self.state.project_name.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Project name"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .child(self.state.project_name.clone()),
                        ),
                )
            })
            .when(self.state.folder_path.is_some(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Folder"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .child(self.state.folder_path.clone().unwrap_or_default()),
                        ),
                )
            })
            .when(!self.state.git_remote.is_empty() || !self.state.git_account.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Git"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .child(format!(
                                    "{} — {}",
                                    self.state.git_auth,
                                    if self.state.git_account.is_empty() {
                                        "no account set".to_string()
                                    } else {
                                        self.state.git_account.clone()
                                    }
                                )),
                        ),
                )
            })
            .when(!self.state.agents.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Agents"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .child(format!(
                                    "{} agent{}",
                                    self.state.agents.len(),
                                    if self.state.agents.len() == 1 { "" } else { "s" }
                                )),
                        ),
                )
            })
    }

    // --- Navigation button row ----------------------------------------------

    fn render_nav_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let step = self.state.step;
        let is_first = step == OnboardingStep::Folder;
        let is_last = step == OnboardingStep::Confirm;
        let is_skippable = !step.required();

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                Button::new("cancel")
                    .label("Cancel")
                    .ghost()
                    .small()
                    .on_click(cx.listener(Self::handle_cancel)),
            )
            .child(div().flex_1())
            .when(!is_first, |this| {
                this.child(
                    Button::new("back")
                        .label("← Back")
                        .outline()
                        .small()
                        .on_click(cx.listener(Self::handle_back)),
                )
            })
            .when(is_skippable, |this| {
                this.child(
                    Button::new("skip")
                        .label("Skip")
                        .ghost()
                        .small()
                        .on_click(cx.listener(Self::handle_skip)),
                )
            })
            .when(!is_last, |this| {
                this.child(
                    Button::new("next")
                        .label("Next →")
                        .primary()
                        .small()
                        .on_click(cx.listener(Self::handle_next)),
                )
            })
            .when(is_last, |this| {
                this.child(
                    Button::new("confirm")
                        .label("Create Project")
                        .primary()
                        .small()
                        .on_click(cx.listener(Self::handle_confirm)),
                )
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
            .flex()
            .flex_col()
            .h_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "Step {} of {} — {}",
                                step.display_number(),
                                OnboardingStep::TOTAL,
                                step.label(),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .children((0..OnboardingStep::TOTAL).map(|i| {
                                let is_current = i == step as usize;
                                let is_done = i < step as usize;
                                div()
                                    .w(px(6.))
                                    .h(px(6.))
                                    .rounded_full()
                                    .when(is_current, |this| {
                                        this.bg(colors.tab_active_indicator)
                                    })
                                    .when(is_done, |this| {
                                        this.bg(colors.tab_active_indicator.opacity(0.4))
                                    })
                                    .when(!is_current && !is_done, |this| {
                                        this.bg(cx.theme().muted_foreground.opacity(0.3))
                                    })
                            })),
                    ),
            )
            .child(div().flex_1().overflow_hidden().child(step_content))
            .child(nav_buttons)
    }
}

// ---------------------------------------------------------------------------
// Git config parsing
// ---------------------------------------------------------------------------

/// Parse the remote origin URL from a `.git/config` file.
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
