use crate::{BarPosition, NotchConfig, Point, Rect};

/// Semantic routes currently supported by the notch.
///
/// This is deliberately independent from the renderer. The UI may animate or
/// paint a route differently without changing the application state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NotchState {
    #[default]
    Idle,
    Launcher,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NotchEdge {
    #[default]
    Top,
    Bottom,
}

impl From<BarPosition> for NotchEdge {
    fn from(position: BarPosition) -> Self {
        match position {
            BarPosition::Top => Self::Top,
            BarPosition::Bottom => Self::Bottom,
        }
    }
}

/// Renderer-independent geometry for the notch container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NotchGeometry {
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub corner_size: f32,
    pub edge: NotchEdge,
}

impl NotchGeometry {
    pub fn new(width: f32, height: f32, radius: f32, corner_size: f32, edge: NotchEdge) -> Self {
        let width = finite_or(width, 1.0).max(1.0);
        let height = finite_or(height, 1.0).max(1.0);
        let corner_size = finite_or(corner_size, 0.0)
            .max(0.0)
            .min(width / 2.0)
            .min(height);
        let radius = finite_or(radius, 0.0)
            .max(0.0)
            .min(width / 2.0)
            .min(height / 2.0);
        Self {
            width,
            height,
            radius,
            corner_size,
            edge,
        }
    }

    pub fn for_state(config: &NotchConfig, state: NotchState) -> Self {
        match state {
            NotchState::Idle => Self::new(
                config.collapsed_width as f32,
                config.collapsed_height as f32,
                config.corner_radius as f32,
                config.corner_size as f32,
                config.edge.into(),
            ),
            NotchState::Launcher => Self::new(
                config.width as f32,
                config.expanded_height as f32,
                config.corner_radius as f32,
                config.corner_size as f32,
                config.edge.into(),
            ),
        }
    }

    pub fn bounds(self, surface_width: f32, surface_height: f32) -> Rect {
        let surface_width = finite_or(surface_width, self.width).max(self.width);
        let surface_height = finite_or(surface_height, self.height).max(self.height);
        let x = (surface_width - self.width) / 2.0;
        let y = match self.edge {
            NotchEdge::Top => 0.0,
            NotchEdge::Bottom => surface_height - self.height,
        };
        Rect::from_xywh(x, y, self.width, self.height)
    }

    /// Returns a conservative union of rectangles matching the interactive
    /// portion of the notch. The concave top/bottom corner cutouts remain
    /// transparent to the compositor.
    pub fn input_rects(self, surface_width: f32, surface_height: f32) -> Vec<Rect> {
        let bounds = self.bounds(surface_width, surface_height);
        let corner = self.corner_size.min(self.width / 2.0);
        let middle_width = self.width - 2.0 * corner;
        let mut rects = Vec::with_capacity(3);

        let body_height = (self.height - corner).max(0.0);
        match self.edge {
            NotchEdge::Top => {
                if body_height > 0.0 {
                    rects.push(Rect::from_xywh(
                        bounds.origin.x,
                        bounds.origin.y,
                        self.width,
                        body_height,
                    ));
                }
                if middle_width > 0.0 && corner > 0.0 {
                    rects.push(Rect::from_xywh(
                        bounds.origin.x + corner,
                        bounds.bottom() - corner,
                        middle_width,
                        corner,
                    ));
                }
            }
            NotchEdge::Bottom => {
                if middle_width > 0.0 && corner > 0.0 {
                    rects.push(Rect::from_xywh(
                        bounds.origin.x + corner,
                        bounds.origin.y,
                        middle_width,
                        corner,
                    ));
                }
                if body_height > 0.0 {
                    rects.push(Rect::from_xywh(
                        bounds.origin.x,
                        bounds.origin.y + corner,
                        self.width,
                        body_height,
                    ));
                }
            }
        }

        rects
    }

    pub fn contains(self, point: Point, surface_width: f32, surface_height: f32) -> bool {
        self.input_rects(surface_width, surface_height)
            .into_iter()
            .any(|rect| rect.contains(point))
    }

    pub fn lerp(self, target: Self, progress: f32) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        Self::new(
            lerp(self.width, target.width, progress),
            lerp(self.height, target.height, progress),
            lerp(self.radius, target.radius, progress),
            lerp(self.corner_size, target.corner_size, progress),
            if progress < 0.5 {
                self.edge
            } else {
                target.edge
            },
        )
    }
}

