//! Onboarding flow for Podium — Sheet container and 7-step card system.
//!
//! ## Architecture
//!
//! `Sheet` in gpui-component is a `RenderOnce` element (stateless, rebuilt each
//! render). Step state must therefore live on `PodiumApp`, not inside the Sheet.
//! `OnboardingState` holds all in-progress form data and the current step index.
//!
//! ## Flow
//!
//! 1. User clicks "Add New Project" → `open_onboarding` called on `PodiumApp`
//! 2. `PodiumApp` initializes `onboarding_state` and calls `open_onboarding_sheet`
//! 3. Each navigation action (Next, Back, Skip, Cancel) updates `onboarding_state`
//!    via an `Arc<dyn Fn>` callback and calls `cx.notify()` to trigger a re-render
//! 4. The callback calls `open_onboarding_sheet_with_current_state` again with the
//!    updated state, rebuilding the Sheet with the new step's content
//! 5. On Step 7 confirm, project is created and the Sheet is closed
//! 6. On Cancel, `onboarding_state` is cleared and the Sheet is closed
//!
//! ## Callback design
//!
//! All navigation callbacks are `Arc<dyn Fn(&mut Window, &mut App) + 'static>`.
//! Using `Arc` rather than generic `impl Fn` lets the callbacks be cloned cheaply
//! into nested closures (nav buttons, on_close handler) without requiring the
//! underlying type to implement `Clone`.
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

use std::sync::Arc;

use gpui::{AnyElement, App, Window};
use gpui::prelude::{FluentBuilder as _, IntoElement, ParentElement, Styled};
use gpui::{div, px};
use gpui_component::{
    ActiveTheme as _,
    Placement,
    Sizable as _,
    StyledExt as _,
    WindowExt as _,
    button::{Button, ButtonVariants as _},
};

use crate::colors::PodiumColorsExt as _;

// ---------------------------------------------------------------------------
// Callback type alias
// ---------------------------------------------------------------------------

/// A navigation callback for the onboarding Sheet.
///
/// `Arc` allows cheap cloning into nested closures (nav buttons, on_close).
pub type Callback = Arc<dyn Fn(&mut Window, &mut App) + 'static>;

// ---------------------------------------------------------------------------
// Step enum
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
            Self::Folder => "Choose Folder",
            Self::Identity => "Project Name",
            Self::Git => "Git Configuration",
            Self::Agents => "Agents",
            Self::KbSources => "Knowledge Sources",
            Self::Services => "Services",
            Self::Confirm => "Confirm & Create",
        }
    }

    /// True if this step is required (cannot be skipped).
    pub fn required(self) -> bool {
        matches!(self, Self::Folder | Self::Identity | Self::Confirm)
    }

    /// The step that follows this one, or `None` if this is the last.
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Folder => Some(Self::Identity),
            Self::Identity => Some(Self::Git),
            Self::Git => Some(Self::Agents),
            Self::Agents => Some(Self::KbSources),
            Self::KbSources => Some(Self::Services),
            Self::Services => Some(Self::Confirm),
            Self::Confirm => None,
        }
    }

    /// The step that precedes this one, or `None` if this is the first.
    pub fn prev(self) -> Option<Self> {
        match self {
            Self::Folder => None,
            Self::Identity => Some(Self::Folder),
            Self::Git => Some(Self::Identity),
            Self::Agents => Some(Self::Git),
            Self::KbSources => Some(Self::Agents),
            Self::Services => Some(Self::KbSources),
            Self::Confirm => Some(Self::Services),
        }
    }
}

// ---------------------------------------------------------------------------
// OnboardingState — lives on PodiumApp
// ---------------------------------------------------------------------------

/// All in-progress form data for the onboarding flow.
///
/// Initialized when the Sheet opens, cleared on cancel or completion.
/// Lives on `PodiumApp` because `Sheet` is a stateless `RenderOnce` element.
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
    /// Project display name — pre-filled from folder name, editable.
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
    /// Create a fresh `OnboardingState` at Step 1 with all fields empty.
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
// Public entry point — open_onboarding_sheet
// ---------------------------------------------------------------------------

