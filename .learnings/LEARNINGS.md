# Learnings

纠错、经验和知识缺口。

---

## [LRN-20260716-001] plugin-architecture

**Logged**: 2026-07-16T11:21:00Z
**Priority**: high
**Status**: pending
**Area**: backend

### Summary
VviTools 的捆绑内置插件不能只有 `plugin.json`，应包含入口契约、README 和资产。

### Details
用户明确纠正“剪贴板只是一个 plugin.json 文件吗？没有代码啥的吗”，后续要求改为真正的“捆绑内置插件”。正确结构是根目录 `plugins/<plugin-id>/` 下至少包含 `plugin.json`、内置入口契约文件、README 和图标资产；高权限能力仍由 Rust/Tauri 宿主桥接执行。

### Suggested Action
后续新增系统级内置能力时，优先采用 `runtime: "builtin"` + `builtin.bridge` + `builtin.host_commands` 的插件目录结构，并确保 `plugins/` 作为 Tauri bundle resource 随正式包分发。

## [LRN-20260710-001] macos

**Logged**: 2026-07-10T05:40:00Z
**Priority**: medium
**Status**: pending
**Area**: infra

### Summary
rubick 4.3.8 macOS arm64 包可安装运行，但官方包签名状态可能导致辅助功能 TCC 授权不生效。

### Details
官方 arm64 dmg 复制到 `/Applications/rubick.app` 后，直接启动会在 `uiohook_worker_start` 中 abort，并输出 `hook_run [1405]: Accessibility API is disabled!`。`codesign --verify` 初始报告资源签名异常；对本地 App 执行 `codesign --force --deep --sign - /Applications/rubick.app` 后，重新授权辅助功能，rubick 进程可稳定运行。

### Suggested Action
后续处理 rubick macOS 打不开时，优先清理 quarantine、检查辅助功能权限；若授权后仍失败，先本机重签名 App，再重置并重新授权辅助功能。

---

## [LRN-20260713-001] product

**Logged**: 2026-07-13T10:52:00Z
**Priority**: high
**Status**: pending
**Area**: frontend

### Summary
VviTools 的产品参照应优先对齐 uTools 式命令启动器，而不是 rubick 式插件管理控制台。

### Details
用户明确纠正“此项目和 uTools 完全不一样啊，应该参考 uTools”。本项目核心体验应是全局快捷键呼出、居中搜索框、键盘搜索命令、执行命令、展示结果；插件市场和插件管理是二级入口，不应占据首屏主结构。

### Suggested Action
后续改 VviTools UI/交互时，默认把首屏做成轻量命令面板；只有进入二级模式时再展示插件市场、已安装插件和权限详情。

---

## [LRN-20260714-001] product

**Logged**: 2026-07-14T03:12:00Z
**Priority**: high
**Status**: pending
**Area**: frontend

### Summary
VviTools 的页面和 UI 应参考 rubick 的开源界面，而不是只参考 uTools 的极简命令面板。

### Details
用户再次纠正：“页面和 ui 可以参考 rubick，它是开源的”。正确方向是：主入口保留 rubick 式顶部搜索框、左侧 logo 进入插件市场、下方历史插件宫格；插件中心采用 rubick feature 页的左侧菜单、顶部插件搜索、插件列表、插件详情和已安装页双栏布局。

### Suggested Action
后续 UI 调整优先对照 rubick `src/renderer/components/search.vue`、`src/renderer/components/result.vue`、`feature/src/App.vue` 和 `feature/src/views/installed/index.vue` 的信息架构，再按 Tauri/Svelte 项目约束轻量复刻。

---

## [LRN-20260714-002] window-management

**Logged**: 2026-07-14T05:41:00Z
**Priority**: medium
**Status**: pending
**Area**: backend

### Summary
VviTools 快捷键呼出搜索窗时必须同时恢复搜索窗口尺寸和位置，不能只调用 `show()` 或 `center()`。

### Details
用户截图反馈“通过快捷键弹出来的弹窗会显示到偏上”。根因是窗口从插件市场大尺寸切回搜索模式时，快捷键唤起路径没有走 `set_launcher_view("launcher")`，会沿用旧窗口高度或旧 top-left。后续又发现点击外部隐藏后再快捷键唤起，若前端仍停留在插件市场视图，会把二级页塞进搜索窗尺寸里。修复方式是把窗口尺寸和可用屏幕居中逻辑抽成 Rust 公共函数，并在快捷键显示窗口时发事件通知前端重置到 launcher 视图。

### Suggested Action
后续新增任何显示主窗口的入口，都应调用统一窗口复位函数，确保搜索页尺寸为 720x420 并按当前显示器 work area 居中；同时要同步前端 `view/query/result/error` 状态回 launcher，避免窗口尺寸和页面视图不一致。

---

## [LRN-20260714-003] desktop-behavior

