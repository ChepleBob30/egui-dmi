//! DMI 组件：对 egui 基础控件的声明式封装，统一颜色与圆角风格。
//! DMI components: declarative wrappers over egui base widgets.

use crate::ui::design::DesignLanguages;
use crate::ui::id::{self, DMIId, DMIWidgetKind};
use egui::{
    Color32, ComboBox, Id, IdSalt, Margin, RadioButton, Response, RichText, Sense, Stroke,
    StrokeKind, TextEdit, TextWrapMode, TextStyle, Ui, WidgetText,
};
use egui::emath::Numeric;
use std::ops::RangeInclusive;

// ============================================================================
// 统一返回载体 DMI update
// ============================================================================

/// 组件 `show()` 的统一返回；当前承载 egui::Response 的状态查询。
/// Shared return of `show()`; currently delegates to egui::Response.
///
/// `DMI标记`: 本结构是 DMI 协议对 Response 的替代占位——
/// 接入 `src/dmi/` 数据通道后，调用方只面向 DMIUpdate，不再接触 Response。
/// Marker: this will replace `Response` once the dmi/ channel lands.
pub struct DMIUpdate {
    /// 组件的 DMI 编号（未来 JSON 调用的 target_id 来源）。
    /// DMI id of the widget, the future JSON target_id source.
    pub id: DMIId,
    /// 底层 egui 响应（迁移期保留，DMI 通道就绪后移除）。
    /// Underlying egui response (kept during migration, removed later).
    pub response: Response,
    /// 本次交互是否改变了绑定数据。 / Whether this interaction changed bound data.
    pub changed: bool,
}

impl DMIUpdate {
    /// 由编号与响应构建，changed 采用 egui changed 语义。
    /// Builds from id and response using egui changed semantics.
    fn from_response(id: DMIId, response: Response) -> Self {
        Self { id, changed: response.changed(), response }
    }

    /// 由编号、响应与显式 changed 构建。
    /// Builds from id, response and an explicit changed flag.
    fn with_changed(id: DMIId, response: Response, changed: bool) -> Self {
        Self { id, changed, response }
    }

    /// 是否被点击。 / Was the widget clicked.
    pub fn clicked(&self) -> bool {
        self.response.clicked()
    }

    /// 是否发生数据变更。 / Did bound data change.
    pub fn changed(&self) -> bool {
        self.changed
    }
}

// ============================================================================
// 按钮 Button
// ============================================================================

/// 按钮视觉类别。 / Visual kind of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DMIButtonKind {
    /// 主要操作，强调色底。 / Primary action with accent fill.
    Primary,
    /// 普通操作，表面色底加描边。 / Normal action, surface fill with outline.
    Secondary,
    /// 危险操作，危险色底。 / Destructive action with danger fill.
    Destructive,
    /// 无边框文本按钮。 / Frameless text button.
    Text,
}

/// 封装按钮：常态/悬停/按下/禁用四态着色、悬停提示与统一圆角。
/// Button wrapper: four-state fills, tooltip and unified radius.
#[derive(Debug, Clone)]
pub struct DMIButton<'a> {
    /// 按钮文字。 / Button label text.
    text: &'a str,
    /// 视觉类别。 / Visual kind.
    kind: DMIButtonKind,
    /// 是否可交互。 / Whether interactive.
    enabled: bool,
    /// 悬停提示文本。 / Hover tooltip text.
    tooltip: Option<String>,
    /// DMI 注册名称（命名不同则额外分配编号）。
    /// Explicit DMI name; a different name allocates its own id.
    dmi_name: Option<String>,
}

impl<'a> DMIButton<'a> {
    /// 构建普通按钮。 / Build a secondary button.
    pub fn new(text: &'a str) -> Self {
        Self { text, kind: DMIButtonKind::Secondary, enabled: true, tooltip: None, dmi_name: None }
    }

    /// 构建主要操作按钮。 / Build a primary action button.
    pub fn primary(text: &'a str) -> Self {
        Self { kind: DMIButtonKind::Primary, ..Self::new(text) }
    }

    /// 构建危险操作按钮（删除等）。 / Build a destructive button (remove etc.).
    pub fn destructive(text: &'a str) -> Self {
        Self { kind: DMIButtonKind::Destructive, ..Self::new(text) }
    }

