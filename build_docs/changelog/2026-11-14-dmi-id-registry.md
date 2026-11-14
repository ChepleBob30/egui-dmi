# 2026-11-14 DMI任务1.2：组件ID注册表（src/ui/id.rs）

- 依据：`build_docs/plan/dmi.md` §1.2；用户批准的设计草案三点决定
  （两级编号、类型枚举、组件本轮一起接入）。
- 核心规则：**复用键 =（egui::Id，DMI名称）**。同一 egui id 下名称不同 ⇒ 额外分配新编号；
  名称相同 ⇒ 跨帧复用同一编号（命名用于帮助 DMI 分配 id，egui id 用于定位组件）。

## 新增 `src/ui/id.rs`

| 项目 | 功能 |
| --- | --- |
| `DMIId { major, minor }` | 两级编号；`DMIId::SYSTEM`(0号保留)、`child()`、`is_system()`；Display 输出 "1"/"1.2"，直接对齐 JSON target_id。 |
| `DMIWidgetKind` | 七种组件类型枚举，`as_str()` 输出 "button"/"text_input"/… 与文档示例一致。 |
| `DMIRecord` | 注册条目 (id, kind, parent, name)，Display 即文档四元组形式。 |
| `DMIRegistry`（私有） | 持久表：records + `(egui::Id,名称)→DMIId` 复用映射 + 主号发号器（从1起）+ 父号子号计数器；`OnceLock<Mutex>` 全局单例，中毒锁自动恢复。 |
| `allocate()` | 登记或复用主编号（未命中键则 next_major 递增）。 |
| `allocate_child()` | 父号下按出现顺序分配小数子号（1.1、1.2…），同键复用。 |
| `name_of()` | 按编号查名称（下拉子项命名需要父名做前缀）。 |
| `list_id()` | 全部条目快照（按分配顺序）。 |

## 修改 `src/ui/components.rs`

- `DMIUpdate` 新增 `pub id: DMIId` 字段（JSON `target_id` 来源，原 DMI 标记兑现）。
- 七个组件全部新增 `.dmi_name("...")` 构建方法；`show()` 末尾调用 `allocate()` 登记自身。
  未显式命名时注册名自动为 `{kind}-{major}`，匿名组件按（egui id，空名）键跨帧复用。
- `DMISelect` 新增 `.individual_ids()`：开启后弹出层每个选项登记 `父名_选项名` 子条目，
  编号为 `父号.1/父号.2`（与 dmi.md 示例 theme_selector_light 形态一致）；
  关闭时（默认）整组只挂一个父编号。新增私有 `sanitize_name()` 清洗选项文本。

## 修改 `src/ui.rs`

- 挂载 `pub mod id;`，再导出 `DMIId / DMIRecord / DMIWidgetKind / list_id`。

## 验证

- `cargo clippy --tests`：0 警告；`cargo test`：5 项通过——
  - `different_names_allocate_extra_id`：同 egui id 不同名称→两个编号；同名称→复用；0 号不外发；
  - `child_ids_follow_parent`：子号 1.1/1.2、parent 指向、按键复用、Display 格式；
  - `named_button_records_dmi_id`：组件 `dmi_name` 落表（id/kind/name 三对）；
  - 原有 design/components 两项回归通过。

## 未做（留给 1.3 及以后）

- JSON 解析与 request_type（ACCESS/STATE/OBSERVATION）、complexity 分发、按编号操作组件。
- 组件消失后的条目回收（条目只增不删，测试项目从简）。
