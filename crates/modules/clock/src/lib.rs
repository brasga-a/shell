//! Compact clock module.

use shell_core::NotchConfig;
use shell_ui_gpui::{ModuleContext, ModuleId, ModuleMetadata, ModuleSize, ModuleView, NotchModule};

#[derive(Default)]
pub struct ClockModule;

impl NotchModule for ClockModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: ModuleId::Clock,
            title: "Clock",
            aliases: &[],
        }
    }

    fn preferred_size(&self, config: &NotchConfig) -> ModuleSize {
        ModuleSize::from_config(config, false)
    }

    fn render(&mut self, context: ModuleContext) -> ModuleView {
        shell_ui_gpui::primitives::module_label(context.clock, &context.tokens)
    }
}