    /// 构建无边框文本按钮。 / Build a frameless text button.
    pub fn text(text: &'a str) -> Self {
        Self { kind: DMIButtonKind::Text, ..Self::new(text) }
    }

    /// 设置是否可交互。 / Set interactivity.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 设置悬停提示。 / Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// 设置 DMI 注册名称。 / Set the DMI registry name.
    pub fn dmi_name(mut self, name: impl Into<String>) -> Self {
        self.dmi_name = Some(name.into());
        self
    }

    /// 渲染按钮：测量文字、分配区域、按状态绘制底色与文字。
    /// Renders: measures text, allocates rect, paints fill and label by state.
    #[must_use = "交互组件结果应被检查 / inspect the interactive widget result"]
    pub fn show(self, ui: &mut Ui) -> DMIUpdate {
        let tokens = DesignLanguages::from_ui(ui);
        let colors = tokens.colors;
        let base = match self.kind {
            DMIButtonKind::Primary => colors.accent,
            DMIButtonKind::Destructive => colors.destructive,
            DMIButtonKind::Secondary => colors.surface,
            DMIButtonKind::Text => Color32::TRANSPARENT,
        };
        // 以按钮文本样式测量文字，加上内边距得到目标尺寸。
        // Measure with button text style, add padding for desired size.
        let label = WidgetText::from(RichText::new(self.text).color(colors.text_primary));
        let galley =
            label.into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextStyle::Button);
        let mut size = galley.size() + ui.spacing().button_padding * 2.0;
        size.y = size.y.max(tokens.spacing.control_height);
        // 禁用时只保留悬停感应，不再响应点击。
        // When disabled keep hover sense only, drop clicks.
        let (rect, response) = ui.allocate_at_least(size, Sense::hover());
        let response =
            if self.enabled { ui.interact(rect, response.id, Sense::click()) } else { response };
        // 状态优先级：禁用 > 按下 > 悬停 > 常态。
        // State priority: disabled > pressed > hover > idle.
        let fill = if !self.enabled {
            colors.disabled_of(base)
        } else if response.is_pointer_button_down_on() {
            colors.pressed_of(base)
        } else if response.hovered() {
            colors.hover_of(base)
        } else {
            base
        };
        // 文本按钮不画底，次要按钮加分隔色描边。
        // Text kind skips fill; secondary kind gets a separator outline.
        if self.kind != DMIButtonKind::Text {
            let stroke = if self.kind == DMIButtonKind::Secondary {
                Stroke::new(
                    1.0,
                    if self.enabled { colors.separator } else { colors.disabled_of(colors.separator) },
                )
            } else {
                Stroke::NONE
            };
            ui.painter()
                .rect(rect, tokens.spacing.corner(tokens.spacing.radius_sm), fill, stroke, StrokeKind::Inside);
        }
        // 文字居中绘制。 / Paint the label centered.
        let text_pos = rect.center() - galley.size() * 0.5;
        ui.painter().galley(text_pos, galley, colors.text_primary);
        // 组装悬停提示并封装统一返回。
        // Attach tooltip then wrap into the shared return type.
        let response = match &self.tooltip {
            Some(hover_text) => response.on_hover_text(hover_text.clone()),
            None => response,
        };
        let dmi_id = id::allocate(response.id, DMIWidgetKind::Button, self.dmi_name.as_deref(), None);
        let clicked = response.clicked();
        DMIUpdate::with_changed(dmi_id, response, clicked)
    }
}

// ============================================================================
// 标签文本 Label
// ============================================================================

/// 封装标签：标题/正文/说明/等宽四档；显式 dmi_name 才登记编号。
/// Label wrapper: four tiers; registers only when explicitly named.
#[derive(Debug, Clone)]
pub struct DMILabel<'a> {
    /// 标签文本。 / Label text.
    text: &'a str,
    /// 是否弱色（说明档）。 / Weak (caption) color flag.
    caption: bool,
    /// 渲染文本样式档位。 / egui text style tier.
    style: TextStyle,
    /// 悬停提示文本。 / Hover tooltip text.
    tooltip: Option<String>,
    /// DMI 注册名称（命名不同则额外分配编号）。
    /// Explicit DMI name; a different name allocates its own id.
    dmi_name: Option<String>,
}

