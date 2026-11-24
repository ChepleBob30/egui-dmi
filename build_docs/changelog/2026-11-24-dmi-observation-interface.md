# 2026-11-24 DMI Phase 4：观察快照与读取类接口 check_widget_status

- 依据：`build_docs/plan/dmi.md` §3/§3.1 + 已批准蓝图
  `build_docs/blueprint/dmi_phase4.md`（裁决 1A 数组、2A 分档、3A 复用码、4A 不输出 parent）。

## 新增

- `src/dmi/observe.rs`：观察快照表。
  - `DMIWidgetSnapshot { basics, flags, rect, visible }`；
  - `DMIBasics` 八形态封闭集（七组件 + `OptionItem` 复合子项）与
    `widget_text()` 主文本映射（输入框=绑定文本、下拉=current、滑块=label 或数值串）；
  - `DMIFlags::from_response`（enabled + hovered/clicked/pressed/focused/changed）；
  - `observe()`（SYSTEM 拒收）/ `snapshot_of()`；全局 OnceLock<Mutex<HashMap>>。
- `src/dmi/interface.rs`：`check_widget_status`——
  OBSERVATION 三档组装（simple 三键；standard + enabled + widget_function 数组形态；
  complex + 五个可用性位 + rect{x,y,w,h} + visible）；
  未注册/无快照均回 TARGET_ID_INVALID（裁决 3A）；ACCESS 分支留桩待 §3.2。
- `src/dmi/call.rs`：分发器新增 `check_widget_status` 路由。

## 修改

- `src/ui/components.rs`：七组件 show() 尾部向 observe 推送现场快照
  （下拉子项/单选组成员推 OptionItem；下拉父在 current 写回后推 Select；
  滑块范围在移入前捕获）。
- `demo/dmi_gallery/src/main.rs`（用户指令：不堆砌组件，改造调用测试区）：
  - **移除原四个 greet 探测按钮**；
  - 新增"DMI 调用测试台"：**三个下拉**（接口 greet/check_widget_status、
    请求类型 OBSERVATION/ACCESS/STATE、复杂度三档）+ **一个调用按钮**，
    按选择组装命令调 `call_dmi`，应答显示于输出行并写交互日志；
  - check_widget_status 目标编号**按注册名 save_button 实时反查**（纠正"固定 1 号"
    与注册顺序不符的问题）；
  - 控制台 pump 由帧首移至**帧尾**。
- `build_docs/blueprint/dmi_phase3.md`：追加 pump 帧尾修订记录（启动竞态：
  线程可在首帧绘制前入队，帧首消费时注册表为空 → 误判 TARGET_ID_INVALID；
  帧尾消费使命令落在本帧注册与快照之后，竞态消除且"当帧完成"语义更完整）。

## 验证

- `cargo test --workspace`：**30/30**（新增 observe 往返/SYSTEM 拒收、
  三档集成断言、注册未渲染防御分支 3 项）；`clippy` 0 警告；
- 端到端（管道控制台，启动即发）：
  - `check_widget_status target 1 complex` → OK 全字段（含 rect/visible）；
  - `ACCESS` → REQUEST_TYPE_UNSUPPORTED；`target 99 simple` → 仅 result；
  - `save_button` 名称反查路径 → OK 且 widget_name 正确；
  - `greet complex` → OK 三字段。

## 回退

- 纯新增 + 局部接线，git 回退即恢复。
