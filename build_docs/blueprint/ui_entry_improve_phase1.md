# UI 触及方法改进蓝图（Phase 1）：UIEntry 声明与 DMIRecord 升级

依据：`build_docs/plan/ui_entry_improve.md` §1、§2。
用户裁决（2026-11-22）：开放问题 1=A（`UIEntry::Direct` 变体）；2=A（`DMIRecord.entries` 字段）；
3=B（首声明优先，复用命中不刷新）；4=C（文本框不声明键入，不设 Typing 变体）；
5=严格检查（2026-11-22 追加）：取消匿名组件与默认触及方法，注册必须同时显式声明
名称与激活列表，缺一即拒绝注册（返回 SYSTEM 哨兵，不渲染占位、不入表、不 panic）。
6=功能映射（2026-11-22 追加，计划 §1 扩展）：`DMIRecord.entries` 扁平列表升级为
`function` 功能映射——每个功能名对应一组触及方式，调用接口只声明功能名；
本条取代裁决 2A 的字段形态，`dmi_entry()` 名称保留（计划 §2），签名改为
（功能名, 激活列表）；Access 消费 function 的 dmi 逻辑由用户明示留待后续阶段。
范围纪律：本阶段只做"触及方法的类型、注册期声明、入表与展示"；
Access 接口的实际执行（模拟点击/聚焦/按键）属于后续 dmi 阶段，不在本蓝图。
egui 0.37 能力核对：`clicked/clicked_by/secondary_clicked/middle_clicked/
double_clicked/hovered/drag_started/has_focus/gained_focus/request_focus`
与 `egui::Key` 全部存在——计划中的 UIEntry 变体集合可原样落地。

---

## 1. UIEntry 契约（ui.rs，计划指定位置）

```rust
/// 组件触及方式：注册期声明，DMI Access 阶段据此驱动组件。
/// Widget touch entry: declared at registration, driven later by Access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UIEntry {
    /// 左键单击（Response::clicked / clicked_by(Primary)）。 / Primary button click.
    LeftClick,
    /// 右键单击（secondary_clicked，兼作上下文菜单触发）。 / Secondary button click.
    RightClick,
    /// 中键单击（middle_clicked）。 / Middle button click.
    CenterClick,
    /// 双击（double_clicked）。 / Double click.
    DoubleClick,
    /// 悬停（hovered，tooltip 类反馈）。 / Pointer hover.
    Hover,
    /// 拖拽（drag_started/dragged/drag_stopped）。 / Pointer drag.
    Drag,
    /// 键盘焦点（gained_focus/has_focus/request_focus）。 / Keyboard focus.
    Focus,
    /// 聚焦后按键激活（如空格勾选、方向键移滑块）。 / Key press while focused.
    PressKey(egui::Key),
    /// 根组件专用：无上级、直接可见（记录 parent 元组的占位方式）。
    /// Root-only placeholder: visible without a parent step.
    Direct,
}
```

设计理由：
- 变体只收录 egui 0.37 有对应 Response API 的行为，声明即可执行，不虚设；
- `Copy + PartialEq + Eq`：单个触及值可自由比较与拷贝，function 映射仅在入表时拷成 owned；
- `PressKey(egui::Key)` 直接复用 egui 键枚举，不造第二个键类型；
- `Direct` 仅用于根组件 `parent` 元组占位（裁决 1A），保持计划给定的 `(DMIId, UIEntry)`
  类型不变；它不作为任何功能项的激活方式出现在 function 映射里，登记接口对此拒绝；
- 暂不派生 serde（JSON 序列化留给 Access 阶段，避免提前绑定格式）。

## 2. DMIRecord 升级（ui/id.rs）

按计划改造 parent 字段，并新增 function 功能映射（§2 的 dmi_entry 声明入表处，裁决 6 定稿）：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DMIRecord {
    pub id: DMIId,
    pub kind: DMIWidgetKind,
    /// (上级编号, 从上级触及本组件的方式)；根组件上级为 0 号。
    /// Parent id plus the entry that reaches this child from the parent.
    pub parent: (DMIId, UIEntry),
    pub name: String,
    /// 功能映射：功能名 → 实现该功能的触及方式列表（注册期声明）。
    /// Function map: capability name to the entries realizing it.
    pub function: Vec<(String, Vec<UIEntry>)>,
}
```

**类型适配说明**：计划写的是 `&[(String, &[UIEntry])]`；记录需长期持有数据，
借用切片无法自持生命周期，存储形态定为等价的 `Vec<(String, Vec<UIEntry>)>`，
构建与登记接口仍接收 `&[(&str, &[UIEntry])]` 借用参数（调用侧零所有权负担）。

Display 形态随之扩展：
`(1.1, "select", (1, LeftClick), "theme_selector_light", [("pick", [LeftClick])])`。

根组件的触及方式（裁决 1A，已敲定）：`parent = (DMIId::SYSTEM, UIEntry::Direct)`，
Display 形态 `(1, "button", (0, Direct), "save_button", [LeftClick, Hover])`；
`function` 字段承载注册期功能映射（裁决 6 取代裁决 2A 的字段形态），`list_id()` 即为唯一事实源。

## 3. 注册接口变更（ui/id.rs）

```rust
/// 登记根组件：名称与功能映射缺一即拒绝（返回 SYSTEM，不入表）。
/// Registers a root widget; missing name or function map is refused outright.
pub fn allocate(
    egui_id: Id,
    kind: DMIWidgetKind,
    name: &str,
    function: &[(&str, &[UIEntry])],
) -> DMIId

