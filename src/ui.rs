//! 界面层总入口：导出设计参数、组件与编号注册表。
//! UI module root: exports design language, components and the id registry.

pub mod components;
pub mod design;
pub mod id;

/// 组件的触及方式，登记时随功能名一起声明。
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
    /// 根组件专用，表示没有上级，不允许出现在功能列表里。
    /// Root placeholder in parent tuples; never valid inside a capability.
    Direct,
}

pub use components::{
    DMICheckbox, DMIButton, DMIButtonKind, DMILabel, DMIRadio, DMISelect, DMISlider,
    DMITextInput, DMIUpdate,
};
pub use design::DesignLanguages;
pub use id::{DMIId, DMIRecord, DMIWidgetKind, list_id};
