//! 组件编号注册表：分配编号、记录位置、支持查询，重复登记不重复写入。
//! Id registry: assigns ids, records widget locations, supports queries.

use crate::ui::UIEntry;
use egui::Id;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

// ============================================================================
// DMI 编号 DMI identifier
// ============================================================================

/// 小数路径最大深度：支持 1.2.3.4 四级嵌套。
/// Maximum dotted path depth, allowing four nested levels.
pub const DMI_MAX_DEPTH: usize = 4;

/// DMI 编号：主号 + 小数路径；depth 为 minors 的有效长度。
/// DMI identifier: major plus dotted path; depth is the valid minors length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DMIId {
    /// 主编号（0 为系统保留）。 / Major number (0 reserved for system).
    pub major: u32,
    /// 小数路径，仅前 depth 项有效。 / Dotted path, first depth entries valid.
    pub minors: [u32; DMI_MAX_DEPTH],
    /// 小数层数 0..=DMI_MAX_DEPTH。 / Number of dotted levels.
    pub depth: u8,
}

impl DMIId {
    /// 系统保留 0 号，用于系统层面接口。 / Reserved id zero for system calls.
    pub const SYSTEM: Self = Self { major: 0, minors: [0; DMI_MAX_DEPTH], depth: 0 };

    /// 由主编号构造（不带小数层）。 / Build from the major number.
    pub const fn new(major: u32) -> Self {
        Self { major, minors: [0; DMI_MAX_DEPTH], depth: 0 }
    }

    /// 是否为系统保留号。 / Whether this is the reserved system id.
    pub const fn is_system(self) -> bool {
        self.major == 0
    }

    /// 是否为不带小数的根号。 / Whether this id carries no dotted levels.
    pub const fn is_root(self) -> bool {
        self.depth == 0
    }

    /// 追加一级小数：1.2 追加 3 得 1.2.3；满深度返回 None。
    /// Append one dotted level; None when the maximum depth is reached.
    pub const fn child(self, minor: u32) -> Option<Self> {
        if self.depth as usize >= DMI_MAX_DEPTH {
            return None;
        }
        let mut path = self.minors;
        path[self.depth as usize] = minor;
        Some(Self { major: self.major, minors: path, depth: self.depth + 1 })
    }

    /// 第 index 层小数，越界返回 0。 / Dotted component at index, zero beyond depth.
    pub const fn level(self, index: usize) -> u32 {
        if index < self.depth as usize {
            self.minors[index]
        } else {
            0
        }
    }
}

impl From<u32> for DMIId {
    fn from(major: u32) -> Self {
        Self::new(major)
    }
}

/// 显示为 "1" / "1.2" / "1.2.3"，与 JSON target_id 字符串形态一致。
/// Displays as "1"/"1.2"/"1.2.3", matching the target_id string form.
impl std::fmt::Display for DMIId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.major)?;
        for index in 0..self.depth as usize {
            write!(formatter, ".{}", self.minors[index])?;
        }
        Ok(())
    }
}

// ============================================================================
// 组件类型与条目 Kinds and records
// ============================================================================

/// 注册表登记的组件类型（与框架组件一一对应）。
/// Registered widget kinds, one per framework component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DMIWidgetKind {
    /// 按钮。 / Button.
    Button,
    /// 文本输入框。 / Text input field.
    TextInput,
    /// 复选框。 / Checkbox.
    Checkbox,
    /// 单选框。 / Radio button.
    Radio,
    /// 下拉菜单。 / Select menu.
    Select,
    /// 滑块。 / Slider.
    Slider,
    /// 标签文本。 / Label.
    Label,
}

impl DMIWidgetKind {
    /// 注册表与 JSON 中的类型字符串。 / Kind string used in records and JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::TextInput => "text_input",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Select => "select",
            Self::Slider => "slider",
            Self::Label => "label",
        }
    }
}

