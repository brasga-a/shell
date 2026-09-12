//! Output-owned shell surface topology and lifecycle state.
//!
//! This module deliberately contains no GPUI or Wayland proxy. It is the
//! platform-facing state machine that a renderer-specific surface adapter can
//! consume later.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use shell_core::{BarConfig, NotchConfig, Output, OutputId, Point, Rect, SurfaceId};

use crate::InputRegion;

/// The two surface topologies evaluated by Milestone 2.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SurfaceTopology {
    /// One fullscreen transparent surface per output.
    Unified,
    /// Separate panel, notch and overlay surfaces per output.
    #[default]
    Independent,
}

impl SurfaceTopology {
    /// Returns the surface roles owned by an output in this topology.
    pub fn roles(self) -> &'static [SurfaceRole] {
        match self {
            Self::Unified => &[SurfaceRole::Unified],
            Self::Independent => &[SurfaceRole::Panel, SurfaceRole::Notch, SurfaceRole::Overlay],
        }
    }
}

/// A logical surface role owned by one output.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SurfaceRole {
    Unified,
    Panel,
    Notch,
    Overlay,
}

/// Configurable geometry for output-owned module surfaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceMetrics {
    pub panel_height: f32,
    pub notch_width: f32,
    pub notch_height: f32,
}

impl Default for SurfaceMetrics {
    fn default() -> Self {
        let bar = BarConfig::default();
        let notch = NotchConfig::default();
        Self {
            panel_height: bar.height as f32,
            notch_width: notch.width as f32,
            notch_height: notch.collapsed_height as f32,
        }
    }
}

impl SurfaceMetrics {
    pub fn new(panel_height: f32, notch_width: f32, notch_height: f32) -> Self {
        Self {
            panel_height,
            notch_width,
            notch_height,
        }
    }

    fn validate(self) -> Result<(), OutputStateError> {
        if !self.panel_height.is_finite()
            || !self.notch_width.is_finite()
            || !self.notch_height.is_finite()
            || self.panel_height <= 0.0
            || self.notch_width <= 0.0
            || self.notch_height <= 0.0
        {
            return Err(OutputStateError::InvalidSurfaceMetrics);
        }
        Ok(())
    }
}

/// A surface owner and its output-local geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceOwner {
    pub id: SurfaceId,
    pub output_id: OutputId,
    pub role: SurfaceRole,
    pub geometry: Rect,
    pub scale: f32,
    pub input_region: InputRegion,
}

impl SurfaceOwner {
    /// Converts a point in logical output coordinates to physical coordinates.
    ///
    /// The conversion is local to the output, so output-global origins are
    /// preserved and only the output-local offset is scaled.
    pub fn logical_to_physical(&self, point: Point) -> Point {
        scale_point(point, self.geometry, self.scale)
    }

    /// Converts a point in physical output coordinates back to logical space.
    pub fn physical_to_logical(&self, point: Point) -> Point {
        unscale_point(point, self.geometry, self.scale)
    }
}

/// Runtime state and surface ownership for one output.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputState {
    pub id: OutputId,
    pub name: String,
    pub geometry: Rect,
    pub scale: f32,
    pub surfaces: BTreeMap<SurfaceRole, SurfaceOwner>,
}

impl OutputState {
    fn try_new(
        output: Output,
        topology: SurfaceTopology,
        metrics: SurfaceMetrics,
        allocator: &mut SurfaceIdAllocator,
    ) -> Result<Self, OutputStateError> {
        validate_output(&output)?;

        let mut surfaces = BTreeMap::new();
        for role in topology.roles() {
            let role = *role;
            let surface_id = allocator.allocate()?;
            surfaces.insert(
                role,
                SurfaceOwner {
                    id: surface_id,
                    output_id: output.id,
                    role,
                    geometry: surface_geometry(output.geometry, role, metrics),
                    scale: output.scale,
                    input_region: InputRegion::CompositorManaged,
                },
            );
        }

        Ok(Self {
            id: output.id,
            name: output.name,
            geometry: output.geometry,
            scale: output.scale,
            surfaces,
        })
    }

