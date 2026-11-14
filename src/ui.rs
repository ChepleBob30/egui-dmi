//! UI 封装模块根：设计语言层、组件层与编号注册表的统一出口。
//! UI module root: exports design language, components and the id registry.

pub mod components;
pub mod design;
pub mod id;

pub use components::{
    DMICheckbox, DMIButton, DMIButtonKind, DMILabel, DMIRadio, DMISelect, DMISlider,
    DMITextInput, DMIUpdate,
};
pub use design::DesignLanguages;
pub use id::{DMIId, DMIRecord, DMIWidgetKind, list_id};