/// 注册表条目：(id, 类型, (上级id, 触及方式), 名称, 功能映射)。
/// Registry record: id, kind, (parent, reach-entry), name, capability map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DMIRecord {
    /// 本组件 DMI 编号。 / This component's DMI id.
    pub id: DMIId,
    /// 组件类型。 / Component kind.
    pub kind: DMIWidgetKind,
    /// (上级编号, 从上级触及本组件的方式)；根组件为 (0, Direct)。
    /// Parent id plus the entry reaching this child; (zero, Direct) for roots.
    pub parent: (DMIId, UIEntry),
    /// 组件名称（注册强制显式声明，无自动名）。 / Explicit name; no auto names.
    pub name: String,
    /// 功能映射：功能名对应触及方式列表，以第一次登记写入的为准。
    /// Capability map: name to realizing entries, stored at first declaration.
    pub function: Vec<(String, Vec<UIEntry>)>,
}

/// 显示为文档五元组形式（功能映射内联）。
/// Displays as the documented five-tuple with the capability map inlined.
impl std::fmt::Display for DMIRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "({}, \"{}\", ({}, {:?}), \"{}\", [",
            self.id, self.kind.as_str(), self.parent.0, self.parent.1, self.name
        )?;
        for (index, (capability, entries)) in self.function.iter().enumerate() {
            if index > 0 {
                write!(formatter, ", ")?;
            }
            write!(formatter, "(\"{}\", {:?})", capability, entries)?;
        }
        write!(formatter, "])")
    }
}

// ============================================================================
// 持久注册表 Persistent registry
// ============================================================================

/// 进程内长期保存的注册表：条目、名称对应表、位置表、版本与副本。
/// Process-lifetime registry: records, name map, location map, revision, copy.
struct DMIRegistry {
    /// 全部条目，按分配顺序。 / All records in allocation order.
    records: Vec<DMIRecord>,
    /// 名称 → DMI 编号（名称是唯一身份键，严格声明制）。
    /// Name to DMI id; the name is the only identity key.
    reused: HashMap<String, DMIId>,
    /// 编号到最近一次 egui 身份的对应表，每次登记都会更新。
    /// DMI id to last seen egui id; refreshed per call without table growth.
    located: HashMap<DMIId, Id>,
    /// 下一个可用的主编号，从 1 开始，0 已留给系统。 / Major allocator starting at one.
    next_major: u32,
    /// 父号 → 已分配子号计数。 / Parent to issued child counter.
    minor_counters: HashMap<DMIId, u32>,
    /// 条目版本号：只有新增条目时才增加。 / Revision counter, bumped only on push.
    revision: u64,
    /// 条目列表的只读副本：新增条目时重建，读取时不复制内容。
    /// Read-only record copy; rebuilt on insert, read without copying.
    snapshot: Arc<Vec<DMIRecord>>,
}

impl Default for DMIRegistry {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            reused: HashMap::new(),
            located: HashMap::new(),
            next_major: 1,
            minor_counters: HashMap::new(),
            revision: 0,
            snapshot: Arc::new(Vec::new()),
        }
    }
}

/// 严格校验功能映射：空映射放行（组件登记路径）；非空时每项名非空、
/// 列表非空、不含 Direct、功能名不重复。
/// Empty maps pass; non-empty maps are validated item by item, strictly.
fn valid_function(function: &[(&str, &[UIEntry])]) -> bool {
    if function.is_empty() {
        return true;
    }
    let mut seen = std::collections::HashSet::new();
    function.iter().all(|(name, entries)| {
        !name.is_empty()
            && !entries.is_empty()
            && !entries.contains(&UIEntry::Direct)
            && seen.insert(*name)
    })
}

