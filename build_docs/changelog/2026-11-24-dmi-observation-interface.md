# 2026-11-24 DMI 第四阶段：组件状态记录与读取类接口 `check_widget_status`

- 依据：`build_docs/plan/dmi.md` 第 3 章与第 3.1 节。
- 实现蓝图：`build_docs/blueprint/dmi_phase4.md`。
  蓝图中已经确认的四项决定如下：功能映射在返回值中使用数组形式；
  除 `enabled` 以外的五个交互状态字段只在 complex 等级输出；
  查不到组件状态记录时复用 `TARGET_ID_INVALID` 错误码；
  `parent` 字段在所有等级都不默认输出。

## 新增内容

- `src/dmi/observe.rs`：组件状态记录表。程序每帧绘制时，组件把自己当前的
  状态写入这张表，读取类接口从这张表中查询。
  - `DMIWidgetSnapshot`：一条完整的状态记录，包含基本信息、交互状态、
    位置矩形和是否可见四个部分。
  - `DMIBasics`：基本信息部分，按组件类型分成八种固定形态：七种组件各一种，
    另外一种是下拉菜单的选项和单选组的成员共用的形态（`OptionItem`）。
    其中的 `widget_text()` 方法负责提取组件的主文本：输入框返回当前输入的
    文本，下拉菜单返回当前选中的值，滑块优先返回标签文字，没有标签时返回
    数值转换成的文字。
  - `DMIFlags`：交互状态部分，包含 `enabled`、`hovered`、`clicked`、
    `pressed`、`focused`、`changed` 六个布尔值，由
    `DMIFlags::from_response` 从 egui 的交互结果中提取。
  - `observe()` 负责写入记录，编号为 `DMIId::SYSTEM` 的未注册组件会被直接
    拒绝写入；`snapshot_of()` 负责读取记录。存储使用进程级的全局表，
    加锁和恢复的方式与 `src/ui/id.rs` 保持一致。
- `src/dmi/interface.rs`：新增 `check_widget_status` 接口。该接口目前只支持
  OBSERVATION 请求类型，按三个详细等级返回不同的内容：
  - simple 等级：返回 `widget_name`、`widget_type`、`widget_text` 三个字段；
  - standard 等级：追加 `enabled` 和 `widget_function` 两个字段，
    `widget_function` 中每个功能对应的触及方式列表使用数组形式；
  - complex 等级：再追加 `hovered`、`clicked`、`pressed`、`focused`、
    `changed` 五个交互状态字段，以及 `rect`（包含 `x`、`y`、`w`、`h`
    四个数值）和 `visible`。
  请求的编号在注册表中不存在，或者存在但组件从未被绘制过（查不到状态记录），
  两种情况都返回 `TARGET_ID_INVALID`。ACCESS 和 STATE 请求暂时返回
  `REQUEST_TYPE_UNSUPPORTED`，等计划文档第 3.2 节明确含义后再实现。
- `src/dmi/call.rs`：在 `call_dmi` 函数中新增 `check_widget_status` 的连接
  （相当于索引表中新增的一条对应关系），把这个接口名指向对应的实现函数。

## 修改内容

- `src/ui/components.rs`：七种组件的 `show()` 方法在返回之前，向状态记录表
  写入自己当前的状态。其中：下拉菜单的每个选项和单选组的每个成员写入共用的
  选项形态；下拉菜单在选中值写回之后才记录，保证记录中的当前值是本帧的
  最新值；滑块的数值范围在范围值被移动走之前先行读取并保存。
- `demo/dmi_gallery/src/main.rs`：改造接口调用测试区。
  - 删除原来的四个固定命令演示按钮（`greet` 的三个等级按钮和一个错误示范按钮）。
  - 新增"DMI 调用测试台"：三个下拉框分别选择接口名称（`greet`、
    `check_widget_status`）、请求类型（OBSERVATION、ACCESS、STATE）和
    详细等级（simple、standard、complex）；一个"调用"按钮按照当前选择
    组装命令字符串，交给 `call_dmi` 函数执行，返回内容显示在输出行中，
    并写入交互日志。
  - `check_widget_status` 的目标编号不再写死为 1，而是按照名称 `save_button`
    在注册表中实时查找对应的编号，这样查找结果不再受组件注册先后顺序的影响。
  - 控制台命令的处理位置，从每帧绘制开始时移动到每帧绘制结束时。
- `build_docs/blueprint/dmi_phase3.md`：补充上述处理位置调整的修订记录。
  调整原因是：控制台线程可能在程序第一帧绘制完成之前就把命令放入队列，
  如果在每帧开始时处理命令，此时注册表还是空的，针对组件编号的命令会被
  错误地判定为 `TARGET_ID_INVALID`；改到每帧结束时处理，命令面对的是本帧
  已经完成注册和状态记录的注册表，这个问题自然消除，同时也更符合计划文档
  "打出指令后，直接在当帧内完成接口操作"的要求。

## 验证结果

- `cargo test --workspace`：30 项测试全部正常通过。本次新增三项：
  状态记录写入后读取一致，并且 `DMIId::SYSTEM` 被拒绝写入；
  绘制一个按钮之后，`check_widget_status` 三个等级的返回内容逐字段核对正确；
  只注册但未绘制的编号返回 `TARGET_ID_INVALID`。
- `cargo clippy --workspace --all-targets`：没有任何警告。
- 端到端测试（把整个流程从头到尾实际跑一遍的验证）：通过管道
  （一种把一段文本直接接到程序标准输入的操作系统传递方式）向程序的标准输入
  连续写入命令，并读取标准输出，
  确认以下行为全部正确：complex 等级返回包含 `rect` 和 `visible` 的完整状态；
  ACCESS 请求返回 `REQUEST_TYPE_UNSUPPORTED`；simple 等级的错误请求只返回
  `result` 字段；按名称查找 `save_button` 的返回内容中 `widget_name` 正确；
  `greet` 的 complex 等级返回三个字段。

## 回退方式

- 本次改动全部是新增文件与少量衔接修改，没有删除既有功能，
  通过 git 回退即可恢复到之前的状态。