impl<'a> DMILabel<'a> {
    /// 构建正文标签。 / Build a body label.
    pub fn body(text: &'a str) -> Self {
        Self { text, caption: false, style: TextStyle::Body, tooltip: None, dmi_name: None }
    }

    /// 构建标题标签。 / Build a heading label.
    pub fn heading(text: &'a str) -> Self {
        Self { style: TextStyle::Heading, ..Self::body(text) }
    }

    /// 构建次要说明标签。 / Build a secondary caption label.
    pub fn caption(text: &'a str) -> Self {
        Self { caption: true, style: TextStyle::Small, ..Self::body(text) }
    }

    /// 构建等宽标签（代码/数据）。 / Build a monospace label (code/data).
    pub fn mono(text: &'a str) -> Self {
        Self { style: TextStyle::Monospace, ..Self::body(text) }
    }

    /// 设置悬停提示。 / Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// 设置 DMI 注册名称。 / Set the DMI registry name.
    pub fn dmi_name(mut self, name: impl Into<String>) -> Self {
        self.dmi_name = Some(name.into());
        self
    }

    /// 渲染标签：字号走 egui 文本样式，颜色取自调色板。
    /// Renders label: size via egui text styles, color via palette.
    pub fn show(self, ui: &mut Ui) -> DMIUpdate {
        let colors = DesignLanguages::from_ui(ui).colors;
        let color = if self.caption { colors.text_secondary } else { colors.text_primary };
        let rich = match self.style {
            TextStyle::Heading => RichText::new(self.text).heading(),
            TextStyle::Small => RichText::new(self.text).small(),
            TextStyle::Monospace => RichText::new(self.text).monospace(),
            _ => RichText::new(self.text),
        }
        .color(color);
        let response = ui.add(egui::Label::new(WidgetText::from(rich)));
        let response = match &self.tooltip {
            Some(hover_text) => response.on_hover_text(hover_text.clone()),
            None => response,
        };
        // 匿名标签不登记：显示文本频繁变动，登记会反哺编号表自我增长。
        // Unnamed labels skip registration to stop registry self-amplification.
        let dmi_id = match &self.dmi_name {
            Some(given) => id::allocate(response.id, DMIWidgetKind::Label, Some(given), None),
            None => DMIId::SYSTEM,
        };
        DMIUpdate::from_response(dmi_id, response)
    }
}

// ============================================================================
// 文本输入框 Text input
// ============================================================================

/// 封装单行文本输入框：占位符、密码模式与统一框体风格。
/// Single-line field wrapper: placeholder, password and styled frame.
pub struct DMITextInput<'a> {
    /// 绑定文本缓冲。 / Bound text buffer.
    buffer: &'a mut String,
    /// 占位提示文本。 / Placeholder hint text.
    placeholder: String,
    /// 是否密码模式。 / Password masking flag.
    password: bool,
    /// 是否可交互。 / Whether interactive.
    enabled: bool,
    /// DMI 注册名称（命名不同则额外分配编号）。
    /// Explicit DMI name; a different name allocates its own id.
    dmi_name: Option<String>,
}

impl<'a> DMITextInput<'a> {
    /// 构建输入框并绑定文本。 / Build a field bound to the text.
    pub fn new(buffer: &'a mut String) -> Self {
        Self { buffer, placeholder: String::new(), password: false, enabled: true, dmi_name: None }
    }

    /// 设置占位提示文本。 / Set the placeholder text.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// 开启密码掩码模式。 / Enable password masking mode.
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// 设置是否可交互。 / Set interactivity.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 设置 DMI 注册名称。 / Set the DMI registry name.
    pub fn dmi_name(mut self, name: impl Into<String>) -> Self {
        self.dmi_name = Some(name.into());
        self
    }