**Logged**: 2026-07-14T07:06:00Z
**Priority**: high
**Status**: pending
**Area**: backend

### Summary
VviTools 应保持 rubick/uTools 式后台工具形态：隐藏 Dock 图标，常驻状态栏图标，通过快捷键或状态栏唤起。

### Details
用户要求“保持和 rubick 以及 uTools 一样，并且在状态栏添加图标”。rubick 源码中通过 `app.dock.hide()`、`LSUIElement: 1`、窗口 `skipTaskbar` 达成同类行为。VviTools 在 Tauri 中应使用 macOS `ActivationPolicy::Accessory` 和 `set_dock_visibility(false)` 隐藏 Dock，并创建 tray/status bar icon，提供显示/退出入口。

### Suggested Action
后续新增启动、退出、窗口显示相关逻辑时，应维护状态栏工具模型；不要让主窗口成为普通常驻 Dock 应用。

---

## [LRN-20260714-004] clipboard

**Logged**: 2026-07-14T07:58:00Z
**Priority**: high
**Status**: pending
**Area**: backend

### Summary
VviTools 的剪贴板应作为宿主核心工具实现，而不是只作为插件 action 的复制能力。

### Details
用户明确纠正“我要的是完整的剪贴板工具”。正确方向是内置后台剪贴板监听、持久化历史、搜索、恢复复制、删除和清空等完整工具能力；插件侧 copy action 只是复用核心剪贴板服务的一个调用入口。当前先实现文本剪贴板历史闭环，后续图片和文件剪贴板应继续放在主进程核心服务中做跨平台封装。

### Suggested Action
后续扩展剪贴板能力时，优先完善主进程 clipboard service，再通过权限模型暴露给插件；不要让插件直接访问系统剪贴板。

---

## [LRN-20260714-005] clipboard

**Logged**: 2026-07-14T08:10:00Z
**Priority**: medium
**Status**: pending
**Area**: frontend

### Summary
VviTools 剪贴板工具需要支持类似 Paste 的底部快捷面板。

### Details
用户要求“剪贴板可以支持快捷键在底部弹出剪贴板内容，比如 pasted”。当前实现使用 Alt + V 打开剪贴板，窗口复用主窗口但切换为 960x360，并定位到当前显示器 work area 底部居中；Alt + Space 仍负责搜索启动器。

### Suggested Action
后续如果扩展剪贴板体验，应优先优化底部面板的键盘导航、选中复制、分组和图片/文件预览，而不是只在插件中心里展示剪贴板页。

---

## [LRN-20260714-006] clipboard

**Logged**: 2026-07-14T08:44:00Z
**Priority**: high
**Status**: pending
**Area**: frontend

### Summary
VviTools 主窗口不能承载剪贴板历史列表；剪贴板历史必须是独立窗口，主窗口只展示系统剪贴板应用信息页。

### Details
用户截图纠正：“正确是独立的页面啊，怎么还是在 vvitools 里面的”“vvitools 里面只需要应用信息页”。正确结构是：VviTools 插件市场/应用中心里展示“系统剪贴板”的应用详情、介绍和预览；Alt + V 打开独立的 clipboard 窗口，只在该窗口中展示剪贴板历史、搜索、复制、删除、清空。

### Suggested Action
后续新增系统工具时，区分应用信息页与实际工具窗口；主 VviTools 负责发现和配置，工具运行界面应按场景使用独立窗口或独立面板。

## [LRN-20260716-clipboard-file-priority] clipboard

**Logged**: 2026-07-16T16:22:55
**Priority**: high
**Status**: done
**Area**: backend/frontend

### Summary
VviTools 文件剪贴板应优先识别 file URL，不要让文件图标或预览图落入图片历史。

### Details
macOS 复制 PDF/文件时可能同时暴露文件名文本和图标/预览图。剪贴板监听应先读取 NSPasteboard file URL 类型，命中文件后记录为 kind=file，并跳过 image/text 通道。

### Suggested Action
后续扩展文件剪贴板时继续在 Rust 主进程统一处理 file URL、复制回写和 UI 展示，避免前端或图片通道猜测文件类型。

---

## [LRN-20260716-tauri-transparent-radius] frontend

**Logged**: 2026-07-16T17:41:41
**Priority**: medium
**Status**: done
**Area**: frontend/config

### Summary
Tauri 透明浮窗圆角需要透明缓冲区，不要让可见 CSS 外壳直接贴满 WebView 边界。

### Details
VviTools 剪贴板浮窗即使启用 transparent/decorations=false，若 `.rubick-window` 直接 `100vw/100vh` 绘制边框、圆角和阴影，圆角/阴影会被原生窗口边界裁切，视觉上残留直角边框。

### Suggested Action
透明浮窗应把 Tauri 窗口尺寸加大一圈，并让前端可见容器使用 `width/height: calc(100vw - inset)` 加 `margin` 留出透明缓冲区。

---
