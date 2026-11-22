# DMI 编号机制升级蓝图：两级 DMIId → 任意深度小数路径

依据：`build_docs/plan/dmi.md` §1.2（子组件编号）与 Phase 2 评审中用户问题 1
（"Select 这类组件是否可能出现子组件套子组件"）。
范围纪律：本蓝图只做 `ui/id.rs` 编号表示与发号机制的升级契约；
是否在本阶段落地嵌套实例（分组下拉）仍待裁决 A/B，不在本文范围；
`DMICommand.payload` 透传修订属 dmi_phase2.md，亦不在本文范围。

---

## 1. 动机与现状判定

- 现契约 `DMIId { major: u32, minor: u32 }` 只能表达两层（`1`、`1.2`）；
- 当前七个组件内不存在"子组件套子组件"的真实场景（Select 弹出项、Radio 组成员均为叶子）；
- 分组下拉（组为子 `1.1`、组内选项为孙 `1.1.1`）、子菜单、容器类组件在路线图上必然出现；
- 结论：机制先行升级，实例随第一个容器组件自然落地。升级必须满足：
  向后兼容（现有两级号是新表示的 depth≤1 特例）、热路径零分配（allocate 每帧执行）、
  reuse 键语义不变（名称仍是身份，深度只是坐标）。

## 2. 新契约（ui/id.rs）

### 2.1 DMIId 表示

```rust
/// 小数路径最大深度：支持 1.2.3.4 四级嵌套。 / Maximum dotted path depth.
pub const DMI_MAX_DEPTH: usize = 4;

/// DMI 编号：主号 + 小数路径。depth 为 minors 的有效长度。
/// DMI identifier: major plus dotted path; depth is the valid minors length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DMIId {
    /// 主编号（0 为系统保留）。 / Major number (0 reserved for system).
    pub major: u32,
    /// 小数路径，仅前 depth 项有效。 / Dotted path, first `depth` entries valid.
    pub minors: [u32; DMI_MAX_DEPTH],
    /// 小数层数 0..=DMI_MAX_DEPTH。 / Number of dotted levels.
    pub depth: u8,
}
```

选型论证：定长数组 + 深度计数，而非 `Vec<u32>`/`SmallVec`——
`DMIId` 必须保持 `Copy + Hash + Ord`（它是 `minor_counters`、`located` 的键，
每帧 `allocate` 热路径），定长数组无堆分配、无容量分支，24 字节可接受；
深度上限 4 层对 UI 组件树（下拉→分组→子项→子子项）留有余量，超限显式拒绝而非截断。

### 2.2 方法契约（旧 → 新）

```rust
impl DMIId {
    /// 系统保留 0 号：major=0, depth=0。 / Reserved system id.
    pub const SYSTEM: Self;
    /// 主号构造（depth=0）。 / Root constructor.
    pub const fn new(major: u32) -> Self;
    /// 是否系统保留号。语义不变。 / Reserved check, unchanged.
    pub const fn is_system(self) -> bool;
    /// 是否根号（原 minor==0 判定的替代）。 / Root check, replaces minor==0.
    pub const fn is_root(self) -> bool;
    /// 追加一级小数：1.2 → 1.2.3；满深度返回 None。
    /// Append one dotted level; None when DMI_MAX_DEPTH reached.
    pub const fn child(self, minor: u32) -> Option<DMIId>;
    /// 第 index 层小数（越界返回 0）。 / Dotted component at index.
    pub const fn level(self, index: usize) -> u32;
}
/// "1" / "1.2" / "1.2.3"，与 JSON target_id 字符串形态一致。
impl std::fmt::Display for DMIId;
/// 主号便捷构造不变。 / Root-from-u32 unchanged.
impl From<u32> for DMIId;
```

破坏性变更说明：字段 `minor` 删除，改用 `depth`/`level(0)`；
调用点仅 `components.rs` 测试两处（`record.id.minor == 0` → `is_root()`，
`minors` 收集 → `level(0)`）与本文 §5 测试改写，库内其余逻辑不直接触碰 minor 字段。

### 2.3 发号机制（注册表侧）

- `minor_counters: HashMap<DMIId, u32>` **无需改动**：本就以任意 `DMIId` 为键，
  父号是 `1` 还是 `1.1` 都独立计数——递归发号天然成立；
