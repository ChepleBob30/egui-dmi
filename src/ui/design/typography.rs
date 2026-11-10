//! 设计语言 - 字体令牌：字号档位与中文字体装载。
//! Design language typography tokens: font sizes and CJK font loading.

use egui::{Context, FontData, FontDefinitions, FontFamily, FontId, Style, TextStyle};
use std::sync::Arc;

/// 字号档位，统一标签/按钮/正文文本大小。
/// Font size scale for labels, buttons and body text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypographyTokens {
    /// 大标题字号。 / Heading font size.
    pub heading_size: f32,
    /// 正文字号。 / Body font size.
    pub body_size: f32,
    /// 按钮文字字号。 / Button text font size.
    pub button_size: f32,
    /// 次要说明字号。 / Caption font size.
    pub caption_size: f32,
    /// 等宽文本字号。 / Monospace font size.
    pub mono_size: f32,
    /// 额外行间距。 / Extra line spacing.
    pub line_extra: f32,
}

impl TypographyTokens {
    /// 深浅两主题共用的默认字体档位。 / Default sizes shared by both themes.
    pub const DEFAULT: Self = Self {
        heading_size: 22.0,
        body_size: 15.0,
        button_size: 15.0,
        caption_size: 12.5,
        mono_size: 14.0,
        line_extra: 2.0,
    };

    /// 把字号映射为 egui 的五档文本样式。 / Map sizes onto egui's five text styles.
    pub fn apply(&self, style: &mut Style) {
        style.text_styles.insert(TextStyle::Heading, FontId::proportional(self.heading_size));
        style.text_styles.insert(TextStyle::Body, FontId::proportional(self.body_size));
        style.text_styles.insert(TextStyle::Button, FontId::proportional(self.button_size));
        style.text_styles.insert(TextStyle::Small, FontId::proportional(self.caption_size));
        style.text_styles.insert(TextStyle::Monospace, FontId::monospace(self.mono_size));
        style.spacing.extra_text_line_spacing = self.line_extra;
    }

    /// 主文本 FontId（正文）。 / FontId of primary body text.
    pub fn body(&self) -> FontId {
        FontId::proportional(self.body_size)
    }

    /// 按钮文本 FontId。 / FontId of button text.
    pub fn button(&self) -> FontId {
        FontId::proportional(self.button_size)
    }

    /// 说明文本 FontId。 / FontId of caption text.
    pub fn caption(&self) -> FontId {
        FontId::proportional(self.caption_size)
    }

    /// 标题文本 FontId。 / FontId of heading text.
    pub fn heading(&self) -> FontId {
        FontId::proportional(self.heading_size)
    }
}

/// 常见系统中文字体候选路径（macOS/Windows/Linux）。
/// Candidate system CJK font paths for macOS/Windows/Linux.
const CJK_FONT_CANDIDATES: [(&str, u32); 5] = [
    ("/System/Library/Fonts/PingFang.ttc", 2),
    ("/System/Library/Fonts/Supplemental/Arial Unicode.ttf", 0),
    ("C:/Windows/Fonts/msyh.ttc", 0),
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 2),
    ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0),
];

/// 构建字体定义；探测并追加系统中文字体做回退。
/// Builds font definitions, appending a detected system CJK fallback font.
pub fn build_font_definitions() -> FontDefinitions {
    let mut defs = FontDefinitions::default();
    for (path, face_index) in CJK_FONT_CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            let blob: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);
            defs.font_data
                .insert("cjk".to_owned(), Arc::new(FontData::from_blob(blob, face_index)));
            defs.families
                .entry(FontFamily::Proportional)
                .or_default()
                .push("cjk".to_owned());
            defs.families
                .entry(FontFamily::Monospace)
                .or_default()
                .push("cjk".to_owned());
            break;
        }
    }
    defs
}

/// 装载字体（含中文回退），应用失败时保持默认字体。
/// Installs fonts with CJK fallback; keeps defaults on failure.
pub fn install_fonts(ctx: &Context) {
    ctx.set_fonts(build_font_definitions());
}
