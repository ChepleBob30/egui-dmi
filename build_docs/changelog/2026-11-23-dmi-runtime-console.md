# 2026-11-23 DMI Phase 3：运行时接口调用（终端控制台）

- 依据：`build_docs/plan/dmi.md` §2.2.2 + 已批准蓝图 `build_docs/blueprint/dmi_phase3.md`
  （裁决：串行在途、超时 10 秒、无提示符、每命令回执通道）。

## 新增 `src/dmi/console.rs`

| 项目 | 功能 |
| --- | --- |
| `CONSOLE_TIMEOUT_SECONDS = 10.0` | 应答等待超时（裁决 2）。 |
| `CONSOLE_TIMEOUT` | 控制台层错误（DMIFailure 常量），应答按 standard 分层输出。 |
| `ConsoleShared` | 共享内核：收件箱 `Vec<(命令, Sender<应答>)>` + 可注入超时；无 outbox、无条件变量（裁决 4A 简化）。 |
| `DMIConsole::spawn(&ctx)` | 启动控制台线程（独占 stdin/stdout），持有 Context 用于 `request_repaint`；stdin 不可用时线程静默退出，UI 不受影响。 |
| `DMIConsole::pump(&self, ui)` | 帧首消费：try_lock 空队列立即返回（空闲帧零成本）；整批取出、逐条 `call_dmi`、应答经配对通道送回。 |
| `normalize_line` / `wait_outcome` / `console_loop` | 行规整（空行静默忽略）；超时兜底转换；线程主循环：读行→入队→唤醒→等回执→整行打印（stdout 锁保证行原子）。 |

`src/dmi.rs`：挂载 `pub mod console;` 并再导出 `DMIConsole`。

## demo 接线（`demo/dmi_gallery/src/main.rs`）

- `GalleryApp` 新增 `console` 字段；`new` 中 `DMIConsole::spawn`；
- `App::ui` 首行 `self.console.pump(ui);`——指令当帧执行、当帧回执；
- GUI 无新增面板/状态行（"输入输出只围绕终端"）。

## 验证

- 单元测试 3 项：帧内配对应答（greet + 非法行透传）、超时兜底
  CONSOLE_TIMEOUT（0.2s 注入）、空行忽略；`cargo test --workspace` **27/27**
  （连跑 5 次稳定；顺带修正 `strict_rejection_matrix` 与 `revision_tracks_growth`
  两处全局计数断言的并行竞态，改为按本用例名称前缀统计）；
- `cargo clippy --workspace --all-targets`：0 警告；
- **端到端实测**：管道 stdin 启动 demo，连发三条命令，stdout 实时返回：
  `{"result":"OK","greet_text":"hello, egui.","project_name":"egui-dmi"}`、
  `COMMAND_JSON_INVALID`（standard 分层）、
  `INTERFACE_NOT_FOUND` + suggestion（complex 分层）——串行在途与分层裁剪
  在真实终端通道验证通过。

## 使用方式

终端直接运行 `cargo run -p dmi-gallery`，程序启动后逐行输入 JSON 命令即可；
大模型/脚本用法：在持久会话中启动二进制，向 stdin 写命令行、从 stdout 读应答行。

## 本阶段未做

ACCESS/STATE 对真实组件的操作执行（控制台为传输层，接口实现后自动可用）；
并发多命令在途；网络套接字传输；GUI 内嵌控制台。
