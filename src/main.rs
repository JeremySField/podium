//! Podium application shell — Phase 1 complete, Phase 2 in progress.
//!
//! `PodiumApp` is the root GPUI view. It holds three `PodiumDock` entities
//! (left, bottom, right) and renders the full shell: TitleBar, tab bar,
//! content area with docks, StatusBar, and overlay layers.
//!
//! ## Dark theme — three calls required
//!
//! gpui-component dark mode requires all three of the following in `app.run()`,
//! in this order. Any subset produces either a light palette, a missing mode
//! flag, or OS-level light popup contexts:
//!
//! ```rust
//! theme.colors = *ThemeColor::dark();          // swap the color palette
//! theme.mode = ThemeMode::Dark;                // set the mode flag
//! cx.set_window_appearance(Some(Dark));        // OS-level dark context
//! ```
//!
//! ## Popup/dropdown theming — two fields required
//!
//! gpui-component's `PopupMenu` renders its background via `popover_style(cx)`
//! which reads `cx.theme().tokens.popover` (a `ThemeToken`) — NOT
//! `cx.theme().colors.popover` (an `Hsla`). Both must be set to get a dark
//! dropdown background:
//!
//! ```rust
//! colors.popover = dark_color;                 // sets ThemeColor.popover
//! theme.tokens.popover = dark_color.into();    // sets ThemeTokens.popover (what popover_style reads)
//! ```
//!
//! `ThemeToken` implements `From<Hsla>` so `.into()` works directly.
//!
//! ## ThemeColor fields of note (confirmed from theme_color.rs source)
//!
//! - `popover`        — dropdown/popup Hsla color
//! - `popover_foreground` — dropdown/popup text
//! - `title_bar`      — title bar background
//! - `title_bar_border`
//! - `status_bar`     — status bar background
//! - `tab_bar`        — tab bar background
//! - `overlay`        — modal overlay background
//!
//! ## Phase 1 complete — bugs fixed in Phase 2 start
//!
//! - `.relative()` added to dock render div (resize handle anchor fix)
//! - Hard-coded dock widths replaced with `active_panel_size()` calls
//!
//! ## Deferred to later phases
//!
//! - Phase 2: project switcher Combobox in TitleBar (replaces placeholder)
//! - Phase 2: dock resize handle drag wiring
//! - Phase 2: dock open/close state persisted to config
//! - Phase 2: panel repositioning between docks
//! - Phase 10: full JSON-driven theme system

mod colors;
mod config;
mod dock;
mod onboarding;
mod panel;
mod panel_handle;
mod panels;
mod ssh_config;
mod state;
mod watch;
mod watch_error;

use std::sync::Arc;

use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Render, Styled,
    Window, WindowAppearance, WindowOptions, actions, div, px,
};
use gpui::prelude::FluentBuilder as _;
use gpui_component::{
    ActiveTheme as _,
    IconName,
    Root,
    Sizable as _,
    TitleBar,
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    menu::DropdownMenu as _,
    status_bar::StatusBar,
    theme::{Theme, ThemeColor, ThemeMode},
};

use colors::PodiumColorsExt as _;
use config::{KbSourcesConfig, ProjectsConfig};
use dock::PodiumDock;
use onboarding::{OnboardingState, open_onboarding_sheet};
use panel::PanelPosition;
use panels::{AgentsPanel, FilesPanel, HealthPanel, KnowledgePanel, ReviewPanel, TerminalPanel};

// ---------------------------------------------------------------------------
// Global actions
// ---------------------------------------------------------------------------

actions!(podium, [Quit, OpenOnboarding]);

// ---------------------------------------------------------------------------
// First launch init
// ---------------------------------------------------------------------------