/// Interruptible, renderer-independent transition between two geometries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NotchAnimation {
    from: NotchGeometry,
    to: NotchGeometry,
    current: NotchGeometry,
    elapsed_ms: u64,
    duration_ms: u64,
}

impl NotchAnimation {
    pub fn new(initial: NotchGeometry) -> Self {
        Self {
            from: initial,
            to: initial,
            current: initial,
            elapsed_ms: 0,
            duration_ms: 0,
        }
    }

    pub fn current(self) -> NotchGeometry {
        self.current
    }

    pub fn is_running(self) -> bool {
        self.current != self.to
    }

    pub fn retarget(&mut self, target: NotchGeometry, duration_ms: u64) {
        let current = self.current;
        self.from = current;
        self.to = target;
        self.current = current;
        self.elapsed_ms = 0;
        self.duration_ms = duration_ms;
        if duration_ms == 0 || current == target {
            self.current = target;
            self.from = target;
            self.elapsed_ms = 0;
        }
    }

    /// Advances the transition and returns whether another frame is needed.
    pub fn tick(&mut self, delta_ms: u64) -> bool {
        if !self.is_running() {
            return false;
        }

        self.elapsed_ms = self.elapsed_ms.saturating_add(delta_ms);
        let progress = if self.duration_ms == 0 {
            1.0
        } else {
            (self.elapsed_ms as f32 / self.duration_ms as f32).min(1.0)
        };
        let eased = progress * progress * (3.0 - 2.0 * progress);
        self.current = self.from.lerp(self.to, eased);
        if progress >= 1.0 {
            self.current = self.to;
            self.from = self.to;
            false
        } else {
            true
        }
    }
}

/// Coordinates focus and modal ownership for the notch surface.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FocusManager {
    notch_active: bool,
}

impl FocusManager {
    pub fn activate_notch(&mut self) {
        self.notch_active = true;
    }

    pub fn dismiss_notch(&mut self) {
        self.notch_active = false;
    }

    pub fn owns_keyboard(self) -> bool {
        self.notch_active
    }

    pub fn owns_modal_pointer(self) -> bool {
        self.notch_active
    }

    pub fn click_outside(self, inside_notch: bool) -> bool {
        self.notch_active && !inside_notch
    }

    pub fn escape(self) -> bool {
        self.notch_active
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

#[cfg(test)]
mod tests {
    use super::{FocusManager, NotchAnimation, NotchEdge, NotchGeometry, NotchState};
    use crate::{BarPosition, NotchConfig, Point};

    #[test]
    fn geometry_keeps_concave_corners_inside_surface() {
        let geometry = NotchGeometry::new(420.0, 120.0, 16.0, 32.0, NotchEdge::Top);
        let rects = geometry.input_rects(1920.0, 1080.0);

        assert_eq!(rects.len(), 2);
        assert!(!geometry.contains(Point::new(750.0, 119.0), 1920.0, 1080.0));
        assert!(geometry.contains(Point::new(960.0, 119.0), 1920.0, 1080.0));
    }

    #[test]
    fn geometry_maps_config_to_idle_and_launcher() {
        let config = NotchConfig {
            edge: BarPosition::Bottom,
            ..NotchConfig::default()
        };

        let idle = NotchGeometry::for_state(&config, NotchState::Idle);
        let launcher = NotchGeometry::for_state(&config, NotchState::Launcher);

        assert!(idle.width < launcher.width);
        assert!(idle.height < launcher.height);
        assert_eq!(idle.edge, NotchEdge::Bottom);
    }

    #[test]
    fn animation_retargets_from_current_value() {
        let idle = NotchGeometry::new(180.0, 32.0, 14.0, 28.0, NotchEdge::Top);
        let expanded = NotchGeometry::new(420.0, 420.0, 14.0, 28.0, NotchEdge::Top);
        let mut animation = NotchAnimation::new(idle);
        animation.retarget(expanded, 100);
        assert!(animation.tick(50));
        let halfway = animation.current();
        animation.retarget(idle, 100);
        assert!(animation.tick(50));
        assert!(animation.current().width < halfway.width);
        assert!(!animation.tick(100));
        assert_eq!(animation.current(), idle);
    }

    #[test]
    fn focus_manager_handles_modal_dismissal() {
        let mut manager = FocusManager::default();
        assert!(!manager.owns_keyboard());
        manager.activate_notch();
        assert!(manager.owns_keyboard());
        assert!(manager.click_outside(false));
        assert!(manager.escape());
        manager.dismiss_notch();
        assert!(!manager.owns_modal_pointer());
    }
}