    /// 渲染输入框：框体底色/描边/圆角来自设计语言。
    /// Renders the field with frame colors and radius from tokens.
    #[must_use = "交互组件结果应被检查 / inspect the interactive widget result"]
    pub fn show(self, ui: &mut Ui) -> DMIUpdate {
        let tokens = DesignLanguages::from_ui(ui);
        let mut edit = TextEdit::singleline(self.buffer)
            .margin(Margin::same(tokens.spacing.pad_vertical.round() as i8))
            .interactive(self.enabled);
        if !self.placeholder.is_empty() {
            edit = edit.hint_text(self.placeholder.clone());
        }
        if self.password {
            edit = edit.password(true);
        }
        // 框体：表面底色 + 分隔描边 + 小圆角。
        // Frame: surface fill, separator outline, small radius.
        let frame = egui::Frame::default()
            .fill(tokens.colors.surface)
            .stroke(Stroke::new(1.0, tokens.colors.separator))
            .corner_radius(tokens.spacing.corner(tokens.spacing.radius_sm));
        let response = ui.add(edit.frame(frame));
        let dmi_id = id::allocate(response.id, DMIWidgetKind::TextInput, self.dmi_name.as_deref(), None);
        DMIUpdate::from_response(dmi_id, response)
    }
}

// ============================================================================
// 复选框 Checkbox
// ============================================================================

/// 封装复选框：绑定布尔值，统一启用与悬停提示。
/// Checkbox wrapper: binds a bool with shared enable and tooltip.
pub struct DMICheckbox<'a> {
    /// 绑定布尔值。 / Bound boolean value.
    checked: &'a mut bool,
    /// 标签文本。 / Label text.
    text: &'a str,
    /// 是否可交互。 / Whether interactive.
    enabled: bool,
    /// 悬停提示文本。 / Hover tooltip text.
    tooltip: Option<String>,
    /// DMI 注册名称（命名不同则额外分配编号）。
    /// Explicit DMI name; a different name allocates its own id.
    dmi_name: Option<String>,
}

impl<'a> DMICheckbox<'a> {
    /// 构建复选框并绑定布尔值。 / Build a checkbox bound to the bool.
    pub fn new(checked: &'a mut bool, text: &'a str) -> Self {
        Self { checked, text, enabled: true, tooltip: None, dmi_name: None }
    }

    /// 设置是否可交互。 / Set interactivity.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 设置悬停提示。 / Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// 设置 DMI 注册名称。 / Set the DMI registry name.
    pub fn dmi_name(mut self, name: impl Into<String>) -> Self {
        self.dmi_name = Some(name.into());
        self
    }

    /// 渲染复选框。 / Renders the checkbox.
    #[must_use = "交互组件结果应被检查 / inspect the interactive widget result"]
    pub fn show(self, ui: &mut Ui) -> DMIUpdate {
        let box_widget = egui::Checkbox::new(self.checked, self.text);
        let response = ui.add_enabled(self.enabled, box_widget);
        let response = match &self.tooltip {
            Some(hover_text) => response.on_hover_text(hover_text.clone()),
            None => response,
        };
        let dmi_id = id::allocate(response.id, DMIWidgetKind::Checkbox, self.dmi_name.as_deref(), None);
        DMIUpdate::from_response(dmi_id, response)
    }
}

// ============================================================================
// 单选框 Radio
// ============================================================================

/// 封装单选框：调用方依据 clicked 写回选中值。
/// Radio wrapper: caller writes the value back on clicked.
pub struct DMIRadio<'a> {
    /// 当前是否选中。 / Whether this option is selected now.
    selected: bool,
    /// 标签文本。 / Label text.
    text: &'a str,
    /// 是否可交互。 / Whether interactive.
    enabled: bool,
    /// DMI 注册名称（命名不同则额外分配编号）。
    /// Explicit DMI name; a different name allocates its own id.
    dmi_name: Option<String>,
    /// 组名：同组单选框归属统一父号并领取小数子号。
    /// Group name: radios share one parent id and get dotted child ids.
    group: Option<String>,
}

impl<'a> DMIRadio<'a> {
    /// 构建单选框。 / Build a radio button.
    pub fn new(selected: bool, text: &'a str) -> Self {
        Self { selected, text, enabled: true, dmi_name: None, group: None }
    }

    /// 设置是否可交互。 / Set interactivity.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 设置 DMI 注册名称。 / Set the DMI registry name.
    pub fn dmi_name(mut self, name: impl Into<String>) -> Self {
        self.dmi_name = Some(name.into());
        self
    }

    /// 归入单选组：父号统一可寻址，子号形如 父号.序号（dmi.md 两种编号并存）。
    /// Joins a radio group: one parent id plus dotted child ids.
    pub fn group(mut self, name: impl Into<String>) -> Self {
        self.group = Some(name.into());
        self
    }

