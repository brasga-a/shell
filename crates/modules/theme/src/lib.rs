//! Theme controller module. The shared theme snapshot is supplied by the
//! application context; this crate does not define a second theme format.

use shell_core::NotchConfig;
use shell_ui_gpui::{ModuleContext, ModuleId, ModuleMetadata, ModuleSize, ModuleView, NotchModule};

#[derive(Default)]
pub struct ThemeModule;

impl NotchModule for ThemeModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: ModuleId::Theme,
            title: "Theme",
            aliases: &[],
        }
    }

    fn preferred_size(&self, config: &NotchConfig) -> ModuleSize {
        ModuleSize::from_config(config, false)
    }

    fn render(&mut self, context: ModuleContext) -> ModuleView {
        shell_ui_gpui::primitives::module_label("Theme", &context.tokens)
    }
}
