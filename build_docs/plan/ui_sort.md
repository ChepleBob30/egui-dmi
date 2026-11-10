# UI整理功能实现说明文档

## 简述

此功能目的是整理egui中的几个使用组件，框定DMI修改范围，并统一组件风格，便于统一管理。

## 要求

重点是建立稳定的 UI 架构和视觉规范。

它解决的问题是：统一管理UI，避免DMI处理数据无从下手。

先实现基础组件，包括：按钮，文本输入框，复选框，单选框，下拉菜单，滑块，标签文本。

结构建议：

```text
├── src/ui/design/
│   ├── languages.rs
│   ├── colors.rs
│   ├── typography.rs
│   ├── spacing.rs
│   ├── motion.rs
│   ├── accessibility.rs
│   └── ...
├── src/ui/components/
│   ├── button.rs
│   └── ...
└── docs/design-system.md
```

## 设计语言 - languages

集中定义颜色、间距、圆角、字体、控件高度和动画时间，以便后续DMI统一管理。

```rust
pub struct ColorLanguages {
    pub background: Color32,
    pub surface: Color32,
    pub surface_elevated: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub separator: Color32,
    pub accent: Color32,
    pub destructive: Color32,
}
```

## 组件

调用时，直接使用封装好的egui_dmi组件。

```rust
DMIButton::primary("hello world")
DMIButton::destructive("remove")
```

组件统一负责颜色、圆角、内边距、hover/pressed/focused/disabled 状态、tooltip、焦点、可访问名称、动画和鼠标/触摸目标大小。
