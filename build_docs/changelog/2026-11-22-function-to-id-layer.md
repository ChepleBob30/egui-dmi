# 2026-11-22 回调：function 字段收敛到 id 存储机制，组件层不再持有

- 依据：用户指令"回调 components.rs 改动，所有 DMI 组件中均不要添加 function 字段，
  只在 id 存储机制中保留"。

## 改动

- `src/ui/components.rs`：
  - 删除七组件的 `function` 字段、`dmi_entry()` 构建方法与实现侧功能常量
    （PICK_FUNCTION / RADIO_*_FUNCTION）；
  - `register()` 登记门收敛为名称单参数：无名返回 `SYSTEM`，有名即注册
    （功能映射传空）；radio 组与 select 子项登记同样传空映射，
    `touch`（parent 元组触及方式）保持不变；
  - 测试同步：帧回归、命名按钮、分组下拉、命名标签用例移除 dmi_entry 声明；
    `undeclared_component_stays_unregistered` 收敛为"无名不注册"单一断言。
- `src/ui/id.rs`：
  - `DMIRecord.function` 字段、`allocate`/`allocate_child` 的 `function` 参数与
    非空时的逐项严格校验全部保留（功能映射语义完整存活于 id 层，
    供直接登记路径与后续 dmi 逻辑使用）；
  - 校验口径调整：空功能映射放行（组件登记即此形态），非空仍逐项严查
    （名非空、列表非空、不含 Direct、功能名不重复）；
  - 拒绝矩阵测试相应更新：空映射从拒绝项改为合法注册断言。
- `demo/dmi_gallery/src/main.rs`：移除全部 `.dmi_entry(...)` 声明；
  调色盘分区的直接 `id::allocate` 调用保留功能映射实参（id 层 API 示范）。
- 文档：`docs/design-system.md`、`docs/id-system.md`（§4/§6/§7）、
  `build_docs/structure.md` 措辞同步。

## 验证

- `cargo test --workspace`：24/24；`cargo clippy --workspace --all-targets`：0 警告；
- demo 冒烟 5 秒稳定、无错误输出。
