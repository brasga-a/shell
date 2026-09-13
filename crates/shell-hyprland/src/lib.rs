//! Hyprland compositor adapter.
//!
//! Unix IPC, Hyprland JSON and event names are intentionally kept in this
//! crate. Callers consume only `shell-core` domain types and events.

mod events;
mod ipc;
mod translate;

use std::{thread, time::Duration};

use shell_core::{
    CompositorCapabilities, CompositorError, CompositorEventStream, CompositorPort,
    CompositorSnapshot, FullscreenState, Output, OutputId, Window, WindowId, Workspace,
    WorkspaceId,
};

pub use events::HyprlandEventStream;
pub use ipc::HyprlandPaths;

use crate::ipc::HyprlandIpc;
use crate::translate::{
    active_window, active_workspace_id, monitor_snapshot, parse_windows, parse_workspaces,
};

/// Hyprland adapter using the compositor's command and event Unix sockets.
#[derive(Clone, Debug, Default)]
pub struct HyprlandCompositor {
    paths: Option<HyprlandPaths>,
}

impl HyprlandCompositor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an adapter with explicit paths, which is useful for tests and
    /// for callers that already resolved the Hyprland runtime directory.
    pub fn with_paths(paths: HyprlandPaths) -> Self {
        Self { paths: Some(paths) }
    }

    pub fn paths_from_environment() -> Result<HyprlandPaths, CompositorError> {
        HyprlandPaths::from_environment()
    }

    fn ipc(&self) -> Result<HyprlandIpc, CompositorError> {
        let paths = match &self.paths {
            Some(paths) => paths.clone(),
            None => HyprlandPaths::from_environment()?,
        };
        Ok(HyprlandIpc::new(paths))
    }

    fn json_request(&self, command: &str) -> Result<serde_json::Value, CompositorError> {
        let response = self.ipc()?.request(command)?;
        serde_json::from_str(&response).map_err(|error| CompositorError::InvalidData {
            message: format!("Hyprland response for '{command}' is not valid JSON: {error}"),
        })
    }

    pub fn event_stream(&self) -> Result<HyprlandEventStream, CompositorError> {
        HyprlandEventStream::connect(self.clone())
    }

    pub(crate) fn snapshot_for_events(&self) -> Result<CompositorSnapshot, CompositorError> {
        self.snapshot()
    }
}

impl CompositorPort for HyprlandCompositor {
    fn outputs(&self) -> Result<Vec<Output>, CompositorError> {
        let value = self.json_request("j/monitors")?;
        monitor_snapshot(&value).map(|(outputs, _)| outputs)
    }

    fn workspaces(&self) -> Result<Vec<Workspace>, CompositorError> {
        let focused = self.focused_workspace()?;
        let value = self.json_request("j/workspaces")?;
        parse_workspaces(&value, focused)
    }

    fn windows(&self) -> Result<Vec<Window>, CompositorError> {
        let focused = self.focused_window()?.map(|window| window.id);
        let value = self.json_request("j/clients")?;
        parse_windows(&value, focused)
    }

    fn focus_workspace(&self, workspace_id: WorkspaceId) -> Result<(), CompositorError> {
        let selector = self.workspace_selector(workspace_id)?;
        let command = format!(
            "dispatch hl.dsp.focus({{ workspace = {} }})",
            lua_string(&selector)
        );
        command_result(&self.ipc()?.request(&command)?, &command)
    }

    fn focus_window(&self, window_id: WindowId) -> Result<(), CompositorError> {
        let selector = format!("address:0x{:x}", window_id.get());
        let command = format!(
            "dispatch hl.dsp.focus({{ window = {} }})",
            lua_string(&selector)
        );
        command_result(&self.ipc()?.request(&command)?, &command)
    }

    fn focused_output(&self) -> Result<Option<OutputId>, CompositorError> {
        let value = self.json_request("j/monitors")?;
        monitor_snapshot(&value).map(|(_, focused)| focused)
    }

    fn focused_workspace(&self) -> Result<Option<WorkspaceId>, CompositorError> {
        let value = self.json_request("j/activeworkspace")?;
        active_workspace_id(&value)
    }

    fn focused_window(&self) -> Result<Option<Window>, CompositorError> {
        let value = self.json_request("j/activewindow")?;
        active_window(&value)
    }

