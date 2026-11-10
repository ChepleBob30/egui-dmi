# 2026-11-09 UI整理任务A：设计语言层

- 依据：`build_docs/plan/ui_sort.md`（经用户批准的5点施工补充）。
- 目标：集中定义颜色、字体、间距、动画、可访问性令牌，作为 DMI 统一管理范围，
  并提供一键写入 egui 全局样式的入口。

## 新增文件

| 文件 | 功能 |
| --- | --- |
| `src/ui.rs` | UI 模块根，挂载 design 子模块（组件层在任务B加入）。 |
| `src/ui/design.rs` | 设计语言模块根，汇总并再导出各令牌子模块。 |
| `src/ui/design/colors.rs` | `ColorLanguages` 调色板（plan 约定10字段）+ 深/浅两套 + hover/press/disabled/选中混色规则。 |
| `src/ui/design/typography.rs` | `TypographyTokens` 字号档位映射 egui 五档文本样式；`build_font_definitions` 探测 macOS/Windows/Linux 系统中文字体追加为回退，失败静默降级。 |
| `src/ui/design/spacing.rs` | `SpacingTokens` 间隙/内边距/圆角/控件尺寸档位，`apply` 写入 `style.spacing`。 |
| `src/ui/design/motion.rs` | `MotionTokens` 悬停/按下/焦点/切换动画时长；`animate` 基于 `Context::animate_bool_with_time` 并在动画期间请求重绘。 |
| `src/ui/design/accessibility.rs` | `AccessibilityTokens` 44点最小触摸目标与焦点环参数；`DMIComponentRole` 角色枚举提供中英可访问名称组装。 |
| `src/ui/design/languages.rs` | `DesignLanguages` 聚合全部令牌；`build_visuals`/`apply` 把令牌翻译为 egui `Visuals`（含五档控件状态色、选中色、圆角、禁用透明度）；`install(ctx)` 一次装配深浅两套样式+字体。 |

## 修改文件

- `src/lib.rs`：挂载 `pub mod ui;`。

## 验证

- `cargo check` 通过；
- `cargo test` 通过：`install_writes_both_themes`（深浅两主题的 panel 底色/强调色/间距均与设计语言一致）。

## 约定

- 所有标识符 ≤30 字符，无缩写；每方法/功能块双语注释 ≤40 中文字。
- `own_message`/`other_message` 为聊天气泡组件预留，本阶段仅入调色板不进 egui 样式映射。
