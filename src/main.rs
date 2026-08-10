//! Podium application shell — Phase 1 complete, Phase 2 in progress.
//!
//! `PodiumApp` is the root GPUI view. It holds three `PodiumDock` entities
//! (left, bottom, right) and renders the full shell: TitleBar, tab bar,
//! content area with docks, StatusBar, and overlay layers.
//!
//! ## Theme system
//!
//! Podium ships a Gruvbox Dark theme as `assets/themes/gruvbox-dark.json`,
//! embedded at compile time via `include_str!`. At startup:
//!
//! 1. `gpui_component::init(cx)` — initialises the ThemeRegistry with the
//!    built-in default light/dark themes
//! 2. `ThemeRegistry::load_themes_from_str` — registers the Gruvbox Dark theme
//!    in the `themes` map under the name "Gruvbox Dark"
//! 3. Look up "Gruvbox Dark" from the registry and call `Theme::apply_config`
//!    directly — this is required because `load_themes_from_str` does NOT
//!    update `default_themes`, so `Theme::change(Dark)` would still pick up
//!    "Default Dark" as the dark theme. Direct `apply_config` bypasses this.
//! 4. `cx.set_window_appearance(Some(WindowAppearance::Dark))` — OS-level dark
//!
//! `apply_config` sets both `ThemeColor` fields and `ThemeTokens` in one pass
//! via the `apply_color!` / `apply_background_color!` macros in `schema.rs`.
//!
//! Phase 10: add additional themes (One Dark, Solarized, etc.) and a theme
//! selector in Settings.
//!
//! ## Onboarding architecture
//!
//! `OnboardingSheet` is a proper GPUI entity that owns its own state and drives
//! all navigation internally via `cx.listener`. `PodiumApp` creates the entity,
//! opens the Sheet once, and subscribes to `OnboardingEvent::ProjectCreated`.
//! No `Arc<dyn Fn>` callbacks — see `onboarding.rs` for the full pattern.
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
//! - Phase 10: additional theme choices (One Dark, Solarized, etc.)

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

use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Render, Styled, Subscription,
    Window, WindowAppearance, WindowOptions, actions, div, px,
};
use gpui::prelude::FluentBuilder as _;
use gpui_component::{
    ActiveTheme as _,
    IconName,
    Placement,
    Root,
    Sizable as _,
    StyledExt as _,
    TitleBar,
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    menu::DropdownMenu as _,
    status_bar::StatusBar,
    theme::{Theme, ThemeMode, ThemeRegistry},
};

use colors::PodiumColorsExt as _;
use config::{KbSourcesConfig, ProjectsConfig};
use dock::PodiumDock;
use onboarding::{OnboardingEvent, OnboardingSheet};
use panel::PanelPosition;
use panels::{AgentsPanel, FilesPanel, HealthPanel, KnowledgePanel, ReviewPanel, TerminalPanel};

// ---------------------------------------------------------------------------
// Embedded theme
// ---------------------------------------------------------------------------

/// Gruvbox Dark theme — embedded at compile time from assets/themes/gruvbox-dark.json.
/// Loaded into the ThemeRegistry at startup and applied directly via apply_config.
/// Phase 10 adds additional themes and a theme selector in Settings.
const GRUVBOX_DARK: &str = include_str!("../assets/themes/gruvbox-dark.json");

// ---------------------------------------------------------------------------
// Global actions
// ---------------------------------------------------------------------------

actions!(podium, [Quit, OpenOnboarding]);

// ---------------------------------------------------------------------------
// First launch init
// ---------------------------------------------------------------------------

fn first_launch_init() {
    let directory = config::podium_config_dir();

    if let Err(error) = std::fs::create_dir_all(&directory) {
        eprintln!("podium: failed to create config dir {}: {}", directory.display(), error);
        return;
    }

    let projects_path = config::projects_toml_path();
    if !projects_path.exists() {
        if let Err(error) = config::ProjectsConfig::default().save() {
            eprintln!("podium: failed to write projects.toml: {}", error);
        }
    }

    let kb_sources_path = config::kb_sources_toml_path();
    if !kb_sources_path.exists() {
        if let Err(error) = config::KbSourcesConfig::default().save() {
            eprintln!("podium: failed to write kb_sources.toml: {}", error);
        }
    }
}

// ---------------------------------------------------------------------------
// PodiumApp — root view
// ---------------------------------------------------------------------------

struct PodiumApp {
    left_dock: Entity<PodiumDock>,
    bottom_dock: Entity<PodiumDock>,
    right_dock: Entity<PodiumDock>,
    projects_config: ProjectsConfig,
    kb_sources_config: KbSourcesConfig,
    onboarding_sheet: Option<Entity<OnboardingSheet>>,
    _onboarding_subscription: Option<Subscription>,
}

