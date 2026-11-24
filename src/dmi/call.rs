//! 接口总调用区：命令契约、解析、分层应答构造与分发器。
//! Dispatcher area: command contract, parsing, tiered responses, routing.

use crate::dmi::interface;
use crate::ui::id::DMIId;
use serde_json::{Map, Value};

// ============================================================================
// 请求契约 Command contract
// ============================================================================

/// 请求类型：DMI 接口三大类（ACCESS/STATE/OBSERVATION）。
/// The three request kinds of the DMI interface taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DMIRequestType {
    /// 声明目标组件，导航与点击由 DMI 自动完成。 / Declare a target widget.
    Access,
    /// 直接设置组件目标状态。 / Set widget state directly.
    State,
    /// 只读获取组件状态，不改动界面。 / Read state without touching the UI.
    Observation,
}

impl DMIRequestType {
    /// 按大写名称解析请求类型，未知返回 None。
    /// Parse the uppercase request kind; None when unknown.
    fn parse(text: &str) -> Option<Self> {
        match text {
            "ACCESS" => Some(Self::Access),
            "STATE" => Some(Self::State),
            "OBSERVATION" => Some(Self::Observation),
            _ => None,
        }
    }
}

/// 返回值细节档位：simple < standard < complex，有序可比。
/// Detail tiers of responses, ordered from low to high.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DMIComplexity {
    /// 最简应答。 / Minimal response.
    Simple,
    /// 标准应答：失败附带原因与错误码。 / Standard: failures carry reason and code.
    Standard,
    /// 完整应答：附带全部字段与修复建议。 / Full: every field plus suggestion.
    Complex,
}

impl DMIComplexity {
    /// 按小写名称解析档位，未知返回 None。
    /// Parse the lowercase tier name; None when unknown.
    fn parse(text: &str) -> Option<Self> {
        match text {
            "simple" => Some(Self::Simple),
            "standard" => Some(Self::Standard),
            "complex" => Some(Self::Complex),
            _ => None,
        }
    }
}

/// 一条已解析的 DMI 调用命令（信封四字段）。
/// A parsed command envelope carrying the four agreed fields.
#[derive(Debug, Clone, PartialEq)]
pub struct DMICommand {
    /// 请求类型。 / Request kind.
    pub request_type: DMIRequestType,
    /// 接口名称。 / Interface name to route to.
    pub interface: String,
    /// 目标组件编号（系统接口为 0 号）。 / Target widget id (zero for system).
    pub target_id: DMIId,
    /// 返回值细节档位。 / Requested detail tier.
    pub complexity: DMIComplexity,
}

// ============================================================================
// 失败描述 Failure records
// ============================================================================

/// 失败描述：错误码 + 完整句失败原因 + 修复建议。
/// Failure record: code, full-sentence reason, and suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DMIFailure {
    /// 稳定错误码（如 INTERFACE_NOT_FOUND）。 / Stable machine-readable code.
    pub code: &'static str,
    /// 人类可读完整句失败原因。 / Human-readable failure sentence.
    pub reason: &'static str,
    /// complex 档附带的修复建议。 / Suggestion appended at complex tier.
    pub suggestion: &'static str,
}

/// 命令 JSON 非法（含字段缺失与类型不符）。 / Invalid command JSON or fields.
pub const COMMAND_JSON_INVALID: DMIFailure = DMIFailure {
    code: "COMMAND_JSON_INVALID",
    reason: "Command JSON invalid.",
    suggestion: "Send a full JSON object with request_type, interface, target_id, complexity.",
};
/// request_type 不是三类大写值之一。 / Unknown request type value.
pub const REQUEST_TYPE_INVALID: DMIFailure = DMIFailure {
    code: "REQUEST_TYPE_INVALID",
    reason: "Request type unknown.",
    suggestion: "Use ACCESS, STATE or OBSERVATION in uppercase.",
};
/// complexity 字段存在但值非法。 / Complexity field present but invalid.
pub const COMPLEXITY_INVALID: DMIFailure = DMIFailure {
    code: "COMPLEXITY_INVALID",
    reason: "Complexity value invalid.",
    suggestion: "Use simple, standard or complex in lowercase.",
};
/// target_id 无法解析为编号。 / Target id cannot be parsed as an id.
pub const TARGET_ID_INVALID: DMIFailure = DMIFailure {
    code: "TARGET_ID_INVALID",
    reason: "Target id invalid.",
    suggestion: "Use an integer or dotted id such as 0 or 1.2.",
};
/// 接口名没有对应实现。 / No interface matches the requested name.
pub const INTERFACE_NOT_FOUND: DMIFailure = DMIFailure {
    code: "INTERFACE_NOT_FOUND",
    reason: "Interface not found.",
    suggestion: "Check available interfaces; greet is registered.",
};
/// 接口不支持该请求类型。 / Interface does not support this request kind.
pub const REQUEST_TYPE_UNSUPPORTED: DMIFailure = DMIFailure {
    code: "REQUEST_TYPE_UNSUPPORTED",
    reason: "Interface mode unsupported.",
    suggestion: "Call greet with request_type OBSERVATION.",
};
/// 系统级接口只接受 0 号目标。 / System interfaces accept only target id zero.
pub const TARGET_ID_MISMATCH: DMIFailure = DMIFailure {
    code: "TARGET_ID_MISMATCH",
    reason: "Target id not accepted.",
    suggestion: "System interfaces take target id 0.",
};

