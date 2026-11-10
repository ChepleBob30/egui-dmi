//! 设计语言模块根：汇总令牌子模块并统一对外出口。
//! Design module root: bundles token submodules into one public face.

pub mod accessibility;
pub mod colors;
pub mod languages;
pub mod motion;
pub mod spacing;
pub mod typography;

pub use accessibility::{AccessibilityTokens, DMIComponentRole};
pub use colors::ColorLanguages;
pub use languages::DesignLanguages;
pub use motion::MotionTokens;
pub use spacing::SpacingTokens;
pub use typography::TypographyTokens;
