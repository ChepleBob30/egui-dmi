# 声明式模型接口（DMI，Declarative Model Interface）开发

## 摘要

本部分开发涉及项目核心部分：构建以大语言模型调用为基础的声明式模型接口。
主要按照以下顺序推进：组件统一管理分配编号 -> 调用途径完善 -> 接口正式开发。
开发时，按照我给出的编号分步处理。

## 1. 基础架构

### 1.1 DMI调用规范

DMI采用JSON格式进行调用，一个示例如下。
```json
{ "request_type": "OBSERVATION", "interface": "greet", "target_id": 0, "complexity": "simple" }
```
在这个示例中，`request_type`定义接口类型，包括`ACCESS` `STATE` `OBSERVATION`三种，含义会在后面详细说明。`interface`对应你需要调用的接口名称。`target_id`是这个操作对应的组件ID，ID分配问题会在后面说明。`complexity`是标记接口返回值细节程度的字段，包含`simple` `standard` `complex`三种模式，不同的接口对于返回值复杂程度有着不同的定义，具体需参考接口文档。

一些可能的返回值为：

```json
{ "result": "OK", "greet_text": "hello." }
{ "result": "FAILED" }
```

`result`标明你的此次调用结果，`OK`代表成功，后面会跟一些返回参数；`FAILED`代表失败，`reason`会简要说明失败原因。

### 1.2 组件ID

在本项目中，程序启动时保存一个列表`id`，所有egui组件调用都要经过重新处理的`components`。每当调用组件，系统在`id`中扩展一个新元素，示例如下：

```rust
(1, "button", (0, UIEntry::Direct), "text_color_plate", ("open_plate", [UIEntry::LeftClick]))
```

`id`列表持久存在，通过egui的`id`确定每一个组件对应的`DMI id`。

出于可读性考虑，对于单选框，下拉菜单这种同时产生多个组件的工具，`id`分配分两种：
- 所有组件统一归属于一个整数，使用此`id`则同时修改所有子组件；
- 给每个子组件分配带小数点的`id`。
一个示例如下：
```rust
(1, "select", (0, UIEntry::Direct), "theme_selector", ("open_selector", [UIEntry::LeftClick])),
(1.1, "select", (1, UIEntry::LeftClick), "theme_selector_light", ("confirm", [UIEntry::LeftClick, UIEntry::RightClick])),
(1.2, "select", (1, UIEntry::LeftClick), "theme_selector_dark", ("confirm", [UIEntry::LeftClick, UIEntry::RightClick])),
```

第一个元素代表本组件的`id`，第二个元素代表本组件的类型，第三个元素代表上级组件`id`及其触及方法，第四个元素代表本组件名称，第五个元素代表该组件的功能及其触发方式。

特别地，0号为系统预留编号，用于调用系统层面的接口。如果一个组件以不同形式被多次调用（例如，一个调色盘同时应用于改变文字颜色和背景颜色），则它会拥有多个`id`。

完成后，添加`list_id()`方法来查看`id`分配。

## 2. DMI初步实现

### 2.1 DMI基础

DMI代码全部写在src/dmi目录下，其分为两种调用方式：运行时调用和硬编码调用。其接口包含三种类型：Access、State、Observation。
此外，DMI设定Rules抽象层，在ui系统层面全局管理组件；同时加入标签，支持轻量化修改组件状态。
以上内容将会在下方展开说明。

### 2.2 调用方式

依赖一个JSON相关的库，使用特殊的JSON字符串格式来进行开发，以下使用`JString`代指。开发中以实际类型名为准。

一个可能的调用接口如下：
```rust
pub fn call_dmi(command: JString) -> JString {
    // ...
    match command.interface {
        "greet" => {
            match command.request_type {
                "OBSERVATION" => {
                    // ...
                },
            };
            // ...
        },
        _ => {
            // ...
        },
    };
    // ...
}
```
此处，command接收一个完整的JSON字符串，经过接口操作后返回JSON字符串。command包含的内容在下文详细说明，其中有一个`interface`参数，凭此调用接口。

#### 2.2.1 硬编码调用

如题，直接在代码中调用`call_dmi`并输入特定的JSON字符串即可固定调用接口，适用于稳定的人工测试。

#### 2.2.2 运行时调用