impl PodiumApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let left_dock = cx.new(|cx| PodiumDock::new(PanelPosition::Left, cx));
        let bottom_dock = cx.new(|cx| PodiumDock::new(PanelPosition::Bottom, cx));
        let right_dock = cx.new(|cx| PodiumDock::new(PanelPosition::Right, cx));

        left_dock.update(cx, |dock, cx| {
            dock.add_panel(cx.new(|cx| FilesPanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| AgentsPanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| KnowledgePanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| ReviewPanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| HealthPanel::new(cx)), cx);
        });

        bottom_dock.update(cx, |dock, cx| {
            dock.add_panel(cx.new(|cx| TerminalPanel::new(cx)), cx);
        });

        let projects_config = ProjectsConfig::load().unwrap_or_default();
        let kb_sources_config = KbSourcesConfig::load().unwrap_or_default();

        Self {
            left_dock,
            bottom_dock,
            right_dock,
            projects_config,
            kb_sources_config,
            onboarding_sheet: None,
            _onboarding_subscription: None,
        }
    }

    fn open_onboarding(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sheet_entity = cx.new(|cx| OnboardingSheet::new(window, cx));

        let subscription = cx.subscribe(
            &sheet_entity,
            |this, _sheet, event, cx| match event {
                OnboardingEvent::ProjectCreated(entry) => {
                    this.projects_config.add_project(entry.clone());
                    this.onboarding_sheet = None;
                    this._onboarding_subscription = None;
                    cx.notify();
                }
            },
        );

        let sheet_entity_for_builder = sheet_entity.clone();
        window.open_sheet_at(Placement::Left, cx, move |sheet, _window, _cx| {
            sheet
                .size(px(420.))
                .margins(px(73.))
                .resizable(false)
                .overlay(true)
                .overlay_closable(false)
                .child(sheet_entity_for_builder.clone())
        });

        self.onboarding_sheet = Some(sheet_entity);
        self._onboarding_subscription = Some(subscription);
    }

    fn toggle_panel_by_priority(&mut self, priority: u32, cx: &mut Context<Self>) {
        for dock_entity in [&self.left_dock, &self.bottom_dock, &self.right_dock] {
            let handled = dock_entity.update(cx, |dock, cx| {
                let index = dock
                    .panels()
                    .enumerate()
                    .find(|(_, panel)| panel.activation_priority(cx) == priority)
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
                        .primary()
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
            dock.panels().enumerate().for_each(|(i, panel)| {
                tab_info.push((
                    panel.activation_priority(cx),
                    panel.icon_tooltip(cx),
                    active_idx == Some(i),
                ));
            });
        }
        tab_info.sort_by_key(|(priority, _, _)| *priority);

        let left_open = self.left_dock.read(cx).is_open();
        let bottom_open = self.bottom_dock.read(cx).is_open();
        let right_open = self.right_dock.read(cx).is_open();

        let left_width = self.left_dock.read(cx).active_panel_size(cx).unwrap_or(px(280.));
        let right_width = self.right_dock.read(cx).active_panel_size(cx).unwrap_or(px(280.));
        let bottom_height = self.bottom_dock.read(cx).active_panel_size(cx).unwrap_or(px(240.));

        let left_dock = self.left_dock.clone();
        let bottom_dock = self.bottom_dock.clone();
        let right_dock = self.right_dock.clone();

        let content_area = self.render_content_area(cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.content_background)
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
            .child(
                StatusBar::new()
                    .bg(colors.title_bar_background)
                    .left("Podium")
                    .right("Phase 2"),
            )
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

        // gpui_component::init registers the built-in default light/dark themes.
        gpui_component::init(cx);

        // Register Gruvbox Dark in the themes map.
        if let Err(error) = ThemeRegistry::global_mut(cx).load_themes_from_str(GRUVBOX_DARK) {
            eprintln!("podium: failed to load Gruvbox Dark theme: {}", error);
        }

        // Apply Gruvbox Dark directly via apply_config.
        //
        // Theme::change(Dark) picks up theme.dark_theme, which is set from
        // default_themes — not from load_themes_from_str. Calling apply_config
        // directly bypasses that and applies exactly the theme we loaded.
        let gruvbox_config = ThemeRegistry::global(cx)
            .themes()
            .get("Gruvbox Dark")
            .cloned();

        if let Some(config) = gruvbox_config {
            Theme::global_mut(cx).apply_config(&config);
        } else {
            eprintln!("podium: Gruvbox Dark theme not found after load — using default dark");
            Theme::change(ThemeMode::Dark, None, cx);
        }

        cx.set_window_appearance(Some(WindowAppearance::Dark));

        cx.on_action(|_: &Quit, cx| cx.quit());
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
