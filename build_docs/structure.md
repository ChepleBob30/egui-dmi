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
│   │   ├── id.rs                  # 组件ID注册表：DMIId 两级编号（0号系统保留）、(egui id,名称)复用映射、
│   │   │                          #   DMIWidgetKind 类型枚举、allocate/allocate_child 发号、list_id() 查询。
│   │   ├── design.rs              # 设计语言（单文件）：
│   │   │                          #   ColorLanguages 深/浅调色板 + hover/press/disabled/选中混色规则；
│   │   │                          #   SpacingTokens 内边距/圆角/控件尺寸令牌；
│   │   │                          #   DesignLanguages 聚合 + apply/install 写入 egui 全局样式（深浅两套）。
│   │   └── components.rs          # DMI 组件（单文件）：每组件带 dmi_name 注册；DMIUpdate 携带 id；
│   │                              #   DMISelect 支持 individual_ids 小数子号；
│   │                              #   DMIUpdate 统一返回（DMI 取代 Response 的占位标记）；
│   │                              #   DMIButton(primary/destructive/text…)自绘四态着色；
│   │                              #   DMILabel / DMITextInput / DMICheckbox / DMIRadio / DMISelect / DMISlider。
│   └── dmi/                       # （空）预留给 DMI 声明接口任务。
├── demo/
│   └── dmi_gallery/               # eframe 组件画廊示例（workspace 成员，cargo run -p dmi-gallery）。
│       ├── Cargo.toml             # 依赖：eframe 0.37 + path 依赖 egui-dmi。
│       └── src/main.rs            # GalleryApp：七组件实测 + list_id() 实时注册表 + 交互日志 + 主题切换。
├── docs/
│   ├── design-system.md           # 设计语言与组件视觉规范说明。
│   └── id-system.md               # DMI 组件ID分配系统：运作流程、reuse 机制逐方法设计理由、边界与演进。
└── build_docs/
    ├── restricts.md               # 开发规范（命名/注释/任务流程等约束）。
    ├── structure.md               # 本文件：项目结构与各文件功能说明。
    ├── plan/                      # 各功能实现说明文档（ui_sort.md 等）。
    └── changelog/                 # 人类可读改动日志，每任务一文件。
```

## 变更记录说明

- 2026-11-14：`DMIRadio` 新增 `.group()` 统一父号+小数子号（补 dmi.md 单选框编号规则）；
  demo 章节标题命名入表；详见 changelog/2026-11-14-demo-three-findings.md。
- 2026-11-14：新增 `docs/id-system.md`（ID 分配系统与 reuse 机制说明文档）。
- 2026-11-14：性能修复——匿名标签不登记切断注册表自增长循环；id 复用键命名化防漂移重号；
  注册表加版本号+Arc 快照，demo 面板按版本门控重建（详见 changelog/2026-11-14-performance-fix.md）。
- 2026-11-14：新增 `demo/dmi_gallery`（eframe 画廊，根 Cargo.toml 加 workspace）；
  组件 `#[must_use]` 由 DMIUpdate 结构体移至六个交互组件 show()。
- 2026-11-14：新增 `src/ui/id.rs`（DMI 任务1.2：组件ID注册表），`DMIUpdate` 加 `id` 字段，
  组件接入 `.dmi_name()/.individual_ids()`；复用键=(egui id,名称)，命名不同即另发新号。
- 2026-11-10：`src/ui/design/` 六个令牌子文件合并为单文件 `design.rs`；
  移除 motion/typography/accessibility 令牌，`SpacingTokens` 删除全部 `gap_*` 字段；
  同日新增 `components.rs`，七个基础组件构建方法一次性就位。
