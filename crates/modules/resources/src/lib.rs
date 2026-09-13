//! Resource/status indicators hosted by the Notch.

use gpui::{MouseButton, div, prelude::*, px, rgb};
use shell_core::{NotchConfig, ShellCommand};
use shell_ui_gpui::{ModuleContext, ModuleId, ModuleMetadata, ModuleSize, ModuleView, NotchModule};

#[derive(Default)]
pub struct ResourcesModule;

impl NotchModule for ResourcesModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: ModuleId::Resources,
            title: "Resources",
            aliases: &["network", "audio", "battery", "media", "workspaces"],
        }
    }

    fn preferred_size(&self, config: &NotchConfig) -> ModuleSize {
        ModuleSize::from_config(config, false)
    }

    fn render(&mut self, context: ModuleContext) -> ModuleView {
        let label = match context.module_key.as_str() {
            "audio" => "◖".to_owned(),
            "battery" => "▰".to_owned(),
            "media" => "▶".to_owned(),
            "workspaces" => context
                .snapshot
                .focused_workspace
                .map_or_else(|| "WS —".to_owned(), |workspace| format!("WS {workspace}")),
            _ => "▮▮▮".to_owned(),
        };

        let mut element = div()
            .flex()
            .items_center()
            .text_color(rgb(context.tokens.colors.foreground))
            .text_size(px(context.tokens.typography.label_size as f32))
            .child(label);

        if context.module_key == "workspaces"
            && let Some(workspace) = context.snapshot.focused_workspace
        {
            let sender = context.commands.clone();
            element = element.on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                if let Err(error) = sender.send(ShellCommand::FocusWorkspace(workspace)) {
                    tracing::warn!(%error, ?workspace, "could not enqueue workspace focus command");
                }
            });
        }

        element.into_any_element()
    }
}
