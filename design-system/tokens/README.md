# 设计令牌 (Design Tokens)

设计令牌是设计系统的基础原子，用于确保跨界面的视觉一致性。

## 1. 颜色 (Colors)

### 主题色

| Token | 值 | 用途 |
|-------|-----|------|
| `color.primary` | #2563eb (blue-600) | 主按钮、重要强调 |
| `color.primary.hover` | #1d4ed8 (blue-700) | 主按钮悬停 |
| `color.primary.light` | #dbeafe (blue-100) | 选中背景、徽章 |

### 语义色

| Token | 值 | 用途 |
|-------|-----|------|
| `color.success` | #16a34a (green-600) | 成功状态 |
| `color.warning` | #ca8a04 (yellow-600) | 警告状态 |
| `color.error` | #dc2626 (red-600) | 错误/危险操作 |
| `color.info` | #0284c7 (sky-600) | 信息提示 |

### 中性色

| Token | 值 | 用途 |
|-------|-----|------|
| `color.text.primary` | #111827 (gray-900) | 主要文字 |
| `color.text.secondary` | #6b7280 (gray-500) | 次要文字 |
| `color.text.muted` | #9ca3af (gray-400) | 占位符、禁用文字 |
| `color.border` | #e5e7eb (gray-200) | 边框线 |
| `color.bg.default` | #ffffff | 默认背景 |
| `color.bg.subtle` | #f3f4f6 (gray-100) | 次要背景 |
| `color.bg.hover` | #f9fafb (gray-50) | 悬停背景 |

### 语言标识色

| 语言 | 值 |
|------|-----|
| JavaScript | #f7df1e |
| TypeScript | #3178c6 |
| Python | #3776ab |
| Rust | #dea584 |
| Go | #00add8 |
| Java | #ed8b00 |
| C++ | #00599c |

---

## 2. 字体 (Typography)

### 字体族

```css
font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
```

### 字号

| Token | 值 | 用途 |
|-------|-----|------|
| `text.xs` | 12px | 辅助说明、小标签 |
| `text.sm` | 14px | 正文、次要信息 |
| `text.base` | 16px | 主要内容 |
| `text.lg` | 18px | 页面标题 |
| `text.xl` | 20px | 区块标题 |
| `text.2xl` | 24px | 页面大标题 |

### 字重

| Token | 值 | 用途 |
|-------|-----|------|
| `font.normal` | 400 | 正文 |
| `font.medium` | 500 | 强调文字 |
| `font.bold` | 700 | 标题、重要信息 |

---

## 3. 间距 (Spacing)

基于 4px 网格系统。

| Token | 值 | 用途 |
|-------|-----|------|
| `space.1` | 4px | 紧凑间距 |
| `space.2` | 8px | 小间距 |
| `space.3` | 12px | 中小间距 |
| `space.4` | 16px | 中等间距 |
| `space.5` | 20px | 中大间距 |
| `space.6` | 24px | 大间距 |
| `space.8` | 32px | 区块间距 |
| `space.10` | 40px | 大区块间距 |

---

## 4. 圆角 (Border Radius)

| Token | 值 | 用途 |
|-------|-----|------|
| `radius.sm` | 4px | 小按钮、标签 |
| `radius.md` | 6px | 输入框、中按钮 |
| `radius.lg` | 8px | 卡片、大按钮 |
| `radius.xl` | 12px | 大卡片、模态框 |
| `radius.full` | 9999px | 圆形头像、胶囊按钮 |

---

## 5. 阴影 (Shadows)

| Token | 值 | 用途 |
|-------|-----|------|
| `shadow.sm` | 0 1px 2px rgba(0,0,0,0.05) | 轻微凸起 |
| `shadow.md` | 0 4px 6px rgba(0,0,0,0.1) | 卡片默认 |
| `shadow.lg` | 0 10px 15px rgba(0,0,0,0.1) | 弹窗、浮层 |
| `shadow.xl` | 0 20px 25px rgba(0,0,0,0.15) | 模态框 |

---

## 6. 动画 (Animation)

### 过渡时长

| Token | 值 | 用途 |
|-------|-----|------|
| `duration.fast` | 150ms | 微交互、hover |
| `duration.normal` | 200ms | 常规过渡 |
| `duration.slow` | 300ms | 页面切换 |

### 缓动函数

```css
ease-out: cubic-bezier(0.16, 1, 0.3, 1);    /* 进入动画 */
ease-in: cubic-bezier(0.7, 0, 0.84, 0);     /* 退出动画 */
ease-in-out: cubic-bezier(0.65, 0, 0.35, 1); /* 状态切换 */
```

---

## 7. 布局尺寸

| Token | 值 | 用途 |
|-------|-----|------|
| `sidebar.width` | 256px | 侧边栏宽度 |
| `header.height` | 64px | 顶部栏高度 |
| `table.row.height` | 48px | 表格行高 |
| `button.height.sm` | 32px | 小按钮高度 |
| `button.height.md` | 40px | 中按钮高度 |
| `button.height.lg` | 48px | 大按钮高度 |

---

## 8. 图标 (Icons)

使用 Font Awesome 6 Free。

### 常用图标映射

| 用途 | 图标名 |
|------|--------|
| 添加/新增 | `fa-plus` |
| 编辑 | `fa-pen` |
| 删除 | `fa-trash` |
| 搜索 | `fa-magnifying-glass` |
| 设置 | `fa-gear` |
| 返回 | `fa-arrow-left` |
| 复制 | `fa-copy` |
| 下载 | `fa-download` |
| 外部链接 | `fa-external-link` |
| 展开 | `fa-chevron-down` |
| 折叠 | `fa-chevron-right` |
| 文件夹 | `fa-folder` |
| 标签 | `fa-tag` |
| 插件 | `fa-plug` |
| AI | `fa-wand-magic-sparkles` |
| 语言 | `fa-language` |

---

## 9. 状态颜色

### 生命周期状态

| 状态 | 背景色 | 文字色 |
|------|--------|--------|
| 待探索 (TO_EXPLORE) | blue-100 | blue-700 |
| 研究中 (DIVING) | yellow-100 | yellow-700 |
| 已落地 (IN_USE) | green-100 | green-700 |
| 弃用 (ABANDONED) | gray-100 | gray-600 |

### 组件状态

| 状态 | 表现 |
|------|------|
| Default | 正常显示 |
| Hover | 背景色加深、鼠标指针变化 |
| Active | 按下效果（颜色更深） |
| Disabled | opacity-50、cursor-not-allowed |
| Loading | 显示 spinner、禁用交互 |
| Error | 红色边框 `#dc2626` |
| Success | 绿色边框 `#16a34a` |