# 2026-11-14 三现象归因与修复（radio 组编号 / 标签可见性 / 子项对照）

## 用户观察与归因

1. **label 不在 DMI id 清单** —— 设计使然：匿名标签免登记是性能修复的反馈环闸口
   （注册表显示行即标签，默认登记会自我放大）。修复：demo 为章节标题补 `dmi_name`
   演示"命名即入表"，界面文案同时注明该规则。
2. **三个 radio 各占一个整数、无统一父号** —— 真缺口：dmi.md §1.2 规定单选框
   属"同时产生多个组件"的工具，应支持统一父号/小数子号两种编号。
3. **operation_mode 展开无子项、theme_selector 有** —— 设计使然：仅 theme_selector
   开启 `.individual_ids()`，正是两种编号模式的对照；与点击顺序无关的稳定性符合预期。

## 改动

- `src/ui/components.rs`：
  - `DMIRadio` 新增 `group` 字段与 `.group(name)` 构建方法：组名首现经 `allocate`
    发统一整数父号；每个 radio 以 `"{组名}_{选项文本}"`（`sanitize_name` 清洗）经
    `allocate_child` 领取 父号.序号 子号；`DMIUpdate.id` 返回子号；
  - 顺带修正此前批量插 `#[must_use]` 时属性与文档注释的顺序错位；
  - 新增测试 `radio_group_registers_parent_and_children`（3 帧渲染 → 恰 1 父 + 3 子，
    子号 .1/.2/.3、parent 指向正确、不重号）、`named_label_registers_anonymous_does_not`
    （命名标签入表 kind=label；匿名零登记）。
- `demo/dmi_gallery/src/main.rs`：画质三 radio 改 `.group("quality_radio")`；
  五个章节标题加 `dmi_name("section_*_label")` 演示命名标签入表；
  选择区分组文案注明两种编号模式与标签登记规则（说明行本身即匿名标签示范）。
- `docs/id-system.md`：§4 补 `DMIRadio::group` 条目与设计理由。

## 验证

- `cargo test --workspace`：10/10 通过；`cargo clippy --workspace --all-targets`：0 警告。
- GUI 截图核验路径因系统未授予 Accessibility 权限放弃，行为以单元/集成测试为准。
