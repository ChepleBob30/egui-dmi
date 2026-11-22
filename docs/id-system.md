# DMI 组件ID分配系统说明

本文对应实现：`src/ui/id.rs`（注册表核心）、`src/ui/components.rs`（组件登记路径）、
`demo/dmi_gallery/src/main.rs`（消费端示范）。设计依据：`build_docs/plan/dmi.md` §1.2。

---

## 1. 系统定位

ID 分配系统解决一件事：**让大模型（DMI）有一个稳定、可读、可枚举的句柄去指代画面上
的每一个组件**。egui 每帧重建 UI，组件没有天然的持久身份；本系统在组件与 egui 身份
之间架一层登记表：

- **登记时机**：组件 `show()` 渲染时自动登记（或刷新），调用方零手动成本；
- **编号语义**：整数号 = 一个组件实例；小数号 = 复合组件的子项；0 号 = 系统保留；
- **两条不变式**：
  1. 命名不同 ⇒ 必是新编号（哪怕 egui id 相同——调色盘一拖二场景）；
  2. 命名相同 ⇒ 必复用原编号（哪怕 egui id 漂移、帧数无限增长——绝不重复入表）。

## 2. 数据结构总览

```text
REGISTRY: OnceLock<Mutex<DMIRegistry>>        进程内唯一
DMIRegistry
 ├─ records:       Vec<DMIRecord>             按分配顺序的条目表
 ├─ reused:        HashMap<String, DMIId>       名称 → 编号（唯一身份键，严格声明制）
 ├─ located:       HashMap<DMIId, Id>         编号 → 最近一次 egui id（定位通道）
 ├─ next_major:    u32                        主号发号器（从 1 起，0 已预留）
 ├─ minor_counters:HashMap<DMIId, u32>        父号 → 已发子号计数
 ├─ revision:      u64                        版本号，仅真实入表时 +1
 └─ snapshot:      Arc<Vec<DMIRecord>>        共享只读快照，入表时重建
```

### 2.1 `DMIId { major, minors, depth }`

主号 + 小数路径编号，`DMI_MAX_DEPTH = 4` 层上限：`1`、`1.2`、`1.2.3.4` 均可表达，
`depth` 是 `minors` 数组的有效长度。定长数组而非 `Vec`：`DMIId` 是 `minor_counters`
与 `located` 的哈希键、每帧 `allocate` 热路径触碰，必须保持 `Copy + Hash + Ord` 零分配。

| 方法 | 做什么 | 为什么这样设计 |
| --- | --- | --- |
| `DMIId::SYSTEM`（常量） | 系统保留 0 号 | dmi.md 规定 0 号留给系统层接口；作为 `parent` 缺省值还兼表"根组件"，省去 Option。 |
| `DMIId::new(major)` | 由主号构造 | `const fn`，发号路径零运行时开销。 |
| `is_system()` | 判断是否 0 号 | 调用方（如未登记的标签）需要一个"无编号"哨兵；用它而不是 `Option`，保持四元组 `parent` 字段类型统一。 |
| `is_root()` | 判断是否不带小数层 | 取代旧 `minor == 0` 判定；根号即主号本身。 |
| `child(minor)` | 追加一层小数，返回 `Option` | 子号继承父 `major`，路径逐级追加（`1 → 1.1 → 1.1.1`）；满深度返回 `None`，深度是坐标不是身份，追加不改变已有前缀。 |
| `level(index)` | 读取第 index 层小数 | 越界返回 0；供排序、命名拼接与序列化使用，避免外部直接触碰数组。 |
| `From<u32>` | 主号隐式转换 | JSON/测试里写 `1` 即 `DMIId`，减少样板。 |
| `Display` | 输出 `1` / `1.2` / `1.2.3` | 与 dmi.md 的 JSON `target_id` 字面完全一致；两级以上小数在 JSON 中须用字符串形态（`"1.1.1"`），因为 `1.1.1` 不是合法 JSON 数字。 |

### 2.2 `DMIWidgetKind` 与 `as_str()`

七组件类型枚举。**用枚举而非自由字符串**：类型集是框架封闭集，非法值进不来；
`as_str()` 输出与 dmi.md 示例一致的 `"button"` / `"select"` 等字符串，展示层需要
字符串时再降维，存储层保持类型安全。

### 2.3 `DMIRecord` 与 `Display`

五元组 `(id, kind, (parent, reach), name, function)`：`parent` 是
`(上级编号, 从上级触及本组件的方式)` 元组，根组件为 `(0, Direct)`；`function` 是
`功能名 → 触及列表` 的能力映射，注册期声明、首声明入表。`Display` 直接打印成
`(1, "select", (0, Direct), "theme_selector", [("expand", [LeftClick, Hover])])`——
人类可读、模型可读、将来可被逐行解析，三者同一形态。