/// 登记子组件：同样强制名称与功能映射；touch 为从父触及子的方式；
/// 父号必须已在表中登记，否则拒绝（返回 SYSTEM）。
/// Registers a child; the parent must already exist or the call is refused.
pub fn allocate_child(
    egui_id: Id,
    kind: DMIWidgetKind,
    name: &str,
    parent: DMIId,
    touch: UIEntry,
    function: &[(&str, &[UIEntry])],
) -> DMIId
```

**严格校验规则（裁决 5 与 6，全部在 id.rs 内核执行）**：
1. `name` 非空、`function` 非空——缺一即拒绝；
2. 每个功能项：功能名非空、其激活列表非空、列表不含 `Direct`——任一不满足整体拒绝；
3. 同一组件内功能名不得重复，重复视为声明混乱，整体拒绝；
4. `allocate_child` 额外要求 `parent` 已存在于注册表（组件实现保证先父后子的顺序）；
3. 拒绝的唯一后果：返回 `DMIId::SYSTEM` 哨兵、不产生条目、不 panic——
   未声明 DMI 身份的组件照常渲染，只是不归 DMI 管理；
4. `DMIId::SYSTEM` 由此成为公开语义："该组件未注册"，调用方据此判断。

**匿名键整体移除**：`ReuseKey` 枚举删除，`reused: HashMap<String, DMIId>`——
名称成为唯一身份键，"位置键 + 自动名 `{kind}-{major}`"两种旧机制一并作废；
`locate`、深度守卫、快照/版本、首声明策略（裁决 3B）不受影响。

- 复用命中路径的 function 策略（裁决 3B，已敲定）：**首声明优先**——
  记录创建时写入功能映射，此后同名复用只刷新定位（`located`），不改写映射。
  推论与纪律：同一组件名称的功能集合以首次登记为准，调用方修改声明需换新名称，
  该语义换取"注册表内容对给定名称恒定"的可预测性；
- reuse 键、发号器、深度守卫、快照/版本、locate 通道全部不变——
  parent 元组化只影响记录存储与 Display，不影响身份判定；
- `name_of/locate/registry_revision/list_id` 签名不变。

## 4. 组件侧 dmi_entry（ui/components.rs）

七个组件统一新增构建方法与字段：

```rust
/// 追加一项功能声明：功能名 + 实现它的触及方式列表（可多次调用）。
/// Declares one capability with the entries realizing it; repeatable.
pub fn dmi_entry(
    mut self,
    function: impl Into<String>,
    entries: &[UIEntry],
) -> Self   // 内部 function: Vec<(String, Vec<UIEntry>)> 字段
```

**登记出口统一形态（裁决 5 与 6）**：`dmi_name` 与非空 `function` 映射齐备才登记，
缺一即 `DMIUpdate.id == DMIId::SYSTEM` 且不入表：

```rust
// 每个组件 show() 尾部的登记分支（以按钮为例）。
// The registration gate shared by all components (button shown).
let dmi_id = match &self.dmi_name {
    Some(given) if !self.function.is_empty() => {
        let declared: Vec<(&str, &[UIEntry])> = self.function.iter()
            .map(|(name, entries)| (name.as_str(), entries.as_slice()))
            .collect();
        id::allocate(response.id, DMIWidgetKind::Button, given, &declared)
    }
    _ => DMIId::SYSTEM,
};
```

**功能参考表（纯文档，不进代码——裁决 5 取消内置默认，未声明即拒绝）**；
功能名是应用层语义，下表给出框架建议命名，仍遵守"声明即能力"纪律：

| 组件 | 功能名 → 触及列表（建议） | 依据 |
| --- | --- | --- |
| DMIButton | "press" → [LeftClick]；"hover" → [Hover] | 自绘路径仅主键点击 + hover |
| DMILabel | "hover" → [Hover] | tooltip 悬停；命名标签无主动交互 |
| DMITextInput | "focus" → [LeftClick, Focus] | 点击聚焦为实；键入不声明（裁决 4C） |
| DMICheckbox | "toggle" → [LeftClick, Focus, PressKey(Space)] | egui Checkbox 聚焦空格激活 |
| DMIRadio | "select" → [LeftClick, Focus, PressKey(Space)] | egui RadioButton 同上 |
| DMISelect（父） | "expand" → [LeftClick, Hover] | 弹出层由左键开合 |
| DMISelect（子/孙） | touch=LeftClick；"pick" → [LeftClick] | 弹出层内点选 |
| DMIRadio group（子） | touch=LeftClick；"select" → [LeftClick] | 组成员点选 |
| DMISlider | "set" → [LeftClick, Drag, Focus, PressKey(Left), PressKey(Right)] | egui Slider 方向键步进 |

子项/孙项的 touch 与 function 由**组件实现代码在登记时显式传入**（DMISelect、
DMIRadio::group 内部声明 `"pick"`/`"select"` 功能），属于实现方显式声明、
而非调用方可省略的缺省，与裁决 5 不冲突；面向应用层的强制声明只约束
`dmi_name`/`dmi_entry` 两个构建方法。调用接口（Access）阶段，DMI 按功能名查表，
再按该功能的触及列表执行操作——"调用时只需声明功能名"即此语义，
dmi 逻辑改造由用户明示留待后续。
| DMISlider | [LeftClick, Drag, Hover, Focus, PressKey(Left), PressKey(Right)] | egui Slider 方向键步进 |

文本输入框的键入表达（裁决 4C，已敲定）：不新增 Typing 变体、不枚举按键，
DMI 对文本框的写入通道留给 STATE 接口的值设置语义，触及列表只声明
`[LeftClick, Focus]` 两项当前实现确实检测的行为。

## 5. 迁移清单

| 位置 | 改动 |
| --- | --- |
| `src/ui.rs` | UIEntry 枚举定义 + 再导出（DMIRecord 消费方经 ui::id 路径） |
| `src/ui/id.rs` | DMIRecord 字段变更（parent 元组 + function 映射）；allocate/allocate_child 签名扩展与校验内核；Display；既有测试断言适配 |
| `src/ui/components.rs` | 七组件 `dmi_entry(功能名, 列表)` 构建方法与 function 字段；双参数登记门（缺一返回 SYSTEM 不注册）；select/radio-group 子项由实现代码显式传 touch 与 function；测试适配 |
| `demo/dmi_gallery` | 全部交互组件补 `dmi_entry` 显式声明（当前 demo 组件均已有 dmi_name，缺一侧即被新规则拒绝）；注册表行文本随 Display 自动变化，缓存机制不动 |
| `src/dmi/*` | 无接口变化；`greet` 等系统接口与本裁决无关（0 号不注册） |
| `docs/id-system.md`、`docs/design-system.md` | 契约表、Display 形态、参考声明表与双参数强制规范同步 |

## 6. 测试计划

1. 根登记：function 映射入表、`parent = (SYSTEM, Direct)`、Display 金串比对；
2. 子登记：select 子项 parent 元组 `(父号, LeftClick)`、实现声明的 `"pick"` 功能入表；
   同名不同 function 复登记断言映射保持首声明不变（裁决 3B）；
3. 组件双参数登记：按参考表显式声明的 button/checkbox/slider 渲染后，
   list_id 断言 function 映射与声明逐项相等；
4. `dmi_entry("save", &[LeftClick])` 多次追加：功能项顺序稳定、各自列表独立；
5. function 往返：登记 `[("save",[LeftClick]),("hover",[Hover])]` 后
   list_id 断言映射逐项相等、顺序保持、Display 金串比对；
6. **拒绝路径矩阵（裁决 5 与 6 专项）**：空名称、空 function、功能项空列表、
   功能名重复、触及列表含 Direct、子登记父号不存在——六种输入均返回 `SYSTEM`、
   表长度不变、版本不变；
7. 组件出口双参数：未调 `dmi_name` 或未调 `dmi_entry` 的组件渲染后
   `DMIUpdate.id.is_system()` 为真且 list_id 无对应条目；
8. 既有测试适配：所有组件测试补 `dmi_entry` 功能声明；原"匿名 label- 前缀零登记"
   断言语义并入第 7 项统一覆盖；
9. 回归：reuse 不增长、深度守卫、版本门控全部在新字段下通过。

## 7. 风险与对策

| 风险 | 对策 |
| --- | --- |
| 声明与能力脱节（声明了未检测的行为） | 参考表严格对齐现有检测；扩展检测（右键/双击）留给 Access 阶段，届时同步改表 |
| 未声明组件静默脱离 DMI 管理 | 拒绝注册是显式语义（id=SYSTEM 可判）；demo 与测试全量声明，文档把"双参数强制"列为接入规范 |
| 旧匿名位置键移除影响面 | ReuseKey 为私有类型、自动名仅存在于测试断言；删除后 reuse 语义更简单（名称唯一键） |
| DMIRecord 公共字段类型破坏 | 消费方仅 demo/tests/docs，全量迁移；无持久化数据 |
| function 传参与拷贝开销 | 声明侧借用参数（`&[(&str, &[UIEntry])]`），仅首次入表时拷成 owned 映射（裁决 3B 首声明优先），复用命中零拷贝零改写 |
| 计划借用形态与记录自持冲突 | 存储用 Vec 等价形态、接口保留借用参数，语义一致（§2 类型适配说明） |
| UIEntry 未来 JSON 化 | 变体名即字符串形态（"LeftClick"），Access 阶段再补 serde 派生 |

## 8. 本蓝图不做

Access 接口的触及执行；运行时控制台；UIEntry 的 serde 绑定；
组件新增右键/双击等实际检测能力；注册表条目回收。