编写一个控制台，在图形化界面显示的同时支持在终端中接收规范的JSON字符串输入，然后自动调用`call_dmi`并返回JSON字符串，如果输入出错则返回报错JSON字符串。
这一方法可以让大模型和用户实时调用接口，以便进行调试。

特别强调以下问题：
- 使用独立线程接收输入，不要阻塞ui渲染。
- 打出指令后，直接在当帧内完成接口操作。
- 输入输出只围绕终端，不包含图形化部分。
- 同时适配人类打字操作和大模型指令操作。

### 2.3 接口作用

Access用于让智能体声明目标组件，底层的导航与基础点击操作由DMI自动完成；State用于直接设置特定组件的目标状态（如滑块位置、文本勾选），无需模拟复杂的鼠标拖拽或键盘组合键操作；Observation用于在不改动界面的前提下获取特定组件的状态，与Access的区别在于，Observation不会改变UI已有状态。

### 2.4 示例接口

不妨先编写一个示例接口`greet`。此接口用于查看项目名称等信息，验证程序是否正常工作。
该接口只有Observation一种模式，返回值为欢迎语、项目名(`complexity`设置至少达到`standard`才出现)、项目依赖及其版本号(`complexity`设置至少达到`complex`才出现)。
对于`greet`这种不针对特定组件的接口，声明`target_id`为系统预留编号0即可。
同时，对于调用失败的接口，如果输入`complexity`设置为`standard`及更高，则附带失败原因；设置为`complex`，则再附带
调用的输入输出值示例如下：
```json
{ "request_type": "OBSERVATION", "interface": "greet", "target_id": 0, "complexity": "simple" }

{ "result": "OK", "greet_text": "hello." }
{ "result": "FAILED" }

{ "request_type": "OBSERVATION", "interface": "greet", "target_id": 0, "complexity": "standard" }

{ "result": "OK", "greet_text": "hello, egui.", "project_name": "egui-dmi" }
{ "result": "FAILED", "reason": "Project name not found.", "error_code": "PROJECT_NAME_NOT_FOUND" }

{ "request_type": "OBSERVATION", "interface": "greet", "target_id": 0, "complexity": "complex" }

{ "result": "OK", "greet_text": "hello, egui-dmi.", "project_name": "egui-dmi", "dependencies": "egui 0.37.0" }
{ "result": "FAILED", "reason": "Project name not found.", "error_code": "PROJECT_NAME_NOT_FOUND", "suggestion": "Try again later." }
```

## 3 读取类接口

读取型接口，包括Observation和Access，主要开放基本信息、可用性、上下文三条。
展开来说，基本信息指用于达成组件本身作用的信息，例如输入框的文本，单选按钮的选项及选中项等。可用性基本囊括`egui Response`的一系列字段，包括组件是否可用。上下文主要包含组件的定位信息及状态，例如坐标位置、可见性等。

### 3.1 Observation接口实现

对于任意组件，都可以使用`check_widget_status`接口来检查状态。该接口支持Observation和Access两种类型，此处先说明Observation类型。一个调用示例如下：
```json
{ "request_type": "OBSERVATION", "interface": "check_widget_status", "target_id": 1, "complexity": "simple" }

{ "result": "OK", "widget_name": "enable_quick_search", "widget_type": "button", "widget_text": "Enable quick search" }
{ "result": "FAILED" }

{ "request_type": "OBSERVATION", "interface": "check_widget_status", "target_id": 1, "complexity": "standard" }

{ "result": "OK", "widget_name": "enable_quick_search", "widget_type": "button", "widget_text": "Enable quick search", "enabled": true, "widget_function": { "apply": "LeftClick" } }
{ "result": "FAILED", "reason": "Target id invalid.", "error_code": "TARGET_ID_INVALID" }

{ "request_type": "OBSERVATION", "interface": "check_widget_status", "target_id": 1, "complexity": "standard" }

{ "result": "OK", "widget_name": "enable_quick_search", "widget_type": "button", "widget_text": "Enable quick search", "enabled": true, "widget_function": { "apply": "LeftClick" }, "rect": { "x": 120.0, "y": 500.0, "w": 200.0, "h": 24.0 },
"visible": true }
{ "result": "FAILED", "reason": "Target id invalid.", "error_code": "TARGET_ID_INVALID", "suggestion": "Recheck your id list and call it properly." }
```
此示例并不完全对应实际调用效果，仅供参考。
