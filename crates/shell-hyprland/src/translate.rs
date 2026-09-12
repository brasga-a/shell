use serde_json::Value;
use shell_core::{
    CompositorError, Output, OutputId, Rect, Window, WindowId, Workspace, WorkspaceId,
};

fn object<'a>(
    value: &'a Value,
    context: &str,
) -> Result<&'a serde_json::Map<String, Value>, CompositorError> {
    value
        .as_object()
        .ok_or_else(|| invalid(context, "expected JSON object"))
}

fn array<'a>(value: &'a Value, context: &str) -> Result<&'a Vec<Value>, CompositorError> {
    value
        .as_array()
        .ok_or_else(|| invalid(context, "expected JSON array"))
}

fn invalid(context: &str, message: impl Into<String>) -> CompositorError {
    CompositorError::InvalidData {
        message: format!("{context}: {}", message.into()),
    }
}

fn required_str<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<&'a str, CompositorError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(context, format!("missing string field '{key}'")))
}

fn number(
    object: &serde_json::Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<f64, CompositorError> {
    object
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| invalid(context, format!("missing finite number field '{key}'")))
}

fn integer(
    object: &serde_json::Map<String, Value>,
    key: &str,
    context: &str,
) -> Result<i64, CompositorError> {
    object
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| invalid(context, format!("missing integer field '{key}'")))
}

fn bool_or_false(object: &serde_json::Map<String, Value>, key: &str) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn array_number(value: &Value, index: usize, context: &str) -> Result<f32, CompositorError> {
    value
        .as_array()
        .and_then(|values| values.get(index))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .map(|value| value as f32)
        .ok_or_else(|| {
            invalid(
                context,
                format!("missing finite array value at index {index}"),
            )
        })
}

fn optional_workspace_id(value: Option<&Value>) -> Option<WorkspaceId> {
    value
        .and_then(Value::as_object)
        .and_then(|workspace| workspace.get("id"))
        .and_then(Value::as_i64)
        .map(WorkspaceId::new)
}

fn parse_window_id(value: &Value, context: &str) -> Result<WindowId, CompositorError> {
    let address = value
        .as_str()
        .ok_or_else(|| invalid(context, "window address is not a string"))?;
    let address = address.strip_prefix("0x").unwrap_or(address);
    u64::from_str_radix(address, 16)
        .map(WindowId::new)
        .map_err(|error| {
            invalid(
                context,
                format!("invalid window address '{address}': {error}"),
            )
        })
}

fn logical_monitor_geometry(
    monitor: &serde_json::Map<String, Value>,
    context: &str,
) -> Result<Rect, CompositorError> {
    let scale = number(monitor, "scale", context)? as f32;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(invalid(
            context,
            "monitor scale must be finite and positive",
        ));
    }
    let transform = monitor
        .get("transform")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let x = number(monitor, "x", context)? as f32 / scale;
    let y = number(monitor, "y", context)? as f32 / scale;
    let width = number(monitor, "width", context)? as f32 / scale;
    let height = number(monitor, "height", context)? as f32 / scale;

    if matches!(transform.rem_euclid(4), 1 | 3) {
        Ok(Rect::from_xywh(x, y, height, width))
    } else {
        Ok(Rect::from_xywh(x, y, width, height))
    }
}

pub(crate) fn monitor_snapshot(
    value: &Value,
) -> Result<(Vec<Output>, Option<OutputId>), CompositorError> {
    let monitors = array(value, "j/monitors")?;
    let mut outputs = Vec::with_capacity(monitors.len());
    let mut focused = None;

    for (index, monitor) in monitors.iter().enumerate() {
        let context = format!("j/monitors[{index}]");
        let monitor = object(monitor, &context)?;
        let id = integer(monitor, "id", &context)?;
        let id =
            u64::try_from(id).map_err(|_| invalid(&context, "monitor id cannot be negative"))?;
        let geometry = logical_monitor_geometry(monitor, &context)?;
        let output = Output::new(
            OutputId::new(id),
            required_str(monitor, "name", &context)?,
            geometry,
            number(monitor, "scale", &context)? as f32,
        );
        if !output.is_valid() {
            return Err(invalid(
                &context,
                "monitor scale must be finite and positive",
            ));
        }
        if bool_or_false(monitor, "focused") {
            focused = Some(output.id);
        }
        outputs.push(output);
    }

    Ok((outputs, focused))
}

