//! 组件状态记录表：组件每帧写入当前状态，供读取接口查询。
//! Observation store: widgets push per-frame state for read interfaces.

use crate::ui::id::DMIId;
use egui::Response;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// 组件当前状态记录：基本信息、交互状态、位置、是否可见。
/// Per-widget observation: basics, availability flags, context geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct DMIWidgetSnapshot {
    /// 类型专属基本信息。 / Type-specific basic information.
    pub basics: DMIBasics,
    /// 可用性位合集。 / Collected availability flags.
    pub flags: DMIFlags,
    /// 上下文矩形 (x, y, w, h)。 / Context rect as (x, y, w, h).
    pub rect: (f32, f32, f32, f32),
    /// 本帧是否可见。 / Whether visible this frame.
    pub visible: bool,
}

/// 基本信息按组件类型分成八种固定形态。
/// Closed basic-info set: seven widgets plus composite child items.
#[derive(Debug, Clone, PartialEq)]
pub enum DMIBasics {
    /// 按钮：文本。 / Button with its label.
    Button { text: String },
    /// 标签：文本与四种字号形式之一。 / Label with text tier.
    Label { text: String, tier: &'static str },
    /// 输入框：绑定文本、占位符、密码位。 / Field with bound text and modes.
    TextInput { text: String, placeholder: String, password: bool },
    /// 复选框：文本与勾选态。 / Checkbox with checked state.
    Checkbox { text: String, checked: bool },
    /// 单选框：文本与选中态。 / Radio with selected state.
    Radio { text: String, selected: bool },
    /// 下拉菜单：当前值与候选数组。 / Select with current value and options.
    Select { current: String, options: Vec<String> },
    /// 滑块：标签、数值与范围。 / Slider with label, value and range.
    Slider { label: Option<String>, value: f64, minimum: f64, maximum: f64 },
    /// 下拉子/孙项与单选组成员共用形态。 / Shared option shape for child items.
    OptionItem { text: String, selected: bool },
}

impl DMIBasics {
    /// 主文本映射：check_widget_status 的 widget_text 字段来源。
    /// Primary text mapping feeding the widget_text response field.
    pub fn widget_text(&self) -> String {
        match self {
            Self::Button { text }
            | Self::Label { text, .. }
            | Self::Checkbox { text, .. }
            | Self::Radio { text, .. }
            | Self::OptionItem { text, .. } => text.clone(),
            Self::TextInput { text, .. } => text.clone(),
            Self::Select { current, .. } => current.clone(),
            Self::Slider { label, value, .. } => {
                label.clone().unwrap_or_else(|| value.to_string())
            }
        }
    }
}

/// 可用性位：声明值与上一帧交互事实的合集。
/// Availability: declared enablement plus last-frame interaction facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DMIFlags {
    /// 组件声明的可交互。 / Declared interactivity.
    pub enabled: bool,
    /// 上一帧悬停。 / Hovered last frame.
    pub hovered: bool,
    /// 上一帧点击。 / Clicked last frame.
    pub clicked: bool,
    /// 指针按下于其上。 / Pointer held down on it.
    pub pressed: bool,
    /// 持有键盘焦点。 / Holding keyboard focus.
    pub focused: bool,
    /// 上一帧数据变更。 / Bound data changed last frame.
    pub changed: bool,
}

impl DMIFlags {
    /// 从交互响应提取可用性位。 / Extracts flags from an egui response.
    pub fn from_response(response: &Response, enabled: bool) -> Self {
        Self {
            enabled,
            hovered: response.hovered(),
            clicked: response.clicked(),
            pressed: response.is_pointer_button_down_on(),
            focused: response.has_focus(),
            changed: response.changed(),
        }
    }
}

/// 响应矩形转 (x, y, w, h) 元组。 / Converts the response rect to a tuple.
pub fn response_rect(response: &Response) -> (f32, f32, f32, f32) {
    let rect = response.rect;
    (rect.min.x, rect.min.y, rect.width(), rect.height())
}

/// 全局状态记录表；未注册组件（SYSTEM）的写入会被拒绝。
/// Global state-record store; system ids are refused.
static OBSERVE_STORE: OnceLock<Mutex<HashMap<DMIId, DMIWidgetSnapshot>>> = OnceLock::new();

/// 写入或覆盖一条状态记录；未注册的编号直接跳过。
/// Records one observation; unregistered ids are skipped silently.
pub fn observe(dmi_id: DMIId, snapshot: DMIWidgetSnapshot) {
    if dmi_id.is_system() {
        return;
    }
    let store = OBSERVE_STORE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.insert(dmi_id, snapshot);
}

