# 2026-11-22 多层组件落地：DMISelect::grouped 分组下拉

- 依据：用户指令"给demo添加多层组件，直接开工"（免蓝图）。
- 承接：编号机制升级蓝图 dmi_phase2_id.md 预留的命名法与深度守卫，本次首次落地真实实例。

## 改动

- `src/ui/components.rs`：
  - `DMISelect` 新增 `groups: &'a [(&'a str, &'a [Value])]` 字段与
    `grouped(label, current, groups)` 构造器（扁平 `new()` 行为不变）；
  - 弹出层渲染分两路：扁平逐项可选；分组模式组头为不可选弱色标签、组内逐项可选；
    渲染时收集（组名/选项文本, egui id），渲染后按出现顺序登记：
    组 → `父号.n`（名称 `{父名}_{组名}`），选项 → `父号.n.m`（名称 `{父名}_{组名}_{选项名}`）；
  - 分组模式自动启用小数编号（`individual_ids` 仅对扁平生效）；登记全走名称键，
    收起/展开/漂移不重号；
  - 新测试 `grouped_select_closed_registers_parent_only`：收起态只登记父号。
- `demo/dmi_gallery/src/main.rs`：
  - 静态 `PRESET_GROUPS`（画面/声音/网络 三组七项，零每帧分配）；
  - 选择区新增"预设"行：`DMISelect::grouped("preset_selector", ...).dmi_name("preset_selector")`，
    展开后注册表面板出现真实三层号（如 `preset_selector_画面 → N.1`、
    `preset_selector_画面_明亮 → N.1.1`）；切换写交互日志；
  - 分区追加多层编号说明行（匿名标签，本身即不登记示范）。
- `docs/design-system.md`、`docs/id-system.md`：grouped 契约、登记路径与实例落地标注。

## 验证

- `cargo test --workspace`：16/16 通过；`cargo clippy --workspace --all-targets`：0 警告；
- demo 冒烟运行 5 秒稳定、无错误输出。
- 三层发号链、深度上限、名称复用等机制行为由 id.rs 测试
  （grandchild_chain_allocates_under_child 等）覆盖；本实例复用同一 `allocate_child` 通道。