impl DMIRegistry {
    /// 登记或复用主编号；命中只刷新定位，绝不触碰条目列表。
    /// Register or reuse; hits only refresh locating, never the table.
    fn allocate(
        &mut self,
        egui_id: Id,
        kind: DMIWidgetKind,
        name: &str,
        function: &[(&str, &[UIEntry])],
    ) -> DMIId {
        // 名称为空或功能映射不合法时拒绝登记，直接返回 SYSTEM。
        // Missing name or invalid function map means the registration is refused.
        if name.is_empty() || !valid_function(function) {
            return DMIId::SYSTEM;
        }
        if let Some(existing) = self.reused.get(name) {
            self.located.insert(*existing, egui_id);
            return *existing;
        }
        let major = self.next_major;
        self.next_major += 1;
        let new_id = DMIId::new(major);
        self.push_record(DMIRecord {
            id: new_id,
            kind,
            parent: (DMIId::SYSTEM, UIEntry::Direct),
            name: name.to_owned(),
            function: function
                .iter()
                .map(|(cap_name, entries)| (cap_name.to_string(), entries.to_vec()))
                .collect(),
        });
        self.reused.insert(name.to_owned(), new_id);
        self.located.insert(new_id, egui_id);
        new_id
    }

    /// 在父号下按出现顺序分配小数子号，名称键复用不重号。
    /// Allocates dotted child ids under a parent; name key prevents repeats.
    fn allocate_child(
        &mut self,
        egui_id: Id,
        kind: DMIWidgetKind,
        name: &str,
        parent: DMIId,
        touch: UIEntry,
        function: &[(&str, &[UIEntry])],
    ) -> DMIId {
        // 子项同样受严格声明制约束；touch 为 Direct 或父号未登记均拒绝。
        // Children use the same checks; Direct touch or unknown parent is refused.
        if name.is_empty() || touch == UIEntry::Direct || !valid_function(function) {
            return DMIId::SYSTEM;
        }
        if !self.records.iter().any(|record| record.id == parent) {
            return DMIId::SYSTEM;
        }
        if let Some(existing) = self.reused.get(name) {
            self.located.insert(*existing, egui_id);
            return *existing;
        }
        let counter = self.minor_counters.entry(parent).or_insert(0);
        *counter += 1;
        // 达到层数上限的请求被拒绝：不分配编号，也不写入条目。
        // Depth guard: a fifth-level request is refused, nothing registered.
        let Some(new_id) = parent.child(*counter) else {
            debug_assert!(false, "DMI child id exceeds DMI_MAX_DEPTH");
            return DMIId::SYSTEM;
        };
        self.push_record(DMIRecord {
            id: new_id,
            kind,
            parent: (parent, touch),
            name: name.to_owned(),
            function: function
                .iter()
                .map(|(cap_name, entries)| (cap_name.to_string(), entries.to_vec()))
                .collect(),
        });
        self.reused.insert(name.to_owned(), new_id);
        self.located.insert(new_id, egui_id);
        new_id
    }

    /// 写入新条目：追加记录，更新只读副本，版本号加一。
    /// Inserts a record, refreshing the read-only copy and the revision.
    fn push_record(&mut self, record: DMIRecord) {
        self.records.push(record);
        self.revision += 1;
        self.snapshot = Arc::new(self.records.clone());
    }
}

/// 全局唯一的注册表。 / Global registry handle.
static REGISTRY: OnceLock<Mutex<DMIRegistry>> = OnceLock::new();

/// 加锁访问注册表；线程崩溃后仍取回数据继续使用。
/// Locks the registry; recovers data from a poisoned lock.
fn with_registry<R>(action: impl FnOnce(&mut DMIRegistry) -> R) -> R {
    let registry = REGISTRY.get_or_init(Mutex::default);
    let mut guard = registry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    action(&mut guard)
}

/// 登记根组件：名称必填；功能映射可空，非空时逐项严格校验。
/// Registers a root widget; name required, function map validated if given.
pub fn allocate(
    egui_id: Id,
    kind: DMIWidgetKind,
    name: &str,
    function: &[(&str, &[UIEntry])],
) -> DMIId {
    with_registry(|registry| registry.allocate(egui_id, kind, name, function))
}

