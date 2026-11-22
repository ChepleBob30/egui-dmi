# 2026-11-22 编号机制升级：两级 DMIId → 任意深度小数路径（A 方案）

- 依据：`build_docs/blueprint/dmi_phase2_id.md`（用户批准，A=仅机制、不落地嵌套组件实例）。
- 用户同期裁决：`DMICommand.payload` 透传修订不写入 dmi_phase2.md。

## 改动

- `src/ui/id.rs`：
  - `DMIId` 表示替换：`{ major, minor }` → `{ major, minors: [u32; DMI_MAX_DEPTH], depth }`，
    `DMI_MAX_DEPTH = 4`（定长数组保 `Copy + Hash + Ord` 热路径零分配）；
  - 新增 `is_root()`、`level(index)`；`child(minor)` 返回 `Option`（满深度 `None`）；
    `Display` 输出 `1` / `1.2` / `1.2.3`；
  - `allocate_child` 深度守卫：第 5 层请求 debug `debug_assert` 提示、
    release 返回 `SYSTEM` 哨兵且不入表；`minor_counters` 以任意 `DMIId` 为键，
    孙层发号天然成立，`ReuseKey`/快照/版本/定位全部签名不变。
- `src/ui/components.rs`：组测试 4 处 `minor` 判定迁移到 `is_root()/level(0)`；组件逻辑零改动。
- `docs/id-system.md`：§2.1 契约表、§3.3 深度守卫、§6 三层预留说明、§7 深度上限边界行。
- `build_docs/blueprint/dmi_phase2.md`：target_id 形态约定联动
  （数字仅一级小数，`1.25` 即单级 25；两级以上用点分字符串；测试项同步）。

## 测试

- 新增：`multi_level_construction_and_display`、`grandchild_chain_allocates_under_child`
  （1 → 1.1 → 1.1.1/1.1.2，parent 链、名称复用、不增长）、
  `child_at_max_depth_returns_none`、`depth_limit_panics_in_debug_build`（debug）/
  `depth_limit_rejects_without_registering`（release，cfg 分档）、`order_compares_path_segments`。
- `cargo test --lib` debug 15/15；`cargo test --lib --release` 15/15；
  `cargo clippy --workspace --all-targets` 0 警告。

## 回退

- 纯表示升级、无持久化数据，git 直接回退即恢复两级形态。
