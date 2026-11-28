//! 运行时控制台：在终端输入和界面绘制循环之间传递命令。
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

/// 线程与界面共享的数据：待处理命令、结果送回通道和超时。
/// Shared core: inbox of (command, reply-sender) plus test-tunable timeout.
struct ConsoleShared {
    /// 待处理命令列表：控制台线程写入，界面每帧结束时取走。
    /// Inbox: pushed by the reader thread, drained at frame start.
    inbox: Mutex<Vec<(String, Sender<String>)>>,
    /// 等待结果返回的最长时间（秒）。 / Reply wait timeout in seconds.
    timeout_seconds: f32,
}

/// 运行时控制台：保存共享数据；控制台线程由 spawn 启动。
/// Runtime console handle; the reader thread starts with `spawn`.
pub struct DMIConsole {
    /// 与控制台线程共享的数据。 / Core shared with the console thread.
    shared: Arc<ConsoleShared>,
}

impl DMIConsole {
    /// 启动控制台线程；没有终端输入时，线程直接安静地结束。
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

    /// 每帧结束时调用：取出全部命令，逐条执行并送回各自的结果。
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

/// 整理一行输入：去掉空白；空行返回 None，不做应答。
/// Normalizes one input line; blank input is skipped silently.
fn normalize_line(line: &str) -> Option<String> {
    let command = line.trim().to_owned();
    (!command.is_empty()).then_some(command)
}

/// 等待一条命令的结果；超时则返回 CONSOLE_TIMEOUT 应答。
/// Waits the paired reply; a timeout becomes a CONSOLE_TIMEOUT response.
fn wait_outcome(receiver: &Receiver<String>, seconds: f32) -> String {
    receiver
        .recv_timeout(Duration::from_secs_f32(seconds))
        .unwrap_or_else(|_| fail_response(&CONSOLE_TIMEOUT, DMIComplexity::Standard))
}

/// 控制台线程主循环：读一行、排队、唤醒界面、等待结果、打印。
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

    /// 创建不启动线程的控制台对象，供队列逻辑测试使用。
    /// Threadless console handle with a controllable timeout.
    fn test_console(timeout_seconds: f32) -> DMIConsole {
        DMIConsole {
            shared: Arc::new(ConsoleShared {
                inbox: Mutex::new(Vec::new()),
                timeout_seconds,
            }),
        }
    }

    /// 模拟终端注入一条命令，返回它的结果接收端。
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

    /// 命令在 pump 调用中全部执行，结果按原顺序送回。
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

    /// 没有执行 pump 时，等待超时会得到 CONSOLE_TIMEOUT。
    /// Unanswered commands resolve to CONSOLE_TIMEOUT via recv timeout.
    #[test]
    fn unanswered_command_times_out() {
        let console = test_console(0.2);
        let receiver = submit(&console.shared, "{\"request_type\":\"OBSERVATION\"}");
        let response = wait_outcome(&receiver, console.shared.timeout_seconds);
        assert!(response.contains("CONSOLE_TIMEOUT"), "{response}");
    }

    /// 空行被忽略；普通行去掉空白后作为命令处理。
    /// Blank lines skip; content lines are trimmed into commands.
    #[test]
    fn blank_lines_are_skipped() {
        assert_eq!(normalize_line("   \n"), None);
        assert_eq!(normalize_line(" {} \r\n"), Some("{}".to_owned()));
    }
}