    fn fullscreen_state(&self) -> Result<FullscreenState, CompositorError> {
        Ok(match self.focused_window()? {
            Some(window) if window.fullscreen => FullscreenState::Active {
                window_id: window.id,
            },
            _ => FullscreenState::Inactive,
        })
    }

    fn capabilities(&self) -> CompositorCapabilities {
        CompositorCapabilities {
            monitors: true,
            workspaces: true,
            windows: true,
            focused_output: true,
            focused_workspace: true,
            focused_window: true,
            fullscreen_state: true,
            focus_workspace: true,
            focus_window: true,
            event_stream: true,
        }
    }

    fn snapshot(&self) -> Result<CompositorSnapshot, CompositorError> {
        let mut last_snapshot = None;
        for attempt in 0..2 {
            let snapshot = self.snapshot_once()?;
            if snapshot_is_consistent(&snapshot) {
                return Ok(snapshot);
            }
            last_snapshot = Some(snapshot);
            if attempt == 0 {
                thread::sleep(Duration::from_millis(5));
            }
        }

        Ok(canonicalize_snapshot(
            last_snapshot.expect("snapshot retry always produces a value"),
        ))
    }

    fn subscribe_events(&self) -> Result<Box<dyn CompositorEventStream>, CompositorError> {
        Ok(Box::new(self.event_stream()?))
    }
}

impl HyprlandCompositor {
    fn workspace_selector(&self, workspace_id: WorkspaceId) -> Result<String, CompositorError> {
        if workspace_id.get() >= 0 {
            return Ok(workspace_id.get().to_string());
        }

        let workspace = self
            .workspaces()?
            .into_iter()
            .find(|workspace| workspace.id == workspace_id)
            .ok_or_else(|| CompositorError::Operation {
                operation: format!("focus workspace {workspace_id}"),
                message: "workspace disappeared before it could be focused".to_owned(),
            })?;
        workspace_selector_for(workspace_id, workspace.name)
    }

    fn snapshot_once(&self) -> Result<CompositorSnapshot, CompositorError> {
        let monitors_value = self.json_request("j/monitors")?;
        let (monitors, focused_output) = monitor_snapshot(&monitors_value)?;

        let focused_workspace = self.focused_workspace()?;
        let workspaces_value = self.json_request("j/workspaces")?;
        let workspaces = parse_workspaces(&workspaces_value, focused_workspace)?;

        let focused_window = self.focused_window()?;
        let windows_value = self.json_request("j/clients")?;
        let windows = parse_windows(
            &windows_value,
            focused_window.as_ref().map(|window| window.id),
        )?;

        let fullscreen = match &focused_window {
            Some(window) if window.fullscreen => FullscreenState::Active {
                window_id: window.id,
            },
            _ => FullscreenState::Inactive,
        };

        Ok(CompositorSnapshot {
            monitors,
            workspaces,
            windows,
            focused_output,
            focused_workspace,
            focused_window,
            fullscreen,
        })
    }
}

fn snapshot_is_consistent(snapshot: &CompositorSnapshot) -> bool {
    let output_exists = |id| snapshot.monitors.iter().any(|output| output.id == id);
    let workspace_exists = |id| {
        snapshot
            .workspaces
            .iter()
            .any(|workspace| workspace.id == id)
    };
    let window_exists = |id| snapshot.windows.iter().any(|window| window.id == id);

    snapshot.focused_output.is_none_or(output_exists)
        && snapshot.focused_workspace.is_none_or(workspace_exists)
        && snapshot
            .focused_window
            .as_ref()
            .is_none_or(|window| window_exists(window.id))
        && match snapshot.fullscreen {
            FullscreenState::Inactive => true,
            FullscreenState::Active { window_id } => snapshot
                .focused_window
                .as_ref()
                .is_some_and(|window| window.id == window_id && window.fullscreen),
        }
}

