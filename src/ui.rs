//! UI 封装模块根：设计语言层、组件层与编号注册表的统一出口。
//! UI module root: exports design language, components and the id registry.

pub mod components;
pub mod design;
pub mod id;

/// 组件触及方式：注册期随功能名声明，Access 阶段据此驱动组件。
/// Widget touch entry: declared per capability at registration, driven by Access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UIEntry {
    /// 左键单击（Response::clicked）。 / Primary button click.
    LeftClick,
    /// 右键单击（secondary_clicked）。 / Secondary button click.
    RightClick,
    /// 双击（double_clicked）。 / Double click.
    DoubleClick,
    /// 中键单击（middle_clicked）。 / Middle button click.
    CenterClick,
    /// 悬停（hovered）。 / Pointer hover.
    Hover,
    /// 拖拽（drag_started/dragged）。 / Pointer drag.
    Drag,
    /// 键盘焦点（gained_focus/request_focus）。 / Keyboard focus.
    Focus,
    /// 聚焦后按键激活（如空格勾选、方向键步进）。 / Key press while focused.
    PressKey(egui::Key),
    /// 根组件 parent 占位：无上级、直接可见；禁止出现在功能列表。
    /// Root placeholder in parent tuples; never valid inside a capability.
    Direct,
}

pub use components::{
    DMICheckbox, DMIButton, DMIButtonKind, DMILabel, DMIRadio, DMISelect, DMISlider,
    DMITextInput, DMIUpdate,
};
pub use design::DesignLanguages;
pub use id::{DMIId, DMIRecord, DMIWidgetKind, list_id};
