//! Centralized, renderer-friendly design tokens.

use shell_core::{ColorConfig, ThemeConfig, ThemeScale};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColorTokens {
    pub background: u32,
    pub background_overlay: u32,
    pub foreground: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypographyTokens {
    pub body_size: u32,
    pub label_size: u32,
    pub title_size: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesignTokens {
    pub colors: ColorTokens,
    pub radius: ThemeScale,
    pub spacing: ThemeScale,
    pub motion: shell_core::MotionConfig,
    pub typography: TypographyTokens,
}

impl DesignTokens {
    pub fn from_config(config: &ThemeConfig) -> Self {
        let defaults = ThemeConfig::default();
        let background = parse_color(&config.colors, true)
            .unwrap_or_else(|| parse_color(&defaults.colors, true).expect("default color"));
        let foreground = parse_color(&config.colors, false)
            .unwrap_or_else(|| parse_color(&defaults.colors, false).expect("default color"));

        Self {
            colors: ColorTokens {
                background,
                background_overlay: (background << 8) | 0xe6,
                foreground,
            },
            radius: config.radius,
            spacing: config.spacing,
            motion: config.motion,
            typography: TypographyTokens {
                body_size: config.typography.body_size,
                label_size: config.typography.label_size,
                title_size: config.typography.title_size,
            },
        }
    }
}

impl Default for DesignTokens {
    fn default() -> Self {
        Self::from_config(&ThemeConfig::default())
    }
}

fn parse_color(colors: &ColorConfig, background: bool) -> Option<u32> {
    let value = if background {
        &colors.background
    } else {
        &colors.foreground
    };
    let value = value.strip_prefix('#').unwrap_or(value);
    (value.len() == 6)
        .then(|| u32::from_str_radix(value, 16).ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use shell_core::ThemeConfig;

    use super::DesignTokens;

    #[test]
    fn resolves_theme_colors_for_renderers() {
        let mut config = ThemeConfig::default();
        config.colors.background = "#112233".to_string();
        config.colors.foreground = "#DDEEFF".to_string();

        let tokens = DesignTokens::from_config(&config);

        assert_eq!(tokens.colors.background, 0x112233);
        assert_eq!(tokens.colors.background_overlay, 0x112233e6);
        assert_eq!(tokens.colors.foreground, 0xddeeff);
    }
}