## 3. reuse 机制（核心）

### 3.1 复用键：名称即唯一身份

复用表只有一个 `HashMap<String, DMIId>`——**名称是唯一身份键**。这是两步收敛的结果：

1. 最早版键为 `(egui::Id, 名称)` 二元组，实测引发两类故障：**重复入表**
   （egui 自动 id 由布局位置派生，状态一变位置漂移 ⇒ 同组件重发号）与
   **自我增长反馈环**（注册表显示行本身是匿名组件也走登记 ⇒ 表变长 → 行变多 →
   再登记，见 `build_docs/changelog/2026-11-14-performance-fix.md`）；
2. 第一步收敛拆开"身份"与"定位"：显式命名只按名称、匿名按 egui id 位置键；
   严格声明制（UI 触及方法改进裁决）完成第二步收敛：**匿名位置键删除**——
   未声明名称与功能映射的组件一律不注册（`DMIUpdate.id` 即 `SYSTEM` 哨兵），
   注册表只收录"有名有能"的组件。

这正对应 dmi.md 那句话：
> egui id 的意义是让 DMI 定位组件，而命名的意义是帮助 DMI 分配 id。

- **名称即身份**：布局怎么漂，`case_theme_selector` 永远是同一个号，
  漂移被吸收进 `located` 定位表；
- **定位是快照**：`located` 在每次登记调用时刷新最新 egui id，供 Access 阶段
  按号反查组件。

哈希键里名称 `to_owned` 一次是每次 `allocate` 唯一的常态分配（O(名称长度)），
换来命中路径完全不动条目表——实测帧成本可忽略，列表稳定性优先。

### 3.2 `allocate()`（私有内核 + 公开壳）——复用主路径

私有 `DMIRegistry::allocate` 的步骤：

```text
1. 严格校验：name 非空；function 非空；每个功能项名非空、触及列表非空、
   列表不含 Direct、功能名不重复——任一不满足 → 返回 SYSTEM，不入表
2. reused.get(name) 命中？
   是 → located.insert(既有id, 本帧egui_id) → return 既有id        ← 不触碰 records/
                                                              revision/snapshot
   否 → major = next_major++
      push_record(...)（parent=(0,Direct)、function 拷入 owned 映射；
                       入表+版本+快照，见 3.4）
      reused.insert(name, new_id); located.insert(new_id, egui_id)
```

公开壳 `pub fn allocate(egui_id, kind, name, function)` 只负责加锁转发
（见 §5）。三条设计理由：

- **命中即返回**是 reuse 的硬要求：重复调用 = 刷新定位 + 返回旧号，
  列表零更新——"避免每一轮都重复更新列表"由这条不变式结构性保证；
- **严格声明制**：没有自动名。功能映射是 DMI 操作组件的语义来源，
  无名或无能的条目不可信，宁缺毋滥；
- **同 egui id + 不同名 ⇒ 两个号**依然天然成立：两个名字就是两个键，
  互不遮挡。这正是"一个调色盘同时改文字色与背景色 → 拥有两个 id"的实现。

### 3.3 `allocate_child()`——子项的复用与发号

与 `allocate` 同构，差异只有两点，各有理由：

- **键固定为名称**（`{父名}_{子名}`）：子项总在显式命名路径上，对布局漂移免疫；
  弹出层开合、选项排序变化都不会重发子号；
- **touch 与父号校验**：`touch` 不得为 `Direct`（那是根占位），父号必须已在表中
  ——组件实现保证先父后子的登记顺序；子项功能映射由实现代码显式声明
  （选项 `"pick"`、组成员 `"select"`、组头 `"hover"`），与调用方严格声明制同口径；
- **发号用 `minor_counters[parent] += 1`**：子号按"首次出现顺序"分配
  （`1.1, 1.2…`），与选项在列表中的序号解耦——已登记的旧选项命中复用，
  永远不会因为插了新选项而全体挪号；
- **深度守卫**：`parent.child(counter)` 满 4 层返回 `None`，此时显式拒绝——
  debug 档 `debug_assert` 提示，release 档返回 `SYSTEM` 哨兵且不入表。
  宁可拒绝不截断；`minor_counters` 以任意 `DMIId` 为键，孙层发号天然成立。

### 3.4 `push_record()`——唯一允许列表增长的出口

```rust
records.push(record);  revision += 1;  snapshot = Arc::new(records.clone());
```