    fn update(
        &mut self,
        output: Output,
        metrics: SurfaceMetrics,
    ) -> Result<Option<OutputChange>, OutputStateError> {
        validate_output(&output)?;
        if self.name == output.name
            && self.geometry == output.geometry
            && self.scale == output.scale
        {
            return Ok(None);
        }

        let change = OutputChange {
            previous_geometry: self.geometry,
            previous_scale: self.scale,
        };

        self.name = output.name;
        self.geometry = output.geometry;
        self.scale = output.scale;
        for (role, surface) in &mut self.surfaces {
            surface.geometry = surface_geometry(output.geometry, *role, metrics);
            surface.scale = output.scale;
        }

        Ok(Some(change))
    }

    pub fn surface(&self, role: SurfaceRole) -> Option<&SurfaceOwner> {
        self.surfaces.get(&role)
    }

    pub fn logical_to_physical(&self, point: Point) -> Point {
        scale_point(point, self.geometry, self.scale)
    }

    pub fn physical_to_logical(&self, point: Point) -> Point {
        unscale_point(point, self.geometry, self.scale)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OutputChange {
    previous_geometry: Rect,
    previous_scale: f32,
}

/// A change emitted while reconciling output state.
#[derive(Clone, Debug, PartialEq)]
pub enum OutputTransition {
    Added(OutputState),
    Changed {
        output: OutputState,
        previous_geometry: Rect,
        previous_scale: f32,
    },
    Removed {
        id: OutputId,
        surfaces: Vec<SurfaceOwner>,
    },
    Focused {
        previous: Option<OutputId>,
        current: Option<OutputId>,
    },
}

/// A stable output registry that handles hotplug snapshots without restart.
#[derive(Clone, Debug)]
pub struct OutputRegistry {
    topology: SurfaceTopology,
    metrics: SurfaceMetrics,
    outputs: BTreeMap<OutputId, OutputState>,
    focused_output: Option<OutputId>,
    allocator: SurfaceIdAllocator,
}

impl OutputRegistry {
    pub fn new(topology: SurfaceTopology) -> Self {
        Self {
            topology,
            metrics: SurfaceMetrics::default(),
            outputs: BTreeMap::new(),
            focused_output: None,
            allocator: SurfaceIdAllocator::default(),
        }
    }

    pub fn topology(&self) -> SurfaceTopology {
        self.topology
    }

    pub fn surface_metrics(&self) -> SurfaceMetrics {
        self.metrics
    }

    /// Applies module geometry before the next compositor reconciliation.
    /// Existing output surfaces are relaid out atomically as well.
    pub fn set_surface_metrics(&mut self, metrics: SurfaceMetrics) -> Result<(), OutputStateError> {
        metrics.validate()?;
        if self.metrics == metrics {
            return Ok(());
        }

        self.metrics = metrics;
        for state in self.outputs.values_mut() {
            for (role, surface) in &mut state.surfaces {
                surface.geometry = surface_geometry(state.geometry, *role, metrics);
            }
        }
        Ok(())
    }

    pub fn outputs(&self) -> &BTreeMap<OutputId, OutputState> {
        &self.outputs
    }

    pub fn get(&self, id: OutputId) -> Option<&OutputState> {
        self.outputs.get(&id)
    }

    pub fn focused_output(&self) -> Option<OutputId> {
        self.focused_output
    }

    /// Reconciles a compositor snapshot with the current runtime state.
    pub fn reconcile(
        &mut self,
        outputs: impl IntoIterator<Item = Output>,
    ) -> Result<Vec<OutputTransition>, OutputStateError> {
        let mut staged = self.clone();
        let transitions = staged.reconcile_in_place(outputs, None)?;
        *self = staged;
        Ok(transitions)
    }

    /// Reconciles a snapshot while applying the compositor's focused output
    /// atomically with output add/change/remove transitions. `None` is an
    /// explicit "no focused output" value, not a request for fallback focus.
    pub fn reconcile_with_focus(
        &mut self,
        outputs: impl IntoIterator<Item = Output>,
        focused_output: Option<OutputId>,
    ) -> Result<Vec<OutputTransition>, OutputStateError> {
        let mut staged = self.clone();
        let transitions = staged.reconcile_in_place(outputs, Some(focused_output))?;
        *self = staged;
        Ok(transitions)
    }

    fn reconcile_in_place(
        &mut self,
        outputs: impl IntoIterator<Item = Output>,
        requested_focus: Option<Option<OutputId>>,
    ) -> Result<Vec<OutputTransition>, OutputStateError> {
        let mut transitions = Vec::new();
        let mut seen = BTreeSet::new();

        for output in outputs {
            if !seen.insert(output.id) {
                return Err(OutputStateError::DuplicateOutput(output.id));
            }

            if let Some(current) = self.outputs.get_mut(&output.id) {
                if let Some(change) = current.update(output, self.metrics)? {
                    transitions.push(OutputTransition::Changed {
                        output: current.clone(),
                        previous_geometry: change.previous_geometry,
                        previous_scale: change.previous_scale,
                    });
                }
            } else {
                let state =
                    OutputState::try_new(output, self.topology, self.metrics, &mut self.allocator)?;
                transitions.push(OutputTransition::Added(state.clone()));
                self.outputs.insert(state.id, state);
            }
        }

        let removed = self
            .outputs
            .keys()
            .filter(|id| !seen.contains(id))
            .copied()
            .collect::<Vec<_>>();
        for id in removed {
            if let Some(state) = self.outputs.remove(&id) {
                transitions.push(OutputTransition::Removed {
                    id,
                    surfaces: state.surfaces.into_values().collect(),
                });
            }
        }

        let next_focus = match requested_focus {
            Some(focused_output) => {
                if let Some(output_id) = focused_output
                    && !self.outputs.contains_key(&output_id)
                {
                    return Err(OutputStateError::UnknownOutput(output_id));
                }
                focused_output
            }
            None => self
                .focused_output
                .filter(|id| self.outputs.contains_key(id))
                .or_else(|| self.outputs.keys().next().copied()),
        };
        if next_focus != self.focused_output {
            let previous = self.focused_output;
            self.focused_output = next_focus;
            transitions.push(OutputTransition::Focused {
                previous,
                current: next_focus,
            });
        }

        Ok(transitions)
    }

    /// Changes the focused output without allowing an unknown output id.
    pub fn set_focused_output(
        &mut self,
        output_id: Option<OutputId>,
    ) -> Result<Option<OutputTransition>, OutputStateError> {
        if let Some(output_id) = output_id
            && !self.outputs.contains_key(&output_id)
        {
            return Err(OutputStateError::UnknownOutput(output_id));
        }

        let previous = self.focused_output;
        if previous == output_id {
            return Ok(None);
        }
        self.focused_output = output_id;
        Ok(Some(OutputTransition::Focused {
            previous,
            current: output_id,
        }))
    }
}

#[derive(Clone, Debug, Default)]
struct SurfaceIdAllocator {
    next: u64,
}

impl SurfaceIdAllocator {
    fn allocate(&mut self) -> Result<SurfaceId, OutputStateError> {
        self.next = self
            .next
            .checked_add(1)
            .ok_or(OutputStateError::SurfaceIdExhausted)?;
        Ok(SurfaceId::new(self.next))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputStateError {
    InvalidScale(OutputId),
    InvalidGeometry(OutputId),
    InvalidSurfaceMetrics,
    DuplicateOutput(OutputId),
    UnknownOutput(OutputId),
    SurfaceIdExhausted,
}

impl fmt::Display for OutputStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScale(id) => write!(formatter, "output {id} has an invalid scale"),
            Self::InvalidGeometry(id) => write!(formatter, "output {id} has invalid geometry"),
            Self::InvalidSurfaceMetrics => write!(formatter, "surface metrics are invalid"),
            Self::DuplicateOutput(id) => write!(formatter, "output {id} appeared more than once"),
            Self::UnknownOutput(id) => write!(formatter, "output {id} is not registered"),
            Self::SurfaceIdExhausted => write!(formatter, "surface id space exhausted"),
        }
    }
}

impl std::error::Error for OutputStateError {}

fn validate_output(output: &Output) -> Result<(), OutputStateError> {
    if !output.scale.is_finite() || output.scale <= 0.0 {
        return Err(OutputStateError::InvalidScale(output.id));
    }
    if !output.geometry.origin.x.is_finite()
        || !output.geometry.origin.y.is_finite()
        || !output.geometry.size.width.is_finite()
        || !output.geometry.size.height.is_finite()
        || output.geometry.size.width <= 0.0
        || output.geometry.size.height <= 0.0
    {
        return Err(OutputStateError::InvalidGeometry(output.id));
    }
    Ok(())
}

fn surface_geometry(output: Rect, role: SurfaceRole, metrics: SurfaceMetrics) -> Rect {
    match role {
        SurfaceRole::Unified | SurfaceRole::Overlay => output,
        SurfaceRole::Panel => Rect::from_xywh(
            output.origin.x,
            output.origin.y,
            output.size.width,
            metrics.panel_height,
        ),
        SurfaceRole::Notch => {
            let width = output.size.width.min(metrics.notch_width);
            Rect::from_xywh(
                output.origin.x + (output.size.width - width) / 2.0,
                output.origin.y,
                width,
                metrics.notch_height,
            )
        }
    }
}

fn scale_point(point: Point, geometry: Rect, scale: f32) -> Point {
    Point::new(
        geometry.origin.x + (point.x - geometry.origin.x) * scale,
        geometry.origin.y + (point.y - geometry.origin.y) * scale,
    )
}

fn unscale_point(point: Point, geometry: Rect, scale: f32) -> Point {
    Point::new(
        geometry.origin.x + (point.x - geometry.origin.x) / scale,
        geometry.origin.y + (point.y - geometry.origin.y) / scale,
    )
}

#[cfg(test)]
mod tests {
    use shell_core::{Output, OutputId, Point, Rect};

