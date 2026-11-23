# 设计系统与组件规范（egui-dmi）

## 定位

本项目是面向大模型工作的测试项目：设计语言只保留**颜色**与**间距**两组令牌，
作为 DMI（声明式模型接口）可管理的修改范围，其余动画/字体/可访问性参数
全部回归 egui 默认，避免测试面过宽。

## 设计语言入口

```rust
// 应用启动时装配一次（深浅两套样式同时写入 egui 全局 Style）。
egui_dmi::ui::DesignLanguages::install(&ctx);
```

## 颜色令牌 ColorLanguages

| 字段 | 深色 DARK | 浅色 LIGHT |
| --- | --- | --- |
| background | #17181C | #F4F5F8 |
| surface | #22242A | #FAFAFC |
| surface_elevated | #2B2E36 | #FFFFFF |
| text_primary | #E8EAF0 | #1D2230 |
| text_secondary | #9BA1AE | #5F6673 |
| separator | #3A3E48 | #D9DCE3 |
| accent | #5B8CFF | #2F6FE4 |
| destructive | #E5484D | #D2382E |
| own_message（气泡预留） | #2F5FD0 | #D6E6FF |
| other_message（气泡预留） | #313540 | #EFF0F3 |

派生规则（gamma 空间混色）：

- `hover_of(base)`：base 向 accent 混 12%；
- `pressed_of(base)`：base 向 accent 混 24%；
- `disabled_of(base)`：base 向 surface 混 55%；
- `selection_fill()`：accent 向 surface 混 55%。

## 间距令牌 SpacingTokens

| 字段 | 值 | 用途 |
| --- | --- | --- |
| pad_horizontal / pad_vertical | 12 / 6 | 按钮内边距、输入框 margin |
| radius_sm / md / lg | 6 / 8 / 12 | 控件 / 弹层 / 窗口圆角 |
| control_height / control_height_sm | 32 / 26 | 常规 / 紧凑控件目标高度 |
| field_width / combo_width / slider_width | 240 / 160 / 200 | 控件默认宽度 |

## 组件

统一模式：**构建方法 → 链式修饰（含 `dmi_name` 登记名）→ `show(ui) -> DMIUpdate`**。
声明了名称的组件才注册编号，未声明即 `DMIUpdate.id == SYSTEM`（可判定"未注册"）；
功能映射（`UIEntry` 触及方式组成的 `function`）由 id 存储机制承载，
组件层不持有该字段：

```rust
DMIButton::primary("保存设置").dmi_name("save_button").show(ui);
```

```rust
DMIButton::primary("hello world").show(ui);
DMIButton::destructive("remove").enabled(can_delete).tooltip("删除此项").show(ui);
DMILabel::heading("标题").show(ui);
DMITextInput::new(&mut text).placeholder("搜索…").show(ui);
DMICheckbox::new(&mut checked, "启用加速").show(ui);
DMIRadio::new(is_selected, "选项 A").show(ui);
DMISelect::new("模式", &mut current, &options).show(ui);
DMISelect::grouped("预设", &mut current, PRESET_GROUPS).show(ui);  // 多层编号
DMISlider::new(&mut volume, 0.0..=100.0).show(ui);
```

- `DMIButton` 自绘四态（禁用 > 按下 > 悬停 > 常态），其余组件委托 egui 原生控件、
  由安装后的全局样式统一着色。
- 注册表条目为五元组 `(id, 类型, (上级id, 触及方式), 名称, [功能映射])`，
  `list_id()` 快照直接可读（详见 `docs/id-system.md`）；
- `DMIUpdate` 是纯数据记录 `{ id, clicked, changed }`（`Copy + PartialEq`）：
  `egui::Response` 已在组件出口处消化完毕、不再外泄；`id` 即组件在注册表中的编号
  （如 `3`、`5.2`、`6.1.2`，0 号系统保留），也是未来 JSON 调用的 `target_id`。
- 编号复用规则：键 =（egui id, DMI 名称）。同一 egui id 下**命名不同即额外分配新编号**；
  名称相同则跨帧复用。组件可用 `.dmi_name("theme_selector")` 显式命名（便于 DMI 分配 id），
  `DMISelect` 还可用 `.individual_ids()` 让每个选项获得 `父名_选项名` 的 `父号.序号` 子条目；
  `DMISelect::grouped` 分组下拉自动启用多层编号：组登记 `父号.n`，组内选项登记 `父号.n.m`。
  查询接口：`ui::id::list_id()` 输出全部 (id, 类型, 上级id, 名称) 条目。

## DMI 统一调用入口

```rust
// 硬编码调用：完整 JSON 命令进，JSON 应答出（src/dmi/call.rs::call_dmi）。
let response: String = egui_dmi::dmi::call_dmi(
    "{\"request_type\":\"OBSERVATION\",\"interface\":\"greet\",\"target_id\":0,\"complexity\":\"standard\"}",
);
```

- 命令信封四字段：`request_type`（ACCESS/STATE/OBSERVATION 大写）、`interface`、
  `target_id`（数字一级小数，多级用点分字符串）、`complexity`（simple/standard/complex 小写）；
- 失败应答分层：simple 仅 `result`；standard 加 `reason` 与 `error_code`；
  complex 再加 `suggestion`；解析失败的命令统一按 standard 兜底；
- 示例接口 `greet`（OBSERVATION、0 号）三档分别返回欢迎语、欢迎语+项目名、
  欢迎语+项目名+依赖；demo 画廊"DMI 自检"分区即该入口的界面化演示。

运行时调用（dmi.md §2.2.2）：`DMIConsole::spawn(&ctx)` 启动独立线程读终端
JSON 行，UI 帧首 `pump(ui)` 当帧执行并回执；串行在途、超时 10 秒、
stdout 纯 JSON 行无提示符；输入输出只围绕终端，GUI 不出现控制台形态。

## DMI 修改范围

大模型/DMI 层允许触碰的全部参数 = `ColorLanguages` 字段 + `SpacingTokens` 字段
+ 各组件构建方法参数；其余 egui 样式不在框架管理范围内。
