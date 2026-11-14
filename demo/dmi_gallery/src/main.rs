//! DMI 组件画廊：用 eframe 展示七组件、设计语言与编号注册表。
//! DMI gallery: eframe demo of the seven components and the id registry.

use eframe::egui;
use egui_dmi::ui::id::{self, DMIWidgetKind};
use egui_dmi::ui::{
    DesignLanguages, DMICheckbox, DMIButton, DMITextInput, DMILabel, DMIRadio, DMISelect, DMISlider,
};

/// 画廊应用：持有各组件绑定的数据与交互日志。
/// Gallery app: bound widget data plus an interaction log.
struct GalleryApp {
    /// 用户名输入绑定。 / Bound username text.
    username: String,
    /// 密码输入绑定。 / Bound password text.
    password: String,
    /// 通知开关绑定。 / Notification checkbox state.
    notifications_on: bool,
    /// 画质单选组当前值。 / Current quality radio value.
    quality_mode: String,
    /// 画质候选项。 / Quality radio options.
    quality_options: Vec<String>,
    /// 外观下拉当前值（驱动主题）。 / Appearance select value driving theme.
    appearance_name: String,
    /// 外观候选项。 / Appearance options.
    appearance_options: Vec<String>,
    /// 运行模式下拉当前值。 / Runtime mode select value.
    operation_name: String,
    /// 运行模式候选项。 / Runtime mode options.
    operation_options: Vec<String>,
    /// 速度滑块绑定。 / Speed slider value.
    speed_value: f32,
    /// 音量滑块绑定。 / Volume slider value.
    volume_value: i32,
    /// 交互日志（最新在前）。 / Interaction log, newest first.
    interaction_log: Vec<String>,
    /// 注册表行文本缓存，仅版本变化时重建。
    /// Cached registry rows, rebuilt only on revision change.
    registry_cache: Vec<String>,
    /// 缓存对应的注册表版本号。 / Registry revision the cache was built at.
    registry_revision: u64,
}

