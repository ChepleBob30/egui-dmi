# DMI Phase 3 蓝图：运行时接口调用（终端控制台）

依据：`build_docs/plan/dmi.md` §2.2.2 及其四条特别强调。
用户裁决（2026-11-23）：1=串行在途；2=超时 10 秒；3=A 无提示符纯 JSON 行；
4=A 每命令回执通道（据此简化：outbox 与条件变量移除，线程直接 recv_timeout 配对回执）。
范围纪律：控制台只做"终端 ↔ UI 帧循环"的传输层；`call_dmi`、命令契约、greet 接口
全部复用 Phase 2 成果，零改动；ACCESS/STATE 的组件操作语义不在本阶段（控制台先就位）。

---

## 1. 需求 → 设计映射（四条强调逐条对应）

| 计划要求 | 设计落点 |
| --- | --- |
| 独立线程接收输入，不阻塞 UI 渲染 | 控制台线程独占 stdin/stdout；UI 线程从不等待它，只通过共享队列通信 |
| 打出指令后，直接在当帧内完成接口操作 | 命令进 inbox 队列 + `request_repaint` 唤醒；UI 在帧首 `pump()` 一次性清空队列并执行 `call_dmi`，应答同帧进 outbox |
| 输入输出只围绕终端，不包含图形化部分 | 只读写进程 stdin/stdout；GUI 不出现任何控制台面板、状态行或回声 |
| 同时适配人类打字与大模型指令 | 行协议：一行一条 JSON 命令、一行一条 JSON 应答；串行请求-响应（上一条应答前不读下一行），人眼可读、模型可解析、脚本可管道 |

## 2. 线程与数据流

```text
┌ 控制台线程（spawn 时启动，stdin EOF 即退出）────────────────┐
│ 读一行 → 去空白、跳过空行                                    │
│   → 创建回执通道 (tx, rx) → inbox.push((命令, tx))            │
│   → ctx.request_repaint()                                    │
│   → rx.recv_timeout(CONSOLE_TIMEOUT_SECONDS)：                │
│       收到 → println(应答) + flush → 读下一行                  │
│       超时 → println(CONSOLE_TIMEOUT 应答) → 读下一行          │
└──────────────────────────────────────────────────────────────┘
┌ UI 帧循环（eframe）──────────────────────────────────────────┐
│ App::ui 首行 console.pump(ui)：                               │
│   try_lock inbox；空则立即返回（空闲帧零成本）                 │
│   非空 → 整批取出 → 逐条 call_dmi → 应答按序入 outbox →       │
│   notify_all 唤醒控制台线程                                    │
└──────────────────────────────────────────────────────────────┘
```

- **串行在途**：任一时刻至多一条命令在途（上一条应答未出，线程不读下一行），
  请求-响应严格配对、顺序绝对确定——这是行协议对"人类 + 模型"两种操作者
  都最稳的形态（裁决 1 已敲定：串行在途）；
- **当帧完成**：pump 在帧首执行，命令效果（含未来对组件的操作）发生在同一帧；
- **唤醒链**：`request_repaint` 保证空闲界面也会起新帧，指令不会滞留到下次人工交互。

## 3. 契约定义（src/dmi/console.rs）

```rust
/// 控制台应答等待超时（秒），裁决 2 定为 10 秒。
/// Console response timeout in seconds, fixed at ten by ruling 2.
pub const CONSOLE_TIMEOUT_SECONDS: f32 = 10.0;

/// 控制台层错误：UI 未在超时内消费命令（区别于 call_dmi 的命令级错误）。
/// Console-layer failure: the frame loop never consumed the command.
pub const CONSOLE_TIMEOUT: DMIFailure = DMIFailure {
    code: "CONSOLE_TIMEOUT",
    reason: "Console timeout.",
    suggestion: "Check that the app calls DMIConsole::pump each frame.",
};

/// 运行时控制台：终端行协议与 UI 帧循环之间的桥。
/// Runtime console bridging terminal lines and the UI frame loop.
pub struct DMIConsole {
    /// 与控制台线程共享的收件队列（裁决 4A 后仅剩这一个原语）。
    /// Shared inbox queue; the only primitive left after ruling 4A.
    shared: Arc<ConsoleShared>,
}

impl DMIConsole {
    /// 启动控制台线程（持有 Context 用于唤醒重绘）；stdin 不可用时线程静默退出。
    /// Spawns the reader thread; silent exit when stdin is unavailable.
    pub fn spawn(ctx: &egui::Context) -> Self;

    /// 帧首调用：清空收件队列并逐条执行 call_dmi，应答入发件队列并唤醒线程。
    /// Pump at frame start: executes every queued command within this frame.
    pub fn pump(&self, ui: &mut egui::Ui);
}

/// 共享内核：收件箱（命令 + 回执发送端）与测试可缩短的超时。
/// Shared core: inbox of (command, reply-sender) plus test-tunable timeout.
struct ConsoleShared {
    inbox: Mutex<Vec<(String, std::sync::mpsc::Sender<String>)>>,
    timeout_seconds: f32,
}
```

