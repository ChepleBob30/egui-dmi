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
 ├─ reused:        HashMap<ReuseKey, DMIId>   复用键 → 编号（reuse 机制的索引）
 ├─ located:       HashMap<DMIId, Id>         编号 → 最近一次 egui id（定位通道）
 ├─ next_major:    u32                        主号发号器（从 1 起，0 已预留）
 ├─ minor_counters:HashMap<DMIId, u32>        父号 → 已发子号计数
 ├─ revision:      u64                        版本号，仅真实入表时 +1
 └─ snapshot:      Arc<Vec<DMIRecord>>        共享只读快照，入表时重建
```

### 2.1 `DMIId { major, minor }`

两级整数编号。`minor == 0` 表示纯整数号；`1.2` 即 `{major:1, minor:2}`。

| 方法 | 做什么 | 为什么这样设计 |
| --- | --- | --- |
| `DMIId::SYSTEM`（常量） | 系统保留 0 号 | dmi.md 规定 0 号留给系统层接口；作为 `parent` 缺省值还兼表"根组件"，省去 Option。 |
| `DMIId::new(major)` | 由主号构造 | `const fn`，发号路径零运行时开销。 |
| `is_system()` | 判断是否 0 号 | 调用方（如未登记的标签）需要一个"无编号"哨兵；用它而不是 `Option`，保持四元组 `parent` 字段类型统一。 |
| `child(minor)` | 生成父号下子号 | 子号必须继承父 `major`（`1.1/1.2` 与父 `1` 同段），保证编号按组件聚簇、可读。 |
| `From<u32>` | 主号隐式转换 | JSON/测试里写 `1` 即 `DMIId`，减少样板。 |
| `Display` | 输出 `1` / `1.2` | 与 dmi.md 的 JSON `target_id` 字面完全一致，未来序列化零转换。 |

### 2.2 `DMIWidgetKind` 与 `as_str()`

七组件类型枚举。**用枚举而非自由字符串**：类型集是框架封闭集，非法值进不来；
`as_str()` 输出与 dmi.md 示例一致的 `"button"` / `"select"` 等字符串，展示层需要
字符串时再降维，存储层保持类型安全。

### 2.3 `DMIRecord` 与 `Display`

四元组 `(id, kind, parent, name)`，字段顺序即 dmi.md 文档顺序。`Display` 直接打印成
`(1, "select", 0, "theme_selector")` —— 人类可读、模型可读、将来可被逐行解析，
三者同一形态。

## 3. reuse 机制（核心）

### 3.1 `ReuseKey`：复用键 = 身份的定义

```rust
enum ReuseKey {
    Named(String),     // 显式命名：名称即身份
    Anonymous(Id),     // 匿名：egui id 位置键
}
```

这是整个系统最重要的一次取舍。旧版复用键是 `(egui::Id, 名称)` 二元组，实测引发两类
故障：

1. **重复入表**：egui 自动 id 由控件在 Ui 中的位置派生。状态一变（日志多一行、
   下拉开合、文本增删导致宽度变化），控件 egui id 漂移 → 同一逻辑组件被当成新组件
   → 重发号。面板每帧渲染又每帧登记，编号表单调膨胀。
2. **自我增长反馈环**：注册表显示行本身是匿名组件、也走登记 → 表变长 → 显示行变多
   → 再登记 → 循环放大（详见 `build_docs/changelog/2026-11-14-performance-fix.md`）。

新设计把"身份"与"定位"拆开，正好对应 dmi.md 那句话：
> egui id 的意义是让 DMI 定位组件，而命名的意义是帮助 DMI 分配 id。

- **显式命名 ⇒ 只按名称**。名称是调用方/模型给的逻辑身份，与渲染位置无关；
  布局怎么漂，`case_theme_selector` 永远是同一个号。漂移被吸收进 `located` 定位表。
- **匿名 ⇒ 按 egui id**。没名字就没有稳定身份可用，位置键是唯一选择；
  接受"布局漂移可能重号"的弱语义，并在文档和 demo 规范中要求**交互控件必须命名**。

哈希键里名称 `to_owned` 一次是每次 `allocate` 唯一的常态分配（O(名称长度)），
换来命中路径完全不动条目表——实测帧成本可忽略，列表稳定性优先。

### 3.2 `allocate()`（私有内核 + 公开壳）——复用主路径

私有 `DMIRegistry::allocate` 的步骤：

```text
1. 组键：Some(name) → Named(name)；None → Anonymous(egui_id)
2. reused.get(key) 命中？
   是 → located.insert(既有id, 本帧egui_id)  → return 既有id      ← 不触碰 records/
                                                              revision/snapshot
   否 → major = next_major++
      stored_name = 给定名 或 "{kind}-{major}"
      push_record(...)（入表+版本+快照，见 3.4）
      reused.insert(key, new_id); located.insert(new_id, egui_id)
