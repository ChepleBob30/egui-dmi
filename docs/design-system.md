# 设计系统与组件规范（egui-dmi）

## 定位

本项目是面向大模型工作的测试项目。设计参数只保留**颜色**与**间距**两组，
作为 DMI（声明式模型接口，Declarative Model Interface）可以统一管理的修改范围。
动画、字体、可访问性等其他外观参数全部使用 egui 的默认值，目的是缩小测试范围。

## 设计参数的入口

```rust
// 应用启动时调用一次。深色和浅色两种外观模式的样式会同时写入 egui 的全局样式表。
egui_dmi::ui::DesignLanguages::install(&ctx);
```

## 颜色参数 `ColorLanguages`

下表是两个外观模式各一套的基础颜色（十六进制颜色值）：

| 字段 | 深色模式 | 浅色模式 |
| --- | --- | --- |
| `background` | #17181C | #F4F5F8 |
| `surface` | #22242A | #FAFAFC |
| `surface_elevated` | #2B2E36 | #FFFFFF |
| `text_primary` | #E8EAF0 | #1D2230 |
| `text_secondary` | #9BA1AE | #5F6673 |
| `separator` | #3A3E48 | #D9DCE3 |
| `accent` | #5B8CFF | #2F6FE4 |
| `destructive` | #E5484D | #D2382E |
| `own_message`（预留给聊天气泡组件） | #2F5FD0 | #D6E6FF |
| `other_message`（预留给聊天气泡组件） | #313540 | #EFF0F3 |

状态颜色由基础颜色按比例混合得到（使用 egui 的 `lerp_to_gamma`，
也就是在 gamma 编码下的线性混合，gamma 编码是 egui 存储颜色的默认方式）：

- `hover_of(base)`：把 `base` 向 `accent` 方向混合 12%，得到悬停时的颜色；
- `pressed_of(base)`：把 `base` 向 `accent` 方向混合 24%，得到按下时的颜色；
- `disabled_of(base)`：把 `base` 向 `surface` 方向混合 55%，得到禁用时的颜色；
- `selection_fill()`：把 `accent` 向 `surface` 方向混合 55%，得到文本选中区域的底色。

## 间距参数 `SpacingTokens`

| 字段 | 数值 | 用途 |
| --- | --- | --- |
| `pad_horizontal` / `pad_vertical` | 12 / 6 | 按钮内部留白，输入框文字与边框的距离 |
| `radius_sm` / `radius_md` / `radius_lg` | 6 / 8 / 12 | 控件、弹出层、窗口三种圆角半径 |
| `control_height` / `control_height_sm` | 32 / 26 | 常规控件与紧凑控件的目标高度 |
| `field_width` / `combo_width` / `slider_width` | 240 / 160 / 200 | 输入框、下拉菜单、滑块的默认宽度 |

## 组件

所有组件遵循同一个使用模式：**构建方法 → 链式设置（其中包括用 `dmi_name`
声明注册名称）→ 调用 `show(ui)` 得到 `DMIUpdate`**。

只有声明了注册名称的组件才会进入编号注册表；没有声明名称的组件照常显示，
但 `DMIUpdate.id` 等于 `DMIId::SYSTEM`，可以据此判断该组件没有被注册。
功能映射（每个功能名称对应哪些 `UIEntry` 触及方式）保存在编号注册表这一层，
组件对象自己不持有这个字段。

```rust
DMIButton::primary("保存设置").dmi_name("save_button").show(ui);
```

各组件的调用写法：

```rust
DMIButton::primary("hello world").show(ui);
DMIButton::destructive("remove").enabled(can_delete).tooltip("删除此项").show(ui);
DMILabel::heading("标题").show(ui);
DMITextInput::new(&mut text).placeholder("搜索…").show(ui);
DMICheckbox::new(&mut checked, "启用加速").show(ui);
DMIRadio::new(is_selected, "选项 A").show(ui);
DMISelect::new("模式", &mut current, &options).show(ui);
DMISelect::grouped("预设", &mut current, PRESET_GROUPS).show(ui);  // 分组下拉，三层编号
DMISlider::new(&mut volume, 0.0..=100.0).show(ui);
```

补充说明：

