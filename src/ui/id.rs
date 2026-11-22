//! 组件ID注册表：DMI 编号分配、定位映射与修订快照（零帧重复入表）。
//! Id registry: DMI numbering, locating map, revision-stamped snapshots.

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

/// 注册表条目：(id, 类型, 上级id, 名称) 四元组。
/// Registry record: the (id, kind, parent_id, name) tuple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DMIRecord {
    /// 本组件 DMI 编号。 / This component's DMI id.
    pub id: DMIId,
    /// 组件类型。 / Component kind.
    pub kind: DMIWidgetKind,
    /// 上级组件编号（根组件为 0）。 / Parent id (system zero for roots).
    pub parent: DMIId,
    /// 组件名称（DMI 可读名，自动或显式）。 / Component name, explicit or auto.
    pub name: String,
}

/// 显示为文档示例的四元组形式。 / Displays as the documented tuple form.
impl std::fmt::Display for DMIRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "({}, \"{}\", {}, \"{}\")",
            self.id,
            self.kind.as_str(),
            self.parent,
            self.name
        )
    }
}

// ============================================================================
// 持久注册表 Persistent registry
// ============================================================================

/// 复用键：显式命名只按名称，匿名按 egui id 位置键。
/// Reuse key: named entries key on name only, anonymous on egui id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ReuseKey {
    /// 显式名称（与 egui id 无关，布局漂移不再重号）。 / Explicit name, drift-proof.
    Named(String),
    /// 匿名组件的 egui id 键。 / Positional egui id key for anonymous widgets.
    Anonymous(Id),
}

/// 进程内持久注册表：条目、复用映射、定位表与修订快照。
/// Process-lifetime registry: records, reuse map, locating, revision snapshot.
struct DMIRegistry {
    /// 全部条目，按分配顺序。 / All records in allocation order.
    records: Vec<DMIRecord>,
    /// 复用键 → DMI 编号。 / Reuse key to DMI id map.
    reused: HashMap<ReuseKey, DMIId>,
    /// DMI 编号 → 最近一次调用的 egui id（定位通道，每帧更新但不入表）。
    /// DMI id to last seen egui id; refreshed per call without table growth.
    located: HashMap<DMIId, Id>,
    /// 主编号发号器（从 1 起，0 已预留）。 / Major allocator starting at one.
    next_major: u32,
    /// 父号 → 已分配子号计数。 / Parent to issued child counter.
    minor_counters: HashMap<DMIId, u32>,
    /// 条目版本号：仅入表时递增。 / Revision counter, bumped only on push.
    revision: u64,
    /// 条目共享快照：入表时重建，读取零拷贝。
    /// Shared snapshot, rebuilt on push; reads are one Arc clone.
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

impl DMIRegistry {
    /// 登记或复用主编号；命中只刷新定位，绝不触碰条目列表。
    /// Register or reuse; hits only refresh locating, never the table.
    fn allocate(
        &mut self,
        egui_id: Id,
        kind: DMIWidgetKind,
        name: Option<&str>,
        parent: Option<DMIId>,
    ) -> DMIId {
        // 显式命名：名称即身份；匿名：egui id 位置键。
        // Named: name is identity; anonymous: positional egui id.
        let key = match name {
            Some(given) => ReuseKey::Named(given.to_owned()),
            None => ReuseKey::Anonymous(egui_id),
        };
        if let Some(existing) = self.reused.get(&key) {
            self.located.insert(*existing, egui_id);
            return *existing;
        }
        let major = self.next_major;
        self.next_major += 1;
        let new_id = DMIId::new(major);
        let stored_name = match name {
            Some(given) => given.to_owned(),
            None => format!("{}-{major}", kind.as_str()),
        };
        self.push_record(DMIRecord {
            id: new_id,
            kind,
            parent: parent.unwrap_or(DMIId::SYSTEM),
            name: stored_name,
        });
        self.reused.insert(key, new_id);
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
    ) -> DMIId {
        let key = ReuseKey::Named(name.to_owned());
        if let Some(existing) = self.reused.get(&key) {
            self.located.insert(*existing, egui_id);
            return *existing;
        }
        let counter = self.minor_counters.entry(parent).or_insert(0);
        *counter += 1;
        // 深度上限防御：第 5 层请求显式拒绝，不发号、不入表。
        // Depth guard: a fifth-level request is refused, nothing registered.
        let Some(new_id) = parent.child(*counter) else {
            debug_assert!(false, "DMI child id exceeds DMI_MAX_DEPTH");
            return DMIId::SYSTEM;
        };
        self.push_record(DMIRecord {
            id: new_id,
            kind,
            parent,
            name: name.to_owned(),
        });
        self.reused.insert(key, new_id);
        self.located.insert(new_id, egui_id);
        new_id
    }