impl GalleryApp {
    /// 从创建上下文初始化：装配设计语言并定默认值。
    /// Initializes from context: installs design language, sets defaults.
    fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        DesignLanguages::install(&creation_context.egui_ctx);
        Self {
            username: String::new(),
            password: String::new(),
            notifications_on: true,
            quality_mode: "中画质".to_owned(),
            quality_options: ["低画质", "中画质", "高画质"].map(str::to_owned).to_vec(),
            appearance_name: "深色".to_owned(),
            appearance_options: ["浅色", "深色"].map(str::to_owned).to_vec(),
            operation_name: "标准".to_owned(),
            operation_options: ["标准", "高性能", "节能"].map(str::to_owned).to_vec(),
            speed_value: 1.0,
            volume_value: 60,
            interaction_log: Vec::new(),
            registry_cache: Vec::new(),
            registry_revision: 0,
        }
    }

    /// 记录一条组件交互（带 DMI 编号）。 / Logs one widget interaction with its DMI id.
    fn push_log(&mut self, note: String) {
        self.interaction_log.insert(0, note);
        self.interaction_log.truncate(12);
    }

    /// 按钮区：三类样式 + 禁用态 + 悬停提示。
    /// Buttons section: kinds, disabled state, tooltip.
    fn show_buttons_section(&mut self, ui: &mut egui::Ui) {
        DMILabel::heading("按钮").dmi_name("section_buttons_label").show(ui);
        ui.horizontal(|ui| {
            let save_update = DMIButton::primary("保存设置")
                .dmi_name("save_button")
                .tooltip("写入当前配置")
                .show(ui);
            if save_update.clicked() {
                self.push_log(format!("[{}] 保存设置被点击", save_update.id));
            }
            let delete_update = DMIButton::destructive("删除账号")
                .dmi_name("delete_button")
                .enabled(!self.username.is_empty())
                .tooltip("仅用户名非空时可删")
                .show(ui);
            if delete_update.clicked() {
                self.push_log(format!("[{}] 删除账号被点击", delete_update.id));
            }
            let learn_update = DMIButton::text("了解更多").dmi_name("learn_button").show(ui);
            if learn_update.clicked() {
                self.push_log(format!("[{}] 了解更多被点击", learn_update.id));
            }
        });
    }

    /// 输入区：用户名与密码框、复选框。
    /// Fields section: username, password, checkbox.
    fn show_fields_section(&mut self, ui: &mut egui::Ui) {
        DMILabel::heading("文本输入与复选框").dmi_name("section_fields_label").show(ui);
        ui.horizontal(|ui| {
            ui.label("用户名");
            let username_update = DMITextInput::new(&mut self.username)
                .dmi_name("username_field")
                .placeholder("输入用户名…")
                .show(ui);
            if username_update.changed() {
                self.push_log(format!("[{}] 用户名长度 {}", username_update.id, self.username.len()));
            }
        });
        ui.horizontal(|ui| {
            ui.label("密码　");
            let password_update = DMITextInput::new(&mut self.password)
                .dmi_name("password_field")
                .password(true)
                .placeholder("输入密码…")
                .show(ui);
            if password_update.changed() {
                // 日志只记长度，不记录密码内容。
                // Log length only, never the password itself.
                self.push_log(format!("[{}] 密码长度 {}", password_update.id, self.password.len()));
            }
        });
        let notify_update = DMICheckbox::new(&mut self.notifications_on, "开启通知")
            .dmi_name("notify_checkbox")
            .show(ui);
        if notify_update.changed() {
            self.push_log(format!("[{}] 通知开关 → {}", notify_update.id, self.notifications_on));
        }
    }

    /// 选择区：单选组 + 两个下拉（外观下拉开启子项编号）。
    /// Choices section: radio group and two selects (appearance uses child ids).
    fn show_choices_section(&mut self, ui: &mut egui::Ui) {
        DMILabel::heading("单选框与下拉菜单").dmi_name("section_choices_label").show(ui);
        DMILabel::caption(
            "编号对照：theme_selector 开启子项编号（展开后出现 父号.n）；operation_mode 整组统一父号，无子项。",
        )
        .show(ui);
        DMILabel::caption(
            "标签登记规则：章节标题带 dmi_name 已入表；本行为匿名标签，故意不登记（性能设计）。",
        )
        .show(ui);
        ui.horizontal(|ui| {
            ui.label("画质　");
            for option in self.quality_options.clone() {
                // 组模式：quality_radio 首现得统一父号，各选项领 父号.序号。
                // Group mode: shared parent id plus dotted child ids.
                let radio_update = DMIRadio::new(self.quality_mode == option, &option)
                    .group("quality_radio")
                    .show(ui);
                if radio_update.clicked() {
                    self.quality_mode = option.clone();
                    self.push_log(format!("[{}] 画质切换 → {option}", radio_update.id));
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("外观　");
            // 子项独立编号：选项会登记 父号.1/父号.2 条目。
            // Dotted child ids: options register parent.1/parent.2 records.
            let appearance_update = DMISelect::new(
                "theme_selector",
                &mut self.appearance_name,
                &self.appearance_options,
            )
            .dmi_name("theme_selector")
            .individual_ids()
            .show(ui);
            if appearance_update.changed() {
                self.push_log(format!("[{}] 外观切换 → {}", appearance_update.id, self.appearance_name));
                // 仅在选中变化时切换主题，避免每帧写入选项。
                // Switch theme only on change, not every frame.
                let chosen_theme = if self.appearance_name == "浅色" {
                    egui::Theme::Light
                } else {
                    egui::Theme::Dark
                };
                ui.ctx().set_theme(chosen_theme);
            }
            ui.label("模式　");
            let operation_update = DMISelect::new(
                "operation_mode",
                &mut self.operation_name,
                &self.operation_options,
            )
            .dmi_name("operation_mode")
            .show(ui);
            if operation_update.changed() {
                self.push_log(format!("[{}] 模式切换 → {}", operation_update.id, self.operation_name));
            }
        });
    }

    /// 滑块区：浮点与整数各一个。 / Sliders section: one float and one integer.
    fn show_sliders_section(&mut self, ui: &mut egui::Ui) {
        DMILabel::heading("滑块").dmi_name("section_sliders_label").show(ui);
        ui.horizontal(|ui| {
            let speed_update = DMISlider::new(&mut self.speed_value, 0.0..=2.0)
                .label("速度")
                .dmi_name("speed_slider")
                .show(ui);
            if speed_update.changed() {
                self.push_log(format!("[{}] 速度 → {:.2}", speed_update.id, self.speed_value));
            }
        });
        ui.horizontal(|ui| {
            let volume_update = DMISlider::new(&mut self.volume_value, 0..=100)
                .label("音量")
                .dmi_name("volume_slider")
                .show(ui);
            if volume_update.changed() {
                self.push_log(format!("[{}] 音量 → {}", volume_update.id, self.volume_value));
            }
        });
    }

    /// 规则演示：同一 egui id 按不同命名分配两个编号。
    /// Rule demo: one egui id with two names gets two DMI ids.
    fn show_palette_rule_section(&mut self, ui: &mut egui::Ui) {
        DMILabel::heading("规则演示：命名不同 ⇒ 额外分配编号")
            .dmi_name("section_rules_label")
            .show(ui);
        DMILabel::caption("同一块调色板同时用于文字颜色与背景颜色（对齐 dmi.md 示例）。")
            .show(ui);
        let shared_egui_id = egui::Id::NULL.with("shared_color_palette");
        let text_plate_id =
            id::allocate(shared_egui_id, DMIWidgetKind::Button, Some("text_color_plate"), None);
        let background_plate_id = id::allocate(
            shared_egui_id,
            DMIWidgetKind::Button,
            Some("background_color_plate"),
            None,
        );
        DMILabel::mono(&format!("text_color_plate ⇒ id {text_plate_id}")).show(ui);
        DMILabel::mono(&format!("background_color_plate ⇒ id {background_plate_id}")).show(ui);
    }

    /// 右侧栏：实时编号注册表快照。 / Side panel: live id registry snapshot.
    fn show_registry_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("registry_panel")
            .default_size(360.0)
            .show(ui, |ui| {
                DMILabel::heading("DMI 编号注册表").show(ui);
                DMILabel::caption("list_id() 快照，仅注册表版本变化时重建。").show(ui);
                // 版本门控：无新条目时零拷贝复用缓存行。
                // Revision gate: reuse cached rows unless the table grew.
                let revision = id::registry_revision();
                if revision != self.registry_revision {
                    self.registry_revision = revision;
                    self.registry_cache = id::list_id().iter().map(ToString::to_string).collect();
                }
                let rows = std::mem::take(&mut self.registry_cache);
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    for line in &rows {
                        DMILabel::mono(line).show(ui);
                    }
                });
                self.registry_cache = rows;
            });
    }

    /// 底部栏：最近交互日志（最新一条在最上）。
    /// Bottom panel: recent interaction log, newest on top.
    fn show_log_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("log_panel").show(ui, |ui| {
            DMILabel::caption("最近交互日志").show(ui);
            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                if self.interaction_log.is_empty() {
                    DMILabel::mono("（暂无交互，操作左侧组件后这里会追加编号记录）").show(ui);
                }
                for line in self.interaction_log.iter() {
                    DMILabel::mono(line).show(ui);
                }
            });
        });
    }
}

impl eframe::App for GalleryApp {
    /// 每帧渲染：面板布局 + 组件分区 + 主题同步。
    /// Renders each frame: panels, sections, theme sync.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("header_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                DMILabel::heading("DMI 组件画廊 · egui-dmi 测试用例").show(ui);
                let theme_text = format!("当前{}主题", self.appearance_name);
                DMILabel::caption(&theme_text).show(ui);
            });
        });
        self.show_log_panel(ui);
        self.show_registry_panel(ui);
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.show_buttons_section(ui);
                ui.separator();
                self.show_fields_section(ui);
                ui.separator();
                self.show_choices_section(ui);
                ui.separator();
                self.show_sliders_section(ui);
                ui.separator();
                self.show_palette_rule_section(ui);
            });
        });
    }
}

/// 程序入口：创建窗口并启动画廊。
/// Entry point: builds the window and runs the gallery.
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 660.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DMI 组件画廊",
        options,
        Box::new(|creation_context| Ok(Box::new(GalleryApp::new(creation_context)))),
    )
}
