# DMI Phase 2 蓝图：统一调用接口、greet 示例与 DMIUpdate 纯数据化（硬编码调用）

依据：`build_docs/plan/dmi.md` §2.1、§2.2（仅硬编码）、§2.3、§2.4；
用户已批准六项决定：失败分层严格按 §2.4；greet 拒绝非 0 号；complexity 非法即失败；
入口签名 `&str -> String`；文件切分为 call.rs（接口总调用区）+ interface.rs（各接口具体实现区），
不使用 mod.rs；扩展 demo 内容。
范围纪律：本阶段只交付 `call_dmi` 统一入口、`greet` 一个接口、`DMIUpdate` 去 Response 化；
运行时控制台、Rules 抽象层、ACCESS/STATE 语义、组件寻址接口留待后续阶段。

---

## 1. 文件结构

```text
src/
├── lib.rs                # 追加 pub mod dmi;
├── dmi.rs                # 模块根（约10行）：声明子模块 + 再导出 call_dmi 与契约类型。
│                         #   现行 Rust 风格弃用 mod.rs，根文件与目录并存；
│                         #   实现代码全部位于 src/dmi/ 目录，满足 plan §2.1。
└── dmi/
    ├── call.rs           # 接口总调用区：命令契约类型、解析、应答构造器、分发器 call_dmi。
    └── interface.rs      # 各接口具体实现区：greet 实现；后续接口在本文件分区追加或另立文件。
```

依赖追加：`serde_json = { version = "1", features = ["preserve_order"] }`
（preserve_order 保证应答键序按插入输出、`result` 恒在首位；serde_json 与 indexmap
均已在本地 crates 缓存，无离线构建风险）。

## 2. 契约定义（call.rs）

### 2.1 请求侧类型

```rust
/// 请求类型：DMI 接口三大类（§2.3 语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DMIRequestType { Access, State, Observation }

/// 返回值细节档位：simple < standard < complex（§1.1，有序可比）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DMIComplexity { Simple, Standard, Complex }

/// 一条已解析的 DMI 调用命令。
#[derive(Debug, Clone, PartialEq)]
pub struct DMICommand {
    pub request_type: DMIRequestType,   // "ACCESS"/"STATE"/"OBSERVATION"（大写）
    pub interface: String,              // 接口名，如 "greet"
    pub target_id: DMIId,               // 数字或字符串：0→SYSTEM，1.2→{1,2}
    pub complexity: DMIComplexity,      // "simple"/"standard"/"complex"（小写）
}
```

解析入口（私有，公开面只有 `call_dmi`）：

```rust
/// JSON 字符串 → 命令；字段缺失/类型不符 → COMMAND_JSON_INVALID；
/// request_type 非三类大写值 → REQUEST_TYPE_INVALID；
/// complexity 字段存在但值非法 → COMPLEXITY_INVALID（决定3：不做宽容缺省）。
/// 签名：fn parse_command(raw: &str) -> Result<DMICommand, DMIFailure>
```

`target_id` 解析：接受 JSON 数字（`0`、`1.2`、`1.25`）或字符串（`"1.2"`、`"1.2.5"`）；
实现走"字符串化后按小数点拆分"，避免 f64 减法精度问题。形态约定（编号机制升级后，
见 `dmi_phase2_id.md`）：**数字形态只表达一级小数**，小数位数字整体作为该层数值
（`1.25` 即 `1.25` 号，不是 `1.2.5`）；**两级及以上小数必须使用点分字符串**
（`"1.2.5"`），总层数不得超过 `DMI_MAX_DEPTH`。非法形态（超层数、非数字段、负数）
报 `TARGET_ID_INVALID`。函数 `parse_target_id(&serde_json::Value) -> Option<DMIId>`
置于 call.rs，多级 `DMIId` 表示本身由 `ui/id.rs` 定义。

### 2.2 应答侧构造（集中两个函数，禁止散落拼串）

```rust
/// 成功应答：键值对按传入顺序写入，result 恒首位。
/// 签名：fn ok_response(pairs: Vec<(String, serde_json::Value)>) -> String

/// 失败应答：按命令档位裁剪字段（见 2.4）。
/// 签名：fn fail_response(failure: &DMIFailure, complexity: DMIComplexity) -> String

/// 失败描述：错误码 + 完整句失败原因 + 修复建议。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DMIFailure {
    pub code: &'static str,        // 如 "INTERFACE_NOT_FOUND"
    pub reason: &'static str,      // 如 "Interface not found."
    pub suggestion: &'static str,  // 如 "Check the interface name against list_id output."
}
```

