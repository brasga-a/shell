//! Player module boundary. Media state is expected to arrive through a
//! MediaPort/MPRIS adapter owned by the application composition root.

use shell_core::NotchConfig;
use shell_ui_gpui::{ModuleContext, ModuleId, ModuleMetadata, ModuleSize, ModuleView, NotchModule};

#[derive(Default)]
pub struct PlayerModule;

impl NotchModule for PlayerModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: ModuleId::Player,
            title: "Player",
            aliases: &[],
        }
    }

    fn preferred_size(&self, config: &NotchConfig) -> ModuleSize {
        ModuleSize::from_config(config, false)
    }

    fn render(&mut self, context: ModuleContext) -> ModuleView {
        shell_ui_gpui::primitives::module_label("Player", &context.tokens)
    }
}
