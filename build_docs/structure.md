# 项目结构

## 摘要

- 本文档用树形图说明`egui-dmi`项目组成以及各文件功能。
- 结构细则见`build_docs/plan/`下各功能文档；开发规范见`build_docs/restricts.md`。

## 结构图

```text
egui-dmi/
├── Cargo.toml                     # 包清单：库名 egui_dmi，依赖 egui 0.37。
├── src/
│   ├── lib.rs                     # 项目简介与模块挂载入口。
│   ├── ui.rs                      # UI 模块根：设计语言层与组件层的统一出口。
│   ├── ui/
│   │   ├── design.rs              # 设计语言模块根：汇总令牌子模块并再导出。
│   │   └── design/
│   │       ├── languages.rs       # DesignLanguages 聚合全部令牌；apply/install 写入 egui 全局样式（深浅两套）。
│   │       ├── colors.rs          # ColorLanguages 调色板（背景/表面/文本/分隔/强调/危险/气泡）与状态混色规则。
│   │       ├── typography.rs      # TypographyTokens 字号档位；build_font_definitions 装载系统中文字体回退。
│   │       ├── spacing.rs         # SpacingTokens 间隙/内边距/圆角/控件尺寸档位。
│   │       ├── motion.rs          # MotionTokens 状态动画时长与渐变辅助（动画期自动请求重绘）。
│   │       └── accessibility.rs   # AccessibilityTokens 触摸目标/焦点环参数；DMIComponentRole 可访问名称组装。
│   └── ui/
│       ├── components.rs          # （任务B）组件层模块根：DMIKind/DMISize/DMIUpdate 共用定义。
│       └── components/            # （任务B）DMIButton 等七个封装组件，一组件一文件。
├── demo/
│   └── dmi_gallery/               # （任务E）eframe 组件画廊示例程序。
├── docs/
│   └── design-system.md           # （任务E）设计语言与组件视觉规范文档。
└── build_docs/
    ├── restricts.md               # 开发规范（命名/注释/任务流程等约束）。
    ├── structure.md               # 本文件：项目结构与各文件功能说明。
    ├── plan/                      # 各功能实现说明文档（ui_sort.md 等）。
    └── changelog/                 # 人类可读改动日志，每任务一文件。
```

## 备注

- `src/dmi/` 目录当前为空，预留给后续 DMI 声明接口任务。
- 标注（任务B/E）的路径为已批准计划的占位说明，文件在对应任务完成时才会出现。