    /// 渲染单选框；changed 语义为"本次被点选"。
    /// Renders radio; changed means "picked on this click".
    #[must_use = "交互组件结果应被检查 / inspect the interactive widget result"]
    pub fn show(self, ui: &mut Ui) -> DMIUpdate {
        let radio = RadioButton::new(self.selected, self.text);
        let response = ui.add_enabled(self.enabled, radio);
        let clicked = response.clicked();
        // 组模式：组名首现发统一父号，本控件按"组名_选项文本"领取小数子号。
        // Group mode: first sighting issues the shared parent id; this widget
        // takes a dotted child id named "<group>_<option>".
        let dmi_id = match &self.group {
            Some(group_name) => {
                let parent_id =
                    id::allocate(response.id, DMIWidgetKind::Radio, Some(group_name), None);
                let child_name = format!("{group_name}_{}", sanitize_name(self.text));
                id::allocate_child(response.id, DMIWidgetKind::Radio, &child_name, parent_id)
            }
            None => id::allocate(response.id, DMIWidgetKind::Radio, self.dmi_name.as_deref(), None),
        };
        DMIUpdate::with_changed(dmi_id, response, clicked)
    }
}

// ============================================================================
// 下拉菜单 Select
// ============================================================================

/// 封装下拉菜单：绑定当前值与候选列表，选中即写回。
/// Select wrapper: binds current value and options, writes back on pick.
pub struct DMISelect<'a, Value: PartialEq + Clone + std::fmt::Display> {
    /// 菜单标识文本（兼作 id 盐）。 / Label acting as combo id salt.
    label: &'a str,
    /// 绑定当前值。 / Bound current value.
    current: &'a mut Value,
    /// 候选项列表。 / Candidate options list.
    options: &'a [Value],
    /// 是否可交互。 / Whether interactive.
    enabled: bool,
    /// DMI 注册名称（命名不同则额外分配编号）。
    /// Explicit DMI name; a different name allocates its own id.
    dmi_name: Option<String>,
    /// 子项单独编号开关。 / Issue dotted ids for each option.
    individual_ids: bool,
}

impl<'a, Value: PartialEq + Clone + std::fmt::Display> DMISelect<'a, Value> {
    /// 构建下拉菜单并绑定当前值与候选项。
    /// Build a select bound to current value and options.
    pub fn new(label: &'a str, current: &'a mut Value, options: &'a [Value]) -> Self {
        Self { label, current, options, enabled: true, dmi_name: None, individual_ids: false }
    }

    /// 设置是否可交互。 / Set interactivity.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 设置 DMI 注册名称。 / Set the DMI registry name.
    pub fn dmi_name(mut self, name: impl Into<String>) -> Self {
        self.dmi_name = Some(name.into());
        self
    }

    /// 开启子项"父号.序号"独立编号模式。
    /// Enable dotted child ids like parent.1, parent.2.
    pub fn individual_ids(mut self) -> Self {
        self.individual_ids = true;
        self
    }

    /// 渲染下拉菜单：弹出层逐项可选，选中写回并置 changed。
    /// Renders combo; picking an option writes back with changed set.
    #[must_use = "交互组件结果应被检查 / inspect the interactive widget result"]
    pub fn show(self, ui: &mut Ui) -> DMIUpdate {
        let tokens = DesignLanguages::from_ui(ui);
        let combo = ComboBox::from_id_salt(IdSalt::new(self.label.to_owned()))
            .width(tokens.spacing.combo_width)
            .selected_text(self.current.to_string());
        // 禁用态通过 add_enabled_ui 屏蔽整棵子 UI。
        // Disabled state blocks the whole child ui via add_enabled_ui.
        let mut picked: Option<Value> = None;
        let mut drawn_options: Vec<(String, Id)> = Vec::new();
        let outer = ui.add_enabled_ui(self.enabled, |ui| {
            combo.show_ui(ui, |ui| {
                for option in self.options {
                    let option_response =
                        ui.selectable_label(option == self.current, option.to_string());
                    drawn_options.push((option.to_string(), option_response.id));
                    if option_response.clicked() {
                        picked = Some(option.clone());
                    }
                }
                picked.clone()
            })
        });
        let inner = outer.inner;
        let dmi_id = id::allocate(inner.response.id, DMIWidgetKind::Select, self.dmi_name.as_deref(), None);
        // 独立编号模式：为已绘制选项登记 父号.序号 子条目。
        // Dotted mode: register parent.n records for drawn options.
        if self.individual_ids {
            let base_name = id::name_of(dmi_id).unwrap_or_default();
            for (option_text, option_egui_id) in drawn_options {
                let child_name = format!("{base_name}_{}", sanitize_name(&option_text));
                id::allocate_child(option_egui_id, DMIWidgetKind::Select, &child_name, dmi_id);
            }
        }
        let chosen = inner.inner.flatten();
        let changed = chosen.is_some();
        if let Some(value) = chosen {
            *self.current = value;
        }
        DMIUpdate::with_changed(dmi_id, inner.response, changed)
    }
}