    /// 入表：追加条目并重建快照、递增版本号。
    /// Pushes a record, rebuilding the shared snapshot and revision.
    fn push_record(&mut self, record: DMIRecord) {
        self.records.push(record);
        self.revision += 1;
        self.snapshot = Arc::new(self.records.clone());
    }
}

/// 全局注册表句柄。 / Global registry handle.
static REGISTRY: OnceLock<Mutex<DMIRegistry>> = OnceLock::new();

/// 加锁访问注册表；中毒锁取回内部数据继续。
/// Locks the registry; recovers data from a poisoned lock.
fn with_registry<R>(action: impl FnOnce(&mut DMIRegistry) -> R) -> R {
    let registry = REGISTRY.get_or_init(Mutex::default);
    let mut guard = registry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    action(&mut guard)
}

/// 登记并取回 DMI 编号：显式命名只按名称复用，布局漂移不重号。
/// Registers a widget; explicit names are identity, layout drift is harmless.
pub fn allocate(
    egui_id: Id,
    kind: DMIWidgetKind,
    name: Option<&str>,
    parent: Option<DMIId>,
) -> DMIId {
    with_registry(|registry| registry.allocate(egui_id, kind, name, parent))
}

/// 为复合组件子项分配"父号.序号"，名称需完整（含父名前缀）。
/// Allocates a "parent.n" child id; name carries the full prefixed label.
pub fn allocate_child(egui_id: Id, kind: DMIWidgetKind, name: &str, parent: DMIId) -> DMIId {
    with_registry(|registry| registry.allocate_child(egui_id, kind, name, parent))
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

/// 按编号取最近一次调用的 egui id（DMI 定位通道）。
/// Latest egui id seen for this DMI id, the locating channel.
pub fn locate(dmi_id: DMIId) -> Option<Id> {
    with_registry(|registry| registry.located.get(&dmi_id).copied())
}

/// 条目版本号：仅新登记时变化，供 UI 判断是否需要重建列表。
/// Revision number; UI rebuilds lists only when this changes.
pub fn registry_revision() -> u64 {
    with_registry(|registry| registry.revision)
}

/// 查看当前全部编号分配：Arc 快照，读取零拷贝。
/// Snapshot of all id assignments; Arc clone, zero copy.
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

    /// 同 egui id 不同命名 ⇒ 两个编号；命名相同 ⇒ 跨漂移复用不重号。
    /// Different names get ids; same name reuses even with drifting egui id.
    #[test]
    fn named_identity_survives_egui_drift() {
        let _guard = test_mutex().lock().unwrap_or_else(|p| p.into_inner());
        let first = allocate(
            Id::NULL.with("drift_a"),
            DMIWidgetKind::Button,
            Some("case_drift_button"),
            None,
        );
        let drifted = allocate(
            Id::NULL.with("drift_b"),
            DMIWidgetKind::Button,
            Some("case_drift_button"),
            None,
        );
        assert_eq!(first, drifted, "布局漂移后同名必须复用，不重复入表");
        let shared_egui_id = Id::NULL.with("drift_shared");
        let red = allocate(
            shared_egui_id,
            DMIWidgetKind::Button,
            Some("case_names_red"),
            None,
        );
        let blue = allocate(
            shared_egui_id,
            DMIWidgetKind::Button,
            Some("case_names_blue"),
            None,
        );
        assert_ne!(red, blue, "命名不同必须额外分配");
        assert_eq!(locate(blue), Some(shared_egui_id));
    }

    /// 反复登记不得使列表增长：同名 10 次只产生 1 条。
    /// Repeated registration must not grow the list; ten calls, one record.
    #[test]
    fn repeated_allocation_does_not_grow() {
        let _guard = test_mutex().lock().unwrap_or_else(|p| p.into_inner());
        for round in 0..10 {
            allocate(
                Id::NULL.with(("grow_probe", round)),
                DMIWidgetKind::Checkbox,
                Some("case_grow_probe"),
                None,
            );
        }
        let hits = list_id()
            .iter()
            .filter(|record| record.name == "case_grow_probe")
            .count();
        assert_eq!(hits, 1, "同名重复分配不得重复入表");
    }

    /// 子项编号形如 父号.1/父号.2，parent 指向父组件。
    /// Children get dotted ids parent.1/parent.2 with parent link set.
    #[test]
    fn child_ids_follow_parent() {
        let _guard = test_mutex().lock().unwrap_or_else(|p| p.into_inner());
        let parent = allocate(
            Id::NULL.with("case_parent"),
            DMIWidgetKind::Select,
            Some("case_theme_selector"),
            None,
        );
        let light = allocate_child(
            Id::NULL.with("case_option_light"),
            DMIWidgetKind::Select,
            "case_theme_selector_light",
            parent,
        );
        let dark = allocate_child(
            Id::NULL.with("case_option_dark"),
            DMIWidgetKind::Select,
            "case_theme_selector_dark",
            parent,
        );
        let light_again = allocate_child(
            Id::NULL.with("case_option_light"),
            DMIWidgetKind::Select,
            "case_theme_selector_light",
            parent,
        );
        assert_eq!(light, light_again, "子项按名称复用");
        assert_eq!(light, parent.child(1).unwrap());
        assert_eq!(dark, parent.child(2).unwrap());
        assert_eq!(light.to_string(), format!("{}.1", parent.major));
        let records = list_id();
        let child_record = records
            .iter()
            .find(|record| record.id == dark)
            .expect("子项条目存在");
        assert_eq!(child_record.parent, parent);
        assert_eq!(child_record.name, "case_theme_selector_dark");
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

    /// 三层发号链：父 1 → 子 1.1 → 孙 1.1.1/1.1.2，parent 链与复用一致。
    /// Three-level chain: parent, child, grandchildren with parent links.
    #[test]
    fn grandchild_chain_allocates_under_child() {
        let parent = allocate(
            Id::NULL.with("chain_root"),
            DMIWidgetKind::Select,
            Some("case_chain_root"),
            None,
        );
        let child = allocate_child(
            Id::NULL.with("chain_group"),
            DMIWidgetKind::Select,
            "case_chain_root_group",
            parent,
        );
        let first = allocate_child(
            Id::NULL.with("chain_item_a"),
            DMIWidgetKind::Select,
            "case_chain_root_group_a",
            child,
        );
        let second = allocate_child(
            Id::NULL.with("chain_item_b"),
            DMIWidgetKind::Select,
            "case_chain_root_group_b",
            child,
        );
        let first_again = allocate_child(
            Id::NULL.with("chain_item_a_moved"),
            DMIWidgetKind::Select,
            "case_chain_root_group_a",
            child,
        );
        assert_eq!(child, parent.child(1).unwrap());
        assert_eq!(first, child.child(1).unwrap(), "孙号 = 1.1.1");
        assert_eq!(second, child.child(2).unwrap(), "孙号 = 1.1.2");
        assert_eq!(first, first_again, "孙项按名称复用，egui id 漂移无关");
        let records = list_id();
        let first_record = records.iter().find(|record| record.id == first).unwrap();
        assert_eq!(first_record.parent, child, "孙项 parent 指向子而非根");
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

    /// debug 档：超限发号按蓝图约定 panic 提示（防御性边界）。
    /// Debug builds panic on over-deep allocation, as the blueprint prescribes.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "exceeds DMI_MAX_DEPTH")]
    fn depth_limit_panics_in_debug_build() {
        let mut deep = allocate(
            Id::NULL.with("deep_dbg_root"),
            DMIWidgetKind::Select,
            Some("case_deep_dbg_root"),
            None,
        );
        for level in 1..=DMI_MAX_DEPTH {
            let name = format!("case_deep_dbg_level_{level}");
            deep = allocate_child(
                Id::NULL.with(format!("deep_dbg_salt_{level}")),
                DMIWidgetKind::Select,
                &name,
                deep,
            );
        }
        allocate_child(
            Id::NULL.with("deep_dbg_overflow"),
            DMIWidgetKind::Select,
            "case_deep_dbg_overflow",
            deep,
        );
    }

    /// release 档：超限发号返回 SYSTEM 哨兵且不入表。
    /// Release builds refuse with SYSTEM and register nothing.
    #[cfg(not(debug_assertions))]
    #[test]
    fn depth_limit_rejects_without_registering() {
        let mut deep = allocate(
            Id::NULL.with("deep_root"),
            DMIWidgetKind::Select,
            Some("case_deep_root"),
            None,
        );
        for level in 1..=DMI_MAX_DEPTH {
            let name = format!("case_deep_level_{level}");
            deep = allocate_child(
                Id::NULL.with(format!("deep_salt_{level}")),
                DMIWidgetKind::Select,
                &name,
                deep,
            );
        }
        let before =
            list_id().iter().filter(|record| record.name.starts_with("case_deep")).count();
        let rejected = allocate_child(
            Id::NULL.with("deep_overflow"),
            DMIWidgetKind::Select,
            "case_deep_overflow",
            deep,
        );
        assert!(rejected.is_system(), "超限子号必须被拒绝");
        let after =
            list_id().iter().filter(|record| record.name.starts_with("case_deep")).count();
        assert_eq!(before, after, "拒绝路径不得入表");
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

    /// 版本号仅在真实入表时变化。 / Revision only moves on real pushes.
    #[test]
    fn revision_tracks_growth() {
        let _guard = test_mutex().lock().unwrap_or_else(|p| p.into_inner());
        let name = "case_revision_probe";
        let start = registry_revision();
        let first = allocate(
            Id::NULL.with("rev_a"),
            DMIWidgetKind::Slider,
            Some(name),
            None,
        );
        assert!(registry_revision() > start, "入表必须递增版本");
        let held = registry_revision();
        let reused = allocate(
            Id::NULL.with("rev_b"),
            DMIWidgetKind::Slider,
            Some(name),
            None,
        );
        assert_eq!(first, reused);
        assert_eq!(held, registry_revision(), "复用不得递增版本");
    }
}
