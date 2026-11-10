//! 设计语言 - 动画令牌：状态过渡时长与缓动辅助。
//! Design language motion tokens: transition durations and easing.

use egui::{Context, Id};

/// 动画时长令牌，统一控件状态过渡节奏。
/// Duration tokens unifying widget state transitions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionTokens {
    /// 悬停渐变时长（秒）。 / Hover fade duration in seconds.
    pub hover_seconds: f32,
    /// 按下渐变时长（秒）。 / Press fade duration in seconds.
    pub press_seconds: f32,
    /// 焦点环渐变时长（秒）。 / Focus ring fade duration in seconds.
    pub focus_seconds: f32,
    /// 勾选/展开切换时长（秒）。 / Toggle/expand duration in seconds.
    pub toggle_seconds: f32,
}

impl MotionTokens {
    /// 默认动画节奏。 / Default motion schedule.
    pub const DEFAULT: Self = Self {
        hover_seconds: 0.10,
        press_seconds: 0.06,
        focus_seconds: 0.15,
        toggle_seconds: 0.12,
    };

    /// 二次缓出曲线 t*(2-t)，输入0..1。 / Quadratic ease-out on unit input.
    pub fn ease_out_quad(t: f32) -> f32 {
        t * (2.0 - t)
    }

    /// 布尔状态渐变；未结束则请求下一帧重绘。
    /// Animates a boolean; requests repaint while transition is running.
    pub fn animate(&self, ctx: &Context, id: Id, on: bool, seconds: f32) -> f32 {
        let progress = ctx.animate_bool_with_time(id, on, seconds);
        if progress > 0.0 && progress < 1.0 {
            ctx.request_repaint();
        }
        progress
    }
}