    use super::{
        OutputRegistry, OutputStateError, OutputTransition, SurfaceMetrics, SurfaceRole,
        SurfaceTopology,
    };

    fn output(id: u64, name: &str, x: f32, scale: f32) -> Output {
        Output::new(
            OutputId::new(id),
            name,
            Rect::from_xywh(x, 0.0, 1920.0, 1080.0),
            scale,
        )
    }

    #[test]
    fn topology_experiments_have_distinct_surface_ownership() {
        let mut unified = OutputRegistry::new(SurfaceTopology::Unified);
        let mut independent = OutputRegistry::new(SurfaceTopology::Independent);

        unified.reconcile([output(1, "DP-1", 0.0, 1.0)]).unwrap();
        independent
            .reconcile([output(1, "DP-1", 0.0, 1.0)])
            .unwrap();

        assert_eq!(unified.get(OutputId::new(1)).unwrap().surfaces.len(), 1);
        assert_eq!(independent.get(OutputId::new(1)).unwrap().surfaces.len(), 3);
        assert!(
            independent
                .get(OutputId::new(1))
                .unwrap()
                .surface(SurfaceRole::Notch)
                .is_some()
        );
    }

    #[test]
    fn configured_surface_metrics_shape_owned_surfaces() {
        let mut registry = OutputRegistry::new(SurfaceTopology::Independent);
        registry
            .set_surface_metrics(SurfaceMetrics::new(48.0, 360.0, 32.0))
            .expect("metrics are valid");
        registry.reconcile([output(1, "DP-1", 0.0, 1.0)]).unwrap();

        let state = registry.get(OutputId::new(1)).expect("output exists");
        assert_eq!(
            state
                .surface(SurfaceRole::Panel)
                .unwrap()
                .geometry
                .size
                .height,
            48.0
        );
        assert_eq!(
            state
                .surface(SurfaceRole::Notch)
                .unwrap()
                .geometry
                .size
                .width,
            360.0
        );
        assert_eq!(
            state
                .surface(SurfaceRole::Notch)
                .unwrap()
                .geometry
                .size
                .height,
            32.0
        );
    }

