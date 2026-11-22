# 2026-11-22 DMI Phase 2：统一调用接口 call_dmi 与 greet 示例（硬编码调用）

- 依据：已批准蓝图 `build_docs/blueprint/dmi_phase2.md`（六项决定 + 访问风格 A）。
- 纪律：payload 透传未写入（用户决定）；运行时控制台、Rules 层、ACCESS/STATE 实现未做。

## 新增

| 文件 | 内容 |
| --- | --- |
| `src/dmi.rs` | 模块根（无 mod.rs）：声明 call/interface 两区，再导出 `call_dmi` 与契约类型。 |
| `src/dmi/call.rs` | 接口总调用区：`DMIRequestType`（三类）、`DMIComplexity`（有序三档）、`DMICommand`（信封四字段）、`DMIFailure` + 7 个错误码常量；`parse_command`（字段缺失→COMMAND_JSON_INVALID，值非法→专属码）、`parse_target_id`（数字单级小数、点分字符串多级、超 `DMI_MAX_DEPTH` 拒绝）、`ok_response`/`fail_response`（分层裁剪：simple 仅 result；standard + reason + error_code；complex + suggestion；解析失败兜底 standard）、`call_dmi` 分发器。 |
| `src/dmi/interface.rs` | greet 实现：校验 OBSERVATION（否则 REQUEST_TYPE_UNSUPPORTED）→ 校验 0 号（否则 TARGET_ID_MISMATCH）→ 三档应答（hello. / +project_name(env CARGO_PKG_NAME) / +dependencies "egui 0.37.0"）。 |

## 修改

- `Cargo.toml`：新增 `serde_json = { version = "1", features = ["preserve_order"] }`
  （应答键序稳定，result 恒首位）。
- `src/lib.rs`：挂载 `pub mod dmi;`。
- `src/ui/components.rs`（蓝图 §4 DMIUpdate 纯数据化）：
  - `DMIUpdate` 改为 `{ id, clicked, changed }` 三公有字段，`Copy + PartialEq`，
    `response` 字段与 `clicked()/changed()` 方法删除——Response 不再越过 show() 出口；
  - 构造器：`new(id, clicked, changed)` 公开、`from_parts(id, &Response)` 私有
    （label/text_input/checkbox/slider 四处出口迁移）；button/radio/select 用
    `new(...)` 显式状态位；
  - 测试：`button_update_defaults_idle` 改字段访问；新增 `dmi_update_is_pure_data`
    （Copy + Eq 编译期即证明无 Response 泄漏）。
- `demo/dmi_gallery/src/main.rs`：
  - 全部交互判定迁移到字段访问（`update.clicked` / `update.changed`）；
  - 新增"DMI 自检（硬编码调用）"分区：4 个按钮（greet simple/standard/complex +
    ACCESS 非法请求示范，均 `dmi_name` 登记）调用 `egui_dmi::dmi::call_dmi`，
    应答 JSON 实时显示于匿名 mono 行并写交互日志。
- `docs/design-system.md`：DMIUpdate 纯数据条目 + "DMI 统一调用入口"小节；
  `docs/id-system.md`：构造器措辞更新（from_parts 私有、Response 不出出口）；
  `build_docs/structure.md`：树与描述同步（含修正过时的"两级编号/占位标记"表述）。

## 验证

- `cargo test --workspace`：**21/21**（新增 dmi 测试 4 项：三档应答逐字符比对、
  失败分层裁剪、非法输入专属错误码与 standard 兜底、target_id 形态约定；
  DMIUpdate 纯数据 1 项）；
- `cargo clippy --workspace --all-targets`：0 警告；
- demo 冒烟 5 秒稳定无错误输出。

## 回退

- 全部为新增模块 + 局部迁移，git 回退即恢复；无持久化数据结构变更。
