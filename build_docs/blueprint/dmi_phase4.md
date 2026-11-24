# DMI Phase 4 蓝图：读取类接口 check_widget_status（Observation）

依据：`build_docs/plan/dmi.md` §3、§3.1（2026-11-24 更新）。
对应计划阶段：§3 读取类接口（承接 dmi_phase2 命令契约、dmi_phase3 运行时通道）。
用户裁决（2026-11-24）：1=A `widget_function` 数组原样；2=A `enabled` 留 standard、
其余可用性位进 complex；3=A 无快照防御分支复用 `TARGET_ID_INVALID`；
4=A `parent` 三档均不默认输出。
范围纪律：只实现 `check_widget_status` 的 OBSERVATION 类型；ACCESS 类型分支留桩
（§3.1 声明支持两种，其执行语义属 §3.2 及以后）；STATE 接口不在本阶段。

---

## 1. 信息从哪来：静态 + 动态双源

`check_widget_status` 的应答由两个数据源拼装：

| 数据源 | 内容 | 载体 |
| --- | --- | --- |
| 注册表（已有） | `widget_name`、`widget_type`、`parent`、`widget_function` | `ui/id.rs` 的 `DMIRecord` 快照（`list_id()`） |
| 观察快照（新增） | 绑定值、勾选态、选项、Response 可用性位、rect、visible | 新模块 `src/dmi/observe.rs`，组件 `show()` 每帧推送 |

设计理由：绑定数据（文本、勾选、选中值）只有组件在渲染现场才拿得到，接口侧无法反推；
因此采用**组件推送快照**模型——组件 show() 尾部（登记门之后）把一份
`DMIWidgetSnapshot` 写入全局观察表，接口读取最近一份。DMI 代码仍全部位于
`src/dmi/`（observe.rs 归 dmi 目录，组件侧只多一行推送调用，
`id == SYSTEM` 未注册时跳过推送，不产生垃圾快照）。

## 2. 契约定义

### 2.1 观察快照（src/dmi/observe.rs）

```rust
/// 组件运行观察快照：基本信息 + 可用性 + 上下文（最近一次渲染现场）。
/// Per-widget observation: basics, availability flags, context geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct DMIWidgetSnapshot {
    /// 基本信息（按类型封闭集）。 / Type-specific basic information.
    pub basics: DMIBasics,
    /// 可用性位（egui Response 系列）。 / Availability flags from egui Response.
    pub flags: DMIFlags,
    /// 上下文：矩形（x, y, w, h）。 / Context: rect as (x, y, w, h).
    pub rect: (f32, f32, f32, f32),
    /// 上下文：本帧是否可见。 / Context: visible this frame.
    pub visible: bool,
}

/// 基本信息封闭集，与七组件 + 复合子项一一对应。
/// Closed basic-info set: seven widgets plus composite child items.
#[derive(Debug, Clone, PartialEq)]
pub enum DMIBasics {
    Button    { text: String },
    Label     { text: String, tier: &'static str },
    TextInput { text: String, placeholder: String, password: bool },
    Checkbox  { text: String, checked: bool },
    Radio     { text: String, selected: bool },
    Select    { current: String, options: Vec<String> },
    Slider    { label: Option<String>, value: f64, minimum: f64, maximum: f64 },
    /// 下拉子/孙项与单选组成员共用的选项形态。
    /// Shared option shape for select children and radio-group members.
    OptionItem { text: String, selected: bool },
}

/// 可用性位：声明值与上一帧交互事实的合集。
/// Availability: declared enablement plus last-frame interaction facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DMIFlags {
    pub enabled: bool,   // 组件声明的可交互（register 时的 enabled 参数）
    pub hovered: bool,   // Response::hovered
    pub clicked: bool,   // Response::clicked（上一帧）
    pub pressed: bool,   // is_pointer_button_down_on
    pub focused: bool,   // has_focus
    pub changed: bool,   // Response::changed（上一帧数据变更）
}

/// 写入/覆盖快照（组件 show() 尾部调用；SYSTEM 跳过）。
/// Records one observation; unregistered widgets are skipped.
pub fn observe(dmi_id: DMIId, snapshot: DMIWidgetSnapshot)

/// 读取最近快照。 / Reads the latest snapshot for an id.
pub fn snapshot_of(dmi_id: DMIId) -> Option<DMIWidgetSnapshot>
```

存储：`OnceLock<Mutex<HashMap<DMIId, DMIWidgetSnapshot>>>`，与注册表同口径
（进程级、中毒锁恢复、控制台线程读取安全）。

### 2.2 接口实现（src/dmi/interface.rs 追加）

```rust
/// check_widget_status 的 OBSERVATION 实现：三档分层返回组件现场信息。
/// Observation handler for check_widget_status with three detail tiers.
pub fn check_widget_status(command: &DMICommand) -> String
```

执行流程：
1. `request_type != OBSERVATION` → `REQUEST_TYPE_UNSUPPORTED`（ACCESS 留桩至 §3.2）；
2. 注册表查 `target_id`（`list_id()` 中找记录）→ 未注册 →
   `TARGET_ID_INVALID`（文案沿用计划示例：reason "Target id invalid."、
   suggestion "Recheck your id list and call it properly."）；
3. 观察表查快照 → 未命中（组件注册了但尚未渲染，理论防御分支）→
   `TARGET_ID_INVALID`（裁决 3A：错误表不扩容）；
