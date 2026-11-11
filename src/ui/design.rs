//! 设计语言：集中定义颜色与间距令牌，并提供写入 egui 全局样式的入口。
//! Design language: color/spacing tokens plus the egui global style installer.

use egui::style::{WidgetVisuals, Widgets};
use egui::{Color32, Context, CornerRadius, Stroke, Style, Theme, Ui, Visuals, vec2};

// ============================================================================
// 颜色令牌 Colors
// ============================================================================

/// DMI 统一管理的基础调色板，字段即颜色修改范围。
/// The base palette managed by DMI; fields define the color scope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorLanguages {
    /// 应用整体背景色。 / Whole application background.
    pub background: Color32,
    /// 常规控件与卡片表面色。 / Surface of normal widgets and cards.
    pub surface: Color32,
    /// 悬浮层（弹窗/菜单）表面色。 / Surface of elevated layers (popups, menus).
    pub surface_elevated: Color32,
    /// 主文本颜色。 / Primary text color.
    pub text_primary: Color32,
    /// 次要说明文本颜色。 / Secondary caption text color.
    pub text_secondary: Color32,
    /// 分隔线与控件描边颜色。 / Separator and widget outline color.
    pub separator: Color32,
    /// 强调色，主要操作与选中态。 / Accent for primary actions and selection.
    pub accent: Color32,
    /// 危险操作颜色（删除等）。 / Destructive action color (remove etc.).
    pub destructive: Color32,
}

impl ColorLanguages {
    /// 深色调色板。 / Dark mode palette.
    pub const DARK: Self = Self {
        background: Color32::from_rgb(23, 24, 28),
        surface: Color32::from_rgb(34, 36, 42),
        surface_elevated: Color32::from_rgb(43, 46, 54),
        text_primary: Color32::from_rgb(232, 234, 240),
        text_secondary: Color32::from_rgb(155, 161, 174),
        separator: Color32::from_rgb(58, 62, 72),
        accent: Color32::from_rgb(91, 140, 255),
        destructive: Color32::from_rgb(229, 72, 77),
    };

    /// 浅色调色板。 / Light mode palette.
    pub const LIGHT: Self = Self {
        background: Color32::from_rgb(244, 245, 248),
        surface: Color32::from_rgb(250, 250, 252),
        surface_elevated: Color32::from_rgb(255, 255, 255),
        text_primary: Color32::from_rgb(29, 34, 48),
        text_secondary: Color32::from_rgb(95, 102, 115),
        separator: Color32::from_rgb(217, 220, 227),
        accent: Color32::from_rgb(47, 111, 228),
        destructive: Color32::from_rgb(210, 56, 46),
    };

    /// 按主题取对应调色板。 / Pick the palette matching a theme.
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::DARK,
            Theme::Light => Self::LIGHT,
        }
    }

    /// 基础色叠加强调色后的悬停态混色。 / Hover mix: base tinted with accent.
    pub fn hover_of(&self, base: Color32) -> Color32 {
        base.lerp_to_gamma(self.accent, 0.12)
    }

    /// 基础色叠加强调色后的按下态混色。 / Pressed mix: base tinted deeper.
    pub fn pressed_of(&self, base: Color32) -> Color32 {
        base.lerp_to_gamma(self.accent, 0.24)
    }

    /// 禁用态混色：向表面色靠拢淡化。 / Disabled mix: faded toward surface.
    pub fn disabled_of(&self, base: Color32) -> Color32 {
        base.lerp_to_gamma(self.surface, 0.55)
    }

    /// 文本选中区域底色。 / Background of selected text range.
    pub fn selection_fill(&self) -> Color32 {
        self.accent.lerp_to_gamma(self.surface, 0.55)
    }
}

// ============================================================================
// 间距令牌 Spacing
// ============================================================================

