# 2026-11-14 文档：DMI 组件ID分配系统说明

- 依据：用户指令"在 docs/ 下单开文件阐明 id 分配系统运作流程，着重 reuse 机制，
  覆盖每个方法及设计理由"。
- 纯文档任务，未改动任何代码。

## 新增 `docs/id-system.md`（221 行）

| 章节 | 内容 |
| --- | --- |
| §1-2 | 系统定位、两条不变式、全部数据结构与 `DMIId`/`DMIWidgetKind`/`DMIRecord` 逐方法设计理由表 |
| §3 | **reuse 机制核心**：`ReuseKey`（Named/Anonymous）取舍由来（旧版两类故障）、`allocate` 命中即返回不动表、`allocate_child` 子号策略、`push_record` 入表唯一出口、`locate` 定位通道方向选择、`name_of`、`list_id` Arc 快照与 `registry_revision` 门控闭环 |
| §4 | 组件登记路径：`dmi_name`、show() 末尾登记、`DMILabel` 条件登记、`individual_ids`+`sanitize_name`、`DMIUpdate` 构造器约束 |
| §5 | 并发与生命周期：OnceLock 单例理由、中毒锁恢复、发号确定性 |
| §6 | demo 外观下拉的逐帧完整实例 |
| §7 | 已知边界与演进路线表 |
| §8 | 六个测试与机制的对应关系 |

## 更新

- `build_docs/structure.md`：docs 节点加入 `id-system.md` 及功能说明。
