//! DMI 层总入口：声明各子模块，并导出对外的接口。
//! DMI entry file: declares submodules and exports the public interface.

pub mod call;
pub mod console;
pub mod interface;
pub mod observe;

pub use call::{call_dmi, DMICommand, DMIComplexity, DMIFailure, DMIRequestType};
pub use console::DMIConsole;
pub use observe::{DMIBasics, DMIFlags, DMIWidgetSnapshot, snapshot_of};
