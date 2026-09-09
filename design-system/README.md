# OS-Compass 设计系统

## 概述

OS-Compass 设计系统是一套用于指导产品界面开发的规范和资源集合，确保产品在不同页面和组件间保持视觉与交互的一致性。

## 设计理念

| 理念 | 说明 |
|------|------|
| **本地优先** | 界面清晰反馈数据存储在本地 |
| **高效专注** | 减少视觉噪音，突出内容本身 |
| **可扩展性** | 预留插件和自定义空间 |

## 目录结构

```
design-system/
├── README.md           # 本文件，设计系统总览
├── 使用文档.md          # 使用指南
├── tokens/             # 设计令牌（颜色、字体、间距等）
│   └── README.md
├── components/         # UI 组件说明
│   └── README.md
├── patterns/           # 页面级模式与交互规范
│   └── README.md
└── public/             # HTML 原型文件（可直接浏览器预览）
    ├── index.html      # 入口页
    ├── main-interface.html  # 主界面（看板视图）
    ├── list-view.html  # 列表视图
    ├── project-detail.html  # 项目详情（6标签页）
    ├── category-manager.html  # 分类管理
    ├── import-form.html  # 项目导入
    ├── search-view.html  # 搜索视图
    ├── settings.html   # 设置中心
    ├── plugin-manager.html  # 插件管理
    └── obsidian-layout.html  # Obsidian 风格参考
```

## 设计决策

| 决策项 | 选择 | 理由 |
|--------|------|------|
| 配色方案 | 深色主题为主 | 开发者友好，减少视觉疲劳 |
| 图标库 | Font Awesome 6 | 生态丰富，风格统一 |
| 布局风格 | 侧边栏 + 主内容 | 类 Obsidian，符合目标用户习惯 |
| 交互模式 | 直接操作 | 右键菜单、拖拽排序、点击复制 |

## 版本信息

- 当前版本：v1.0
- 更新日期：2026-06-26
- 技术栈：Tailwind CSS 3 + Font Awesome 6