### 3.1 应答配对方式（裁决 4A 定稿）

控制台线程为每条命令创建 `mpsc` 通道，发送端随命令入队、接收端由线程
`recv_timeout` 等待；pump 执行后把应答经配对发送端送回——**应答与命令一一配对**，
发件箱与条件变量因此整体移除，未来放开并行也无需改造队列结构。

### 3.2 错误契约

- 输入行不是合法 JSON、缺字段、非法档位等：全部由 `call_dmi` 现有错误码应答
  （COMMAND_JSON_INVALID 等），控制台层不拦截、不改写、绝不吞行；
- 控制台层唯一新增错误：`CONSOLE_TIMEOUT`（裁决 2 定稿：10 秒超时，
  应答按 standard 分层输出）；
- 空行：静默忽略，不应答（人类回车误触不产生噪声应答）。

## 4. demo 接线（demo/dmi_gallery）

- `main`：`DesignLanguages::install` 之后 `DMIConsole::spawn(&cc.egui_ctx)`，实例移入
  `GalleryApp.console` 字段；
- `App::ui` 首行：`self.console.pump(ui);`；
- 除此之外 GUI 零改动（四条强调之三：输出只围绕终端）。

## 5. 使用样例（人类与模型同形）

```text
$ cargo run -p dmi-gallery
{"request_type":"OBSERVATION","interface":"greet","target_id":0,"complexity":"standard"}
{"result":"OK","greet_text":"hello, egui.","project_name":"egui-dmi"}
{not json
{"result":"FAILED","reason":"Command JSON invalid.","error_code":"COMMAND_JSON_INVALID"}
```

stdout 为纯 JSON 行协议、无提示符（裁决 3A），人类与模型读法一致。
大模型（含 Codex 自身）用法：在持久 exec 会话中启动二进制 → 向会话 stdin 写入
JSON 行 → 从会话输出读取应答行——即"程序运行时实时调用接口"的通道，
无需任何额外网络端口。

## 6. 测试计划

1. 队列内核（不占真实 stdin）：向 inbox 注入命令 → `pump`（`__run_test_ui`）→
   断言应答内容与时序；多条命令同帧全部消费、顺序保持；
2. 非法行经同一链路得到 COMMAND_JSON_INVALID 应答（透传断言）；
3. 超时路径：注入命令后不 pump，回执通道 recv_timeout 以测试短超时
   （0.2s 字段注入）返回 CONSOLE_TIMEOUT 应答；
4. 空行忽略：注入 "  " 不产生应答、不消耗在途名额；
5. demo 端到端冒烟：后台会话启动二进制 → 管道写入 greet 命令 → 断言 stdout
   出现对应 JSON 行 → 结束进程。

## 7. 可能存在的问题与对策

| 风险 | 对策 |
| --- | --- |
| 双线程同写 stdout 造成行交错 | 控制台线程每次 println 持有 stdout 锁完成整行写入；GUI 侧本阶段无 stdout 输出 |
| 无终端启动（macOS 双击/Finder） | 线程读 stdin 立即 EOF 静默退出，UI 功能不受影响；文档标注控制台仅终端启动可用 |
| 忘记调用 pump | 命令积压 → CONSOLE_TIMEOUT 显式报错，协议自解释 |

**修订（2026-11-24）**：pump 位置由帧首改为**帧尾**——端到端实测发现启动竞态：
控制台线程在首帧绘制前即可入队，帧首消费时注册表为空，
针对组件编号的命令被误判 TARGET_ID_INVALID；帧尾消费使命令落在
本帧全部注册与快照之后，"当帧完成"语义更完整且启动竞态自然消除。
| 应答丢失导致线程永等 | recv_timeout 兜底：超时即回 CONSOLE_TIMEOUT，循环继续下一条 |
| 空闲帧 pump 开销 | inbox 用 try_lock + len 判定，空队列时单次原子读返回 |
| 单帧命令积压过多拖慢帧 | 本阶段整批处理并文档标注上限策略留待需求出现（测试项目从简） |
| 锁中毒 | 与 id 层同口径：into_inner 恢复，登记/传输路径均为追加式写入 |

## 8. 本蓝图不做

ACCESS/STATE 对真实组件的操作执行（接口尚无实现，控制台保持传输层无关性）；
网络套接字/多客户端；GUI 内嵌控制台与命令历史；流式大应答；
并发多命令在途（裁决 1：本阶段串行，回执通道已为并行留路）。

## 9. 裁决记录（2026-11-23，全部敲定）

1. 串行在途：上一条应答未出，不读下一行；
2. 超时 10 秒（`CONSOLE_TIMEOUT_SECONDS = 10.0`），错误码 CONSOLE_TIMEOUT 定稿；
3. 无提示符，stdout 纯 JSON 行；
4. 每命令 mpsc 回执通道；outbox 与条件变量随之从设计中移除。
