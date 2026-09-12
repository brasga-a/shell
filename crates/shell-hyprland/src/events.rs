use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;

use shell_core::{CompositorError, CompositorEvent, CompositorEventKind, CompositorEventStream};
use tracing::{debug, warn};

use crate::HyprlandCompositor;

/// Translates Hyprland's `event>>payload` stream into application events.
pub struct HyprlandEventStream {
    reader: BufReader<UnixStream>,
    compositor: HyprlandCompositor,
}

impl HyprlandEventStream {
    pub(crate) fn connect(compositor: HyprlandCompositor) -> Result<Self, CompositorError> {
        let ipc = compositor.ipc()?;
        let stream = ipc.connect_events()?;
        Ok(Self {
            reader: BufReader::new(stream),
            compositor,
        })
    }

    /// Reconnects the long-lived event socket after a compositor restart.
    pub fn reconnect(&mut self) -> Result<(), CompositorError> {
        let stream = self.compositor.ipc()?.connect_events()?;
        self.reader = BufReader::new(stream);
        Ok(())
    }

    fn translate_event(name: &str) -> Option<CompositorEventKind> {
        Some(match name {
            "monitoradded" | "monitoraddedv2" | "monitorremoved" | "monitorremovedv2" => {
                CompositorEventKind::Output
            }
            "workspace" | "workspacev2" | "createworkspace" | "destroyworkspace"
            | "renameworkspace" | "moveworkspace" | "moveworkspacev2" | "activespecial" => {
                CompositorEventKind::Workspace
            }
            "openwindow" | "openwindowv2" | "closewindow" | "closewindowv2" | "movewindow"
            | "movewindowv2" | "windowtitle" | "windowtitlev2" | "urgent" | "minimized" | "pin" => {
                CompositorEventKind::Window
            }
            "activewindow" | "activewindowv2" | "focusedmon" | "focusedmonv2" => {
                CompositorEventKind::Focus
            }
            "fullscreen" | "fullscreenstate" => CompositorEventKind::Fullscreen,
            "configreloaded" => CompositorEventKind::Output,
            _ => return None,
        })
    }
}

impl CompositorEventStream for HyprlandEventStream {
    fn next_event(&mut self) -> Result<CompositorEvent, CompositorError> {
        let mut line = String::new();
        let mut reconnected = false;
        loop {
            line.clear();
            let bytes = match self.reader.read_line(&mut line) {
                Ok(bytes) => bytes,
                Err(error) if !reconnected => {
                    warn!(%error, "Hyprland event socket read failed; attempting reconnect");
                    self.reconnect()?;
                    reconnected = true;
                    continue;
                }
                Err(error) => {
                    return Err(CompositorError::Connection {
                        message: format!("could not read Hyprland event socket: {error}"),
                    });
                }
            };
            if bytes == 0 {
                if !reconnected {
                    warn!("Hyprland event socket closed; attempting reconnect");
                    self.reconnect()?;
                    reconnected = true;
                    continue;
                }
                return Err(CompositorError::Connection {
                    message: "Hyprland event socket closed after reconnect".to_owned(),
                });
            }

            let line = line.trim_end_matches(['\r', '\n']);
            let Some((name, payload)) = line.split_once(">>") else {
                return Err(CompositorError::InvalidData {
                    message: format!("malformed Hyprland event line: {line}"),
                });
            };
            let Some(kind) = Self::translate_event(name) else {
                debug!(event = name, payload, "ignoring unsupported Hyprland event");
                continue;
            };

            // Payloads differ between Hyprland versions and event names.
            // Refreshing a coherent snapshot at the adapter boundary prevents
            // that protocol detail from leaking into application state.
            let snapshot = self.compositor.snapshot_for_events()?;
            return Ok(CompositorEvent { kind, snapshot });
        }
    }
}

#[cfg(test)]
mod tests {
    use shell_core::CompositorEventKind;

    use super::HyprlandEventStream;

    #[test]
    fn classifies_relevant_events_without_exposing_names() {
        assert_eq!(
            HyprlandEventStream::translate_event("workspacev2"),
            Some(CompositorEventKind::Workspace)
        );
        assert_eq!(
            HyprlandEventStream::translate_event("activewindowv2"),
            Some(CompositorEventKind::Focus)
        );
        assert_eq!(
            HyprlandEventStream::translate_event("fullscreen"),
            Some(CompositorEventKind::Fullscreen)
        );
        assert_eq!(
            HyprlandEventStream::translate_event("windowtitlev2"),
            Some(CompositorEventKind::Window)
        );
        assert_eq!(
            HyprlandEventStream::translate_event("configreloaded"),
            Some(CompositorEventKind::Output)
        );
        assert_eq!(HyprlandEventStream::translate_event("unknown"), None);
    }
}