// ============================================================================
// 解析 Parsing
// ============================================================================

/// 解析 target_id：数字只表达一级小数（1.25 即单级 25），
/// 两级及以上必须使用点分字符串（"1.2.5"），层数超上限拒绝。
/// Numbers carry one dotted level; deeper paths must be dotted strings.
pub fn parse_target_id(value: &Value) -> Option<DMIId> {
    match value {
        Value::Number(number) => parse_dotted_path(&number.to_string()),
        Value::String(text) => parse_dotted_path(text),
        _ => None,
    }
}

/// 按小数点拆分路径逐级追加，负数与非法段直接拒绝。
/// Split on dots and append each level; reject negatives and bad parts.
fn parse_dotted_path(text: &str) -> Option<DMIId> {
    let mut parts = text.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let mut parsed_id = DMIId::new(major);
    for part in parts {
        let level = part.parse::<u32>().ok()?;
        parsed_id = parsed_id.child(level)?;
    }
    Some(parsed_id)
}

/// JSON 字符串 → 命令；按错误优先级返回首个失败描述。
/// Parse the command JSON; report the first failure by priority.
fn parse_command(raw: &str) -> Result<DMICommand, DMIFailure> {
    let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(raw) else {
        return Err(COMMAND_JSON_INVALID);
    };
    let Some(request_text) = fields.get("request_type").and_then(Value::as_str) else {
        return Err(COMMAND_JSON_INVALID);
    };
    let Some(request_type) = DMIRequestType::parse(request_text) else {
        return Err(REQUEST_TYPE_INVALID);
    };
    let Some(interface_name) = fields.get("interface").and_then(Value::as_str) else {
        return Err(COMMAND_JSON_INVALID);
    };
    let Some(target_value) = fields.get("target_id") else {
        return Err(COMMAND_JSON_INVALID);
    };
    let Some(target_id) = parse_target_id(target_value) else {
        return Err(TARGET_ID_INVALID);
    };
    let Some(complexity_text) = fields.get("complexity").and_then(Value::as_str) else {
        return Err(COMMAND_JSON_INVALID);
    };
    let Some(complexity) = DMIComplexity::parse(complexity_text) else {
        return Err(COMPLEXITY_INVALID);
    };
    Ok(DMICommand {
        request_type,
        interface: interface_name.to_owned(),
        target_id,
        complexity,
    })
}

// ============================================================================
// 应答构造 Response builders（集中两函数，禁止散落拼串）
// ============================================================================

/// 成功应答：result 恒在首位，其余键值按传入顺序写入。
/// Success response: result first, then pairs in the given order.
pub fn ok_response(pairs: Vec<(String, Value)>) -> String {
    let mut object = Map::new();
    object.insert("result".to_owned(), Value::String("OK".to_owned()));
    for (key, value) in pairs {
        object.insert(key, value);
    }
    Value::Object(object).to_string()
}

/// 失败应答分层：simple 仅 result；standard 加 reason 与 error_code；
/// complex 再加 suggestion（严格按计划文档 §2.4）。
/// Tiered failures: simple result only, standard adds reason and code,
/// complex adds the suggestion.
pub fn fail_response(failure: &DMIFailure, complexity: DMIComplexity) -> String {
    let mut object = Map::new();
    object.insert("result".to_owned(), Value::String("FAILED".to_owned()));
    if complexity >= DMIComplexity::Standard {
        object.insert("reason".to_owned(), Value::String(failure.reason.to_owned()));
        object.insert("error_code".to_owned(), Value::String(failure.code.to_owned()));
    }
    if complexity >= DMIComplexity::Complex {
        object.insert("suggestion".to_owned(), Value::String(failure.suggestion.to_owned()));
    }
    Value::Object(object).to_string()
}

// ============================================================================
// 分发器 Dispatcher
// ============================================================================

