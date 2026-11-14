# 声明式模型接口（DMI，Declarative Model Interface）开发

## 摘要

本部分开发涉及项目核心部分：构建以大语言模型调用为基础的声明式模型接口。
主要按照以下顺序推进：组件统一管理分配编号 -> 调用途径完善 -> 接口正式开发。
开发时，按照我给出的编号分步处理。

## 1.1 DMI调用规范

DMI采用JSON格式进行调用，一个示例如下。
```json
{ "request_type": "OBSERVATION", "interface": "check_status", "target_id": 0, "complexity": "complex" }
```
在这个示例中，`request_type`定义接口类型，包括`ACCESS` `STATE` `OBSERVATION`三种，含义会在后面详细说明。`interface`对应你需要调用的接口名称。`target_id`是这个操作对应的组件ID，ID分配问题会在后面说明。`complexity`是标记接口返回值细节程度的字段，包含`simple` `standard` `complex`三种模式，不同的接口对于返回值复杂程度有着不同的定义，具体需参考接口文档。

一些可能的返回值为：

```json
{ "result": "OK", "activated": false }
{ "result": "FAILED", "reason": "ID not found." }
```

`result`标明你的此次调用结果，`OK`代表成功，后面会跟一些返回参数；`FAILED`代表失败，`reason`会简要说明失败原因。

## 1.2 组件ID

在本项目中，程序启动时保存一个列表`id`，所有egui组件调用都要经过重新处理的`components`。每当调用组件，系统在`id`中扩展一个新元素，示例如下：

```rust
(1, "button", 0, "text_color_plate")
```

`id`列表持久存在，通过egui的`id`确定每一个组件对应的`DMI id`。

出于可读性考虑，对于单选框，下拉菜单这种同时产生多个组件的工具，`id`分配分两种：
1. 所有组件统一归属于一个整数，使用此`id`则同时修改所有子组件；
2. 给每个子组件分配带小数点的`id`。
一个示例如下：
```rust
(1, "select", 0, "theme_selector"),
(1.1, "select", 0, "theme_selector_light"),
(1.2, "select", 0, "theme_selector_dark"),
```

第一个元素代表本组件的`id`，第二个元素代表本组件的类型，第三个元素代表上级组件`id`，第四个元素代表本组件名称。

特别地，0号为系统预留编号，用于调用系统层面的接口。如果一个组件以不同形式被多次调用（例如，一个调色盘同时应用于改变文字颜色和背景颜色），则它会拥有多个`id`。

完成后，添加`list_id()`方法来查看`id`分配。