// ============================================================================
// 滑块 Slider
// ============================================================================

/// 封装滑块：绑定数值与范围，支持标签与悬停提示。
/// Slider wrapper: binds value and range with optional label and tooltip.
pub struct DMISlider<'a, Value: Numeric> {
    /// 绑定数值。 / Bound numeric value.
    value: &'a mut Value,
    /// 取值范围。 / Value range bounds.
    range: RangeInclusive<Value>,
    /// 标签文本。 / Optional label text.
    label: Option<&'a str>,
    /// 悬停提示文本。 / Hover tooltip text.
    tooltip: Option<String>,
    /// DMI 注册名称（命名不同则额外分配编号）。
    /// Explicit DMI name; a different name allocates its own id.
    dmi_name: Option<String>,
}

impl<'a, Value: Numeric> DMISlider<'a, Value> {
    /// 构建滑块并绑定数值与范围。 / Build a slider bound to value and range.
    pub fn new(value: &'a mut Value, range: impl Into<RangeInclusive<Value>>) -> Self {
        Self { value, range: range.into(), label: None, tooltip: None, dmi_name: None }
    }

    /// 设置标签文本。 / Set the label text.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// 设置悬停提示。 / Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// 设置 DMI 注册名称。 / Set the DMI registry name.
    pub fn dmi_name(mut self, name: impl Into<String>) -> Self {
        self.dmi_name = Some(name.into());
        self
    }

    /// 渲染滑块。 / Renders the slider.
    #[must_use = "交互组件结果应被检查 / inspect the interactive widget result"]
    pub fn show(self, ui: &mut Ui) -> DMIUpdate {
        let mut slider = egui::Slider::new(self.value, self.range);
        if let Some(label) = self.label {
            slider = slider.text(label);
        }
        let response = ui.add(slider);
        let response = match &self.tooltip {
            Some(hover_text) => response.on_hover_text(hover_text.clone()),
            None => response,
        };
        let dmi_id = id::allocate(response.id, DMIWidgetKind::Slider, self.dmi_name.as_deref(), None);
        DMIUpdate::from_response(dmi_id, response)
    }
}