/// DMI 统一调用入口：接收完整 JSON 字符串，返回应答 JSON 字符串。
/// The single DMI entry point: full JSON command in, JSON response out.
/// 命令本身解析失败时无法得知请求档位，统一按 standard 兜底输出。
/// Unparsable commands fall back to the standard tier for readability.
pub fn call_dmi(command: &str) -> String {
    let parsed = match parse_command(command) {
        Ok(parsed) => parsed,
        Err(failure) => return fail_response(&failure, DMIComplexity::Standard),
    };
    match parsed.interface.as_str() {
        "greet" => interface::greet(&parsed),
        "check_widget_status" => interface::check_widget_status(&parsed),
        _ => fail_response(&INTERFACE_NOT_FOUND, parsed.complexity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 组装一条 greet 命令文本。 / Build one greet command string.
    fn greet_command(request_type: &str, target_id: &str, complexity: &str) -> String {
        format!(
            "{{\"request_type\":\"{request_type}\",\"interface\":\"greet\",\"target_id\":{target_id},\"complexity\":\"{complexity}\"}}"
        )
    }

    /// greet 三档成功应答逐字段与键序对齐计划示例。
    /// The three success tiers match the planned examples exactly.
    #[test]
    fn greet_tiers_match_planned_examples() {
        assert_eq!(
            call_dmi(&greet_command("OBSERVATION", "0", "simple")),
            "{\"result\":\"OK\",\"greet_text\":\"hello.\"}"
        );
        assert_eq!(
            call_dmi(&greet_command("OBSERVATION", "0", "standard")),
            "{\"result\":\"OK\",\"greet_text\":\"hello, egui.\",\"project_name\":\"egui-dmi\"}"
        );
        assert_eq!(
            call_dmi(&greet_command("OBSERVATION", "0", "complex")),
            "{\"result\":\"OK\",\"greet_text\":\"hello, egui-dmi.\",\"project_name\":\"egui-dmi\",\"dependencies\":\"egui 0.37.0\"}"
        );
    }

    /// 失败分层裁剪：simple 仅 result，standard 带原因与码，complex 带建议。
    /// Failure tiering truncates fields exactly as the plan prescribes.
    #[test]
    fn failure_tiers_are_truncated_by_complexity() {
        // simple 截断只适用于可解析命令；缺字段命令走 standard 兜底档。
        // Simple truncation applies to parsed commands only.
        let unknown_simple = "{\"request_type\":\"OBSERVATION\",\"interface\":\"unknown_one\",\"target_id\":0,\"complexity\":\"simple\"}";
        assert_eq!(call_dmi(unknown_simple), "{\"result\":\"FAILED\"}");
        let standard = "{\"request_type\":\"OBSERVATION\",\"interface\":\"unknown_one\",\"target_id\":0,\"complexity\":\"standard\"}";
        assert_eq!(
            call_dmi(standard),
            "{\"result\":\"FAILED\",\"reason\":\"Interface not found.\",\"error_code\":\"INTERFACE_NOT_FOUND\"}"
        );
        let complex = "{\"request_type\":\"OBSERVATION\",\"interface\":\"unknown_one\",\"target_id\":0,\"complexity\":\"complex\"}";
        let response = call_dmi(complex);
        assert!(response.contains("\"suggestion\""), "complex 档必须带建议");
        assert!(response.starts_with("{\"result\":\"FAILED\""), "result 必须首位");
    }

    /// 非法输入逐一命中专属错误码，解析失败兜底 standard 档。
    /// Invalid inputs hit dedicated codes with the standard-tier fallback.
    #[test]
    fn invalid_inputs_map_to_dedicated_codes() {
        let broken = call_dmi("{not json");
        assert!(broken.contains("COMMAND_JSON_INVALID"), "解析失败兜底 standard");
        assert!(broken.contains("\"reason\""));
        let lowercase_type = call_dmi(&greet_command("observation", "0", "simple"));
        assert!(lowercase_type.contains("REQUEST_TYPE_INVALID"));
        let bad_tier = call_dmi(&greet_command("OBSERVATION", "0", "ultra"));
        assert!(bad_tier.contains("COMPLEXITY_INVALID"));
        let wrong_target = call_dmi(&greet_command("OBSERVATION", "1.2", "complex"));
        assert!(wrong_target.contains("TARGET_ID_MISMATCH"));
        let access_greet = call_dmi(&greet_command("ACCESS", "0", "complex"));
        assert!(access_greet.contains("REQUEST_TYPE_UNSUPPORTED"));
    }

    /// target_id 形态约定：数字单级小数、字符串多级路径、超限拒绝。
    /// Number form allows one dotted level, strings carry deeper paths.
    #[test]
    fn target_id_forms_parse_by_convention() {
        assert_eq!(parse_target_id(&Value::from(0)), Some(DMIId::SYSTEM));
        assert_eq!(
            parse_target_id(&serde_json::from_str::<Value>("1.2").unwrap()),
            Some(DMIId::new(1).child(2).unwrap())
        );
        assert_eq!(
            parse_target_id(&serde_json::from_str::<Value>("1.25").unwrap()),
            Some(DMIId::new(1).child(25).unwrap()),
            "数字 1.25 视为单级 25"
        );
        assert_eq!(
            parse_target_id(&Value::String("1.2.5".to_owned())),
            Some(DMIId::new(1).child(2).unwrap().child(5).unwrap())
        );
        assert_eq!(
            parse_target_id(&Value::String("1.2.3.4.5".to_owned())),
            Some(DMIId::new(1).child(2).unwrap().child(3).unwrap().child(4).unwrap().child(5).unwrap()),
            "主号加四段小数恰为深度上限"
        );
        assert_eq!(parse_target_id(&Value::String("1.2.3.4.5.6".to_owned())), None);
        assert_eq!(parse_target_id(&serde_json::from_str::<Value>("-1").unwrap()), None);
        assert_eq!(parse_target_id(&Value::String("abc".to_owned())), None);
    }
}