/// 登记子组件：touch 为从父触及子的方式；父号必须已登记；校验口径同根登记。
/// Registers a child with its parent-reach entry; unknown parent is refused.
pub fn allocate_child(
    egui_id: Id,
    kind: DMIWidgetKind,
    name: &str,
    parent: DMIId,
    touch: UIEntry,
    function: &[(&str, &[UIEntry])],
) -> DMIId {
    with_registry(|registry| registry.allocate_child(egui_id, kind, name, parent, touch, function))
}

/// 按编号查询组件名称。 / Queries the component name by DMI id.
pub fn name_of(dmi_id: DMIId) -> Option<String> {
    with_registry(|registry| {
        registry
            .records
            .iter()
            .find(|record| record.id == dmi_id)
            .map(|record| record.name.clone())
    })
}

/// 按编号查询最近一次出现的 egui 身份。
/// Latest egui id seen for this DMI id, the locating channel.
pub fn locate(dmi_id: DMIId) -> Option<Id> {
    with_registry(|registry| registry.located.get(&dmi_id).copied())
}

/// 条目版本号：仅新登记时变化，供 UI 判断是否需要重建列表。
/// Revision number; UI rebuilds lists only when this changes.
pub fn registry_revision() -> u64 {
    with_registry(|registry| registry.revision)
}