pub(crate) fn active_workspace_id(value: &Value) -> Result<Option<WorkspaceId>, CompositorError> {
    let object = object(value, "j/activeworkspace")?;
    Ok(object
        .get("id")
        .and_then(Value::as_i64)
        .map(WorkspaceId::new))
}

pub(crate) fn parse_workspaces(
    value: &Value,
    focused: Option<WorkspaceId>,
) -> Result<Vec<Workspace>, CompositorError> {
    let workspaces = array(value, "j/workspaces")?;
    let mut result = Vec::with_capacity(workspaces.len());

    for (index, workspace) in workspaces.iter().enumerate() {
        let context = format!("j/workspaces[{index}]");
        let workspace = object(workspace, &context)?;
        let id = WorkspaceId::new(integer(workspace, "id", &context)?);
        let output_id = workspace
            .get("monitorID")
            .and_then(Value::as_i64)
            .filter(|id| *id >= 0)
            .map(|id| OutputId::new(id as u64));
        let name = workspace
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned);
        result.push(Workspace {
            id,
            output_id,
            name,
            active: focused == Some(id),
        });
    }

    Ok(result)
}

pub(crate) fn parse_windows(
    value: &Value,
    focused: Option<WindowId>,
) -> Result<Vec<Window>, CompositorError> {
    let windows = array(value, "j/clients")?;
    let mut result = Vec::with_capacity(windows.len());

    for (index, window) in windows.iter().enumerate() {
        let context = format!("j/clients[{index}]");
        match parse_window(window, &context, focused) {
            Ok(window) => result.push(window),
            Err(error) => {
                tracing::warn!(%error, "ignoring malformed or disappearing Hyprland client");
            }
        }
    }

    Ok(result)
}

pub(crate) fn active_window(value: &Value) -> Result<Option<Window>, CompositorError> {
    let object = object(value, "j/activewindow")?;
    if object.is_empty() {
        return Ok(None);
    }
    let mut window = parse_window(value, "j/activewindow", None)?;
    window.focused = true;
    Ok(Some(window))
}

