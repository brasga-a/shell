//! Calendar module boundary. Calendar state/navigation will be added here as
//! the shared date model is introduced.

use shell_core::NotchConfig;
use shell_ui_gpui::{ModuleContext, ModuleId, ModuleMetadata, ModuleSize, ModuleView, NotchModule};

#[derive(Default)]
pub struct CalendarModule;

impl NotchModule for CalendarModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: ModuleId::Calendar,
            title: "Calendar",
            aliases: &[],
        }
    }

    fn preferred_size(&self, config: &NotchConfig) -> ModuleSize {
        ModuleSize::from_config(config, false)
    }

    fn render(&mut self, context: ModuleContext) -> ModuleView {
        shell_ui_gpui::primitives::module_label("Calendar", &context.tokens)
    }
}
