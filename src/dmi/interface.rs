//! 各接口具体实现区：当前收录示例接口 greet，后续接口在本文件分区追加。
//! Interface implementations: the sample greet; append more per section.

use crate::dmi::call::{
    fail_response, ok_response, DMICommand, DMIComplexity, DMIRequestType,
    REQUEST_TYPE_UNSUPPORTED, TARGET_ID_MISMATCH,
};
use serde_json::Value;

/// greet 的 OBSERVATION 实现：系统级欢迎与自检信息，target_id 固定 0 号。
/// greet handler: system-level hello and self-check info on target id zero.
/// 校验顺序：请求类型必须 OBSERVATION，目标必须系统 0 号，再按档位组装。
/// Checks the request kind and target first, then assembles by tier.
pub fn greet(command: &DMICommand) -> String {
    if command.request_type != DMIRequestType::Observation {
        return fail_response(&REQUEST_TYPE_UNSUPPORTED, command.complexity);
    }
    if !command.target_id.is_system() {
        return fail_response(&TARGET_ID_MISMATCH, command.complexity);
    }
    let greet_text = match command.complexity {
        DMIComplexity::Simple => "hello.",
        DMIComplexity::Standard => "hello, egui.",
        DMIComplexity::Complex => "hello, egui-dmi.",
    };
    let mut pairs = vec![("greet_text".to_owned(), Value::String(greet_text.to_owned()))];
    if command.complexity >= DMIComplexity::Standard {
        pairs.push((
            "project_name".to_owned(),
            Value::String(env!("CARGO_PKG_NAME").to_owned()),
        ));
    }
    if command.complexity >= DMIComplexity::Complex {
        pairs.push(("dependencies".to_owned(), Value::String("egui 0.37.0".to_owned())));
    }
    ok_response(pairs)
}
