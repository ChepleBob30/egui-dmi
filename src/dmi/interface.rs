//! 各接口具体实现区：当前收录示例接口 greet，后续接口在本文件分区追加。
//! Interface implementations: the sample greet; append more per section.

use crate::dmi::call::{
    fail_response, ok_response, DMICommand, DMIComplexity, DMIRequestType,
    REQUEST_TYPE_UNSUPPORTED, TARGET_ID_INVALID, TARGET_ID_MISMATCH,
};
use crate::dmi::observe;
use crate::ui::UIEntry;
use serde_json::{Value, Map};

/// greet 的 OBSERVATION 实现：系统级欢迎与自检信息，target_id 固定 0 号。
/// greet handler: system-level hello and self-check info on target id zero.
/// 先检查请求类型和目标编号，再按详细等级组装返回内容。
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

/// 触及方式名：Debug 形态即变体名（PressKey 带键名）。
/// Entry name reuses the Debug spelling, e.g. "PressKey(Space)".
fn entry_name(entry: UIEntry) -> String {
    format!("{entry:?}")
}

/// 功能映射序列化：`{"press":["LeftClick"],...}`（裁决 1A 数组原样）。
/// Capability map as JSON object of arrays, per ruling 1A.
fn function_json(function: &[(String, Vec<UIEntry>)]) -> Value {
    let mut object = Map::new();
    for (capability, entries) in function {
        object.insert(
            capability.clone(),
            Value::Array(entries.iter().map(|entry| Value::String(entry_name(*entry))).collect()),
        );
    }
    Value::Object(object)
}

/// 读取组件当前状态的接口，按三个详细等级返回不同内容。
/// Observation handler for check_widget_status, three detail tiers.
/// 依次检查请求类型、注册表记录、状态记录，缺失都返回 TARGET_ID_INVALID。
/// Checks request kind, registry record, then the stored widget state.
pub fn check_widget_status(command: &DMICommand) -> String {
    if command.request_type != DMIRequestType::Observation {
        return fail_response(&REQUEST_TYPE_UNSUPPORTED, command.complexity);
    }
    let records = crate::ui::id::list_id();
    let Some(record) = records.iter().find(|record| record.id == command.target_id) else {
        return fail_response(&TARGET_ID_INVALID, command.complexity);
    };
    let Some(snapshot) = observe::snapshot_of(command.target_id) else {
        return fail_response(&TARGET_ID_INVALID, command.complexity);
    };
    let mut pairs = vec![
        ("widget_name".to_owned(), Value::String(record.name.clone())),
        ("widget_type".to_owned(), Value::String(record.kind.as_str().to_owned())),
        ("widget_text".to_owned(), Value::String(snapshot.basics.widget_text())),
    ];
    if command.complexity >= DMIComplexity::Standard {
        pairs.push(("enabled".to_owned(), Value::Bool(snapshot.flags.enabled)));
        pairs.push(("widget_function".to_owned(), function_json(&record.function)));
    }
    if command.complexity >= DMIComplexity::Complex {
        pairs.push(("hovered".to_owned(), Value::Bool(snapshot.flags.hovered)));
        pairs.push(("clicked".to_owned(), Value::Bool(snapshot.flags.clicked)));
        pairs.push(("pressed".to_owned(), Value::Bool(snapshot.flags.pressed)));
        pairs.push(("focused".to_owned(), Value::Bool(snapshot.flags.focused)));
        pairs.push(("changed".to_owned(), Value::Bool(snapshot.flags.changed)));
        let (x, y, width, height) = snapshot.rect;
        let mut rect = Map::new();
        rect.insert("x".to_owned(), Value::from(x));
        rect.insert("y".to_owned(), Value::from(y));
        rect.insert("w".to_owned(), Value::from(width));
        rect.insert("h".to_owned(), Value::from(height));
        pairs.push(("rect".to_owned(), Value::Object(rect)));
        pairs.push(("visible".to_owned(), Value::Bool(snapshot.visible)));
    }
    ok_response(pairs)
}
