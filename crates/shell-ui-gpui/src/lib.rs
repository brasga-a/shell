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

use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, FontWeight, KeyBinding, KeyDownEvent, Render,
    SharedString, Window, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
    actions, div, layer_shell::*, prelude::*, px, rgb, rgba, size,
};
use gpui_platform::application;
use shell_config::{ConfigEvent, ConfigWatcher};
use shell_core::{
    BarPosition, KeyboardModeConfig, NotchConfig, OutputId, PlatformError, ShellConfig,
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
}

impl Default for FrontendSettings {
    fn default() -> Self {
        let notch = NotchConfig::default();
        Self {
            tokens: DesignTokens::default(),
            width: notch.width as f32,
            height: notch.expanded_height as f32,
        }
    }
}

impl FrontendSettings {
    fn from_config(config: &ShellConfig) -> Self {
        Self {
            tokens: DesignTokens::from_config(&config.theme),
            width: config.notch.width as f32,
            height: config.notch.expanded_height as f32,
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
                        kind: WindowKind::LayerShell(layer_shell_options(&surface_spec)),
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

fn layer_shell_options(spec: &SurfaceSpec) -> LayerShellOptions {
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
        namespace: "linux-shell-milestone-1".to_string(),
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
}
