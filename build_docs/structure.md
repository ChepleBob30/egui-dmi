# 项目结构

## 摘要

- 本文档用树形图说明`egui-dmi`项目组成以及各文件功能。
- 本项目是面向大模型工作的测试项目，结构从简：每个功能层一个文件，一切为测试服务。
- 开发规范见`build_docs/restricts.md`；任务细则见`build_docs/plan/`。

## 结构图

```text
egui-dmi/
├── Cargo.toml                     # 包清单：库名 egui_dmi，edition 2024，依赖 egui 0.37。
├── src/
│   ├── lib.rs                     # 项目简介与模块挂载入口。
│   ├── ui.rs                      # UI 模块根：导出 design 与 components 两层公共 API。
│   ├── ui/
│   │   ├── design.rs              # 设计语言（单文件）：
│   │   │                          #   ColorLanguages 深/浅调色板 + hover/press/disabled/选中混色规则；
│   │   │                          #   SpacingTokens 内边距/圆角/控件尺寸令牌；
│   │   │                          #   DesignLanguages 聚合 + apply/install 写入 egui 全局样式（深浅两套）。
│   │   └── components.rs          # DMI 组件（单文件）：
│   │                              #   DMIUpdate 统一返回（DMI 取代 Response 的占位标记）；
│   │                              #   DMIButton(primary/destructive/text…)自绘四态着色；
│   │                              #   DMILabel / DMITextInput / DMICheckbox / DMIRadio / DMISelect / DMISlider。
│   └── dmi/                       # （空）预留给 DMI 声明接口任务。
├── demo/                          # （待建）组件画廊示例程序。
├── docs/
│   └── design-system.md           # 设计语言与组件视觉规范说明。
└── build_docs/
    ├── restricts.md               # 开发规范（命名/注释/任务流程等约束）。
    ├── structure.md               # 本文件：项目结构与各文件功能说明。
    ├── plan/                      # 各功能实现说明文档（ui_sort.md 等）。
    └── changelog/                 # 人类可读改动日志，每任务一文件。
```

## 变更记录说明

- 2026-11-10：`src/ui/design/` 六个令牌子文件合并为单文件 `design.rs`；
  移除 motion/typography/accessibility 令牌，`SpacingTokens` 删除全部 `gap_*` 字段；
  同日新增 `components.rs`，七个基础组件构建方法一次性就位。