把"入表、版本、快照重建"三步焊死在一个私有方法里：复用路径不可能误触版本，
任何增长必然伴随版本变化。`revision` 因此可以放心当 UI 重建的唯一触发器。
快照在入表时一次性克隆重建：读端永远免费（§3.7），代价是每次真实增长 O(n)——
n 在测试项目规模（数十~数千）完全可接受，超限的演进方案见 §7。

### 3.5 `locate()`——定位通道

`HashMap<DMIId, Id>`，方向是**编号 → 最近一次 egui id**：

- 每次 `allocate` 命中都会刷新（写 map 是 O(1)，不入表）；
- 方向选择理由：一个 egui id 可能合法对应多个 DMI id（调色盘场景），
  `egui→DMI` 做单值映射必然有损；`DMI→egui` 一对一，正是"DMI 用 egui id 定位组件"
  （未来 ACCESS/STATE 接口操作真组件时）需要的形状；
- 该映射每帧收敛：只要组件还在被渲染，`locate(id)` 就返回它当前的 egui id。

### 3.6 `name_of()`——编号回查名称

子项登记需要"父名_选项名"的命名法，组件层只有父 `DMIId`，故提供按号查名。
O(records) 线性扫——只在入表新子号前调用一次，非每帧路径，够用；
若将来成为热点再挂 `name_by_id` 二级索引（§7）。

### 3.7 `list_id()` 与 `registry_revision()`——读端零成本对

- `list_id() -> Arc<Vec<DMIRecord>>`：克隆的只是引用计数，不复制条目。
  消费端（注册表面板）因此可以每帧拿快照而不引发分配风暴；
- `registry_revision() -> u64`：面板先读版本（O(1)），版本未变 ⇒ 连快照都不必取、
  直接用本地缓存的行文本渲染；版本变 ⇒ 重建缓存。**reuse 机制在 UI 层的闭环**：
  库里"命中不动表" + 表外"版本门控不动 UI"，两层共同把每帧成本压到常数次
  锁读取与哈希查找。

## 4. 组件登记路径（components.rs）

- **`.dmi_name(text)`**：七个组件统一的链式命名口。名称给 reuse 键喂"身份"，
  必须让调用方可控；功能映射不在组件层——组件不持有 `function` 字段，
  该语义由 id 存储机制承载（`allocate`/`allocate_child` 的 `function` 参数与
  `DMIRecord.function` 字段），供直接登记路径与后续 dmi 逻辑使用；
- **`register()` 登记门（私有辅助）**：show() 尾部统一走
  `register(egui_id, kind, &dmi_name)`——无名返回 `SYSTEM` 不注册，有名即注册
  （功能映射传空，合法入表）。每个组件的 `show()` 因此只有一行登记代码，
  未命名组件照常渲染但脱离 DMI 管理；
- **`DMIUpdate.id` 即注册凭证**：`is_system()` 为真表示"该组件未注册"，
  调用方可据此自检声明完整性；返回值即编号，无需第二次查表；
- **标签的登记**：与其他组件同门同规（名称 + 功能映射双全才入表）。
  demo 中章节标题显式声明 `("hover", [Hover])` 入表，日志/注册表显示行保持匿名
  不注册——§3.1 反馈环闸口在严格声明制下依然成立；
- **`DMISelect::individual_ids()` + `sanitize_name()`**：开启后对弹出层本轮渲染过的
  每个选项，以其 `option_response.id` 与 `"{父名}_{清洗后的选项文本}"` 调
  `allocate_child`。`sanitize_name` 把选项文本压成小写字母数字下划线片段，
  对齐 dmi.md `theme_selector_light` 命名例。选项文本即名称 ⇒ 复用按文本恒定，
  与控件位置无关；
- **`DMISelect::grouped(label, current, groups)`**：分组下拉，`groups` 为
  `&[(组名, &[选项])]`。弹出层渲染时收集组头与选项的（文本, egui id），渲染后
  按出现顺序登记：组领 `父号.n`（名称 `"{父名}_{组名}"`），组内选项领 `父号.n.m`
  （名称 `"{父名}_{组名}_{选项名}"`）；分组模式自动启用小数编号，无需
  `.individual_ids()`。登记全部走名称键，收起/展开/布局漂移都不重号；
- **`DMIRadio::group(name)`**：单选框是 dmi.md 点名的"同时产生多个组件"的工具。
  组名首现时经 `allocate` 发一个统一整数父号（操作该号即寻址整组），每成员经
  `allocate_child` 领取小数子号，名称为 `"{组名}_{选项文本}"`；`DMIUpdate.id`
  返回子号（点的是哪个），父号留在表里供整组操作——dmi.md 的两种编号方式同时成立。
  设计理由：`DMIRadio` 保持单控件 API（与 demo 循环写法一致），组语义只加一个
  builder，不引入新组件类型；