4. 按档位组装（键序固定 `result` 首位，preserve_order 保证）：

| 档位 | 字段 |
| --- | --- |
| simple | `widget_name`、`widget_type`、`widget_text` |
| standard | + `enabled`、`widget_function`（注册表功能映射，数组原样序列化，裁决 1A：`{"apply":["LeftClick","Hover"]}`） |
| complex | + `hovered`、`clicked`、`pressed`、`focused`、`changed`（裁决 2A）、`rect {x,y,w,h}`、`visible` |

`widget_text` 按类型取主文本：按钮/标签/复选/单选/选项 = text；
输入框 = 当前绑定文本；下拉 = current；滑块 = label 文本（无 label 时取数值字符串）。

## 3. 可返回组件信息总清单（本蓝图核心交付）

### 3.1 全组件通用（任何 target_id 均可返回）

| 分类 | 字段 | 来源 | 档位 |
| --- | --- | --- | --- |
| 标识 | `target_id`（回显）、`widget_name`、`widget_type` | 注册表 | simple |
| 基本信息 | `widget_text`（主文本，按类型映射见 §2.2） | 快照 basics | simple |
| 可用性 | `enabled`（声明位） | 快照 flags | standard |
| 能力 | `widget_function`（功能名→触及方式） | 注册表 function | standard |
| 可用性 | `hovered`、`clicked`、`pressed`、`focused`、`changed` | 快照 flags（上一帧事实） | complex（裁决 2A） |
| 上下文 | `rect {x,y,w,h}`、`visible` | 快照 | complex |
| 上下文 | `parent`（上级 id + 触及方式）、层级编号 | 注册表 | 三档均不默认输出（裁决 4A，经 list_id 查询） |

### 3.2 分类型基本信息（快照 basics 可提供）

| 组件 | 可返回基本信息 | 可用性 | 上下文 |
| --- | --- | --- | --- |
| DMIButton | text | 全套 flags | rect/visible |
| DMILabel | text、tier（heading/body/caption/mono） | 全套 flags | rect/visible |
| DMITextInput | **text（绑定当前文本）**、placeholder、password | 全套 flags | rect/visible |
| DMICheckbox | text、**checked** | 全套 flags | rect/visible |
| DMIRadio | text、**selected** | 全套 flags | rect/visible |
| DMISelect | **current（当前值字符串）**、options（候选数组） | 全套 flags | rect/visible |
| DMISlider | label、**value（f64）**、minimum、maximum | 全套 flags | rect/visible |
| 下拉子/孙项 | text（选项文本）、selected | 全套 flags | rect/visible |
| 单选组成员 | text、selected | 全套 flags | rect/visible |

（加粗 = 只有该类型才有、且必须靠组件推送才能拿到的动态值。）

## 4. demo 与端到端验收

- demo 无需新功能（推送内化在组件里）；
- 端到端：控制台管道发
  `{"request_type":"OBSERVATION","interface":"check_widget_status","target_id":1,"complexity":"complex"}`
  → stdout 返回 `save_button` 的完整现场（含 rect/visible/function）。

## 5. 测试计划

1. observe 写读往返 + SYSTEM 跳过（未注册组件不留快照）；
2. 集成：`__run_test_ui` 渲染一个命名按钮 → `call_dmi(check_widget_status)`
   三档逐字段断言（simple 三键、standard 含 enabled+function、complex 含 rect/visible）；
3. 各类型 basics 覆盖：七组件 + 下拉子项各渲染一次，断言 widget_text 映射与
   类型专属字段（checked/selected/current/value/options）；
4. 错误路径：未注册 id → TARGET_ID_INVALID 分层裁剪；ACCESS 分支 → UNSUPPORTED；
5. 键序金串断言（result 首位、字段顺序稳定）。

## 6. 可能存在的问题与对策

| 风险 | 对策 |
| --- | --- |
| 快照陈旧：组件永久消失后最后快照仍在表里 | 本阶段文档化该行为（接口只承诺"最近一次渲染现场"）；帧纪元 GC 留待条目回收专项（Phase 2 蓝图 §7 已列） |
| 每帧推送的字符串拷贝开销 | 拷贝量 = 文本+选项长度，与注册表规模同阶，测试项目可接受；热点出现再按"内容相等跳过写入"优化 |
| 观察表与注册表生命周期不一致 | 登记门先行：只有拿到非 SYSTEM 编号才推送，两表键集天然一致 |
| 控制台线程读表与 UI 线程写表并发 | 同一把 Mutex，中毒恢复口径与 id 层一致 |
| ACCESS 分支先行占位 | 返回 REQUEST_TYPE_UNSUPPORTED，§3.2 落地时替换，协议不变 |

## 7. 本蓝图不做

ACCESS 的导航与点击执行；STATE 写值接口；`parent` 默认输出（裁决 4A，
需要时经 `list_id()` 查询）；`widget_function` 字符串合并形态（裁决 1A 定稿数组）；
快照过期回收。

## 8. 裁决记录（2026-11-24，全部敲定）

1. `widget_function` 序列化 = 数组原样：`{"apply":["LeftClick","Hover"]}`；
2. 可用性位分档：`enabled` 在 standard，其余五位在 complex；
3. 无快照防御分支复用 `TARGET_ID_INVALID`，错误表不扩容；
4. `parent` 三档均不默认输出。
