//! Small GPUI primitives shared by module crates.

use gpui::{AnyElement, SharedString, div, prelude::*, px, rgb};
use shell_theme::DesignTokens;

/// Common compact module label used by modules that do not need a custom
/// feature surface yet.
pub fn module_label(label: impl Into<SharedString>, tokens: &DesignTokens) -> AnyElement {
    div()
        .flex()
        .items_center()
        .text_color(rgb(tokens.colors.foreground))
        .text_size(px(tokens.typography.label_size as f32))
        .child(label.into())
        .into_any_element()
}
