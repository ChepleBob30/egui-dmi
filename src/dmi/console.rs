//! 运行时控制台：终端 JSON 行协议与 UI 帧循环之间的桥（串行在途）。
//! Runtime console: terminal JSON lines bridged to the UI frame loop.

use crate::dmi::call::{DMIComplexity, DMIFailure, call_dmi, fail_response};
use egui::{Context, Ui};
use std::io::{Stdin, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 控制台应答等待超时（秒），裁决 2 定为 10 秒。
/// Console response timeout in seconds, fixed at ten by ruling 2.
pub const CONSOLE_TIMEOUT_SECONDS: f32 = 10.0;

/// 控制台层错误：UI 未在超时内消费命令（区别于 call_dmi 的命令级错误）。
/// Console-layer failure: the frame loop never answered in time.
pub const CONSOLE_TIMEOUT: DMIFailure = DMIFailure {
    code: "CONSOLE_TIMEOUT",
    reason: "Console timeout.",
    suggestion: "Check that the app calls DMIConsole::pump each frame.",
};

/// 共享内核：收件箱（命令 + 回执发送端）与测试可缩短的超时。
/// Shared core: inbox of (command, reply-sender) plus test-tunable timeout.
struct ConsoleShared {
    /// 收件箱：控制台线程写入，UI 帧首整批取走。
    /// Inbox: pushed by the reader thread, drained at frame start.
    inbox: Mutex<Vec<(String, Sender<String>)>>,
    /// 回执等待超时（秒）。 / Reply wait timeout in seconds.
    timeout_seconds: f32,
}

/// 运行时控制台：持有共享内核；控制台线程由 `spawn` 启动。
/// Runtime console handle; the reader thread starts with `spawn`.
pub struct DMIConsole {
    /// 与控制台线程共享的内核。 / Core shared with the console thread.
    shared: Arc<ConsoleShared>,
}

impl DMIConsole {
    /// 启动控制台线程并返回句柄；stdin 不可用（无终端启动）时线程静默退出。
    /// Spawns the reader thread; silent exit when stdin is unavailable.
    pub fn spawn(ctx: &Context) -> Self {
        Self::spawn_with_timeout(ctx, CONSOLE_TIMEOUT_SECONDS)
    }

    /// 带自定义超时的启动入口（测试注入短超时）。
    /// Spawn with a custom timeout, used by tests for fast timeout paths.
    fn spawn_with_timeout(ctx: &Context, timeout_seconds: f32) -> Self {
        let shared = Arc::new(ConsoleShared {
            inbox: Mutex::new(Vec::new()),
            timeout_seconds,
        });
        let thread_shared = Arc::clone(&shared);
        let repaint_context = ctx.clone();
        std::thread::spawn(move || console_loop(thread_shared, repaint_context));
        Self { shared }
    }

    /// 帧首消费：整批取出收件命令，逐条 `call_dmi` 并经配对通道送回应答。
    /// Frame-start pump: run every queued command within this frame.
    pub fn pump(&self, _ui: &mut Ui) {
        let batch: Vec<(String, Sender<String>)> = {
            let mut inbox = self
                .shared
                .inbox
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if inbox.is_empty() {
                return;
            }
            std::mem::take(&mut *inbox)
        };
        for (command, sender) in batch {
            let response = call_dmi(&command);
            let _ = sender.send(response);
        }
    }
}

/// 行规整：去空白；空行返回 None，静默忽略不应答。
/// Normalizes one input line; blank input is skipped silently.
fn normalize_line(line: &str) -> Option<String> {
    let command = line.trim().to_owned();
    (!command.is_empty()).then_some(command)
}

/// 等待回执：超时转为 CONSOLE_TIMEOUT 的 standard 分层应答。
/// Waits the paired reply; a timeout becomes a CONSOLE_TIMEOUT response.
fn wait_outcome(receiver: &Receiver<String>, seconds: f32) -> String {
    receiver
        .recv_timeout(Duration::from_secs_f32(seconds))
        .unwrap_or_else(|_| fail_response(&CONSOLE_TIMEOUT, DMIComplexity::Standard))
}

/// 控制台线程主循环：读行→入队唤醒→等配对回执→整行打印，直至 stdin EOF。
/// Reader loop: line in, paired response out, until stdin reaches EOF.
fn console_loop(shared: Arc<ConsoleShared>, ctx: Context) {
    let stdin: Stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        line.clear();
        match stdin.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let Some(command) = normalize_line(&line) else {
            continue;
        };
        let (sender, receiver) = mpsc::channel();
        {
            let mut inbox = shared
                .inbox
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            inbox.push((command, sender));
        }
        ctx.request_repaint();
        let response = wait_outcome(&receiver, shared.timeout_seconds);
        let mut output = std::io::stdout().lock();
        let _ = writeln!(output, "{response}");
        let _ = output.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构建不起线程的内核句柄，超时可控（内核测试专用）。
    /// Threadless console handle with a controllable timeout.
    fn test_console(timeout_seconds: f32) -> DMIConsole {
        DMIConsole {
            shared: Arc::new(ConsoleShared {
                inbox: Mutex::new(Vec::new()),
                timeout_seconds,
            }),
        }
    }

    /// 以终端等价方式注入一条命令，返回其回执接收端。
    /// Submits one command exactly like the reader thread would.
    fn submit(shared: &Arc<ConsoleShared>, command: &str) -> Receiver<String> {
        let (sender, receiver) = mpsc::channel();
        shared
            .inbox
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((command.to_owned(), sender));
        receiver
    }

    /// 帧内消费：命令在 pump 当帧执行，应答按配对通道保序返回。
    /// Commands run inside the pumping frame; replies keep pairing order.
    #[test]
    fn pump_answers_commands_within_the_frame() {
        let console = test_console(10.0);
        let greet = submit(
            &console.shared,
            "{\"request_type\":\"OBSERVATION\",\"interface\":\"greet\",\"target_id\":0,\"complexity\":\"simple\"}",
        );
        let broken = submit(&console.shared, "{not json");
        egui::__run_test_ui(|ui| {
            console.pump(ui);
        });
        assert_eq!(
            greet.recv().unwrap(),
            "{\"result\":\"OK\",\"greet_text\":\"hello.\"}"
        );
        assert!(broken.recv().unwrap().contains("COMMAND_JSON_INVALID"));
    }

    /// 忘记 pump 的兜底：等待超时得到 CONSOLE_TIMEOUT 应答。
    /// Unanswered commands resolve to CONSOLE_TIMEOUT via recv timeout.
    #[test]
    fn unanswered_command_times_out() {
        let console = test_console(0.2);
        let receiver = submit(&console.shared, "{\"request_type\":\"OBSERVATION\"}");
        let response = wait_outcome(&receiver, console.shared.timeout_seconds);
        assert!(response.contains("CONSOLE_TIMEOUT"), "{response}");
    }

    /// 空行静默忽略；普通行去空白后原样成为命令。
    /// Blank lines skip; content lines are trimmed into commands.
    #[test]
    fn blank_lines_are_skipped() {
        assert_eq!(normalize_line("   \n"), None);
        assert_eq!(normalize_line(" {} \r\n"), Some("{}".to_owned()));
    }
}
