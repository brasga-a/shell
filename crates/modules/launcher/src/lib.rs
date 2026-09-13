//! Launcher module hosted by the Notch.

use gpui::{FontWeight, div, prelude::*, px, rgb, rgba};
use shell_core::NotchConfig;
use shell_ui_gpui::{ModuleContext, ModuleId, ModuleMetadata, ModuleSize, ModuleView, NotchModule};

const ITEMS: &[(&str, &str, &str)] = &[
    (
        "◉",
        "About Xfce",
        "Information about the Xfce Desktop Environment",
    ),
    (
        "▣",
        "Advanced Network Configuration",
        "Manage and change network connection settings",
    ),
    ("△", "Alacritty", "Terminal"),
    (
        "◌",
        "AsusCtlTray",
        "A tray icon to switch asusctl profiles on the fly",
    ),
    (
        "✣",
        "auto-cpufreq",
        "Automatic CPU frequency and power optimizer",
    ),
    (
        "◈",
        "Avahi SSH Server Browser",
        "Browse for Zeroconf-enabled SSH Servers",
    ),
];

#[derive(Default)]
pub struct LauncherModule;

impl NotchModule for LauncherModule {
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: ModuleId::Launcher,
            title: "Launcher",
            aliases: &[],
        }
    }

    fn preferred_size(&self, config: &NotchConfig) -> ModuleSize {
        ModuleSize::from_config(config, true)
    }

    fn render(&mut self, context: ModuleContext) -> ModuleView {
        let tokens = context.tokens;
        let query_lower = context.query.to_lowercase();
        let rows = ITEMS
            .iter()
            .filter(|(_, name, description)| {
                query_lower.is_empty()
                    || name.to_lowercase().contains(&query_lower)
                    || description.to_lowercase().contains(&query_lower)
            })
            .map(|(icon, name, description)| {
                div()
                    .w_full()
                    .h(px(44.0))
                    .px(px(tokens.spacing.sm as f32))
                    .rounded(px(tokens.radius.sm as f32))
                    .flex()
                    .items_center()
                    .gap(px(tokens.spacing.sm as f32))
                    .bg(rgba(0x242424e8))
                    .text_color(rgb(tokens.colors.foreground))
                    .child(
                        div()
                            .w(px(28.0))
                            .h(px(28.0))
                            .rounded(px(tokens.radius.sm as f32))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgba(0x3b3b3bf0))
                            .text_size(px(tokens.typography.title_size as f32 - 2.0))
                            .child(*icon),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .text_size(px(tokens.typography.body_size as f32))
                                    .font_weight(FontWeight::BOLD)
                                    .child(*name),
                            )
                            .child(
                                div()
                                    .text_size(px(tokens.typography.label_size as f32 - 1.0))
                                    .text_color(rgba(0xaaa6a6e6))
                                    .child(*description),
                            ),
                    )
            });

        div()
            .w(px(context.notch.width as f32))
            .h(px(context.notch.expanded_height as f32))
            .p(px(tokens.spacing.md as f32))
            .flex()
            .flex_col()
            .gap(px(tokens.spacing.sm as f32))
            .text_color(rgb(tokens.colors.foreground))
            .child(
                div()
                    .w_full()
                    .h(px(40.0))
                    .px(px(tokens.spacing.sm as f32))
                    .flex()
                    .items_center()
                    .gap(px(tokens.spacing.sm as f32))
                    .border_b(px(1.0))
                    .border_color(rgba(0x8b8b8b66))
                    .text_size(px(tokens.typography.body_size as f32))
                    .child("⌕")
                    .child(if context.query.is_empty() {
                        "Search...".to_string()
                    } else {
                        context.query
                    }),
            )
            .child(div().flex().flex_col().gap(px(2.0)).children(rows))
            .into_any_element()
    }
}