/// Open the onboarding Sheet from the left side of the window.
///
/// All navigation callbacks are `Arc<dyn Fn>` so they can be cloned cheaply
/// into nested closures without requiring the underlying type to be `Clone`.
///
/// ## Sheet configuration
///
/// - Width: 420px (per ADR-019 and the onboarding spec)
/// - Placement: Left
/// - Overlay: true (dock area dimmed behind Sheet)
/// - Overlay closable: false (user must use Cancel button)
/// - Resizable: false
/// - Margin top: 73px (TitleBar ≈ 36px + tab bar ≈ 37px)
pub fn open_onboarding_sheet(
    state: &OnboardingState,
    window: &mut Window,
    cx: &mut App,
    on_next: Callback,
    on_back: Callback,
    on_skip: Callback,
    on_cancel: Callback,
    on_confirm: Callback,
) {
    let step = state.step;
    let state_clone = state.clone();

    // Clone callbacks needed in multiple closures before moving into the sheet.
    let on_cancel_for_close = on_cancel.clone();

    window.open_sheet_at(Placement::Left, cx, move |sheet, _window, cx| {
        let colors = cx.podium_colors();
        let step_content = render_step_card(&state_clone, cx);

        sheet
            .size(px(420.))
            // margin_top pushes the Sheet below TitleBar + tab bar so both
            // remain visible above the Sheet (per onboarding spec).
            // TitleBar ≈ 36px + tab bar ≈ 37px = 73px total.
            // Phase 2: read actual heights from layout once measurement is available.
            .margins(px(73.))
            .resizable(false)
            .overlay(true)
            .overlay_closable(false)
            .on_close({
                let on_cancel_for_close = on_cancel_for_close.clone();
                move |_event, window, cx| {
                    on_cancel_for_close(window, cx);
                }
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .h_full()
                    .p_4()
                    .gap_4()
                    // Step counter and pip indicators
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
                    // Step card — fills remaining space
                    .child(div().flex_1().overflow_hidden().child(step_content))
                    // Navigation buttons
                    .child(render_nav_buttons(
                        step,
                        on_next.clone(),
                        on_back.clone(),
                        on_skip.clone(),
                        on_cancel.clone(),
                        on_confirm.clone(),
                    )),
            )
    });
}

// ---------------------------------------------------------------------------
// Step card dispatcher
// ---------------------------------------------------------------------------

fn render_step_card(state: &OnboardingState, cx: &mut App) -> AnyElement {
    match state.step {
        OnboardingStep::Folder    => render_step_folder_stub(cx).into_any_element(),
        OnboardingStep::Identity  => render_step_identity_stub(state, cx).into_any_element(),
        OnboardingStep::Git       => render_step_git_stub(cx).into_any_element(),
        OnboardingStep::Agents    => render_step_agents_stub(cx).into_any_element(),
        OnboardingStep::KbSources => render_step_kb_sources_stub(cx).into_any_element(),
        OnboardingStep::Services  => render_step_services_stub(cx).into_any_element(),
        OnboardingStep::Confirm   => render_step_confirm_stub(state, cx).into_any_element(),
    }
}

// ---------------------------------------------------------------------------
// Step card stubs — replaced in build order steps 8–14
// ---------------------------------------------------------------------------

fn render_step_folder_stub(cx: &mut App) -> impl IntoElement {
    // Step 8: replace with native folder picker (rfd), .git/.podium detection
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
                .text_color(cx.theme().muted_foreground)
                .child("No folder selected"),
        )
        .child(
            Button::new("pick-folder")
                .label("Browse…")
                .outline()
                .small(),
        )
}

fn render_step_identity_stub(state: &OnboardingState, cx: &mut App) -> impl IntoElement {
    // Step 9: replace with Input field, validation, uniqueness check
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
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(cx.theme().border)
                .text_sm()
                .when(state.project_name.is_empty(), |this| {
                    this.text_color(cx.theme().muted_foreground).child("Project name")
                })
                .when(!state.project_name.is_empty(), |this| {
                    this.text_color(cx.theme().foreground)
                        .child(state.project_name.clone())
                }),
        )
}

fn render_step_git_stub(cx: &mut App) -> impl IntoElement {
    // Step 10: replace with SSH/HTTPS selector, account input, remote URL
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

fn render_step_agents_stub(cx: &mut App) -> impl IntoElement {
    // Step 11: replace with agent roster builder
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

fn render_step_kb_sources_stub(cx: &mut App) -> impl IntoElement {
    // Step 12: replace with KB source multi-select from global library
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

fn render_step_services_stub(cx: &mut App) -> impl IntoElement {
    // Step 13: replace with service configuration cards
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

fn render_step_confirm_stub(state: &OnboardingState, cx: &mut App) -> impl IntoElement {
    // Step 14: replace with full summary
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
        .when(!state.project_name.is_empty(), |this| {
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
                            .child(state.project_name.clone()),
                    ),
            )
        })
        .when(state.folder_path.is_some(), |this| {
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
                            .child(state.folder_path.clone().unwrap_or_default()),
                    ),
            )
        })
}

// ---------------------------------------------------------------------------
// Navigation buttons
// ---------------------------------------------------------------------------

/// Render the navigation button row at the bottom of the Sheet.
///
/// All callbacks are `Arc<dyn Fn>` — cloned cheaply here rather than
/// requiring the caller to provide separate copies per button.
fn render_nav_buttons(
    step: OnboardingStep,
    on_next: Callback,
    on_back: Callback,
    on_skip: Callback,
    on_cancel: Callback,
    on_confirm: Callback,
) -> impl IntoElement {
    let is_first = step == OnboardingStep::Folder;
    let is_last = step == OnboardingStep::Confirm;
    let is_skippable = !step.required();

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        // Cancel — always left-aligned
        .child(
            Button::new("cancel")
                .label("Cancel")
                .ghost()
                .small()
                .on_click({
                    let on_cancel = on_cancel.clone();
                    move |_event, window, cx| {
                        on_cancel(window, cx);
                    }
                }),
        )
        .child(div().flex_1())
        // Back — all steps except Step 1
        .when(!is_first, |this| {
            this.child(
                Button::new("back")
                    .label("← Back")
                    .outline()
                    .small()
                    .on_click({
                        let on_back = on_back.clone();
                        move |_event, window, cx| {
                            on_back(window, cx);
                        }
                    }),
            )
        })
        // Skip — skippable steps only (Steps 3–6)
        .when(is_skippable, |this| {
            this.child(
                Button::new("skip")
                    .label("Skip")
                    .ghost()
                    .small()
                    .on_click({
                        let on_skip = on_skip.clone();
                        move |_event, window, cx| {
                            on_skip(window, cx);
                        }
                    }),
            )
        })
        // Next — all steps except the last
        .when(!is_last, |this| {
            this.child(
                Button::new("next")
                    .label("Next →")
                    .primary()
                    .small()
                    .on_click({
                        let on_next = on_next.clone();
                        move |_event, window, cx| {
                            on_next(window, cx);
                        }
                    }),
            )
        })
        // Create Project — last step only
        .when(is_last, |this| {
            this.child(
                Button::new("confirm")
                    .label("Create Project")
                    .primary()
                    .small()
                    .on_click(move |_event, window, cx| {
                        on_confirm(window, cx);
                    }),
            )
        })
}
