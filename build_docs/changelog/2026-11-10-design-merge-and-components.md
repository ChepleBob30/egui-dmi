# 2026-11-10 UI整理任务B：设计层合并 + 七组件构建方法

- 依据：用户新原则"测试项目一切从简" + 三点明确指令。
- 性质：改动全部为新增/合并，历史版本可经 git 回退。

## 1. 设计层合并为单文件

- 删除 `src/ui/design/` 整个子目录（6 个令牌子文件），内容合并进 `src/ui/design.rs`。
- 按要求移除：
  - `motion.rs` 全部内容（动画令牌与 animate 辅助）——按钮四态改为即时着色；
  - `typography.rs` 全部内容（字号令牌与中文字体装载）——标签字号回归 egui 默认文本样式；
  - `accessibility.rs` 全部内容（触摸目标/焦点环/角色命名）。
- `SpacingTokens`：删除全部 `gap_*` 字段（xs/sm/md/lg/xl），保留内边距/圆角/尺寸/宽度字段。
- `ColorLanguages`：完整保留（10 字段调色板 + for_theme + 4 个混色方法）。
- `DesignLanguages`：字段收敛为 theme/colors/spacing；删除未被使用的 impl 方法，
  保留链路 install→apply→build_visuals→build_widgets→state_visuals 与 from_ui/dark/light/for_theme。
- `DesignLanguages::apply` 不再写文本样式/动画时长；`install` 不再装载字体。

## 2. 七个基础组件构建方法（`src/ui/components.rs`，单文件）

| 组件 | 构建方法 | 说明 |
| --- | --- | --- |
| `DMIUpdate` | `from_response/clicked/changed` | 统一返回载体；带 `DMI标记` 注释，声明未来以 DMI 数据通道取代 egui::Response。 |
| `DMIButton` | `new/primary/destructive/text` + `enabled/tooltip` | 自绘：测量文字→分配区域→按 禁用>按下>悬停>常态 着色→画框与居中文字；四类 DMIButtonKind。 |
| `DMILabel` | `body/heading/caption/mono` + `tooltip` | 字号走 egui 文本样式档位，颜色取调色板。 |
| `DMITextInput` | `new(&mut String)` + `placeholder/password/enabled` | TextEdit 委托；框体底色/描边/圆角来自令牌。 |
| `DMICheckbox` | `new(&mut bool, text)` + `enabled/tooltip` | CheckBox 委托。 |
| `DMIRadio` | `new(selected, text)` + `enabled` | RadioButton 委托；changed=本次点选。 |
| `DMISelect` | `new(label, &mut Value, &[Value])` + `enabled` | ComboBox 委托，泛型约束 PartialEq+Clone+Display，选中写回。 |
| `DMISlider` | `new(&mut Value, range)` + `label/tooltip` | Slider 委托，泛型 Numeric。 |

- `src/ui.rs`：挂载 `pub mod components;` 并再导出全部组件类型。

## 3. 验证

- `cargo check` / `cargo clippy`：零警告。
- `cargo test`：2 项通过 ——
  `design::install_writes_both_themes`（两主题底色/强调色/间距一致）、
  `components::button_update_defaults_idle`（按钮渲染与 DMIUpdate 空闲态为 false）。

## 4. 回退余地

- 被删除的 6 个令牌子文件内容大多保留在合并文件中；
  motion/typography/accessibility 令牌如需恢复，从本仓库任意先前状态恢复文件即可。
