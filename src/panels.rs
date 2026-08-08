//! Stub panel implementations for Phase 1.
//!
//! Each panel implements `PodiumPanel` with the minimum required to compile
//! and register with the dock. No real content — each renders a centered
//! label. Real implementations come in later phases.
//!
//! ## Phase assignments
//!
//! - FilesPanel:     Phase 5
//! - AgentsPanel:    Phase 7
//! - KnowledgePanel: Phase 9
//! - ReviewPanel:    Phase 9
//! - TerminalPanel:  Phase 3
//! - HealthPanel:    Phase 10
//!
//! ## Toggle actions and dead-code warnings
//!
//! The six `Toggle*Panel` actions defined below are returned by each panel's
//! `toggle_action()` method but are never dispatched in Phase 1. Tab clicks
//! in Phase 1 call `toggle_panel_by_priority` directly (see `main.rs`) to
//! avoid the focus-tree requirement of the action dispatch system.
//!
//! The actions are **not dead code** — they are infrastructure for Phase 2
//! keyboard shortcuts and command palette integration. The compiler warnings
//! are expected and intentional. Do not remove them.

use gpui::{
    Action, App, Context, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement,
    Pixels, Render, Styled, Window, actions, div, px,
};

use gpui_component::IconName;

use crate::panel::{PanelEvent, PanelPosition, PodiumPanel};

// ---------------------------------------------------------------------------
// Toggle actions
//
// One action per panel. Returned by toggle_action() on each panel.
//
// Phase 2: register these with the command palette and key bindings so panels
// can be opened/closed via keyboard shortcuts (e.g. Cmd+1 for Files).
// Phase 1: unused — tab clicks bypass the action system (see module doc).
// ---------------------------------------------------------------------------

actions!(
    podium,
    [
        ToggleFilesPanel,
        ToggleAgentsPanel,
        ToggleKnowledgePanel,
        ToggleReviewPanel,
        ToggleTerminalPanel,
        ToggleHealthPanel,
    ]
);

// ---------------------------------------------------------------------------
// stub_panel! macro
// ---------------------------------------------------------------------------

/// Generate a stub `PodiumPanel` implementation.
///
/// All six Phase 1 panels share identical boilerplate. This macro generates
/// the struct, `new()`, `EventEmitter`, `Focusable`, `Render`, and
/// `PodiumPanel` impl from a compact declaration.
///
/// # Usage
///
/// ```rust
/// stub_panel!(
///     FilesPanel,           // struct name
///     "FilesPanel",         // name() — persistence key, must be unique
///     PanelPosition::Left,  // default position (ADR-028)
///     IconName::Inbox,      // icon shown in tab button
///     "Files",              // icon_tooltip and stub render label
///     ToggleFilesPanel,     // toggle action type
///     100,                  // activation_priority (ADR-028)
/// );
/// ```
macro_rules! stub_panel {
    (
        $name:ident,
        $name_str:literal,
        $default_pos:expr,
        $icon:expr,
        $tooltip:literal,
        $action:ident,
        $priority:literal $(,)?
    ) => {
        pub struct $name {
            focus_handle: FocusHandle,
            position: PanelPosition,
        }

        impl $name {
            pub fn new(cx: &mut App) -> Self {
                Self {
                    focus_handle: cx.focus_handle(),
                    position: $default_pos,
                }
            }
        }

        impl EventEmitter<PanelEvent> for $name {}

        impl Focusable for $name {
            fn focus_handle(&self, _cx: &App) -> FocusHandle {
                self.focus_handle.clone()
            }
        }

        impl Render for $name {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                // Phase 1 stub: centered label only.
                // Real content implemented in the phase assigned above.
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child($tooltip)
            }
        }

        impl PodiumPanel for $name {
            fn name() -> &'static str {
                $name_str
            }

            fn position(&self, _cx: &App) -> PanelPosition {
                self.position
            }

            fn position_is_valid(&self, _position: PanelPosition) -> bool {
                // Phase 1: all positions valid for every panel — user can
                // reposition any panel to any dock in Phase 2.
                true
            }

            fn set_position(&mut self, position: PanelPosition, cx: &mut Context<Self>) {
                self.position = position;
                cx.notify();
            }

            fn default_size(&self, _cx: &App) -> Pixels {
                px(280.)
            }

            fn icon(&self, _cx: &App) -> IconName {
                $icon
            }

            fn icon_tooltip(&self, _cx: &App) -> &'static str {
                $tooltip
            }

            fn toggle_action(&self) -> Box<dyn Action> {
                // Phase 2: this action will be registered with the command
                // palette so keyboard shortcuts can open/close the panel.
                $action.boxed_clone()
            }

            fn activation_priority(&self) -> u32 {
                $priority
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Stub panels — one per line, parameters per stub_panel! doc above.
//
// Dock assignments and priority values are locked by ADR-028.
// Default sizes: all Left dock panels → px(280.) width.
//               TerminalPanel (Bottom) → see note below.
// ---------------------------------------------------------------------------

stub_panel!(
    FilesPanel,
    "FilesPanel",
    PanelPosition::Left,
    IconName::Inbox,
    "Files",
    ToggleFilesPanel,
    100,
);

stub_panel!(
    AgentsPanel,
    "AgentsPanel",
    PanelPosition::Left,
    IconName::Bot,
    "Agents",
    ToggleAgentsPanel,
    200,
);

stub_panel!(
    KnowledgePanel,
    "KnowledgePanel",
    PanelPosition::Left,
    IconName::BookOpen,
    "Knowledge",
    ToggleKnowledgePanel,
    300,
);

stub_panel!(
    ReviewPanel,
    "ReviewPanel",
    PanelPosition::Left,
    IconName::Eye,
    "Review",
    ToggleReviewPanel,
    400,
);

stub_panel!(
    TerminalPanel,
    "TerminalPanel",
    PanelPosition::Bottom,
    IconName::PanelBottom,
    "Terminal",
    ToggleTerminalPanel,
    500,
    // Note: the macro's default_size() returns px(280.) for all panels.
    // TerminalPanel is in the Bottom dock, so default_size() is a *height*,
    // not a width. 280px is usable but taller than necessary — the render
    // in main.rs constrains it to px(240.) for Phase 1.
    // Phase 3: override default_size() in the real TerminalPanel impl to
    // return a sensible default terminal height (recommend px(240.)).
);

stub_panel!(
    HealthPanel,
    "HealthPanel",
    PanelPosition::Left,
    IconName::LayoutDashboard,
    "Health",
    ToggleHealthPanel,
    600,
);