/// Create `%APPDATA%\podium\` and empty config files on first launch.
///
/// Called once at startup before the window opens. Silently no-ops if the
/// directory and files already exist. Errors are logged to stderr but do not
/// crash the app — a missing config file is handled gracefully at load time
/// by returning empty defaults.
///
/// Files created if absent:
/// - `%APPDATA%\podium\projects.toml`
/// - `%APPDATA%\podium\kb_sources.toml`
///
/// `podium_state.toml` is not created here — it is written on first project
/// unload, which is the natural point it first has meaningful content.
fn first_launch_init() {
    let dir = config::podium_config_dir();

    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("podium: failed to create config dir {}: {}", dir.display(), e);
        return;
    }

    let projects_path = config::projects_toml_path();
    if !projects_path.exists() {
        if let Err(e) = config::ProjectsConfig::default().save() {
            eprintln!("podium: failed to write projects.toml: {}", e);
        }
    }

    let kb_sources_path = config::kb_sources_toml_path();
    if !kb_sources_path.exists() {
        if let Err(e) = config::KbSourcesConfig::default().save() {
            eprintln!("podium: failed to write kb_sources.toml: {}", e);
        }
    }
}

// ---------------------------------------------------------------------------
// PodiumApp — root view
// ---------------------------------------------------------------------------

/// Root view for the Podium application.
///
/// Owns the three docks (left, bottom, right) and the loaded config state.
/// Panels are registered into their default docks in `new()` per ADR-028.
///
/// `projects_config` and `kb_sources_config` are loaded from disk in `new()`
/// and kept in sync as projects are added, loaded, and removed.
///
/// `onboarding_state` is `Some` while the onboarding Sheet is open, `None`
/// otherwise. It lives here because `Sheet` is a stateless `RenderOnce`
/// element — step state must persist between renders on the parent entity.
struct PodiumApp {
    left_dock: Entity<PodiumDock>,
    bottom_dock: Entity<PodiumDock>,
    right_dock: Entity<PodiumDock>,
    projects_config: ProjectsConfig,
    kb_sources_config: KbSourcesConfig,
    onboarding_state: Option<OnboardingState>,
}

