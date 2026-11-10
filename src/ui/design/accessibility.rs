//! 设计语言 - 可访问性令牌：触摸目标、焦点环与角色命名。
//! Accessibility tokens: touch targets, focus ring, role naming.

use egui::{Stroke, vec2};

/// 可访问性参数：最小目标尺寸与焦点环样式。
/// Accessibility params: minimal target size and focus ring style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccessibilityTokens {
    /// 鼠标/触摸最小目标尺寸（点）。 / Minimal mouse/touch target size in points.
    pub min_target_size: f32,
    /// 焦点环线宽（点）。 / Focus ring stroke width in points.
    pub focus_ring_width: f32,
    /// 焦点环与控件的间隙（点）。 / Gap between focus ring and widget.
    pub focus_ring_gap: f32,
}

impl AccessibilityTokens {
    /// 默认：44点触摸目标、2点焦点环。 / Default 44pt target, 2pt focus ring.
    pub const DEFAULT: Self = Self {
        min_target_size: 44.0,
        focus_ring_width: 2.0,
        focus_ring_gap: 1.0,
    };

    /// 最小触摸目标的 egui 尺寸向量。 / Minimal touch target as egui Vec2.
    pub fn min_target(&self) -> egui::Vec2 {
        vec2(self.min_target_size, self.min_target_size)
    }

    /// 焦点环描边（用强调色）。 / Focus ring stroke using the accent color.
    pub fn focus_ring_stroke(&self, accent: egui::Color32) -> Stroke {
        Stroke::new(self.focus_ring_width, accent)
    }
}

/// DMI 组件可访问角色枚举。 / Enum of accessible roles of DMI components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DMIComponentRole {
    /// 按钮。 / Button.
    Button,
    /// 文本输入框。 / Text input field.
    TextInput,
    /// 复选框。 / Checkbox.
    CheckBox,
    /// 单选框。 / Radio button.
    RadioButton,
    /// 下拉菜单。 / Select menu.
    Select,
    /// 滑块。 / Slider.
    Slider,
    /// 标签文本。 / Label text.
    Label,
}
