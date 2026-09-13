//! Settings module boundary. Configuration writes stay behind ConfigPort and
//! are intentionally not performed by GPUI widgets.

use shell_core::NotchConfig;
use shell_ui_gpui::{ModuleContext, ModuleId, ModuleMetadata, ModuleSize, ModuleView, NotchModule};

#[derive(Default)]
pub struct SettingsModule;

impl NotchModule for SettingsModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: ModuleId::Settings,
            title: "Settings",
            aliases: &[],
        }
    }

    fn preferred_size(&self, config: &NotchConfig) -> ModuleSize {
        ModuleSize::from_config(config, false)
    }

    fn render(&mut self, context: ModuleContext) -> ModuleView {
        shell_ui_gpui::primitives::module_label("Settings", &context.tokens)
    }
}
