//! GPUI presentation adapter.
//!
//! GPUI-specific types are deliberately contained in this crate. The
//! application core only supplies renderer-independent configuration and
//! domain values.

use std::{
    any::Any,
    cell::RefCell,
    panic::{self, AssertUnwindSafe},
    rc::Rc,
    sync::mpsc,
    thread,
    time::Duration,
};

use chrono::Local;
use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, FontWeight, KeyBinding, KeyDownEvent,
    MouseDownEvent, Path, PathBuilder, Pixels, Render, SharedString, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, actions, canvas, div,
    layer_shell::*, point, prelude::*, px, rgb, rgba, size,
};
use gpui_platform::application;
use shell_config::{ConfigEvent, ConfigWatcher};
use shell_core::{
    AppUsageMetrics, BarPosition, CompositorEvent, CompositorSnapshot, CompositorStateStore,
    FocusManager, KeyboardModeConfig, NotchAnimation, NotchConfig, NotchGeometry, NotchState,
    OutputId, PlatformError, Point, ShellCommand, ShellConfig,
};
use shell_platform::{Anchors, ShellLayer, SurfaceSpec};
use shell_theme::DesignTokens;

actions!(shell_ui_gpui, [Quit]);

#[derive(Clone, Debug)]
pub struct GpuiFrontend {
    surface_spec: Rc<RefCell<SurfaceSpec>>,
    settings: Rc<RefCell<FrontendSettings>>,
}

#[derive(Clone, Debug)]
struct FrontendSettings {
    tokens: DesignTokens,
    width: f32,
    height: f32,
    panel_height: f32,
    notch: NotchConfig,
}

impl Default for FrontendSettings {
    fn default() -> Self {
        let notch = NotchConfig::default();
        Self {
            tokens: DesignTokens::default(),
            width: notch.width as f32,
            height: notch.expanded_height as f32,
            panel_height: shell_core::BarConfig::default().height as f32,
            notch,
        }
    }
}

impl FrontendSettings {
    fn from_config(config: &ShellConfig) -> Self {
        Self {
            tokens: DesignTokens::from_config(&config.theme),
            width: config.notch.width as f32,
            height: config.notch.expanded_height as f32,
            panel_height: config.bar.height as f32,
            notch: config.notch.clone(),
        }
    }
}

impl GpuiFrontend {
    pub fn new(surface_spec: SurfaceSpec) -> Self {
        Self {
            surface_spec: Rc::new(RefCell::new(surface_spec)),
            settings: Rc::new(RefCell::new(FrontendSettings::default())),
        }
    }

    pub fn prepare(&self, config: &ShellConfig) {
        let mut surface_spec = self.surface_spec.borrow_mut();
        surface_spec.anchors = match config.bar.position {
            BarPosition::Top => Anchors::TOP_LEFT_RIGHT,
            BarPosition::Bottom => Anchors::BOTTOM_LEFT_RIGHT,
        };
        surface_spec.exclusive_zone =
            (config.bar.enabled && config.bar.reserve_space).then_some(config.bar.height as i32);
        surface_spec.keyboard = match config.bar.keyboard {
            KeyboardModeConfig::None => shell_platform::KeyboardMode::None,
            KeyboardModeConfig::Exclusive => shell_platform::KeyboardMode::Exclusive,
            KeyboardModeConfig::OnDemand => shell_platform::KeyboardMode::OnDemand,
        };
        drop(surface_spec);

        *self.settings.borrow_mut() = FrontendSettings::from_config(config);
    }

    /// Runs the Milestone 1 interactive GPUI proof of concept.
    pub fn run_viability_app(&self) -> Result<(), PlatformError> {
        self.run_viability_app_inner(None)
    }