/// 清洗选项文本为名称片段（小写，符号转下划线）。
/// Sanitizes option text into a lowercase name slug.
fn sanitize_name(text: &str) -> String {
    text.chars()
        .map(|character| {
            let lowered = character.to_ascii_lowercase();
            if lowered.is_alphanumeric() || lowered == '_' || lowered == '-' {
                lowered
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证按钮构建与 DMIUpdate 封装在未交互时为 false。
    /// Verifies button build and DMIUpdate read false when untouched.
    #[test]
    fn button_update_defaults_idle() {
        let mut clicked = true;
        let mut changed = true;
        egui::__run_test_ui(|ui| {
            DesignLanguages::install(ui.ctx());
            let update = DMIButton::primary("确定").tooltip("提交").show(ui);
            clicked = update.clicked();
            changed = update.changed();
        });
        assert!(!clicked && !changed);
    }

    /// 多帧重复渲染不得使注册表增长：匿名标签免登记、命名组件复用。
    /// 300 frames must not grow the registry: free labels skip, names reuse.
    #[test]
    fn frames_do_not_grow_registry() {
        let mut buffer = String::new();
        let mut flag = false;
        let mut number = 0.5f32;
        let options = vec!["甲".to_owned(), "乙".to_owned()];
        let mut chosen = options[0].clone();
        for _frame in 0..300 {
            egui::__run_test_ui(|ui| {
                DesignLanguages::install(ui.ctx());
                let _ = DMIButton::primary("帧测").dmi_name("case_frame_button").show(ui);
                // 未命名显示行：复刻旧版注册表/日志面板的排版来源。
                // Unnamed display lines: the old registry/log row pattern.
                DMILabel::mono("(1, \"button\", 0, \"case_frame_button\")").show(ui);
                DMILabel::caption("快照说明").show(ui);
                let _ = DMITextInput::new(&mut buffer).dmi_name("case_frame_field").show(ui);
                let _ = DMICheckbox::new(&mut flag, "帧测").dmi_name("case_frame_check").show(ui);
                let _ = DMIRadio::new(true, "帧测").dmi_name("case_frame_radio").show(ui);
                let _ = DMISelect::new("帧测", &mut chosen, &options).dmi_name("case_frame_select").show(ui);
                let _ = DMISlider::new(&mut number, 0.0..=1.0).dmi_name("case_frame_slider").show(ui);
            });
        }
        let grown = id::list_id()
            .iter()
            .filter(|record| record.name.starts_with("case_frame_"))
            .count();
        assert_eq!(grown, 6, "300 帧只应新增本用例的 6 条命名控件（匿名标签不登记）");
    }

    /// 单选组重复渲染：1 父号 + 3 子号，不重号不增长。
    /// Radio group: one parent, three dotted children, stable across frames.
    #[test]
    fn radio_group_registers_parent_and_children() {
        for _frame in 0..3 {
            egui::__run_test_ui(|ui| {
                DesignLanguages::install(ui.ctx());
                for option in ["甲", "乙", "丙"] {
                    let _ = DMIRadio::new(false, option).group("case_group_radio").show(ui);
                }
            });
        }
        let records = id::list_id();
        let group_records: Vec<_> = records
            .iter()
            .filter(|record| record.name.starts_with("case_group_radio"))
            .collect();
        assert_eq!(group_records.len(), 4, "应为 1 个父号 + 3 个子号");
        let parent = group_records.iter().find(|record| record.id.minor == 0).unwrap();
        assert_eq!(parent.name, "case_group_radio");
        let minors: Vec<u32> = group_records
            .iter()
            .filter(|record| record.id.minor != 0)
            .map(|record| record.id.minor)
            .collect();
        assert_eq!(minors, vec![1, 2, 3], "子号按出现顺序 父号.1/.2/.3");
        for record in group_records.iter().filter(|record| record.id.minor != 0) {
            assert_eq!(record.parent, parent.id, "子号 parent 指向父号");
        }
    }

    /// 命名标签登记入表（kind=label），匿名标签保持免登记。
    /// Named labels register as kind label; anonymous ones stay untracked.
    #[test]
    fn named_label_registers_anonymous_does_not() {
        egui::__run_test_ui(|ui| {
            DesignLanguages::install(ui.ctx());
            let _ = DMILabel::heading("章节").dmi_name("case_named_section_label").show(ui);
            DMILabel::caption("匿名行").show(ui);
        });
        let records = id::list_id();
        let named = records
            .iter()
            .find(|record| record.name == "case_named_section_label");
        assert!(named.is_some(), "命名标签应入表");
        assert_eq!(named.unwrap().kind, DMIWidgetKind::Label);
        let anonymous = records
            .iter()
            .filter(|record| record.name.starts_with("label-"))
            .count();
        assert_eq!(anonymous, 0, "匿名标签不得登记");
    }

    /// 验证组件注册名称与编号写入注册表。
    /// Verifies explicit DMI names land in the registry with ids.
    #[test]
    fn named_button_records_dmi_id() {
        use crate::ui::id::list_id;
        let mut dmi_id = DMIId::SYSTEM;
        egui::__run_test_ui(|ui| {
            DesignLanguages::install(ui.ctx());
            let update = DMIButton::primary("注册").dmi_name("case_register_button").show(ui);
            dmi_id = update.id;
        });
        assert!(!dmi_id.is_system());
        assert!(list_id().iter().any(|record| {
            record.id == dmi_id
                && record.name == "case_register_button"
                && record.kind == DMIWidgetKind::Button
        }));
    }
}
