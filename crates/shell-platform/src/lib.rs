//! Platform-facing semantic types.
//!
//! These types describe shell-surface behavior without depending on GPUI or a
//! concrete Wayland client implementation.

mod topology;

use shell_core::{OutputId, Point, Rect};

pub use shell_core::PlatformError;
pub use topology::{
    OutputRegistry, OutputState, OutputStateError, OutputTransition, SurfaceMetrics, SurfaceOwner,
    SurfaceRole, SurfaceTopology,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellLayer {
    Background,
    Bottom,
    Top,
    Overlay,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Anchors {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl Anchors {
    pub const NONE: Self = Self {
        top: false,
        bottom: false,
        left: false,
        right: false,
    };

    pub const TOP: Self = Self {
        top: true,
        ..Self::NONE
    };

    pub const TOP_LEFT_RIGHT: Self = Self {
        top: true,
        left: true,
        right: true,
        bottom: false,
    };

    pub const BOTTOM_LEFT_RIGHT: Self = Self {
        top: false,
        bottom: true,
        left: true,
        right: true,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardMode {
    None,
    Exclusive,
    OnDemand,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InputRegion {
    CompositorManaged,
    Fullscreen,
    Rectangles(Vec<Rect>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceSpec {
    pub output: OutputId,
    pub layer: ShellLayer,
    pub anchors: Anchors,
    pub exclusive_zone: Option<i32>,
    pub keyboard: KeyboardMode,
    pub input_region: InputRegion,
}

impl SurfaceSpec {
    pub fn new(output: OutputId, layer: ShellLayer, anchors: Anchors) -> Self {
        Self {
            output,
            layer,
            anchors,
            exclusive_zone: None,
            keyboard: KeyboardMode::None,
            input_region: InputRegion::CompositorManaged,
        }
    }

    pub fn contains_input(&self, point: Point) -> bool {
        match &self.input_region {
            InputRegion::CompositorManaged | InputRegion::Fullscreen => true,
            InputRegion::Rectangles(rectangles) => {
                rectangles.iter().any(|rect| rect.contains(point))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use shell_core::{OutputId, Point, Rect};

    use super::{Anchors, InputRegion, ShellLayer, SurfaceSpec};

    #[test]
    fn surface_spec_keeps_layer_shell_semantics_outside_the_ui() {
        let mut spec = SurfaceSpec::new(OutputId::new(1), ShellLayer::Top, Anchors::TOP);
        spec.input_region = InputRegion::Rectangles(vec![Rect::from_xywh(0.0, 0.0, 100.0, 20.0)]);

        assert!(spec.contains_input(Point::new(10.0, 10.0)));
        assert!(!spec.contains_input(Point::new(10.0, 30.0)));
    }
}