fn parse_window(
    value: &Value,
    context: &str,
    focused: Option<WindowId>,
) -> Result<Window, CompositorError> {
    let window = object(value, context)?;
    let id = parse_window_id(
        window
            .get("address")
            .ok_or_else(|| invalid(context, "missing window address"))?,
        context,
    )?;
    let workspace_id = optional_workspace_id(window.get("workspace"));
    let output_id = window
        .get("monitor")
        .and_then(Value::as_i64)
        .filter(|id| *id >= 0)
        .map(|id| OutputId::new(id as u64));
    let at = window
        .get("at")
        .ok_or_else(|| invalid(context, "missing window position"))?;
    let size = window
        .get("size")
        .ok_or_else(|| invalid(context, "missing window size"))?;
    let geometry = Rect::from_xywh(
        array_number(at, 0, context)?,
        array_number(at, 1, context)?,
        array_number(size, 0, context)?,
        array_number(size, 1, context)?,
    );
    let fullscreen = window
        .get("fullscreen")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let maximized = fullscreen == 1;
    let fullscreen = matches!(fullscreen, 2 | 3);

    Ok(Window {
        id,
        output_id,
        workspace_id,
        title: window
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        app_id: window
            .get("class")
            .and_then(Value::as_str)
            .map(str::to_owned),
        geometry,
        focused: focused == Some(id),
        maximized,
        fullscreen,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use shell_core::{OutputId, WindowId, WorkspaceId};

    use super::{active_window, monitor_snapshot, parse_windows, parse_workspaces};

    #[test]
    fn translates_monitor_geometry_scale_and_focus() {
        let value = json!([{
            "id": 2,
            "name": "DP-1",
            "x": 1920,
            "y": 0,
            "width": 2560,
            "height": 1440,
            "scale": 1.25,
            "transform": 0,
            "focused": true
        }]);

        let (outputs, focused) = monitor_snapshot(&value).expect("monitor is valid");
        assert_eq!(outputs[0].id, OutputId::new(2));
        assert_eq!(
            outputs[0].geometry,
            shell_core::Rect::from_xywh(1536.0, 0.0, 2048.0, 1152.0)
        );
        assert_eq!(outputs[0].scale, 1.25);
        assert_eq!(focused, Some(OutputId::new(2)));
    }

    #[test]
    fn swaps_logical_axes_for_quarter_turn_transform() {
        let value = json!([{
            "id": 4,
            "name": "DP-2",
            "x": 0,
            "y": 0,
            "width": 1920,
            "height": 1080,
            "scale": 1.0,
            "transform": 1,
            "focused": false
        }]);

        let (outputs, _) = monitor_snapshot(&value).expect("monitor is valid");
        assert_eq!(
            outputs[0].geometry,
            shell_core::Rect::from_xywh(0.0, 0.0, 1080.0, 1920.0)
        );
    }

    #[test]
    fn translates_workspace_and_active_window_without_raw_payloads() {
        let workspaces = json!([{
            "id": 3,
            "name": "3",
            "monitorID": 2
        }]);
        let windows = json!([{
            "address": "0xabc",
            "monitor": 2,
            "workspace": {"id": 3, "name": "3"},
            "at": [10, 20],
            "size": [800, 600],
            "title": "Terminal",
            "class": "foot",
            "fullscreen": 2
        }]);

        let translated_workspaces =
            parse_workspaces(&workspaces, Some(WorkspaceId::new(3))).expect("workspace is valid");
        let translated_windows =
            parse_windows(&windows, Some(WindowId::new(0xabc))).expect("window is valid");

        assert!(translated_workspaces[0].active);
        assert_eq!(translated_windows[0].output_id, Some(OutputId::new(2)));
        assert!(translated_windows[0].focused);
        assert!(translated_windows[0].fullscreen);
        assert!(!translated_windows[0].maximized);
        assert_eq!(
            active_window(&windows[0])
                .expect("active window is valid")
                .expect("focused window exists")
                .id,
            WindowId::new(0xabc)
        );
        assert!(
            active_window(&windows[0])
                .expect("active window is valid")
                .expect("focused window exists")
                .focused
        );
    }

    #[test]
    fn malformed_client_is_skipped_without_failing_snapshot() {
        let windows = json!([
            {"address": "not-an-address"},
            {
                "address": "0xabc",
                "monitor": 0,
                "workspace": {"id": 1},
                "at": [0, 0],
                "size": [10, 10]
            }
        ]);

        let translated = parse_windows(&windows, None).expect("snapshot remains usable");
        assert_eq!(translated.len(), 1);
    }

    #[test]
    fn keeps_negative_named_and_special_workspace_ids() {
        let workspaces = json!([
            {"id": -1337, "name": "named", "monitorID": 0},
            {"id": -99, "name": "special:mail", "monitorID": 0}
        ]);

        let translated = parse_workspaces(&workspaces, Some(WorkspaceId::new(-1337)))
            .expect("negative workspace ids are valid");

        assert_eq!(translated[0].id, WorkspaceId::new(-1337));
        assert!(translated[0].active);
        assert_eq!(translated[1].id, WorkspaceId::new(-99));
    }

    #[test]
    fn distinguishes_maximized_from_fullscreen_modes() {
        let client = |mode| {
            json!({
                "address": "0xabc",
                "monitor": 0,
                "workspace": {"id": 1},
                "at": [0, 0],
                "size": [10, 10],
                "fullscreen": mode
            })
        };

        let maximized = parse_windows(&json!([client(1)]), None).expect("valid client");
        let fullscreen = parse_windows(&json!([client(2)]), None).expect("valid client");
        let fullscreen_alt = parse_windows(&json!([client(3)]), None).expect("valid client");

        assert!(maximized[0].maximized);
        assert!(!maximized[0].fullscreen);
        assert!(!fullscreen[0].maximized);
        assert!(fullscreen[0].fullscreen);
        assert!(fullscreen_alt[0].fullscreen);
    }
}