- `DMIButton` 由本库自己绘制，按"禁用、按下、悬停、常态"四种情况分别着色，
  优先级从禁用开始依次判断；其余组件复用 egui 的原生控件，
  外观由安装设计参数后的全局样式统一控制。
- 注册表里的每条记录包含五部分内容：编号、类型、（上级编号与触及方式）、
  名称、功能映射，可以用 `list_id()` 读出全部内容
  （详见 `docs/id-system.md` 的完整说明）。
- `DMIUpdate` 是纯数据记录 `{ id, clicked, changed }`，可以复制、可以比较。
  egui 的 `Response` 只在组件 `show()` 内部使用，不会返回给调用方。
  其中 `id` 是组件在注册表中的编号（例如 `3`、`5.2`、`6.1.2`，0 号留给系统），
  也就是将来 JSON 调用里 `target_id` 的取值。
- 编号复用规则：名称相同就复用原编号，名称不同就另行分配新编号。
  组件用 `.dmi_name("theme_selector")` 声明名称；
  `DMISelect` 加 `.individual_ids()` 后，每个选项都会以
  "父名称_选项名称"注册为"父编号.序号"的子条目；
  `DMISelect::grouped` 分组下拉自动启用多层编号：
  分组得到"父编号.序号"，分组内的选项再得到下一层编号。

## DMI 统一调用入口

命令与应答都是完整的 JSON 字符串，公开入口只有一个函数：

```rust
// 把一条完整命令交给 call_dmi，得到一条应答字符串。
let response: String = egui_dmi::dmi::call_dmi(
    "{\"request_type\":\"OBSERVATION\",\"interface\":\"greet\",\"target_id\":0,\"complexity\":\"standard\"}",
);
```

命令包含四个公共字段：

- `request_type`：请求类型，取值 ACCESS、STATE、OBSERVATION，大写；
- `interface`：要调用的接口名称；
- `target_id`：目标组件的编号。0 或 1.2 这类值可以写成 JSON 数字，
  两层以上的小数编号必须写成字符串，例如 `"1.2.5"`，
  因为 `1.2.5` 本身不是合法的 JSON 数字；
- `complexity`：返回内容的详细等级，取值 simple、standard、complex，小写。

出错时的应答按详细等级逐级增加内容：simple 只返回 `result`；
standard 追加 `reason`（失败原因）和 `error_code`（错误码）；
complex 再追加 `suggestion`（修改建议）。
命令字符串本身解析失败时，因为无法知道请求的等级，统一按 standard 返回。

已实现的接口：

- `greet`：自检问候接口，只支持 OBSERVATION，`target_id` 固定为 0。
  simple 返回欢迎语；standard 追加项目名称；complex 再追加依赖库及其版本。
- `check_widget_status`：读取指定编号组件的当前状态，只支持 OBSERVATION。
  simple 返回 `widget_name`、`widget_type`、`widget_text`；
  standard 追加 `enabled` 与 `widget_function`；
  complex 再追加五个交互状态字段（`hovered`、`clicked`、`pressed`、
  `focused`、`changed`）以及 `rect`（位置与大小）和 `visible`（是否可见）。
  这些数据来自组件每帧写入 `src/dmi/observe.rs` 的状态记录。
  demo 画廊的"DMI 调用测试台"分区用三个下拉框加一个调用按钮演示了该入口。

运行时调用：程序在终端里启动后，`DMIConsole::spawn` 启动的独立线程会逐行读取
终端输入的标准输入（标准输入是程序默认的输入通道，在终端里就是键盘输入），
把每条 JSON 命令排队，界面在每帧绘制结束时执行 `pump` 完成命令并把结果打印回终端。
同一时间只处理一条命令，超过 10 秒没有结果就返回超时错误；
终端输出只有 JSON 行，没有提示符；图形界面里不显示任何控制台内容。
这样人和大模型程序都可以在软件运行期间实时调用接口。

## DMI 可以修改的范围

大模型（DMI 层）允许修改的全部参数等于：`ColorLanguages` 的全部字段，
加上 `SpacingTokens` 的全部字段，再加上各组件构建方法的参数。
其余 egui 样式不在本框架的管理范围之内。
