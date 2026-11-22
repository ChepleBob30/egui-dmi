# 2026-11-22 UI 触及方法改进：UIEntry、function 功能映射与严格声明制

- 依据：`build_docs/plan/ui_entry_improve.md` + 已批准蓝图
  `build_docs/blueprint/ui_entry_improve_phase1.md`（裁决 1A/2A/3B/4C/5/6 全部落地）。

## 新增/修改

- `src/ui.rs`：新增 `UIEntry` 枚举（LeftClick/RightClick/DoubleClick/CenterClick/
  Hover/Drag/Focus/PressKey(egui::Key)/Direct），逐项对齐 egui 0.37 Response 能力；
  `Direct` 仅作根组件 parent 占位，禁止出现在功能列表（登记接口拒绝）。
- `src/ui/id.rs`：
  - `DMIRecord` 升级五元组：`parent: (DMIId, UIEntry)`（根为 `(0, Direct)`）、
    `function: Vec<(String, Vec<UIEntry>)>`；Display 内联功能映射；
  - **严格声明制**：`allocate(egui_id, kind, name: &str, function)` 与
    `allocate_child(..., touch, function)`——名称空、映射空、功能项空列表、
    功能名重复、列表含 Direct、子登记父号不存在，任一命中返回 `SYSTEM`，
    不入表、不动版本、不 panic；
  - **匿名机制删除**：`ReuseKey` 枚举与自动名 `{kind}-{major}` 移除，
    `reused: HashMap<String, DMIId>` 名称唯一键；
  - 首声明优先（3B）：复用命中只刷新 `located`，不改写 function 映射。
- `src/ui/components.rs`：
  - 七组件新增 `function` 字段与 `dmi_entry(功能名, 触及列表)` 构建方法；
  - 私有 `register()` 登记门统一替换七处出口：`dmi_name` 与非空 `function`
    齐备才注册，缺一 `DMIUpdate.id == SYSTEM`；
  - 实现方显式声明常量：`PICK_FUNCTION`（下拉子项 "pick"）、
    `RADIO_MEMBER_FUNCTION`/`RADIO_GROUP_FUNCTION`（组成员/组父号 "select"）、
    组头 `("hover", [Hover])`；子项登记附带 `touch=LeftClick`。
- `demo/dmi_gallery/src/main.rs`：全部交互组件补齐 `dmi_entry`
  （save/delete/press/edit/toggle/expand/set/probe），六个章节标签声明
  `("hover",[Hover])` 入表；调色盘演示改用新签名
  （`apply_text`/`apply_background` 两个功能名）；
  注册表面板行自动呈现五元组含功能映射。

## 测试

- `cargo test --workspace`：**24/24**（新增 `strict_rejection_matrix` 六路径拒绝、
  `function_roundtrip_and_display` 往返与 Display 金串、
  `undeclared_component_stays_unregistered` 组件侧登记门；
  既有 reuse/深度/版本测试全部在新签名下通过）；
- `cargo clippy --workspace --all-targets`：0 警告；demo 冒烟 5 秒稳定。

## 键名修正记录

- egui 0.37 方向键枚举实际为 `Key::ArrowLeft/ArrowRight`（非旧名 LeftArrow/RightArrow），
  demo 滑块功能声明按实际键名落地。

## 回退

- 公共契约变更（allocate 签名、DMIRecord 字段）均为本仓库内消费，git 回退即恢复。
