# 2026-11-14 性能修复：注册表自我增长循环

## 症状

demo 交互时严重卡顿；改变组件状态瞬间短暂卡死；随操作次数越来越多明显恶化。

## 根因（按影响排序）

1. **注册表自增长反馈环（主因）**：注册表/日志面板的每一行是匿名 `DMILabel`，
   旧逻辑"所有组件 show() 一律登记" ⇒ 显示行本身入表 ⇒ 行数增加 ⇒ 下帧更多行待登记
   ⇒ 条目无限增长；每帧渲染 O(n) 克隆+`to_string`+排版 ⇒ 平方级恶化，表现为拖动滑块时卡死。
2. **复用键含 egui id**：egui 自动 id 由布局位置派生，状态变化引起行位置漂移时
   同名组件被重复分配新号，进一步加速入表。
3. **每帧全量 clone 注册表**：`list_id()` 每帧克隆 Vec+全部 String；Grid 每帧重建。
4. **每帧调用 `set_theme`**（轻微，写入 options 无谓开销）。

## 修复

- `src/ui/components.rs`：`DMILabel` 无显式 `dmi_name` 时**不登记**（返回 id=SYSTEM），
  反馈环从源头切断；有命名仍可登记。
- `src/ui/id.rs`：
  - 复用键改为枚举：显式名称 ⇒ **只按名称**（布局漂移不重号）；匿名 ⇒ 按 egui id；
  - 命中复用时**绝不触碰条目列表**，仅刷新定位表；
  - 新增 `locate()`（DMI id→最近 egui id，"egui id 用于定位"通道）、
    `registry_revision()`（版本号，仅入表递增）、
    `list_id()` 改返 `Arc<Vec<DMIRecord>>` 共享快照（读取零拷贝，入表时重建）。
- `demo/dmi_gallery/src/main.rs`：
  - 注册表面板按 `registry_revision` 门控，仅在版本变化时重建行缓存；Grid 换滚动列表；
  - `set_theme` 移入外观 changed 分支。

## 回归验证

- 新增测试 `frames_do_not_grow_registry`：模拟旧反馈环场景（每帧渲染含 2 个匿名标签 +
  6 个命名控件，共 300 帧），断言注册表只新增 6 条命名条目、匿名标签零入表；
- `repeated_allocation_does_not_grow` / `revision_tracks_growth` /
  `named_identity_survives_egui_drift`：同名漂移复用、版本门控、命名即身份；
- `cargo test --workspace` 8/8 通过（id 测试加互斥锁消除并行计数干扰）；
- `cargo clippy --workspace --all-targets` 0 警告；demo 冒烟运行 5 秒无异常。

## 遗留瓶颈观察（未修，按风险排序）

1. 匿名交互组件（未给 `dmi_name` 的按钮/输入框等）仍按 egui id 位置键登记，
   布局漂移会重号并增长 ⇒ 建议所有 DMI 可操作控件显式命名（demo 已全部命名）。
2. 快照在每次入表时全量克隆重建，O(n)；n<数千可接受，若注册表大规模增长
   应改为分段追加或写时复制链表。
3. 注册表/日志面板仍每帧排版行文本（egui galley 按内容缓存，内容不变则开销低；
   文本高频变化场景可考虑限频刷新）。
4. `push_log` 每条交互 `insert(0)` O(12) 与格式化分配，当前上限 12 条可忽略。