/// 间距/圆角/尺寸令牌，统一控件布局与外观参数。
/// Spacing, radius and size tokens unifying widget layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingTokens {
    /// 控件横向内边距。 / Horizontal padding inside widgets.
    pub pad_horizontal: f32,
    /// 控件纵向内边距。 / Vertical padding inside widgets.
    pub pad_vertical: f32,
    /// 小圆角（按钮/输入框）。 / Small radius (buttons, fields).
    pub radius_sm: f32,
    /// 中圆角（弹层）。 / Medium radius (popups).
    pub radius_md: f32,
    /// 大圆角（窗口）。 / Large radius (windows).
    pub radius_lg: f32,
    /// 常规控件目标高度。 / Normal widget target height.
    pub control_height: f32,
    /// 紧凑控件目标高度。 / Compact widget target height.
    pub control_height_sm: f32,
    /// 文本输入框默认宽度。 / Default text field width.
    pub field_width: f32,
    /// 下拉菜单默认宽度。 / Default combo box width.
    pub combo_width: f32,
    /// 滑块默认宽度。 / Default slider width.
    pub slider_width: f32,
}

impl SpacingTokens {
    /// 深浅两主题共用的默认间距。 / Default spacing shared by both themes.
    pub const DEFAULT: Self = Self {
        pad_horizontal: 12.0,
        pad_vertical: 6.0,
        radius_sm: 6.0,
        radius_md: 8.0,
        radius_lg: 12.0,
        control_height: 32.0,
        control_height_sm: 26.0,
        field_width: 240.0,
        combo_width: 160.0,
        slider_width: 200.0,
    };

    /// 浮点半径转 egui 圆角值。 / Convert float radius into egui CornerRadius.
    pub fn corner(&self, radius: f32) -> CornerRadius {
        CornerRadius::same(radius.round() as u8)
    }

    /// 把间距令牌写入 egui 样式的全局间距。 / Write spacing tokens into egui style.
    pub fn apply(&self, style: &mut Style) {
        style.spacing.button_padding = vec2(self.pad_horizontal, self.pad_vertical);
        style.spacing.interact_size = vec2(self.control_height, self.control_height);
        style.spacing.slider_width = self.slider_width;
        style.spacing.combo_width = self.combo_width;
        style.spacing.text_edit_width = self.field_width;
    }
}

// ============================================================================
// 设计语言聚合 Design languages
// ============================================================================

/// 一个主题的完整设计语言（DMI 修改范围的总入口）。
/// Full design language of a theme; the entry of the DMI scope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesignLanguages {
    /// 归属主题（深/浅）。 / Owning theme (dark or light).
    pub theme: Theme,
    /// 颜色令牌。 / Color tokens.
    pub colors: ColorLanguages,
    /// 间距令牌。 / Spacing tokens.
    pub spacing: SpacingTokens,
}

impl DesignLanguages {
    /// 深色主题的设计语言。 / Design language of dark theme.
    pub fn dark() -> Self {
        Self {
            theme: Theme::Dark,
            colors: ColorLanguages::DARK,
            spacing: SpacingTokens::DEFAULT,
        }
    }

    /// 浅色主题的设计语言。 / Design language of light theme.
    pub fn light() -> Self {
        Self {
            theme: Theme::Light,
            colors: ColorLanguages::LIGHT,
            spacing: SpacingTokens::DEFAULT,
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

    /// 组装单一控件状态视觉（五档复用）。
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

    /// 把设计语言令牌写入一份 egui 样式。
    /// Writes design tokens into an egui style instance.
    pub fn apply(&self, style: &mut Style) {
        self.spacing.apply(style);
        style.visuals = self.build_visuals();
    }

    /// 装配入口：把深浅两套设计语言样式写入上下文。
    /// Installer: applies both dark and light token styles to the context.
    pub fn install(ctx: &Context) {
        for theme in [Theme::Dark, Theme::Light] {
            let languages = Self::for_theme(theme);
            ctx.style_mut_of(theme, |style| languages.apply(style));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert_eq!(
                style.spacing.button_padding.x,
                SpacingTokens::DEFAULT.pad_horizontal
            );
        }
    }
}
