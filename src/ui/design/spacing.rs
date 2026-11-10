//! 设计语言 - 间距令牌：间隙、内边距、圆角与控件尺寸。
//! Design language spacing tokens: gaps, padding, radii, control sizes.

use egui::{CornerRadius, Style, vec2};

/// 间距/圆角/尺寸令牌，统一控件布局与外观参数。
/// Spacing, radius and size tokens unifying widget layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingTokens {
    /// 微间隙（图标与文字）。 / Extra small gap (icon to text).
    pub gap_xs: f32,
    /// 小间隙（行内元素）。 / Small gap (inline items).
    pub gap_sm: f32,
    /// 标准间隙（条目间）。 / Standard gap between items.
    pub gap_md: f32,
    /// 大间隙（分组间）。 / Large gap between groups.
    pub gap_lg: f32,
    /// 特大间隙（区域间）。 / Extra large gap between regions.
    pub gap_xl: f32,
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
        gap_xs: 4.0,
        gap_sm: 6.0,
        gap_md: 8.0,
        gap_lg: 16.0,
        gap_xl: 24.0,
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
        style.spacing.item_spacing = vec2(self.gap_md, self.gap_sm);
        style.spacing.button_padding = vec2(self.pad_horizontal, self.pad_vertical);
        style.spacing.interact_size = vec2(self.control_height, self.control_height);
        style.spacing.slider_width = self.slider_width;
        style.spacing.combo_width = self.combo_width;
        style.spacing.text_edit_width = self.field_width;
    }
}