### 2.3 错误码总表（本阶段全部错误类型）

| code | reason | 触发条件 | suggestion（complex 档附带） |
| --- | --- | --- | --- |
| COMMAND_JSON_INVALID | Command JSON invalid. | 非法 JSON / 缺字段 / 类型不符 | Send a full JSON object with request_type, interface, target_id, complexity. |
| REQUEST_TYPE_INVALID | Request type unknown. | request_type 非三类大写值 | Use ACCESS, STATE or OBSERVATION in uppercase. |
| COMPLEXITY_INVALID | Complexity value invalid. | complexity 字段存在但值非法 | Use simple, standard or complex in lowercase. |
| TARGET_ID_INVALID | Target id invalid. | target_id 无法解析为编号 | Use an integer or dotted id such as 0 or 1.2. |
| INTERFACE_NOT_FOUND | Interface not found. | interface 无匹配实现 | Check available interfaces; greet is registered. |
| REQUEST_TYPE_UNSUPPORTED | Interface mode unsupported. | 对 greet 使用 ACCESS/STATE | Call greet with request_type OBSERVATION. |
| TARGET_ID_MISMATCH | Target id not accepted. | greet 收到非 0 号（决定2） | System interfaces take target id 0. |

### 2.4 失败应答分层（决定1：严格按 §2.4 原文）

| 输入 complexity | FAILED 应答字段 |
| --- | --- |
| simple | 仅 `{"result":"FAILED"}`（standard 及更高才附带失败原因） |
| standard | + `reason` + `error_code` |
| complex | 再 + `suggestion` |

命令解析失败时无法得知请求档位，统一按 **standard** 输出（错误可被模型读到，
又不至于给非法输入编造 complex 内容）；该兜底在文档与代码注释中双重标注。

### 2.5 分发器（唯一公开入口，决定4）

```rust
/// DMI 统一调用入口：接收完整 JSON 字符串，返回应答 JSON 字符串。
/// The single DMI entry point: JSON in, JSON out.
pub fn call_dmi(command: &str) -> String
```

流程：`parse_command` → 失败按 2.4 兜底档位回 FAILED；成功则
`match command.interface.as_str()` 分发：`"greet" => interface::greet(&command)`；
`_ => INTERFACE_NOT_FOUND`。后续接口 = match 增一行 + interface.rs 增一个函数，
分发器与契约层不再变动。

## 3. greet 接口（interface.rs，§2.4 行为表）

```rust
/// greet 的 OBSERVATION 实现：系统级欢迎与自检信息，target_id 固定 0。
/// 校验顺序：request_type 必须 OBSERVATION → target_id 必须 SYSTEM → 按档位组装。
pub fn greet(command: &DMICommand) -> String
```

| complexity | 成功应答 |
| --- | --- |
| simple | `{"result":"OK","greet_text":"hello."}` |
| standard | `{"result":"OK","greet_text":"hello, egui.","project_name":"egui-dmi"}` |
| complex | `{"result":"OK","greet_text":"hello, egui-dmi.","project_name":"egui-dmi","dependencies":"egui 0.37.0"}` |

数据来源：`project_name` 用编译期 `env!("CARGO_PKG_NAME")`；
`dependencies` 用常量字符串（硬编码调用阶段不做运行时依赖枚举）。
"取不到值"类分支（如 PROJECT_NAME_NOT_FOUND）在编译期常量实现下不可达，
错误码语义保留给后续真实接口，不为其编写死代码。

## 4. DMIUpdate 纯数据化重构（components.rs，替换 Response）

### 4.1 新契约

```rust
/// 组件 show() 的统一返回：纯 DMI 数据记录，不再携带 egui::Response。
/// Shared widget return as pure DMI data; the egui Response stays inside show().
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DMIUpdate {
    /// 组件 DMI 编号（JSON target_id 来源）。 / DMI id, the JSON target_id source.
    pub id: DMIId,
    /// 本次交互是否被点击。 / Whether the widget was clicked this frame.
    pub clicked: bool,
    /// 本次交互是否改变了绑定数据。 / Whether bound data changed this frame.
    pub changed: bool,
}

impl DMIUpdate {
    /// 由编号与两个状态位构造。 / Build from id and the two state flags.
    pub fn new(id: DMIId, clicked: bool, changed: bool) -> Self
    /// 从 egui 响应提取状态位（show() 内部私有转换，出口不外泄 Response）。
    /// Private helper: extracts flags before the response is dropped.
    fn from_parts(id: DMIId, response: &egui::Response) -> Self
}
```