    /// Runs the first real shell panel.
    ///
    /// Compositor events and configuration events arrive through channels so
    /// the GPUI render path remains independent from IPC and filesystem I/O.
    pub fn run_panel_app(
        &self,
        initial_snapshot: Option<CompositorSnapshot>,
        compositor_events: Option<mpsc::Receiver<CompositorEvent>>,
        usage_metrics: Option<mpsc::Receiver<AppUsageMetrics>>,
        commands: mpsc::Sender<ShellCommand>,
        state_store: CompositorStateStore,
        watcher: Option<ConfigWatcher>,
    ) -> Result<(), PlatformError> {
        let startup_error = Rc::new(RefCell::new(None));
        let startup_error_for_app = startup_error.clone();
        let surface_spec = self.surface_spec.borrow().clone();
        let settings = self.settings.borrow().clone();
        let config_events = watcher.map(|watcher| {
            let (sender, receiver) = mpsc::channel();
            thread::spawn(move || {
                loop {
                    match watcher.recv_timeout(Duration::from_secs(1)) {
                        Ok(event) => {
                            if sender.send(event).is_err() {
                                break;
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
            receiver
        });
        let frontend_for_reload = self.clone();
        let initial_snapshot = initial_snapshot.unwrap_or_default();
        let debug_tokens = settings.tokens.clone();
        let debug_state_store = state_store.clone();
        let notch_config = settings.notch.clone();
        let notch_tokens = settings.tokens.clone();

        let run_result = catch_gpui_run(|| {
            application().run(move |cx: &mut App| {
                cx.bind_keys([KeyBinding::new("ctrl-q", Quit, None)]);
                cx.on_action(|_: &Quit, cx| cx.quit());

                let result = cx.open_window(
                    WindowOptions {
                        titlebar: None,
                        window_bounds: Some(WindowBounds::Windowed(Bounds {
                            origin: point(px(0.), px(0.)),
                            size: size(px(0.), px(settings.panel_height)),
                        })),
                        window_background: WindowBackgroundAppearance::Transparent,
                        app_id: Some("linux-shell.panel".to_string()),
                        kind: WindowKind::LayerShell(layer_shell_options(
                            &surface_spec,
                            "linux-shell-panel",
                        )),
                        ..Default::default()
                    },
                    |_window, cx| {
                        let config_events = config_events;
                        let compositor_events = compositor_events;
                        let frontend_for_reload = frontend_for_reload.clone();
                        let state_store = state_store.clone();
                        cx.new(|cx| {
                            let view = PanelView::new(
                                initial_snapshot,
                                settings.tokens.clone(),
                                commands,
                                cx,
                            );

                            if let Some(receiver) = config_events {
                                let task = cx.spawn(async move |this, cx| {
                                    loop {
                                        while let Ok(event) = receiver.try_recv() {
                                            match event {
                                                ConfigEvent::Changed(change) => {
                                                    frontend_for_reload.prepare(&change.snapshot);
                                                    let tokens = DesignTokens::from_config(
                                                        &change.snapshot.theme,
                                                    );
                                                    let status = format!(
                                                        "config reload: applied {} file(s) in {} ms",
                                                        change.paths.len(),
                                                        change.duration_ms
                                                    );
                                                    if this
                                                        .update(cx, |view: &mut PanelView, cx| {
                                                            view.tokens = tokens;
                                                            view.status = status.into();
                                                            cx.notify();
                                                        })
                                                        .is_err()
                                                    {
                                                        return;
                                                    }
                                                }
                                                ConfigEvent::Rejected(rejected) => {
                                                    let status = format!(
                                                        "config reload: rejected {} file(s): {}",
                                                        rejected.paths.len(),
                                                        rejected.error
                                                    );
                                                    if this
                                                        .update(cx, |view: &mut PanelView, cx| {
                                                            view.status = status.into();
                                                            cx.notify();
                                                        })
                                                        .is_err()
                                                    {
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                        cx.background_executor()
                                            .timer(Duration::from_millis(100))
                                            .await;
                                    }
                                });
                                task.detach();
                            }

                            if let Some(receiver) = compositor_events {
                                let state_store = state_store.clone();
                                let task = cx.spawn(async move |this, cx| {
                                    loop {
                                        while let Ok(event) = receiver.try_recv() {
                                            let snapshot = event.snapshot;
                                            state_store.replace(snapshot.clone());
                                            if this
                                                .update(cx, |view: &mut PanelView, cx| {
                                                    view.snapshot = snapshot;
                                                    view.status = "compositor: event-driven".into();
                                                    cx.notify();
                                                })
                                                .is_err()
                                            {
                                                return;
                                            }
                                        }
                                        cx.background_executor()
                                            .timer(Duration::from_millis(50))
                                            .await;
                                    }
                                });
                                task.detach();
                            }

                            view
                        })
                    },
                );

                if let Err(error) = result {
                    *startup_error_for_app.borrow_mut() = Some(error.to_string());
                    cx.quit();
                    return;
                }

                if let Some(receiver) = usage_metrics {
                    let result = cx.open_window(
                        WindowOptions {
                            titlebar: None,
                            window_bounds: Some(WindowBounds::Windowed(Bounds {
                                origin: point(px(0.), px(0.)),
                                size: size(px(280.), px(154.)),
                            })),
                            window_background: WindowBackgroundAppearance::Transparent,
                            app_id: Some("linux-shell.debug-overlay".to_string()),
                            kind: WindowKind::LayerShell(debug_layer_shell_options()),
                            ..Default::default()
                        },
                        |window, cx| {
                            // The overlay is visual-only. An empty Wayland
                            // input region lets clicks pass through it.
                            window.set_input_region(Some(&[]));
                            cx.new(|cx| {
                                DebugMetricsView::new(
                                    receiver,
                                    debug_tokens,
                                    debug_state_store,
                                    cx,
                                )
                            })
                        },
                    );
                    if let Err(error) = result {
                        *startup_error_for_app.borrow_mut() = Some(error.to_string());
                        cx.quit();
                        return;
                    }
                }

                if notch_config.enabled {
                    let result = cx.open_window(
                        WindowOptions {
                            titlebar: None,
                            window_bounds: Some(WindowBounds::Windowed(Bounds {
                                origin: point(px(0.), px(0.)),
                                size: size(px(0.), px(0.)),
                            })),
                            window_background: WindowBackgroundAppearance::Transparent,
                            app_id: Some("linux-shell.notch".to_string()),
                            kind: WindowKind::LayerShell(notch_layer_shell_options()),
                            ..Default::default()
                        },
                        |_window, cx| {
                            cx.new(|cx| NotchView::new(notch_config, notch_tokens, cx))
                        },
                    );
                    if let Err(error) = result {
                        *startup_error_for_app.borrow_mut() = Some(error.to_string());
                        cx.quit();
                        return;
                    }
                }

                cx.activate(true);
            });
        });

        if let Err(payload) = run_result {
            return Err(window_initialization_error(format!(
                "GPUI/Wayland panel: {}",
                panic_message(payload)
            )));
        }

        startup_error
            .borrow_mut()
            .take()
            .map_or(Ok(()), |message| Err(window_initialization_error(message)))
    }

    /// Runs the viability surface while consuming debounced configuration
    /// events on a GPUI task. Filesystem callbacks remain outside the render
    /// path; this task only applies already-validated snapshots to the view.
    pub fn run_viability_app_with_watcher(
        &self,
        watcher: ConfigWatcher,
    ) -> Result<(), PlatformError> {
        self.run_viability_app_inner(Some(watcher))
    }

    fn run_viability_app_inner(&self, watcher: Option<ConfigWatcher>) -> Result<(), PlatformError> {
        let startup_error = Rc::new(RefCell::new(None));
        let startup_error_for_app = startup_error.clone();
        let surface_spec = self.surface_spec.borrow().clone();
        let settings = self.settings.borrow().clone();
        let config_events = watcher.map(|watcher| {
            let (sender, receiver) = mpsc::channel();
            thread::spawn(move || {
                loop {
                    match watcher.recv_timeout(Duration::from_secs(1)) {
                        Ok(event) => {
                            if sender.send(event).is_err() {
                                break;
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
            receiver
        });
        let frontend_for_reload = self.clone();

        let run_result = catch_gpui_run(|| {
            application().run(move |cx: &mut App| {
                cx.bind_keys([KeyBinding::new("ctrl-q", Quit, None)]);
                cx.on_action(|_: &Quit, cx| cx.quit());

                let result = cx.open_window(
                    WindowOptions {
                        titlebar: None,
                        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                            None,
                            size(px(settings.width), px(settings.height)),
                            cx,
                        ))),
                        window_background: WindowBackgroundAppearance::Transparent,
                        app_id: Some("linux-shell.gpui-viability".to_string()),
                        kind: WindowKind::LayerShell(layer_shell_options(
                            &surface_spec,
                            "linux-shell-milestone-1",
                        )),
                        ..Default::default()
                    },
                    |window, cx| {
                        let focus_handle = cx.focus_handle();
                        window.focus(&focus_handle, cx);
                        let config_events = config_events;
                        let frontend_for_reload = frontend_for_reload.clone();
                        cx.new(|cx| {
                            let view = ViabilityView {
                            focus_handle,
                            clicks: 0,
                            last_key: "none".into(),
                            tokens: settings.tokens.clone(),
                            reload_status: "config reload: idle".into(),
                            };

                            if let Some(receiver) = config_events {
                                let task = cx.spawn(async move |this, cx| {
                                    loop {
                                        while let Ok(event) = receiver.try_recv() {
                                            match event {
                                                ConfigEvent::Changed(change) => {
                                                    frontend_for_reload.prepare(&change.snapshot);
                                                    let tokens = DesignTokens::from_config(
                                                        &change.snapshot.theme,
                                                    );
                                                    let status = format!(
                                                        "config reload: applied {} file(s) in {} ms",
                                                        change.paths.len(),
                                                        change.duration_ms
                                                    );
                                                    let _ = this.update(cx, |view: &mut ViabilityView, cx| {
                                                        view.tokens = tokens;
                                                        view.reload_status = status.into();
                                                        cx.notify();
                                                    });
                                                }
                                                ConfigEvent::Rejected(rejected) => {
                                                    let status = format!(
                                                        "config reload: rejected {} file(s): {}",
                                                        rejected.paths.len(),
                                                        rejected.error
                                                    );
                                                    let _ = this.update(cx, |view: &mut ViabilityView, cx| {
                                                        view.reload_status = status.into();
                                                        cx.notify();
                                                    });
                                                }
                                            }
                                        }
                                        cx.background_executor()
                                            .timer(Duration::from_millis(100))
                                            .await;
                                    }
                                });
                                task.detach();
                            }

                            view
                        })
                    },
                );

                if let Err(error) = result {
                    *startup_error_for_app.borrow_mut() = Some(error.to_string());
                    cx.quit();
                    return;
                }

                cx.activate(true);
            });
        });

        if let Err(payload) = run_result {
            return Err(window_initialization_error(format!(
                "GPUI/Wayland runtime: {}",
                panic_message(payload)
            )));
        }

        startup_error
            .borrow_mut()
            .take()
            .map_or(Ok(()), |message| Err(window_initialization_error(message)))
    }
}

fn window_initialization_error(message: impl Into<String>) -> PlatformError {
    PlatformError::initialization(format!("GPUI layer-shell window: {}", message.into()))
}

fn catch_gpui_run<F>(run: F) -> Result<(), Box<dyn Any + Send>>
where
    F: FnOnce(),
{
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(AssertUnwindSafe(run));
    panic::set_hook(previous_hook);
    result
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string())
}

impl Default for GpuiFrontend {
    fn default() -> Self {
        Self::new(SurfaceSpec::new(
            OutputId::new(0),
            ShellLayer::Top,
            Anchors::TOP_LEFT_RIGHT,
        ))
    }
}

fn layer_shell_options(spec: &SurfaceSpec, namespace: &str) -> LayerShellOptions {
    let mut anchor = Anchor::empty();
    if spec.anchors.top {
        anchor |= Anchor::TOP;
    }
    if spec.anchors.bottom {
        anchor |= Anchor::BOTTOM;
    }
    if spec.anchors.left {
        anchor |= Anchor::LEFT;
    }
    if spec.anchors.right {
        anchor |= Anchor::RIGHT;
    }

    LayerShellOptions {
        namespace: namespace.to_string(),
        layer: match spec.layer {
            ShellLayer::Background => Layer::Background,
            ShellLayer::Bottom => Layer::Bottom,
            ShellLayer::Top => Layer::Top,
            ShellLayer::Overlay => Layer::Overlay,
        },
        anchor,
        exclusive_zone: spec.exclusive_zone.map(|value| px(value as f32)),
        keyboard_interactivity: match spec.keyboard {
            shell_platform::KeyboardMode::None => KeyboardInteractivity::None,
            shell_platform::KeyboardMode::Exclusive => KeyboardInteractivity::Exclusive,
            shell_platform::KeyboardMode::OnDemand => KeyboardInteractivity::OnDemand,
        },
        ..Default::default()
    }
}

fn debug_layer_shell_options() -> LayerShellOptions {
    LayerShellOptions {
        namespace: "linux-shell-debug-overlay".to_string(),
        layer: Layer::Overlay,
        anchor: Anchor::BOTTOM | Anchor::RIGHT,
        margin: Some((px(0.), px(16.), px(16.), px(0.))),
        keyboard_interactivity: KeyboardInteractivity::None,
        ..Default::default()
    }
}

fn notch_layer_shell_options() -> LayerShellOptions {
    LayerShellOptions {
        namespace: "linux-shell-notch".to_string(),
        layer: Layer::Top,
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        ..Default::default()
    }
}

struct NotchView {
    config: NotchConfig,
    tokens: DesignTokens,
    state: NotchState,
    clock: SharedString,
    query: String,
    focus_manager: FocusManager,
    focus_handle: FocusHandle,
    animation: NotchAnimation,
    animation_task_running: bool,
}

impl NotchView {
    fn new(config: NotchConfig, tokens: DesignTokens, cx: &mut Context<Self>) -> Self {
        let initial_geometry = NotchGeometry::for_state(&config, NotchState::Idle);
        let task = cx.spawn(async move |this, cx| {
            loop {
                if this
                    .update(cx, |view: &mut NotchView, cx| {
                        view.clock = local_clock();
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
                cx.background_executor().timer(Duration::from_secs(1)).await;
            }
        });
        task.detach();

        Self {
            config,
            tokens,
            state: NotchState::Idle,
            clock: local_clock(),
            query: String::new(),
            focus_manager: FocusManager::default(),
            focus_handle: cx.focus_handle(),
            animation: NotchAnimation::new(initial_geometry),
            animation_task_running: false,
        }
    }

    fn set_state(&mut self, state: NotchState, window: &mut Window, cx: &mut Context<Self>) {
        if self.state == state && !self.animation.is_running() {
            return;
        }

        self.state = state;
        match state {
            NotchState::Idle => {
                self.focus_manager.dismiss_notch();
                self.query.clear();
                window.blur(cx);
            }
            NotchState::Launcher => {
                self.focus_manager.activate_notch();
                window.focus(&self.focus_handle, cx);
            }
        }

        let target = NotchGeometry::for_state(&self.config, state);
        let duration_ms = if self.config.animation.enabled {
            self.config.animation.duration_ms
        } else {
            0
        };
        self.animation.retarget(target, duration_ms);
        self.start_animation_task(cx);
        cx.notify();
    }

    fn start_animation_task(&mut self, cx: &mut Context<Self>) {
        if self.animation_task_running || !self.animation.is_running() {
            return;
        }

        self.animation_task_running = true;
        let task = cx.spawn(async move |this, cx| {
            loop {
                let running = match this.update(cx, |view: &mut NotchView, cx| {
                    if view.animation.tick(16) {
                        cx.notify();
                        true
                    } else {
                        view.animation_task_running = false;
                        cx.notify();
                        false
                    }
                }) {
                    Ok(running) => running,
                    Err(_) => return,
                };

                if !running {
                    return;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
            }
        });
        task.detach();
    }

    fn surface_size(window: &Window) -> (f32, f32) {
        let viewport = window.viewport_size();
        (f32::from(viewport.width), f32::from(viewport.height))
    }
}

impl Focusable for NotchView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NotchView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (surface_width, surface_height) = Self::surface_size(window);
        let geometry = self.animation.current();
        let notch_bounds = geometry.bounds(surface_width, surface_height);
        let input_rects = if self.focus_manager.owns_modal_pointer() {
            vec![Bounds {
                origin: point(px(0.), px(0.)),
                size: size(px(surface_width), px(surface_height)),
            }]
        } else {
            geometry
                .input_rects(surface_width, surface_height)
                .into_iter()
                .map(|rect| Bounds {
                    origin: point(px(rect.origin.x), px(rect.origin.y)),
                    size: size(px(rect.size.width), px(rect.size.height)),
                })
                .collect()
        };
        window.set_input_region(Some(&input_rects));

        let focus_handle = self.focus_handle.clone();
        let state = self.state;
        let geometry_for_click = geometry;
        let tokens = self.tokens.clone();
        let paint_tokens = tokens.clone();
        let clock = self.clock.clone();
        let query = self.query.clone();

        div()
            .id("shell-notch-root")
            .track_focus(&focus_handle)
            .size_full()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                    let (surface_width, surface_height) = NotchView::surface_size(window);
                    let point =
                        Point::new(f32::from(event.position.x), f32::from(event.position.y));
                    let inside = geometry_for_click.contains(point, surface_width, surface_height);

                    if view.focus_manager.owns_modal_pointer() {
                        if view.focus_manager.click_outside(inside) {
                            view.set_state(NotchState::Idle, window, cx);
                        }
                    } else if inside {
                        view.set_state(NotchState::Launcher, window, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && view.focus_manager.escape() {
                    view.set_state(NotchState::Idle, window, cx);
                } else if view.focus_manager.owns_keyboard() {
                    let changed = if event.keystroke.key == "backspace" {
                        view.query.pop().is_some()
                    } else if let Some(character) = &event.keystroke.key_char {
                        view.query.push_str(character);
                        true
                    } else {
                        false
                    };
                    if changed {
                        cx.notify();
                    }
                }
            }))
            .child(
                div()
                    .absolute()
                    .left(px(notch_bounds.origin.x))
                    .top(px(notch_bounds.origin.y))
                    .w(px(notch_bounds.size.width))
                    .h(px(notch_bounds.size.height))
                    .child(
                        canvas(
                            move |_, _, _| build_notch_path(geometry),
                            move |_, path: Option<Path<Pixels>>, window, _| {
                                if let Some(path) = path {
                                    window.paint_path(
                                        path,
                                        rgba(paint_tokens.colors.background_overlay),
                                    );
                                }
                            },
                        )
                        .size_full(),
                    )
                    .when(state == NotchState::Launcher, |element| {
                        element.child(launcher_content(
                            &query,
                            &tokens,
                            geometry.width,
                            geometry.height,
                        ))
                    }),
            )
            .when(state == NotchState::Idle, |element| {
                element.child(idle_content(&clock, &tokens))
            })
    }
}

const LAUNCHER_ITEMS: &[(&str, &str, &str)] = &[
    (
        "◉",
        "About Xfce",
        "Information about the Xfce Desktop Environment",
    ),
    (
        "▣",
        "Advanced Network Configuration",
        "Manage and change network connection settings",
    ),
    ("△", "Alacritty", "Terminal"),
    (
        "◌",
        "AsusCtlTray",
        "A tray icon to switch asusctl profiles on the fly",
    ),
    (
        "✣",
        "auto-cpufreq",
        "Automatic CPU frequency and power optimizer",
    ),
    (
        "◈",
        "Avahi SSH Server Browser",
        "Browse for Zeroconf-enabled SSH Servers",
    ),
];

fn idle_content(clock: &SharedString, tokens: &DesignTokens) -> impl IntoElement {
    div()
        .absolute()
        .top_0()
        .left_0()
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(tokens.spacing.sm as f32))
        .text_color(rgb(tokens.colors.foreground))
        .text_size(px(tokens.typography.label_size as f32))
        .child(
            div()
                .text_size(px(tokens.typography.label_size as f32 - 2.0))
                .child("▮▮▮"),
        )
        .child(clock.clone())
}

fn launcher_content(
    query: &str,
    tokens: &DesignTokens,
    width: f32,
    height: f32,
) -> impl IntoElement {
    let query_lower = query.to_lowercase();
    let rows = LAUNCHER_ITEMS
        .iter()
        .filter(|(_, name, description)| {
            query_lower.is_empty()
                || name.to_lowercase().contains(&query_lower)
                || description.to_lowercase().contains(&query_lower)
        })
        .map(|(icon, name, description)| {
            div()
                .w_full()
                .h(px(44.0))
                .px(px(tokens.spacing.sm as f32))
                .rounded(px(tokens.radius.sm as f32))
                .flex()
                .items_center()
                .gap(px(tokens.spacing.sm as f32))
                .bg(rgba(0x242424e8))
                .text_color(rgb(tokens.colors.foreground))
                .child(
                    div()
                        .w(px(28.0))
                        .h(px(28.0))
                        .rounded(px(tokens.radius.sm as f32))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(rgba(0x3b3b3bf0))
                        .text_size(px(tokens.typography.title_size as f32 - 2.0))
                        .child(*icon),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(1.0))
                        .child(
                            div()
                                .text_size(px(tokens.typography.body_size as f32))
                                .font_weight(FontWeight::BOLD)
                                .child(*name),
                        )
                        .child(
                            div()
                                .text_size(px(tokens.typography.label_size as f32 - 1.0))
                                .text_color(rgba(0xaaa6a6e6))
                                .child(*description),
                        ),
                )
        });

    div()
        .absolute()
        .top_0()
        .left_0()
        .w(px(width))
        .h(px(height))
        .p(px(tokens.spacing.md as f32))
        .flex()
        .flex_col()
        .gap(px(tokens.spacing.sm as f32))
        .text_color(rgb(tokens.colors.foreground))
        .child(
            div()
                .w_full()
                .h(px(40.0))
                .px(px(tokens.spacing.sm as f32))
                .flex()
                .items_center()
                .gap(px(tokens.spacing.sm as f32))
                .border_b(px(1.0))
                .border_color(rgba(0x8b8b8b66))
                .text_size(px(tokens.typography.body_size as f32))
                .child("⌕")
                .child(if query.is_empty() {
                    "Search...".to_string()
                } else {
                    query.to_owned()
                }),
        )
        .child(div().flex().flex_col().gap(px(2.0)).children(rows))
}

fn build_notch_path(geometry: NotchGeometry) -> Option<Path<Pixels>> {
    let width = px(geometry.width);
    let height = px(geometry.height);
    let radius_value = geometry
        .radius
        .max(geometry.corner_size)
        .min(geometry.width / 2.0)
        .min(geometry.height / 2.0);
    let radius = px(radius_value);
    let mut builder = PathBuilder::fill();

    match geometry.edge {
        shell_core::NotchEdge::Top => {
            builder.move_to(point(px(0.), px(0.)));
            builder.line_to(point(width, px(0.)));
            builder.line_to(point(width, height - radius));
            builder.curve_to(point(width - radius, height), point(width, height));
            builder.line_to(point(radius, height));
            builder.curve_to(point(px(0.), height - radius), point(px(0.), height));
            builder.line_to(point(px(0.), px(0.)));
        }
        shell_core::NotchEdge::Bottom => {
            builder.move_to(point(radius, px(0.)));
            builder.curve_to(point(px(0.), radius), point(px(0.), px(0.)));
            builder.line_to(point(px(0.), height));
            builder.line_to(point(width, height));
            builder.line_to(point(width, radius));
            builder.curve_to(point(width - radius, px(0.)), point(width, px(0.)));
        }
    }

    builder.close();
    builder.build().ok()
}

struct DebugMetricsView {
    metrics: AppUsageMetrics,
    tokens: DesignTokens,
    outputs: usize,
    workspaces: usize,
}

impl DebugMetricsView {
    fn new(
        receiver: mpsc::Receiver<AppUsageMetrics>,
        tokens: DesignTokens,
        state_store: CompositorStateStore,
        cx: &mut Context<Self>,
    ) -> Self {
        let task = cx.spawn(async move |this, cx| {
            loop {
                while let Ok(metrics) = receiver.try_recv() {
                    let (outputs, workspaces) = state_store
                        .snapshot()
                        .map(|snapshot| (snapshot.monitors.len(), snapshot.workspaces.len()))
                        .unwrap_or_default();
                    if this
                        .update(cx, |view: &mut DebugMetricsView, cx| {
                            view.metrics = metrics;
                            view.outputs = outputs;
                            view.workspaces = workspaces;
                            cx.notify();
                        })
                        .is_err()
                    {
                        return;
                    }
                }
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
            }
        });
        task.detach();

        Self {
            metrics: AppUsageMetrics::default(),
            tokens,
            outputs: 0,
            workspaces: 0,
        }
    }
}

impl Render for DebugMetricsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = self.tokens.clone();
        let cpu = self
            .metrics
            .cpu_percent
            .map_or_else(|| "n/a".to_string(), |value| format!("{value:.1}%"));
        let gpu = self.metrics.gpu_percent.map_or_else(
            || "n/a".to_string(),
            |value| {
                if self.metrics.gpu_is_system {
                    format!("{value:.1}% (sys)")
                } else {
                    format!("{value:.1}%")
                }
            },
        );

        div()
            .id("shell-debug-overlay-root")
            .size_full()
            .flex()
            .justify_end()
            .items_end()
            .p(px(tokens.spacing.md as f32))
            .text_color(rgb(tokens.colors.foreground))
            .child(
                div()
                    .p(px(tokens.spacing.md as f32))
                    .rounded(px(tokens.radius.md as f32))
                    .bg(rgba(tokens.colors.background_overlay))
                    .text_size(px(tokens.typography.label_size as f32))
                    .flex()
                    .flex_col()
                    .gap(px(tokens.spacing.sm as f32))
                    .child(
                        div()
                            .text_size(px(tokens.typography.title_size as f32))
                            .font_weight(FontWeight::BOLD)
                            .child("DEBUG"),
                    )
                    .child(format!("CPU  {cpu}"))
                    .child(format!("RAM  {}", format_bytes(self.metrics.ram_bytes)))
                    .child(format!("GPU  {gpu}"))
                    .child(format!("THR  {}", self.metrics.thread_count))
                    .child(format!(
                        "UP   {}",
                        format_uptime(self.metrics.uptime_seconds)
                    ))
                    .child(format!("OUT  {}   WS  {}", self.outputs, self.workspaces)),
            )
    }
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes / GIB)
    } else {
        format!("{:.1} MiB", bytes / MIB)
    }
}

fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds / 60) % 60;
    let seconds = seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

struct PanelView {
    snapshot: CompositorSnapshot,
    tokens: DesignTokens,
    clock: SharedString,
    status: SharedString,
    commands: mpsc::Sender<ShellCommand>,
}

impl PanelView {
    fn new(
        snapshot: CompositorSnapshot,
        tokens: DesignTokens,
        commands: mpsc::Sender<ShellCommand>,
        cx: &mut Context<Self>,
    ) -> Self {
        let task = cx.spawn(async move |this, cx| {
            loop {
                let clock = local_clock();
                if this
                    .update(cx, |view: &mut PanelView, cx| {
                        view.clock = clock;
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
                cx.background_executor().timer(Duration::from_secs(1)).await;
            }
        });
        task.detach();

        Self {
            snapshot,
            tokens,
            clock: local_clock(),
            status: "compositor: waiting for events".into(),
            commands,
        }
    }
}

impl Render for PanelView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = self.tokens.clone();
        let focused_workspace = self.snapshot.focused_workspace;
        let commands = self.commands.clone();

        let workspaces = self.snapshot.workspaces.iter().map(move |workspace| {
            let workspace_id = workspace.id;
            let focused = workspace.active || Some(workspace_id) == focused_workspace;
            let label = workspace
                .name
                .clone()
                .unwrap_or_else(|| workspace_id.to_string());
            let mut button = div()
                .id(format!("workspace-{workspace_id}"))
                .px(px(tokens.spacing.md as f32))
                .py(px(tokens.spacing.sm as f32))
                .rounded(px(tokens.radius.sm as f32))
                .text_size(px(tokens.typography.label_size as f32))
                .child(label);

            if focused {
                button = button
                    .bg(rgb(tokens.colors.foreground))
                    .text_color(rgb(tokens.colors.background));
            } else {
                button = button.text_color(rgb(tokens.colors.foreground));
            }

            let commands = commands.clone();
            button.on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _cx| {
                if let Err(error) = commands.send(ShellCommand::FocusWorkspace(workspace_id)) {
                    tracing::warn!(%error, ?workspace_id, "could not enqueue workspace focus command");
                }
            })
        });

        div()
            .id("shell-panel-root")
            .size_full()
            .px(px(tokens.spacing.md as f32))
            .flex()
            .items_center()
            .gap(px(tokens.spacing.md as f32))
            .bg(rgba(tokens.colors.background_overlay))
            .text_color(rgb(tokens.colors.foreground))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(tokens.spacing.sm as f32))
                    .flex_1()
                    .children(workspaces),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .text_size(px(tokens.typography.title_size as f32))
                    .font_weight(FontWeight::BOLD)
                    .child(self.clock.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .flex_1()
                    .text_size(px(tokens.typography.label_size as f32))
                    .child(self.status.clone()),
            )
    }
}

fn local_clock() -> SharedString {
    Local::now().format("%H:%M").to_string().into()
}

struct ViabilityView {
    focus_handle: FocusHandle,
    clicks: usize,
    last_key: SharedString,
    tokens: DesignTokens,
    reload_status: SharedString,
}

impl Focusable for ViabilityView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ViabilityView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks;
        let last_key = self.last_key.clone();
        let tokens = self.tokens.clone();
        let reload_status = self.reload_status.clone();

        div()
            .id("gpui-viability-root")
            .track_focus(&self.focus_handle)
            .size_full()
            .p(px(tokens.spacing.lg as f32))
            .flex()
            .flex_col()
            .gap(px(tokens.spacing.md as f32))
            .justify_center()
            .items_center()
            .text_color(rgb(tokens.colors.foreground))
            .child(
                div()
                    .p(px(tokens.spacing.lg as f32))
                    .gap(px(tokens.spacing.md as f32))
                    .flex()
                    .flex_col()
                    .items_center()
                    .rounded(px(tokens.radius.md as f32))
                    .bg(rgba(tokens.colors.background_overlay))
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child("Linux Shell — GPUI viability"),
                    )
                    .child("Wayland layer-shell surface, transparent background and input")
                    .child(format!("Pointer clicks: {clicks}"))
                    .child(format!("Last key: {last_key}"))
                    .child(reload_status)
                    .child("Press Ctrl+Q or close the surface to test shutdown.")
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|view, _event, _window, cx| {
                            view.clicks += 1;
                            cx.notify();
                        }),
                    )
                    .on_key_down(cx.listener(|view, event: &KeyDownEvent, _window, cx| {
                        view.last_key = event.keystroke.to_string().into();
                        cx.notify();
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::window_initialization_error;
    use shell_core::PlatformError;

    #[test]
    fn window_creation_errors_are_platform_initialization_errors() {
        assert_eq!(
            window_initialization_error("renderer unavailable"),
            PlatformError::initialization("GPUI layer-shell window: renderer unavailable")
        );
    }

    #[test]
    fn local_clock_is_renderable_as_hh_mm() {
        let clock = super::local_clock();
        assert_eq!(clock.len(), 5);
        assert_eq!(clock.as_bytes()[2], b':');
    }
}