impl PodiumApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let left_dock = cx.new(|cx| PodiumDock::new(PanelPosition::Left, cx));
        let bottom_dock = cx.new(|cx| PodiumDock::new(PanelPosition::Bottom, cx));
        let right_dock = cx.new(|cx| PodiumDock::new(PanelPosition::Right, cx));

        // Left dock: Files (100), Agents (200), Knowledge (300), Review (400), Health (600)
        // Priority order determines tab bar position — see ADR-028.
        left_dock.update(cx, |dock, cx| {
            dock.add_panel(cx.new(|cx| FilesPanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| AgentsPanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| KnowledgePanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| ReviewPanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| HealthPanel::new(cx)), cx);
        });

        // Bottom dock: Terminal (500) — wide horizontal tool, bottom placement
        // per ADR-028.
        bottom_dock.update(cx, |dock, cx| {
            dock.add_panel(cx.new(|cx| TerminalPanel::new(cx)), cx);
        });

        // Right dock: no panels registered yet (ADR-028).

        // Load config from disk. first_launch_init() has already ensured the
        // files exist, so these return empty defaults at worst.
        let projects_config = ProjectsConfig::load().unwrap_or_default();
        let kb_sources_config = KbSourcesConfig::load().unwrap_or_default();

        Self {
            left_dock,
            bottom_dock,
            right_dock,
            projects_config,
            kb_sources_config,
            onboarding_state: None,
        }
    }

    // --- Onboarding ---------------------------------------------------------

    /// Open the onboarding Sheet with a fresh `OnboardingState`.
    ///
    /// Called directly from the "Add New Project" button via `cx.listener`.
    fn open_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.onboarding_state = Some(OnboardingState::new());
        cx.notify();
        self.open_onboarding_sheet_with_current_state(window, cx);
    }

    /// Re-open the Sheet with the current `onboarding_state`.
    ///
    /// Called after any navigation action mutates the state. Closures are
    /// wrapped in `Arc::new(...)` to satisfy the `Callback` type required by
    /// `open_onboarding_sheet` (`Arc<dyn Fn(&mut Window, &mut App) + 'static>`).
    fn open_onboarding_sheet_with_current_state(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = match &self.onboarding_state {
            Some(s) => s.clone(),
            None => return,
        };

        let entity = cx.entity();

        open_onboarding_sheet(
            &state,
            window,
            cx,
            // on_next
            Arc::new({
                let entity = entity.clone();
                move |window: &mut Window, cx: &mut App| {
                    entity.update(cx, |this, cx| {
                        if let Some(s) = &mut this.onboarding_state {
                            s.advance();
                        }
                        cx.notify();
                        this.open_onboarding_sheet_with_current_state(window, cx);
                    });
                }
            }),
            // on_back
            Arc::new({
                let entity = entity.clone();
                move |window: &mut Window, cx: &mut App| {
                    entity.update(cx, |this, cx| {
                        if let Some(s) = &mut this.onboarding_state {
                            s.go_back();
                        }
                        cx.notify();
                        this.open_onboarding_sheet_with_current_state(window, cx);
                    });
                }
            }),
            // on_skip — same as next: advances past the optional step
            Arc::new({
                let entity = entity.clone();
                move |window: &mut Window, cx: &mut App| {
                    entity.update(cx, |this, cx| {
                        if let Some(s) = &mut this.onboarding_state {
                            s.advance();
                        }
                        cx.notify();
                        this.open_onboarding_sheet_with_current_state(window, cx);
                    });
                }
            }),
            // on_cancel — clear state and close the Sheet
            Arc::new({
                let entity = entity.clone();
                move |window: &mut Window, cx: &mut App| {
                    entity.update(cx, |this, cx| {
                        this.onboarding_state = None;
                        cx.notify();
                    });
                    window.close_sheet(cx);
                }
            }),
            // on_confirm — project creation stub (wired in step 15)
            Arc::new({
                let entity = entity.clone();
                move |window: &mut Window, cx: &mut App| {
                    entity.update(cx, |this, cx| {
                        // Phase 2 step 15: create project from onboarding_state,
                        // write projects.toml, create .podium/ structure.
                        this.onboarding_state = None;
                        cx.notify();
                    });
                    window.close_sheet(cx);
                }
            }),
        );
    }

    // --- Panel toggling -----------------------------------------------------

    /// Toggle the panel with `priority` in whichever dock owns it.
    fn toggle_panel_by_priority(&mut self, priority: u32, cx: &mut Context<Self>) {
        for dock_entity in [&self.left_dock, &self.bottom_dock, &self.right_dock] {
            let handled = dock_entity.update(cx, |dock, cx| {
                let index = dock
                    .panels()
                    .enumerate()
                    .find(|(_, p)| p.activation_priority(cx) == priority)
                    .map(|(i, _)| i);

                let Some(index) = index else { return false; };

                if dock.is_open() && dock.active_panel_index() == Some(index) {
                    dock.set_open(false, cx);
                } else {
                    dock.activate_panel(index, cx);
                    dock.set_open(true, cx);
                }
                true
            });

            if handled {
                cx.notify();
                return;
            }
        }
    }

    // --- Content area -------------------------------------------------------

    /// Render the center content area.
    ///
    /// - No projects → empty state with Add New Project button
    /// - Projects exist → content placeholder (project loading wired in step 16)
    fn render_content_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.podium_colors();

        if self.projects_config.projects.is_empty() {
            div()
                .flex_1()
                .h_full()
                .bg(colors.content_background)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_4()
                .child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .text_sm()
                        .child("No projects yet. Add your first project."),
                )
                .child(
                    Button::new("add-project")
                        .label("Add New Project")
                        .on_click(cx.listener(|this, _event, window, cx| {
                            this.open_onboarding(window, cx);
                        })),
                )
        } else {
            div()
                .flex_1()
                .h_full()
                .bg(colors.content_background)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .text_sm()
                        .child("Podium"),
                )
        }
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for PodiumApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.podium_colors();

        let mut tab_info: Vec<(u32, &'static str, bool)> = Vec::new();
        for dock_entity in [&self.left_dock, &self.bottom_dock, &self.right_dock] {
            let dock = dock_entity.read(cx);
            let active_idx = if dock.is_open() { dock.active_panel_index() } else { None };
            dock.panels().enumerate().for_each(|(i, p)| {
                tab_info.push((
                    p.activation_priority(cx),
                    p.icon_tooltip(cx),
                    active_idx == Some(i),
                ));
            });
        }
        tab_info.sort_by_key(|(priority, _, _)| *priority);

        let left_open = self.left_dock.read(cx).is_open();
        let bottom_open = self.bottom_dock.read(cx).is_open();
        let right_open = self.right_dock.read(cx).is_open();

        let left_width = self.left_dock.read(cx)
            .active_panel_size(cx)
            .unwrap_or(px(280.));
        let right_width = self.right_dock.read(cx)
            .active_panel_size(cx)
            .unwrap_or(px(280.));
        let bottom_height = self.bottom_dock.read(cx)
            .active_panel_size(cx)
            .unwrap_or(px(240.));

        let left_dock = self.left_dock.clone();
        let bottom_dock = self.bottom_dock.clone();
        let right_dock = self.right_dock.clone();

        let content_area = self.render_content_area(cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.content_background)
            // --- TitleBar ---------------------------------------------------
            .child(
                TitleBar::new()
                    .bg(colors.title_bar_background)
                    .border_b_1()
                    .border_color(colors.title_bar_border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("app-menu")
                                    .icon(IconName::Menu)
                                    .ghost()
                                    .small()
                                    .dropdown_menu(|menu, _window, _cx| {
                                        menu.menu("Quit Podium", Box::new(Quit))
                                    }),
                            )
                            .child(
                                div()
                                    .text_color(cx.theme().foreground)
                                    .text_sm()
                                    .child("Podium"),
                            )
                            // Project switcher placeholder.
                            // Phase 2: replace with gpui-component Combobox.
                            .child(
                                div()
                                    .px_2()
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(cx.theme().secondary)
                                    .text_color(cx.theme().secondary_foreground)
                                    .text_sm()
                                    .child("No project"),
                            ),
                    ),
            )
            // --- Tab bar ----------------------------------------------------
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .border_b_1()
                    .border_color(colors.tab_bar_border)
                    .bg(colors.tab_bar_background)
                    .children(tab_info.into_iter().map(|(priority, tooltip, is_active)| {
                        div()
                            .px_3()
                            .py_2()
                            .text_sm()
                            .cursor_pointer()
                            .when(is_active, |this| {
                                this.border_b_2()
                                    .border_color(colors.tab_active_indicator)
                                    .text_color(colors.tab_active_foreground)
                            })
                            .when(!is_active, |this| {
                                this.text_color(colors.tab_inactive_foreground)
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                                    this.toggle_panel_by_priority(priority, cx);
                                }),
                            )
                            .child(tooltip)
                    })),
            )
            // --- Content area -----------------------------------------------
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_row()
                            .overflow_hidden()
                            .child(
                                div()
                                    .when(left_open, |this| this.w(left_width))
                                    .when(!left_open, |this| this.w(px(0.)))
                                    .h_full()
                                    .overflow_hidden()
                                    .child(left_dock),
                            )
                            .child(content_area)
                            .child(
                                div()
                                    .when(right_open, |this| this.w(right_width))
                                    .when(!right_open, |this| this.w(px(0.)))
                                    .h_full()
                                    .overflow_hidden()
                                    .child(right_dock),
                            ),
                    )
                    .child(
                        div()
                            .when(bottom_open, |this| this.h(bottom_height))
                            .when(!bottom_open, |this| this.h(px(0.)))
                            .w_full()
                            .overflow_hidden()
                            .child(bottom_dock),
                    ),
            )
            // --- StatusBar --------------------------------------------------
            .child(
                StatusBar::new()
                    .bg(colors.title_bar_background)
                    .left("Podium")
                    .right("Phase 2"),
            )
            // --- Overlay layers ---------------------------------------------
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx: &mut App| {
        first_launch_init();

        gpui_component::init(cx);

        {
            let theme = Theme::global_mut(cx);
            let mut colors = *ThemeColor::dark();
            let popup_bg: gpui::Hsla = gpui::rgb(0x3c3836).into();
            colors.popover = popup_bg;
            theme.colors = colors;
            theme.tokens.popover = popup_bg.into();
            theme.mode = ThemeMode::Dark;
        }
        cx.set_window_appearance(Some(WindowAppearance::Dark));

        cx.on_action(|_: &Quit, cx| cx.quit());

        // OpenOnboarding action — stub for now. The button uses cx.listener
        // directly (build step 7 decision). Will be used by the project
        // switcher dropdown and keyboard shortcuts in a later step.
        cx.on_action(|_: &OpenOnboarding, _cx| {});

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitleBar::title_bar_options()),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| PodiumApp::new(cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
