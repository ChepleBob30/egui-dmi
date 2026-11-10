//! 设计语言 - 颜色令牌：调色板与状态混色规则。
//! Design language color tokens: palettes and state mixing rules.

use egui::{Color32, Theme};

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

    /// 表面色叠加强调色后的悬停态混色。 / Hover mix: surface tinted with accent.
    pub fn hover_of(&self, base: Color32) -> Color32 {
        base.lerp_to_gamma(self.accent, 0.12)
    }

    /// 表面色叠加强调色后的按下态混色。 / Pressed mix: surface tinted deeper.
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