    #[test]
    fn reconcile_handles_add_change_remove_and_focus() {
        let mut registry = OutputRegistry::new(SurfaceTopology::Independent);
        let initial = registry
            .reconcile([
                output(1, "DP-1", 0.0, 1.0),
                output(2, "HDMI-A-1", 1920.0, 1.25),
            ])
            .unwrap();

        assert!(initial.iter().any(|transition| {
            matches!(
                transition,
                OutputTransition::Focused {
                    current: Some(id),
                    ..
                } if id.get() == 1
            )
        }));
        let changed = registry
            .reconcile([output(2, "HDMI-A-1", 1920.0, 1.5)])
            .unwrap();

        assert_eq!(registry.outputs().len(), 1);
        assert_eq!(registry.focused_output(), Some(OutputId::new(2)));
        assert!(changed.iter().any(|transition| matches!(
            transition,
            OutputTransition::Removed {
                id,
                surfaces
            } if id.get() == 1 && surfaces.len() == 3
        )));
        assert!(changed.iter().any(|transition| matches!(
            transition,
            OutputTransition::Changed {
                previous_scale,
                ..
            } if *previous_scale == 1.25
        )));
    }

    #[test]
    fn fractional_scale_round_trips_without_assuming_one() {
        let mut registry = OutputRegistry::new(SurfaceTopology::Unified);
        registry
            .reconcile([output(1, "DP-1", 100.0, 1.25)])
            .unwrap();
        let state = registry.get(OutputId::new(1)).unwrap();
        let point = Point::new(200.0, 400.0);
        let physical = state.logical_to_physical(point);

        assert_eq!(physical, Point::new(225.0, 500.0));
        assert_eq!(state.physical_to_logical(physical), point);
    }

