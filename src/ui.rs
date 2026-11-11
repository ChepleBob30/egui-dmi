//! UI 封装模块根：设计语言层与组件层的统一出口。
//! UI module root: exports the design language and component layers.

pub mod components;
pub mod design;

pub use components::{
    DMICheckbox, DMIButton, DMIButtonKind, DMILabel, DMIRadio, DMISelect, DMISlider,
    DMITextInput, DMIUpdate,
};
pub use design::DesignLanguages;