访问方式由方法调用改为直接字段（`update.clicked` / `update.changed`），
旧 `clicked()/changed()` 方法删除——调用点仅 demo 与测试，改动清单见 4.3。

### 4.2 设计论证（为何无实现问题）

- 七个组件在 `show()` 内部使用 Response 的三处场景——tooltip 链式挂载、
  按钮四态着色（hovered/is_pointer_button_down_on）、登记用 `response.id`——
  全部发生在构造返回值之前，出口只需带走布尔结果；
- `DMISelect` 原本携带弹出层 InnerResponse，同样在 show() 内消化完毕；
- 纯数据记录可 Copy、可比对、可序列化，正是未来运行时调用把交互结果
  转成 JSON 回传模型的形状；
- 后续若需要 hovered/pressed/focused/rect 等更细状态，按接口需要增补字段即可，
  测试项目不承担历史兼容负担。

### 4.3 迁移清单

- `components.rs`：DMIUpdate 结构体与构造器；七个 show() 尾部改为
  `DMIUpdate::new(...)` / `from_parts(...)`；
- `demo/dmi_gallery/src/main.rs`：`update.clicked()` → `update.clicked`，
  `update.changed()` → `update.changed`（约 12 处）；
- `components.rs` 内测试、`docs/design-system.md`、`docs/id-system.md` §4 同步措辞。

## 5. demo 扩展（决定6：画廊新增 DMI 自检分区）

`show_dmi_probe_section`（约 35 行）：

- 三个按钮：`DMIButton::secondary("greet simple")` / `("greet standard")` /
  `("greet complex")`，分别 `dmi_name("dmi_probe_simple"/"dmi_probe_standard"/"dmi_probe_complex")`，
  点击时构造对应 JSON 字符串调用 `egui_dmi::dmi::call_dmi(...)`，
  返回串写入状态字段 `probe_output: String` 并 push_log；
- 第四个按钮 `("greet 非法请求")`（`dmi_name("dmi_probe_failure")`）发送
  OBSERVATION 之外的 request_type，演示失败分层；
- 输出区：`DMILabel::mono(&self.probe_output)`（匿名，不登记）实时显示最近一次应答；
- 分区标题 `DMILabel::heading("DMI 自检（硬编码调用）").dmi_name("section_probe_label")`。

该分区即 §2.2.1 硬编码调用的活体示例：按钮点击 = 代码内固定 JSON 调用。

## 6. 测试计划

1. greet × simple/standard/complex：字段集合与键序逐一对齐 §2.4 示例；
2. 失败矩阵：非法 JSON、缺字段、小写 request_type、非法 complexity 值、
   `target_id: 1.2` 对 greet、`"interface":"unknown_one"`、greet + ACCESS；
   断言档位裁剪（simple 仅 result；standard 有 reason+error_code；complex 有 suggestion）
   与解析失败的 standard 兜底；
3. `parse_target_id` 对 0 / 1.2 / "1.2" / 1.25（单级 25）/ "1.2.5"（两级）/
   "1.2.3.4.5"（超限）/ -1 / "abc" 的接受与拒绝；
4. 应答键序：`result` 恒为第一键（preserve_order 生效）；
5. DMIUpdate：`button_update_defaults_idle` 改字段断言；
   新增 `dmi_update_is_pure_data`（Copy + PartialEq 可用，无 Response 字段泄漏编译验证）。

## 7. 可能存在的问题与对策

| 风险 | 对策 |
| --- | --- |
| JSON 数字 1.2 浮点精度 | 字符串化拆分小数点，限两位小数，不用 f64 减法 |
| 键序不稳定影响断言 | preserve_order + 测试直接比对完整字符串 |
| 解析失败时无档位信息 | 兜底 standard 输出并在注释与本文档标注 |
| DMIUpdate 失去 Response 高级查询 | 按需增补布尔/几何字段（4.2），不回引 Response |
| 后续接口膨胀 | match 一行 + interface.rs 一函数；契约层冻结 |

## 8. 明确不做（本阶段纪律）

运行时终端控制台与线程接入；Rules 抽象层与标签通道；ACCESS/STATE 实现；
按 target_id 操作真实组件；greet 之外的接口；`src/dmi/` 读写编号注册表。