/// 读取指定编号最近一次写入的状态记录。 / Reads the latest stored state for one id.
pub fn snapshot_of(dmi_id: DMIId) -> Option<DMIWidgetSnapshot> {
    let store = OBSERVE_STORE.get_or_init(|| Mutex::new(HashMap::new()));
    let guard = store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.get(&dmi_id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dmi::call_dmi;
    use crate::ui::design::DesignLanguages;
    use crate::ui::{DMIButton, UIEntry};
    use egui::Id;

    /// 状态记录写入后读取一致，SYSTEM 编号被拒绝写入。
    /// State records round-trip; system ids never enter the store.
    #[test]
    fn observe_roundtrip_skips_system() {
        let probe_id = crate::ui::id::allocate(
            Id::NULL.with("obs_probe"),
            crate::ui::id::DMIWidgetKind::Button,
            "case_observe_probe",
            &[("press", &[UIEntry::LeftClick])],
        );
        let snapshot = DMIWidgetSnapshot {
            basics: DMIBasics::Button { text: "观察".to_owned() },
            flags: DMIFlags {
                enabled: true,
                hovered: false,
                clicked: false,
                pressed: false,
                focused: false,
                changed: false,
            },
            rect: (1.0, 2.0, 3.0, 4.0),
            visible: true,
        };
        observe(probe_id, snapshot.clone());
        assert_eq!(snapshot_of(probe_id).as_ref(), Some(&snapshot));
        observe(DMIId::SYSTEM, snapshot.clone());
        assert!(snapshot_of(DMIId::SYSTEM).is_none());
    }

    /// 绘制命名按钮后，三个等级的应答逐字段核对正确。
    /// Rendered widget answers all three tiers with stable key order.
    #[test]
    fn check_widget_status_tiers_after_render() {
        egui::__run_test_ui(|ui| {
            DesignLanguages::install(ui.ctx());
            let _ = DMIButton::primary("状态检查")
                .dmi_name("case_status_button")
                .show(ui);
        });
        let records = crate::ui::id::list_id();
        let record = records
            .iter()
            .find(|record| record.name == "case_status_button")
            .expect("按钮应已注册");
        let simple = call_dmi(&format!(
            "{{\"request_type\":\"OBSERVATION\",\"interface\":\"check_widget_status\",\"target_id\":{},\"complexity\":\"simple\"}}",
            record.id.major
        ));
        assert!(simple.starts_with("{\"result\":\"OK\""), "{simple}");
        assert!(simple.contains("\"widget_name\":\"case_status_button\""));
        assert!(simple.contains("\"widget_type\":\"button\""));
        assert!(simple.contains("\"widget_text\":\"状态检查\""));
        let standard = call_dmi(&format!(
            "{{\"request_type\":\"OBSERVATION\",\"interface\":\"check_widget_status\",\"target_id\":{},\"complexity\":\"standard\"}}",
            record.id.major
        ));
        assert!(standard.contains("\"enabled\":true"), "{standard}");
        assert!(standard.contains("\"widget_function\""), "{standard}");
        let complex = call_dmi(&format!(
            "{{\"request_type\":\"OBSERVATION\",\"interface\":\"check_widget_status\",\"target_id\":{},\"complexity\":\"complex\"}}",
            record.id.major
        ));
        assert!(complex.contains("\"rect\":{\"x\""), "{complex}");
        assert!(complex.contains("\"visible\":true"), "{complex}");
        assert!(complex.contains("\"focused\""), "{complex}");
    }

    /// 只注册但从未绘制的编号，应答返回 TARGET_ID_INVALID。
    /// Registered-but-never-rendered ids hit the defensive invalid branch.
    #[test]
    fn registered_without_snapshot_is_refused() {
        let ghost_id = crate::ui::id::allocate(
            Id::NULL.with("ghost_probe"),
            crate::ui::id::DMIWidgetKind::Slider,
            "case_ghost_probe",
            &[("set", &[UIEntry::Drag])],
        );
        let response = call_dmi(&format!(
            "{{\"request_type\":\"OBSERVATION\",\"interface\":\"check_widget_status\",\"target_id\":{},\"complexity\":\"standard\"}}",
            ghost_id.major
        ));
        assert!(response.contains("TARGET_ID_INVALID"), "{response}");
    }
}
