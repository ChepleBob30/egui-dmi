# 项目结构

## 摘要

- 本文档用树形图说明`egui-dmi`项目组成以及各文件功能。
- 本项目是面向大模型工作的测试项目，结构从简：每个功能层一个文件，一切为测试服务。
- 开发规范见`build_docs/restricts.md`；任务细则见`build_docs/plan/`。

## 结构图

```text
egui-dmi/
├── Cargo.toml                     # 包清单：库名 egui_dmi，edition 2024，workspace 含 demo；依赖 egui 0.37、serde_json。
├── src/
│   ├── lib.rs                     # 项目简介与模块挂载入口。
│   ├── ui.rs                      # UI 模块根：UIEntry 触及方式枚举 + design/components/id 公共 API 出口。
│   ├── ui/
│   │   ├── id.rs                  # 组件ID注册表：DMIId 主号+小数路径（4层上限）、名称唯一键、function 仅此层承载、
│   │   │                          #   DMIRecord 五元组（parent 元组 + function 功能映射）、list_id() Arc 快照。
│   │   ├── design.rs              # 设计语言（单文件）：
│   │   │                          #   ColorLanguages 深/浅调色板 + hover/press/disabled/选中混色规则；
│   │   │                          #   SpacingTokens 内边距/圆角/控件尺寸令牌；
│   │   │                          #   DesignLanguages 聚合 + apply/install 写入 egui 全局样式（深浅两套）。
│   │   └── components.rs          # DMI 组件（单文件）：七组件以 dmi_name 过登记门（无名不注册），不持有 function；
│   │                              #   DMIUpdate 为纯数据 {id, clicked, changed}（Response 不出 show）；
│   │                              #   DMIButton(primary/destructive/text…)自绘四态着色；
│   │                              #   DMISelect 扁平 individual_ids 子号 / grouped 分组三层号；
│   │                              #   DMIRadio group() 统一父号+小数子号；
│   │                              #   DMILabel / DMITextInput / DMICheckbox / DMISlider。
│   ├── dmi.rs                   # DMI 模块根：声明 call/interface 两区并再导出公开契约。
│   └── dmi/
│       ├── call.rs              # 接口总调用区：DMICommand/DMIRequestType/DMIComplexity/DMIFailure 契约，
│       │                        #   parse_command 解析、ok_response/fail_response 分层应答、call_dmi 分发器。
│       └── interface.rs         # 各接口具体实现区：greet 示例接口（OBSERVATION 三档应答）。
├── demo/
│   └── dmi_gallery/               # eframe 组件画廊示例（workspace 成员，cargo run -p dmi-gallery）。
│       ├── Cargo.toml             # 依赖：eframe 0.37 + path 依赖 egui-dmi。
│       └── src/main.rs            # GalleryApp：七组件实测 + list_id() 注册表面板 + 交互日志 + 主题切换 + DMI 自检分区。
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

- 2026-11-22：UI 触及方法改进落地——`UIEntry` 枚举、`DMIRecord` 五元组（parent 元组 +
  function 功能映射）、严格声明制（匿名机制删除，缺一不注册）、demo 全量补齐声明。
- 2026-11-22：DMI Phase 2 落地——`call_dmi` 统一调用入口 + `greet` 示例接口（硬编码调用）；
  `DMIUpdate` 纯数据化去 Response；demo 新增"DMI 自检"分区；依赖新增 serde_json。
- 2026-11-22：`DMISelect` 新增 `grouped` 分组下拉（组=父号.n，选项=父号.n.m），
  demo"场景预设"落地首个三层编号实例；详见 changelog/2026-11-22-grouped-select-multilevel.md。
- 2026-11-22：`DMIId` 升级为"主号+小数路径"（上限 4 层，`is_root/level/child→Option`），
  `allocate_child` 增加深度守卫；编号机制支持子组件嵌套（A 方案，实例待容器组件落地）。
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
