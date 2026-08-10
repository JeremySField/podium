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
//! | 3 — Git        | No  | SSH/HTTPS auth, remote URL |
//! | 4 — Agents     | No  | Agent roster |
//! | 5 — KB Sources | No  | KB source connections |
//! | 6 — Services   | No  | External service connections |
//! | 7 — Confirm    | —   | Summary + Create Project button |
//!
//! Steps 1 and 2 are required. Steps 3–6 are skippable. Step 7 has no Skip.
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
};

use crate::colors::PodiumColorsExt as _;
use crate::config::ProjectEntry;

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
    /// HTTPS: GitHub username. SSH: SSH config alias.
    pub git_account: String,
    /// Remote URL — pre-filled from detection or editable.
    pub git_remote: String,
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
// OnboardingSheet — GPUI entity
// ---------------------------------------------------------------------------

/// The onboarding Sheet view — a proper GPUI entity that owns its state and
/// drives all navigation internally via `cx.listener`.
///
/// ## Input entity ownership
///
/// `name_input` is an `Entity<InputState>` owned by `OnboardingSheet`. It is
/// created in `new()` alongside a subscription to `InputEvent::Change` that
/// keeps `state.project_name` in sync. The subscription is stored in
/// `_subscriptions` so it stays alive for the sheet's lifetime.
///
/// When Step 8 (folder picker) pre-fills `state.project_name`, the render
/// path for Step 2 calls `input.set_value(...)` to push that value into the
/// `InputState` so the widget displays it correctly. This is a one-way push
/// from `OnboardingState` → `InputState` on render; the subscription handles
/// the reverse direction (user types → `InputState` emits → `state.project_name`
/// updates).
pub struct OnboardingSheet {
    state: OnboardingState,
    focus_handle: FocusHandle,
    /// `InputState` entity for the project name field (Step 2).
    name_input: Entity<InputState>,
    /// All subscriptions for this entity. Stored per Zed Standard Rule 8 —
    /// `Vec<Subscription>` field pattern; subscriptions deregister on drop.
    _subscriptions: Vec<Subscription>,
}

