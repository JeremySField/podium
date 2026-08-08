//! Stub panel implementations for Phase 1.
//!
//! Each panel implements `PodiumPanel` with the minimum required to compile
//! and register with the dock. No real content — each renders an empty div
//! with a centered label. Real implementations come in later phases.
//!
//! ## Phase assignments
//! - FilesPanel:     Phase 5
//! - AgentsPanel:    Phase 7
//! - KnowledgePanel: Phase 9
//! - ReviewPanel:    Phase 9
//! - TerminalPanel:  Phase 3
//! - HealthPanel:    Phase 10

use gpui::{
    Action, App, Context, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement,
    Pixels, Render, Styled, Window, actions, div, px,
};

use gpui_component::IconName;

use crate::panel::{PanelEvent, PanelPosition, PodiumPanel};

// ---------------------------------------------------------------------------
// Toggle actions — one per panel, dispatched by the tab button on click.
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
// Macro — reduce boilerplate for stub panels
// ---------------------------------------------------------------------------

/// Generate a stub `PodiumPanel` implementation.
///
/// Usage:
/// ```
/// stub_panel!(
///     FilesPanel,           // struct name
///     "FilesPanel",         // name() — persistence key
///     PanelPosition::Left,  // default position
///     IconName::Inbox,      // icon
///     "Files",              // icon_tooltip
///     ToggleFilesPanel,     // toggle action
///     100,                  // activation_priority
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
                // Phase 1: all positions valid — user can reposition any panel.
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
                $action.boxed_clone()
            }

            fn activation_priority(&self) -> u32 {
                $priority
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Stub panels
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