/// 查看全部条目：返回共享的只读副本，读取不复制内容。
/// Returns all records as a shared read-only copy, without duplicating.
pub fn list_id() -> Arc<Vec<DMIRecord>> {
    with_registry(|registry| Arc::clone(&registry.snapshot))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 串行化共享全局注册表计数断言的测试。
    /// Serializes tests that assert on shared global counters.
    fn test_mutex() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    /// 测试通用功能声明。 / Shared single-capability declaration for tests.
    const CAP: [(&str, &[UIEntry]); 1] = [("press", &[UIEntry::LeftClick])];
    /// 双功能声明，用于往返与显示断言。 / Two-capability map for roundtrip checks.
    const TWO_CAPS: [(&str, &[UIEntry]); 2] =
        [("save", &[UIEntry::LeftClick]), ("hover", &[UIEntry::Hover])];

    /// 同一 egui 身份不同名称得到两个编号；名称相同永远复用。
    /// Different names get ids; same name reuses even with drifting egui id.
    #[test]
    fn named_identity_survives_egui_drift() {
        let first = allocate(Id::NULL.with("drift_a"), DMIWidgetKind::Button, "case_drift_button", &CAP);
        let drifted = allocate(Id::NULL.with("drift_b"), DMIWidgetKind::Button, "case_drift_button", &CAP);
        assert_eq!(first, drifted, "同名必须复用，不重复写入条目");
        assert!(!first.is_system());
        let shared_egui_id = Id::NULL.with("drift_shared");
        let red = allocate(shared_egui_id, DMIWidgetKind::Button, "case_names_red", &CAP);
        let blue = allocate(shared_egui_id, DMIWidgetKind::Button, "case_names_blue", &CAP);
        assert_ne!(red, blue, "命名不同必须额外分配");
        assert_eq!(locate(blue), Some(shared_egui_id));
    }

    /// 同名登记十次，即使 egui 身份变化也只产生一条记录。
    /// Ten same-name allocations produce exactly one record.
    #[test]
    fn repeated_allocation_does_not_grow() {
        let _guard = test_mutex().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        for round in 0..10 {
            allocate(
                Id::NULL.with(("grow_probe", round)),
                DMIWidgetKind::Checkbox,
                "case_grow_probe",
                &CAP,
            );
        }
        let hits = list_id().iter().filter(|record| record.name == "case_grow_probe").count();
        assert_eq!(hits, 1, "同名重复登记不得重复写入条目");
    }

    /// 子项编号形如 父号.1/父号.2，parent 元组带触及方式。
    /// Children get dotted ids with the reach entry stored in parent tuple.
    #[test]
    fn child_ids_follow_parent() {
        let parent = allocate(
            Id::NULL.with("case_parent"),
            DMIWidgetKind::Select,
            "case_theme_selector",
            &CAP,
        );
        let light = allocate_child(
            Id::NULL.with("case_option_light"),
            DMIWidgetKind::Select,
            "case_theme_selector_light",
            parent,
            UIEntry::LeftClick,
            &CAP,
        );
        let dark = allocate_child(
            Id::NULL.with("case_option_dark"),
            DMIWidgetKind::Select,
            "case_theme_selector_dark",
            parent,
            UIEntry::LeftClick,
            &CAP,
        );
        let light_again = allocate_child(
            Id::NULL.with("case_option_light"),
            DMIWidgetKind::Select,
            "case_theme_selector_light",
            parent,
            UIEntry::LeftClick,
            &CAP,
        );
        assert_eq!(light, light_again, "子项按名称复用");
        assert_eq!(light, parent.child(1).unwrap());
        assert_eq!(dark, parent.child(2).unwrap());
        let records = list_id();
        let child_record = records.iter().find(|record| record.id == dark).expect("子项条目存在");
        assert_eq!(child_record.parent, (parent, UIEntry::LeftClick));
        assert_eq!(child_record.name, "case_theme_selector_dark");
    }

    /// 严格校验矩阵：六种非法声明一律拒绝，表与版本不动。
    /// Strict rejection: six invalid declarations refused without growth.
    #[test]
    fn strict_rejection_matrix() {
        let _guard = test_mutex().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        // 只统计本用例名称前缀，避免其他测试并行写入的干扰。
        // Counting this case's prefix keeps the assertion race-free.
        let rejected_count = || {
            list_id()
                .iter()
                .filter(|record| record.name.starts_with("case_rej_"))
                .count()
        };
        let before = rejected_count();
        assert!(allocate(Id::NULL.with("rej_empty_name"), DMIWidgetKind::Button, "", &CAP).is_system());
        assert!(allocate(
            Id::NULL.with("rej_empty_entries"),
            DMIWidgetKind::Button,
            "case_rej_entries",
            &[("go", &[] as &[UIEntry])]
        )
        .is_system());
        assert!(allocate(
            Id::NULL.with("rej_direct_entry"),
            DMIWidgetKind::Button,
            "case_rej_direct",
            &[("go", &[UIEntry::Direct])]
        )
        .is_system());
        assert!(allocate(
            Id::NULL.with("rej_dup_cap"),
            DMIWidgetKind::Button,
            "case_rej_dup",
            &[("go", &[UIEntry::LeftClick]), ("go", &[UIEntry::Hover])]
        )
        .is_system());
        assert!(allocate_child(
            Id::NULL.with("rej_orphan"),
            DMIWidgetKind::Select,
            "case_rej_orphan",
            DMIId::new(999_999),
            UIEntry::LeftClick,
            &CAP
        )
        .is_system());
        assert_eq!(rejected_count(), before, "被拒绝的登记不得写入条目");
        // 空功能映射是组件登记的合法形态：有名即注册。
        // An empty function map is the normal component registration form.
        let bare = allocate(
            Id::NULL.with("rej_bare_ok"),
            DMIWidgetKind::Button,
            "case_rej_bare_ok",
            &[],
        );
        assert!(!bare.is_system(), "空映射应正常注册");
        assert_eq!(rejected_count(), before + 1, "空映射注册恰好新增一条");
    }

    /// 写入的功能映射与声明逐项相等，显示格式包含功能表。
    /// Capability map roundtrip with the inlined display form.
    #[test]
    fn function_roundtrip_and_display() {
        let probe_id = allocate(
            Id::NULL.with("cap_roundtrip"),
            DMIWidgetKind::Button,
            "case_cap_roundtrip",
            &TWO_CAPS,
        );
        let records = list_id();
        let record = records.iter().find(|record| record.id == probe_id).unwrap().clone();
        assert_eq!(record.function.len(), 2);
        assert_eq!(record.function[0].0, "save");
        assert_eq!(record.function[1].1, vec![UIEntry::Hover]);
        assert_eq!(record.parent, (DMIId::SYSTEM, UIEntry::Direct));
        let text = record.to_string();
        assert!(text.contains(r#"("save", [LeftClick])"#), "{text}");
    }

    /// 多级构造与显示：1.2.3 形态与各访问器边界。
    /// Multi-level construction, display and accessor boundaries.
    #[test]
    fn multi_level_construction_and_display() {
        let root = DMIId::new(1);
        assert!(root.is_root() && !root.is_system());
        let child = root.child(2).unwrap();
        let grand = child.child(3).unwrap();
        assert_eq!(child.to_string(), "1.2");
        assert_eq!(grand.to_string(), "1.2.3");
        assert_eq!(grand.level(0), 2);
        assert_eq!(grand.level(1), 3);
        assert_eq!(grand.level(2), 0, "越界层返回 0");
        assert!(DMIId::SYSTEM.is_system());
    }

    /// 三层编号链：1、1.1、1.1.1，父子对应关系与复用保持一致。
    /// Three-level chain: parent, child, grandchildren with parent links.
    #[test]
    fn grandchild_chain_allocates_under_child() {
        let parent = allocate(
            Id::NULL.with("chain_root"),
            DMIWidgetKind::Select,
            "case_chain_root",
            &CAP,
        );
        let child = allocate_child(
            Id::NULL.with("chain_group"),
            DMIWidgetKind::Select,
            "case_chain_root_group",
            parent,
            UIEntry::LeftClick,
            &CAP,
        );
        let first = allocate_child(
            Id::NULL.with("chain_item_a"),
            DMIWidgetKind::Select,
            "case_chain_root_group_a",
            child,
            UIEntry::LeftClick,
            &CAP,
        );
        let second = allocate_child(
            Id::NULL.with("chain_item_b"),
            DMIWidgetKind::Select,
            "case_chain_root_group_b",
            child,
            UIEntry::LeftClick,
            &CAP,
        );
        let first_again = allocate_child(
            Id::NULL.with("chain_item_a_moved"),
            DMIWidgetKind::Select,
            "case_chain_root_group_a",
            child,
            UIEntry::LeftClick,
            &CAP,
        );
        assert_eq!(child, parent.child(1).unwrap());
        assert_eq!(first, child.child(1).unwrap(), "孙号 = 1.1.1");
        assert_eq!(second, child.child(2).unwrap(), "孙号 = 1.1.2");
        assert_eq!(first, first_again, "子项按名称复用，与位置变化无关");
        let records = list_id();
        let first_record = records.iter().find(|record| record.id == first).unwrap();
        assert_eq!(first_record.parent, (child, UIEntry::LeftClick), "孙项 parent 指向子");
        let chain_count =
            records.iter().filter(|record| record.name.starts_with("case_chain_root")).count();
        assert_eq!(chain_count, 4, "1 父 + 1 子 + 2 孙，复用不增长");
    }

    /// 纯函数边界：满深度后 child() 返回 None，不依赖任何全局状态。
    /// Pure boundary: child() yields None at maximum depth.
    #[test]
    fn child_at_max_depth_returns_none() {
        let mut deep = DMIId::new(7);
        for _level in 0..DMI_MAX_DEPTH {
            deep = deep.child(1).unwrap();
        }
        assert_eq!(deep.depth as usize, DMI_MAX_DEPTH);
        assert_eq!(deep.to_string(), "7.1.1.1.1");
        assert!(deep.child(2).is_none(), "满深度必须拒绝追加");
    }

    /// 调试运行时，超出层数上限直接崩溃提示（防御性边界）。
    /// Debug builds panic on over-deep allocation, as the blueprint prescribes.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "exceeds DMI_MAX_DEPTH")]
    fn depth_limit_panics_in_debug_build() {
        let mut deep = allocate(
            Id::NULL.with("deep_dbg_root"),
            DMIWidgetKind::Select,
            "case_deep_dbg_root",
            &CAP,
        );
        for level in 1..=DMI_MAX_DEPTH {
            let name = format!("case_deep_dbg_level_{level}");
            deep = allocate_child(
                Id::NULL.with(format!("deep_dbg_salt_{level}")),
                DMIWidgetKind::Select,
                &name,
                deep,
                UIEntry::LeftClick,
                &CAP,
            );
        }
        allocate_child(
            Id::NULL.with("deep_dbg_overflow"),
            DMIWidgetKind::Select,
            "case_deep_dbg_overflow",
            deep,
            UIEntry::LeftClick,
            &CAP,
        );
    }

    /// 发布运行时，超限请求返回 SYSTEM 占位值且不写入条目。
    /// Release builds refuse with SYSTEM and register nothing.
    #[cfg(not(debug_assertions))]
    #[test]
    fn depth_limit_rejects_without_registering() {
        let mut deep = allocate(
            Id::NULL.with("deep_root"),
            DMIWidgetKind::Select,
            "case_deep_root",
            &CAP,
        );
        for level in 1..=DMI_MAX_DEPTH {
            let name = format!("case_deep_level_{level}");
            deep = allocate_child(
                Id::NULL.with(format!("deep_salt_{level}")),
                DMIWidgetKind::Select,
                &name,
                deep,
                UIEntry::LeftClick,
                &CAP,
            );
        }
        let before =
            list_id().iter().filter(|record| record.name.starts_with("case_deep")).count();
        let rejected = allocate_child(
            Id::NULL.with("deep_overflow"),
            DMIWidgetKind::Select,
            "case_deep_overflow",
            deep,
            UIEntry::LeftClick,
            &CAP,
        );
        assert!(rejected.is_system(), "超限子号必须被拒绝");
        let after =
            list_id().iter().filter(|record| record.name.starts_with("case_deep")).count();
        assert_eq!(before, after, "被拒绝的登记不得写入条目");
    }

    /// Ord 语义：按小数路径逐段数值比较，不受数组尾部填充影响。
    /// Ordering compares path segment by segment, padding-insensitive.
    #[test]
    fn order_compares_path_segments() {
        let one = DMIId::new(1);
        let one_two = one.child(2).unwrap();
        let one_two_three = one_two.child(3).unwrap();
        let one_ten = one.child(10).unwrap();
        assert!(one < one_two && one_two < one_two_three, "前缀小于扩展");
        assert!(one_two < one_ten, "逐段数值比较，非字典序");
        let mut seen = std::collections::HashSet::new();
        assert!(seen.insert(one_two));
        assert!(!seen.insert(DMIId::new(1).child(2).unwrap()), "等值编号哈希一致");
    }

    /// 版本号只在真正新增条目时增加。 / Revision only moves on real pushes.
    #[test]
    fn revision_tracks_growth() {
        let _guard = test_mutex().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let name = "case_revision_probe";
        let start = registry_revision();
        let first = allocate(Id::NULL.with("rev_a"), DMIWidgetKind::Slider, name, &CAP);
        assert!(registry_revision() > start, "新增条目必须递增版本");
        let reused = allocate(Id::NULL.with("rev_b"), DMIWidgetKind::Slider, name, &CAP);
        assert_eq!(first, reused);
        let hits = list_id().iter().filter(|record| record.name == name).count();
        assert_eq!(hits, 1, "复用不得新增条目");
    }
}
