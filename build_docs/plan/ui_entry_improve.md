# UI触及方法改进

## 摘要

本部分旨在为每一个`egui-dmi`组件提供清晰的交互方式列表，便于DMI进行交互。

## 1 基础结构

在ui.rs中加入`UIEntry`枚举：
```rust
pub enum UIEntry {
    LeftClick,
    RightClick,
    DoubleClick,
    CenterClick,
    Hover,
    Drag,
    Focus,
    PressKey(Key)
}
```
仅供参考，以egui中支持的组件触发方式为准。
同时，改进DMIRecord:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DMIRecord {
    pub id: DMIId,
    pub kind: DMIWidgetKind,
    pub parent: (DMIId, UIEntry),
    pub name: String,
    pub function: Vec<(String, Vec<UIEntry>)>,
}
```
新增parent中的UIEntry，明确从父组件触及子组件的方法；function明确组件自身的功能及其对应操作方式，调用接口时只需声明功能名称即可。

## 2 处理逻辑

由于组件触发方式很难确定，因此，触及方法需要在注册组件时直接声明。
在DMI组件注册中添加`dmi_entry()`方法，内部使用vec或其他列表，每一个元素都代表一种激活方式。

后续如果要调用`Access`接口，DMI就能根据触及方法正确处理。