impl OnboardingSheet {
    /// Create a new `OnboardingSheet` entity at Step 1.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Create the project name InputState with single-line mode (default),
        // placeholder text, and a validate closure that enforces the allowed
        // character set: letters, numbers, spaces, hyphens, underscores only.
        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. ShowFlyer")
                .validate(|text, _cx| {
                    // Empty is allowed — the Next button guards against empty on Step 2.
                    // Non-empty must match: letters, digits, spaces, hyphens, underscores.
                    text.is_empty()
                        || text
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
                })
        });

        // Subscribe to InputEvent::Change to keep state.project_name in sync.
        // The subscription fires on every keystroke after validation passes.
        let name_subscription = cx.subscribe(
            &name_input,
            |this, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let new_name = input.read(cx).value().to_string();
                    this.state.project_name = new_name;
                    // Clear any prior validation error once the user starts editing.
                    this.state.project_name_error = None;
                    cx.notify();
                }
            },
        );

        Self {
            state: OnboardingState::new(),
            focus_handle: cx.focus_handle(),
            name_input,
            _subscriptions: vec![name_subscription],
        }
    }

    // --- Navigation handlers (bound via cx.listener in render) --------------

    fn handle_next(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Step 2 guard: require a non-empty, valid project name before advancing.
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

        // If we just advanced into Step 2, sync the pre-filled name (from
        // folder picker) into the InputState so the widget displays it.
        if self.state.step == OnboardingStep::Identity {
            self.sync_name_input_to_state(window, cx);
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
        // Skip advances past the current optional step — same as Next.
        self.state.advance();
        cx.notify();
    }

    /// Push `state.project_name` into the `name_input` InputState.
    ///
    /// Called when navigating into Step 2 so that a name pre-filled by the
    /// folder picker (Step 1) is displayed in the Input widget. `set_value`
    /// does not emit `InputEvent::Change`, so this does not re-trigger the
    /// subscription and cause a loop.
    fn sync_name_input_to_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current_name = self.state.project_name.clone();
        self.name_input.update(cx, |input, cx| {
            input.set_value(current_name, window, cx);
        });
    }

    // --- Async folder picker ------------------------------------------------

    /// Open the OS native folder picker and apply the result to state.
    ///
    /// Uses `window.spawn` to get an `AsyncWindowContext` after the await
    /// point — the Zed standard pattern for async actions on GPUI entities.
    /// A `WeakEntity` is captured so the async block does not hold a strong
    /// reference across the await (Standing Rule 9 — Zed Standard).
    ///
    /// On folder selection:
    /// - `state.folder_path` is set to the absolute path string
    /// - `.git/` presence is detected → `state.git_detected`
    /// - `.podium/` presence is detected → `state.podium_detected`
    /// - If `.git/config` is present, the `url =` line under `[remote "origin"]`
    ///   is parsed into `state.detected_remote`
    /// - `state.project_name` is pre-filled from the last path component
    ///   (only if the field is still empty — does not overwrite user edits)
    /// - `cx.notify()` triggers re-render
    ///
    /// If the user dismisses the picker (`None` returned), state is unchanged.
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

            // Perform all filesystem inspection synchronously — these are
            // cheap metadata checks, not I/O-heavy operations.
            let folder_path = std::path::Path::new(&path);
            let git_detected = folder_path.join(".git").is_dir();
            let podium_detected = folder_path.join(".podium").is_dir();

            // Parse the remote URL from .git/config if the repo is present.
            // We look for the url = line under [remote "origin"]. If parsing
            // fails for any reason we silently produce None — this is advisory
            // data for pre-filling Step 3, not a required field.
            let detected_remote = if git_detected {
                parse_git_remote_url(&folder_path.join(".git").join("config"))
            } else {
                None
            };

            // Pre-fill the project name from the last path component.
            // Use to_string_lossy so non-UTF8 paths degrade gracefully.
            let folder_name = folder_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();

            cx.update(|_window, cx| {
                entity
                    .upgrade()
                    .map(|sheet| {
                        sheet.update(cx, |this, cx| {
                            this.state.folder_path = Some(path);
                            this.state.git_detected = git_detected;
                            this.state.podium_detected = podium_detected;
                            this.state.detected_remote = detected_remote;
                            // Only pre-fill if the user has not already typed a name.
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
        // Phase 2 step 15: build the full ProjectEntry from self.state.
        // For now emit with a stub entry so the event pipeline is exercised.
        // The stub is replaced when Step 7 confirm card is built.
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
                    .when(!has_path, |this| {
                        this.text_color(cx.theme().muted_foreground)
                    })
                    .when(has_path, |this| {
                        this.text_color(cx.theme().foreground)
                    })
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
                        // danger_foreground is the confirmed text color for error states
                        // at git HEAD 6d7847e — verified from theme_color.rs source.
                        .text_color(cx.theme().danger_foreground)
                        .child(
                            self.state
                                .project_name_error
                                .clone()
                                .unwrap_or_default(),
                        ),
                )
            })
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground.opacity(0.6))
                    .child("Letters, numbers, spaces, hyphens, and underscores only."),
            )
    }

    // --- Step card stubs — replaced in build order steps 10–14 --------------

    fn render_step_git(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Phase 2 step 10: replace with SSH/HTTPS selector, account input, remote URL.
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Configure git authentication for this project. (Optional)"),
            )
    }

    fn render_step_agents(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Phase 2 step 11: replace with agent roster builder.
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Add AI agents to this project. (Optional)"),
            )
    }

    fn render_step_kb_sources(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Phase 2 step 12: replace with KB source multi-select from global library.
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
        // Phase 2 step 13: replace with service configuration cards.
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
        // Phase 2 step 14: replace with full summary display.
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
///
/// Looks for the `url =` key under the `[remote "origin"]` section.
/// Returns `None` if the file cannot be read, the section is absent,
/// or the `url` line is malformed. All failures are silent — this data
/// is advisory pre-fill for Step 3, not a required field.
fn parse_git_remote_url(git_config_path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(git_config_path).ok()?;

    let mut in_origin_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == r#"[remote "origin"]"# {
            in_origin_section = true;
            continue;
        }

        // A new section header ends the origin block.
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