```

公开壳 `pub fn allocate(egui_id, kind, name, parent)` 只负责加锁转发
（见 §5）。三条设计理由：

- **命中即返回**是 reuse 的硬要求：重复调用 = 刷新定位 + 返回旧号，
  列表零更新——"避免每一轮都重复更新列表"由这条不变式结构性保证；
- **自动名 `"{kind}-{major}"`**：匿名组件也要在表里有可读行；号已在发号时确定，
  名字里嵌号避免二次查表拼串；
- **同 egui id + 不同名 ⇒ 两个号**天然成立：两个名字就是两个 `Named` 键，
  互不遮挡。这正是"一个调色盘同时改文字色与背景色 → 拥有两个 id"的实现。

### 3.3 `allocate_child()`——子项的复用与发号

与 `allocate` 同构，差异只有两点，各有理由：

- **键固定为 `Named(name)`**：子项（下拉选项等）总在显式命名路径上（父名_选项名），
  因此同样对布局漂移免疫；弹出层开合、选项排序变化都不会重发子号；
- **发号用 `minor_counters[parent] += 1`**：子号按"首次出现顺序"分配
  （`1.1, 1.2…`），与选项在列表中的序号解耦——已登记的旧选项命中复用，
  永远不会因为插了新选项而全体挪号。

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

- **`.dmi_name(text)`**：全部七个组件统一的链式命名口。设计理由：名称是给
  reuse 键喂的"身份"，必须让调用方可控；
- **`show()` 末尾登记**：每个交互组件用自身交互产物的 `response.id` 作 `egui_id`
  调 `allocate`，结果装进 `DMIUpdate.id`——返回值即编号，调用方无需第二次查表；
- **`DMILabel` 的条件登记**：显式命名才登记（`Some(given) => allocate…`，
  否则 `DMIId::SYSTEM`）。理由：标签是显示面，默认免登记直接掐死 §3.1 的反馈环；
  需要 DMI 寻址的说明文本可以主动命名换取编号；
- **`DMISelect::individual_ids()` + `sanitize_name()`**：开启后对弹出层本轮渲染过的
  每个选项，以其 `option_response.id` 与 `"{父名}_{清洗后的选项文本}"` 调
  `allocate_child`。`sanitize_name` 把选项文本压成小写字母数字下划线片段，
  对齐 dmi.md `theme_selector_light` 命名例。选项文本即名称 ⇒ 复用按文本恒定，
  与控件位置无关；
- **`DMIRadio::group(name)`**：单选框是 dmi.md 点名的"同时产生多个组件"的工具。
  组名首现时经 `allocate` 发一个统一整数父号（操作该号即寻址整组），每成员经
  `allocate_child` 领取小数子号，名称为 `"{组名}_{选项文本}"`；`DMIUpdate.id`
  返回子号（点的是哪个），父号留在表里供整组操作——dmi.md 的两种编号方式同时成立。
  设计理由：`DMIRadio` 保持单控件 API（与 demo 循环写法一致），组语义只加一个
  builder，不引入新组件类型；
- **`DMIUpdate::from_response / with_changed`**：私有构造器强制所有出口都带 id
  字段，避免忘记接线。

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
第 1 帧  DMISelect("theme_selector").individual_ids() 渲染，弹出层关闭
         show → allocate(egui_id=X, Select, "theme_selector") → 未命中 → 发号 1
         （表: (1,"select",0,"theme_selector")  revision:1）
用户点开下拉，第 2 帧 弹出层渲染 "浅色"/"深色"
         allocate_child(egui_id=A, "theme_selector_浅色", 1) → 未命中 → 1.1
         allocate_child(egui_id=B, "theme_selector_深色", 1) → 未命中 → 1.2
         （revision:3；面板检出变化 → 重建三行缓存）
第 3..n 帧 反复渲染/滚动/主题切换，egui id 漂移
         三个键全部命中 reused → 表零增长、版本不变 → 面板连快照都不读
用户选"深色"   → picked 写回，update.changed()==true，update.id == DMIId{1,0}
未来 DMI 调用  {"target_id": 1.2} → list_id 找名 → locate(1.2) 拿当前 egui id 操作真组件
```

## 7. 已知边界与演进路线

| 边界 | 现状代价 | 触发重构的条件与方案 |
| --- | --- | --- |
| 快照重建 O(n) | 每次入表克隆全表 | 条目 > 数千：改分块 Arc 链表或持久化向量 |
| 匿名交互控件漂移重号 | 位置键语义弱 | 约定交互控件必须 `dmi_name`（demo 全量遵守） |
| `name_of` 线性扫 | 仅入表路径，非每帧 | 成为热点则加 `HashMap<String, DMIId>` 二级索引（即 reused 键反用） |
| 条目不回收 | 只增不删 | 未来按帧号打 `last_seen`，做标记-清除式清扫 |

## 8. 测试对应关系

- `named_identity_survives_egui_drift` ↔ §3.1 命名即身份 + §3.5 定位刷新；
- `repeated_allocation_does_not_grow` ↔ §3.2 命中不动表（十次一码一条）；
- `child_ids_follow_parent` ↔ §3.3 子号复用与 `child()` 结构；
- `revision_tracks_growth` ↔ §3.4 版本与增长焊死；
- `frames_do_not_grow_registry`（components.rs）↔ 整条反馈环回归：300 帧 × 混合
  命名控件+匿名标签，表只增命名控件数；
- `test_mutex`：全局单例下的计数断言串行化，测试间零串扰。
