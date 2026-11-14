# 2026-11-14 任务E：eframe 组件画廊 demo（dmi_gallery）

- 依据：`build_docs/restricts.md` 第14条 + 用户指令"在demo里用eframe写一个测试用例，展示新功能"。
- 依赖：新增 `eframe 0.37`（仅 demo 使用，库本体保持只依赖 egui）。

## 新增/修改

| 文件 | 内容 |
| --- | --- |
| `Cargo.toml` | 追加 `[workspace] members = ["demo/dmi_gallery"]`。 |
| `demo/dmi_gallery/Cargo.toml` | 新包 dmi-gallery：eframe 0.37 + path 依赖 egui-dmi。 |
| `demo/dmi_gallery/src/main.rs` | 画廊应用（约280行）：`DesignLanguages::install` 装配设计语言；七个组件全部实测（按钮四类+禁用+tooltip、用户名/密码输入框、复选框、画质单选组、外观/模式下拉（外观开启 `.individual_ids()`）、浮点/整数滑块）；右侧栏 `list_id()` 实时编号注册表；底部栏交互日志（带 `[DMI id]` 前缀）；"规则演示"分区现场展示同一 egui id + `text_color_plate`/`background_color_plate` 两个命名获得两个编号（对齐 dmi.md 示例）；外观下拉驱动深浅主题切换。 |
| `src/ui/components.rs` | 由 demo 反馈修正：`#[must_use]` 从 DMIUpdate 结构体移到六个交互组件的 `show()` 上——纯展示型 `DMILabel` 不再强制检查返回值。 |

## 验证

- `cargo test --workspace`：5/5 通过。
- `cargo clippy --workspace --all-targets`：0 警告。
- 冒烟运行：`./target/debug/dmi-gallery` 窗口启动 5 秒稳定存活、无错误输出后关闭。
- 运行方式：`cargo run -p dmi-gallery`。

## 适配记录（egui/eframe 0.37 API 变化）

- `SidePanel/TopBottomPanel` 合并为 `egui::Panel::left/right/top/bottom`，`default_width→default_size`；
- eframe `App::ui` 新签名 `(ui: &mut egui::Ui, frame: &mut eframe::Frame)`；
- 测试用确定性 id 弃用 `Id::new`，改 `Id::NULL.with(salt)`。