- **`DMIUpdate::new / from_parts`**：纯数据构造器。`from_parts` 私有且只收 `&Response`，
  在 show() 内部提取 `clicked/changed` 两个状态位后 Response 即失效——出口结构上
  不可能再把 Response 带出去，id 字段也不可能被忘记接线。

## 5. 并发与生命周期

- `REGISTRY: OnceLock<Mutex<DMIRegistry>>`：**进程级单例**，跨帧、跨面板持久，
  满足"程序启动时保存一个列表"。不挂 `egui::Context`（demo 测试里每帧可能是不同
  Context 实例；且 Context 数据在重启/persistence 语义下不可控）；
- `with_registry(action)`：统一加锁转发所有公开方法。中毒锁走
  `into_inner` 恢复：登记路径全是追加式写入，单点 panic 不应永久瘫痪 UI 编号；
- **确定性**：发号顺序 = 首次渲染顺序。同一程序布局不变则编号序列完全可复现，
  这正是大模型可依赖"上次记住的号"的前提（配合 §3.5 的每帧定位刷新）。

## 6. 完整流程实例（demo 外观下拉）

```text
第 1 帧  DMISelect.dmi_name("theme_selector").individual_ids()，弹出层关闭
         show → register → allocate(X, Select, "theme_selector", [])
         → 未命中 → 发号 1
         （表: (1,"select",(0,Direct),"theme_selector",[])  revision:1）
用户点开下拉，第 2 帧 弹出层渲染 "浅色"/"深色"
         allocate_child(A, "theme_selector_浅色", 1, LeftClick, []) → 未命中 → 1.1
         allocate_child(B, "theme_selector_深色", 1, LeftClick, []) → 未命中 → 1.2
         （revision:3；面板检出变化 → 重建三行缓存）
第 3..n 帧 反复渲染/滚动/主题切换，egui id 漂移
         三个键全部命中 reused → 表零增长、版本不变 → 面板连快照都不读
用户选"深色"   → picked 写回，update.changed()==true，update.id == DMIId{1,0}
未来 DMI 调用  {"target_id": 1.2} → list_id 找名 → locate(1.2) 拿当前 egui id 操作真组件
```

编号机制支持三层及更深（`1.1.1`，上限 4 层），命名法为"父全名 + '_' + 本项名"；
第一个多层实例已由 `DMISelect::grouped`（demo"场景预设"）落地：
组登记 `父号.n`、组内选项登记 `父号.n.m`（如 `5.1.2`），命名法即 §2.4 预留的
"父全名 + '_' + 本项名"拼接式，`id.rs` 机制未因实例落地再改动。

## 7. 已知边界与演进路线

| 边界 | 现状代价 | 触发重构的条件与方案 |
| --- | --- | --- |
| 快照重建 O(n) | 每次入表克隆全表 | 条目 > 数千：改分块 Arc 链表或持久化向量 |
| 未注册组件脱离 DMI 管理 | 无名 ⇒ id=SYSTEM 不可操作 | Access 接入检查表逐一核对 dmi_name（demo 已全量遵守）；功能映射经 id 层直接登记补录 |
| `name_of` 线性扫 | 仅入表路径，非每帧 | 成为热点则加 `HashMap<String, DMIId>` 二级索引（即 reused 键反用） |
| 条目不回收 | 只增不删 | 未来按帧号打 `last_seen`，做标记-清除式清扫 |
| 嵌套深度上限 | 4 层（`1.2.3.4`），第 5 层显式拒绝 | debug panic 提示 / release 回 `SYSTEM` 不入表；容器组件层级需求超出时调大 `DMI_MAX_DEPTH` 即可 |

## 8. 测试对应关系

- `named_identity_survives_egui_drift` ↔ §3.1 命名即身份 + §3.5 定位刷新；
- `repeated_allocation_does_not_grow` ↔ §3.2 命中不动表（十次一码一条）；
- `child_ids_follow_parent` ↔ §3.3 子号复用与 `child()` 结构；
- `revision_tracks_growth` ↔ §3.4 版本与增长焊死；
- `frames_do_not_grow_registry`（components.rs）↔ 整条反馈环回归：300 帧 × 混合
  命名控件+匿名标签，表只增命名控件数；
- `test_mutex`：全局单例下的计数断言串行化，测试间零串扰。