- `allocate_child` 唯一改动：`parent.child(counter)` 现为 `Option`，
  满深度（第 5 层请求）时**拒绝入表**，返回 `DMIId::SYSTEM` 哨兵并 `debug_assert`；
  该路径在 4 层上限内不可达，属防御性边界（与错误码分层一致：宁可显式拒绝，不可静默截断）；
- `ReuseKey`、`reused`、`located`、`push_record`、`revision`、`snapshot`、
  `list_id`、`name_of`、`locate`、`registry_revision` 全部签名与语义不变；
- 祖孙关系表达：`DMIRecord.parent` 指向直接父号，编号路径前缀与 parent 字段两处一致
  （`1.1.1` 的 parent 是 `1.1`），遍历任意子树可由 parent 链或路径前缀双通道完成。

### 2.4 嵌套命名约定（为 B 方案预留，不实现）

若落地分组下拉：组登记 `"{父名}_{组名}"` → `1.1`；
组内选项登记 `"{父名}_{组名}_{选项名}"` → `1.1.1`；
命名继续承担身份（reuse 键），深度只是坐标——机制与命名法解耦，
任何容器组件接入时只需遵守"父全名 + '_' + 本项名"的拼接式，无需再改 id.rs。

## 3. dmi 层（Phase 2）联动

- `parse_target_id` 规则补充：JSON 数字最多表达一级小数（`1.2`）；
  **两级及以上小数必须使用字符串形态**（`"1.1.1"`），数字 `1.1.1` 本身不是合法 JSON；
  解析实现按 '.' 拆分，段数超过 `DMI_MAX_DEPTH` 报 `TARGET_ID_INVALID`；
- `greet` 只认 `SYSTEM`，不受本次升级影响。

## 4. 迁移清单（批准后执行）

| 位置 | 改动 |
| --- | --- |
| `ui/id.rs` | DMIId 表示与 2.2 方法；`allocate_child` 的 Option 分支；测试改写（§5） |
| `ui/components.rs` | 仅测试两处 minor 判定的替代；组件逻辑零改动（allocate/allocate_child 签名不变） |
| `docs/id-system.md` | §2.1 DMIId 契约表、§3.3 子号描述、§6 实例追加三层示例 |
| `build_docs/blueprint/dmi_phase2.md` | §2.1 target_id 解析补"多级用字符串"一条 |
| changelog/structure | 常规两件套 |

## 5. 测试计划

1. 构造与显示：`new(1).child(2).unwrap().child(3).unwrap()` → Display `"1.2.3"`；
   `is_root`/`level`/`is_system` 边界；
2. 三层发号链：`allocate` 父 `1` → `allocate_child` 子 `1.1` →
   对 `1.1` 再 `allocate_child` 得 `1.1.1`、`1.1.2`；断言 parent 字段链、
   重复登记复用不增长（沿用名称键）；
3. 深度上限：对 `1.1.1.1` 请求子号 → 返回 `SYSTEM` 且不入表（release 行为，
   debug 下断言可控）；
4. 键行为：`DMIId` 作 `HashMap` 键的 Hash/Ord 一致性（数组尾部零值不参与 Ord？——
   Ord 按 (major, minors, depth) 元组序，比较前按 depth 截断，测试固化该语义）；
5. 回归：现有 `child_ids_follow_parent`、`radio_group_registers_parent_and_children`、
   `frames_do_not_grow_registry`、dmi 相关断言全部在新表示下通过。

## 6. 风险与对策

| 风险 | 对策 |
| --- | --- |
| Ord 受 minors 尾部填充影响 | 比较逻辑以 depth 截断后逐段比较，测试固化 |
| 数字 JSON 无法表达三层 | 字符串形态约定写入解析与文档（§3） |
| 第 5 层静默错号 | 显式拒绝 + SYSTEM 哨兵 + debug_assert（§2.3） |
| 表示变更波及历史编号 | 注册表纯内存、无持久化，零迁移成本 |

## 7. 本蓝图不做

分组下拉/子菜单等嵌套组件实例（待 A/B 裁决，B 选定后按 §2.4 命名法另行补契约）；
`DMICommand.payload` 透传（属 dmi_phase2.md）；运行时控制台；ACCESS/STATE。
