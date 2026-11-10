//! 设计语言聚合：令牌汇总与写入 egui 全局样式的入口。
//! Design languages aggregate: tokens bundle and egui style installer.

use super::accessibility::AccessibilityTokens;
use super::colors::ColorLanguages;
use super::motion::MotionTokens;
use super::spacing::SpacingTokens;
use super::typography::{TypographyTokens, install_fonts};
use egui::{Color32, Context, Stroke, Style, Theme, Ui, Visuals};
use egui::style::{WidgetVisuals, Widgets};

/// 一个主题的完整设计语言（DMI 修改范围的总入口）。
/// Full design language of a theme; the entry of the DMI scope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesignLanguages {
    /// 归属主题（深/浅）。 / Owning theme (dark or light).
    pub theme: Theme,
    /// 颜色令牌。 / Color tokens.
    pub colors: ColorLanguages,
    /// 字体令牌。 / Typography tokens.
    pub typography: TypographyTokens,
    /// 间距令牌。 / Spacing tokens.
    pub spacing: SpacingTokens,
    /// 动画令牌。 / Motion tokens.
    pub motion: MotionTokens,
    /// 可访问性令牌。 / Accessibility tokens.
    pub accessibility: AccessibilityTokens,
}

impl DesignLanguages {
    /// 深色主题的设计语言。 / Design language of dark theme.
    pub fn dark() -> Self {
        Self {
            theme: Theme::Dark,
            colors: ColorLanguages::DARK,
            typography: TypographyTokens::DEFAULT,
            spacing: SpacingTokens::DEFAULT,
            motion: MotionTokens::DEFAULT,
            accessibility: AccessibilityTokens::DEFAULT,
        }
    }

    /// 浅色主题的设计语言。 / Design language of light theme.
    pub fn light() -> Self {
        Self {
            theme: Theme::Light,
            colors: ColorLanguages::LIGHT,
            typography: TypographyTokens::DEFAULT,
            spacing: SpacingTokens::DEFAULT,
            motion: MotionTokens::DEFAULT,
            accessibility: AccessibilityTokens::DEFAULT,
        }
    }

    /// 按主题取设计语言。 / Design language of a given theme.
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
        }
    }

    /// 由当前 UI 视觉反推主题设计语言。 / Derive tokens from current UI visuals.
    pub fn from_ui(ui: &Ui) -> Self {
        if ui.visuals().dark_mode {
            Self::dark()
        } else {
            Self::light()
        }
    }

    /// 组装单一控件状态视觉（四态复用）。
    /// Assembles widget visuals for one state, reused across states.
    fn state_visuals(&self, background: Color32, foreground: Color32) -> WidgetVisuals {
        WidgetVisuals {
            bg_fill: background,
            weak_bg_fill: background,
            bg_stroke: Stroke::new(1.0, self.colors.separator),
            corner_radius: self.spacing.corner(self.spacing.radius_sm),
            fg_stroke: Stroke::new(1.0, foreground),
            expansion: 0.0,
        }
    }

    /// 由颜色令牌生成 egui 五档控件状态视觉。
    /// Builds egui's five widget-state visuals from color tokens.
    fn build_widgets(&self) -> Widgets {
        let colors = self.colors;
        let surface = colors.surface;
        Widgets {
            noninteractive: self.state_visuals(surface, colors.text_primary),
            inactive: self.state_visuals(surface, colors.text_primary),
            hovered: self.state_visuals(colors.hover_of(surface), colors.text_primary),
            active: self.state_visuals(colors.pressed_of(surface), colors.text_primary),
            open: self.state_visuals(colors.hover_of(surface), colors.text_primary),
        }
    }

    /// 由颜色令牌生成 egui 全局视觉配置。
    /// Builds egui Visuals (backgrounds, selection, radii) from tokens.
    fn build_visuals(&self) -> Visuals {
        let colors = self.colors;
        let mut visuals = self.theme.default_visuals();
        visuals.dark_mode = matches!(self.theme, Theme::Dark);
        visuals.panel_fill = colors.background;
        visuals.window_fill = colors.surface_elevated;
        visuals.faint_bg_color = colors.background;
        visuals.extreme_bg_color = if visuals.dark_mode {
            colors.background.lerp_to_gamma(Color32::BLACK, 0.35)
        } else {
            colors.background.lerp_to_gamma(Color32::WHITE, 0.55)
        };
        visuals.text_edit_bg_color = Some(colors.surface);
        visuals.code_bg_color = visuals.extreme_bg_color;
        visuals.warn_fg_color = colors.destructive;
        visuals.error_fg_color = colors.destructive;
        visuals.hyperlink_color = colors.accent;
        visuals.override_text_color = None;
        visuals.weak_text_alpha = 0.75;
        visuals.disabled_alpha = 0.45;
        visuals.window_corner_radius = self.spacing.corner(self.spacing.radius_lg);
        visuals.menu_corner_radius = self.spacing.corner(self.spacing.radius_md);
        visuals.window_stroke = Stroke::new(1.0, colors.separator);
        visuals.widgets = self.build_widgets();
        visuals.selection.bg_fill = colors.selection_fill();
        visuals.selection.stroke = Stroke::new(0.0, colors.accent);
        visuals.text_cursor.stroke = Stroke::new(1.5, colors.accent);
        visuals
    }

    /// 把全部设计语言令牌写入一份 egui 样式。
    /// Writes every design token group into an egui style instance.
    pub fn apply(&self, style: &mut Style) {
        self.typography.apply(style);
        self.spacing.apply(style);
        style.visuals = self.build_visuals();
        style.animation_time = self.motion.toggle_seconds;
    }

    /// 装配入口：装载字体并把深浅两套样式写入上下文。
    /// Installer: loads fonts and applies dark/light styles to context.
    pub fn install(ctx: &Context) {
        install_fonts(ctx);
        for theme in [Theme::Dark, Theme::Light] {
            let languages = Self::for_theme(theme);
            ctx.style_mut_of(theme, |style| languages.apply(style));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::design::colors::ColorLanguages;

    /// 验证安装后深浅样式由设计语言接管。
    /// Verifies both themes' styles are taken over after install.
    #[test]
    fn install_writes_both_themes() {
        let ctx = Context::default();
        DesignLanguages::install(&ctx);
        for theme in [Theme::Dark, Theme::Light] {
            let style = ctx.style_of(theme);
            let expected = ColorLanguages::for_theme(theme);
            assert_eq!(style.visuals.panel_fill, expected.background);
            assert_eq!(style.visuals.hyperlink_color, expected.accent);
            assert_eq!(style.spacing.item_spacing.x, SpacingTokens::DEFAULT.gap_md);
        }
    }
}
