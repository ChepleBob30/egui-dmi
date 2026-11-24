//! DMI 模块根：声明总调用区与接口实现区，再导出公开契约。
//! DMI module root: dispatcher area, interface area, and public re-exports.

pub mod call;
pub mod console;
pub mod interface;
pub mod observe;

pub use call::{call_dmi, DMICommand, DMIComplexity, DMIFailure, DMIRequestType};
pub use console::DMIConsole;
pub use observe::{DMIBasics, DMIFlags, DMIWidgetSnapshot, snapshot_of};