    #[test]
    fn invalid_outputs_are_rejected_before_surface_creation() {
        let mut registry = OutputRegistry::new(SurfaceTopology::Unified);
        let error = registry
            .reconcile([output(1, "DP-1", 0.0, 0.0)])
            .unwrap_err();

        assert_eq!(error, OutputStateError::InvalidScale(OutputId::new(1)));
    }

    #[test]
    fn rejected_snapshot_does_not_publish_partial_changes() {
        let mut registry = OutputRegistry::new(SurfaceTopology::Unified);
        registry
            .reconcile([output(1, "DP-1", 0.0, 1.0)])
            .expect("initial output is valid");

        let error = registry
            .reconcile([
                output(1, "DP-1", 0.0, 1.0),
                output(2, "HDMI-A-1", 1920.0, 1.0),
                output(3, "bad", 3840.0, 0.0),
            ])
            .expect_err("invalid later output rejects the snapshot");

        assert_eq!(error, OutputStateError::InvalidScale(OutputId::new(3)));
        assert_eq!(registry.outputs().len(), 1);
        assert!(registry.get(OutputId::new(2)).is_none());
        let transitions = registry
            .reconcile([
                output(1, "DP-1", 0.0, 1.0),
                output(2, "HDMI-A-1", 1920.0, 1.0),
            ])
            .expect("next valid snapshot is applied");
        assert!(transitions.iter().any(|transition| matches!(
            transition,
            OutputTransition::Added(state) if state.id == OutputId::new(2)
        )));
    }

    #[test]
    fn snapshot_focus_is_applied_without_an_intermediate_fallback() {
        let mut registry = OutputRegistry::new(SurfaceTopology::Unified);
        registry
            .reconcile_with_focus(
                [
                    output(1, "DP-1", 0.0, 1.0),
                    output(2, "HDMI-A-1", 1920.0, 1.0),
                ],
                Some(OutputId::new(2)),
            )
            .expect("focused snapshot is valid");

        assert_eq!(registry.focused_output(), Some(OutputId::new(2)));
        let transitions = registry
            .reconcile_with_focus(
                [
                    output(1, "DP-1", 0.0, 1.0),
                    output(2, "HDMI-A-1", 1920.0, 1.0),
                ],
                None,
            )
            .expect("explicitly unfocused snapshot is valid");
        assert_eq!(registry.focused_output(), None);
        assert!(transitions.iter().any(|transition| matches!(
            transition,
            OutputTransition::Focused {
                previous: Some(previous),
                current: None,
            } if *previous == OutputId::new(2)
        )));
    }
}