fn canonicalize_snapshot(mut snapshot: CompositorSnapshot) -> CompositorSnapshot {
    if snapshot
        .focused_output
        .is_some_and(|id| !snapshot.monitors.iter().any(|output| output.id == id))
    {
        snapshot.focused_output = None;
    }
    if snapshot.focused_workspace.is_some_and(|id| {
        !snapshot
            .workspaces
            .iter()
            .any(|workspace| workspace.id == id)
    }) {
        snapshot.focused_workspace = None;
    }

    snapshot.focused_window = snapshot.focused_window.and_then(|focused| {
        snapshot
            .windows
            .iter()
            .find(|window| window.id == focused.id)
            .cloned()
    });
    snapshot.fullscreen = match snapshot.focused_window.as_ref() {
        Some(window) if window.fullscreen => FullscreenState::Active {
            window_id: window.id,
        },
        _ => FullscreenState::Inactive,
    };
    snapshot
}

fn command_result(response: &str, command: &str) -> Result<(), CompositorError> {
    let response = response.trim();
    if response.is_empty() || response.eq_ignore_ascii_case("ok") {
        return Ok(());
    }
    Err(CompositorError::Operation {
        operation: command.to_owned(),
        message: response.to_owned(),
    })
}

fn lua_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn workspace_selector_for(
    workspace_id: WorkspaceId,
    name: Option<String>,
) -> Result<String, CompositorError> {
    if workspace_id.get() >= 0 {
        return Ok(workspace_id.get().to_string());
    }

    let name = name.ok_or_else(|| CompositorError::Operation {
        operation: format!("focus workspace {workspace_id}"),
        message: "named workspace has no selector name".to_owned(),
    })?;
    if let Some(special_name) = name.strip_prefix("special:") {
        Ok(format!("special:{special_name}"))
    } else {
        Ok(format!("name:{name}"))
    }
}

#[cfg(test)]
mod tests {
    use shell_core::{
        CompositorSnapshot, FullscreenState, Output, OutputId, Rect, Window, WindowId, WorkspaceId,
    };

    use super::{command_result, lua_string, workspace_selector_for};

    #[test]
    fn accepts_successful_dispatch_responses() {
        assert!(command_result("ok\n", "dispatch hl.dsp.focus(...)").is_ok());
        assert!(command_result("", "dispatch hl.dsp.focus(...)").is_ok());
    }

    #[test]
    fn rejects_dispatch_error_responses() {
        let error = command_result("unknown dispatcher", "dispatch hl.dsp.focus(...)")
            .expect_err("error response must not be hidden");
        assert!(error.to_string().contains("unknown dispatcher"));
    }

    #[test]
    fn encodes_workspace_focus_for_the_lua_dispatch_api() {
        let selector = "1";
        let command = format!(
            "dispatch hl.dsp.focus({{ workspace = {} }})",
            lua_string(selector)
        );

        assert_eq!(command, "dispatch hl.dsp.focus({ workspace = \"1\" })");
    }

    #[test]
    fn escapes_lua_string_selectors() {
        assert_eq!(lua_string("name:\"mail\""), "\"name:\\\"mail\\\"\"");
    }

    #[test]
    fn resolves_negative_workspace_ids_to_named_selectors() {
        assert_eq!(
            workspace_selector_for(WorkspaceId::new(-1337), Some("mail".to_owned()))
                .expect("named selector exists"),
            "name:mail"
        );
        assert_eq!(
            workspace_selector_for(WorkspaceId::new(-99), Some("special:music".to_owned()))
                .expect("special selector exists"),
            "special:music"
        );
    }

    #[test]
    fn canonicalizes_a_focused_window_that_disappeared_during_snapshot() {
        let focused = Window {
            id: WindowId::new(0xabc),
            output_id: Some(OutputId::new(1)),
            workspace_id: Some(WorkspaceId::new(1)),
            title: "gone".to_owned(),
            app_id: Some("test".to_owned()),
            geometry: Rect::from_xywh(0.0, 0.0, 10.0, 10.0),
            focused: true,
            maximized: false,
            fullscreen: true,
        };
        let snapshot = CompositorSnapshot {
            monitors: vec![Output::new(
                OutputId::new(1),
                "DP-1",
                Rect::from_xywh(0.0, 0.0, 1920.0, 1080.0),
                1.0,
            )],
            focused_output: Some(OutputId::new(1)),
            focused_window: Some(focused),
            fullscreen: FullscreenState::Active {
                window_id: WindowId::new(0xabc),
            },
            ..CompositorSnapshot::default()
        };

        let canonical = super::canonicalize_snapshot(snapshot);

        assert_eq!(canonical.focused_window, None);
        assert_eq!(canonical.fullscreen, FullscreenState::Inactive);
    }
}
