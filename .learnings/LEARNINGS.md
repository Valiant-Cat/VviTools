# Learnings

纠错、经验和知识缺口。

---

## [LRN-20260914-008] frontend

**Logged**: 2026-09-14T16:45:00+08:00
**Priority**: medium
**Status**: done
**Area**: frontend

### Summary
VviTools 应用详情页的返回入口采用 App Store 风格的圆形纯图标按钮。

### Details
用户明确要求详情页返回按钮去掉文字，使用左箭头和圆形浅色背景，提升详情页的简洁度与平台一致性。该样式只用于应用详情层，剪贴板设置等需要表达返回目标的子页面继续保留文字。

### Suggested Action
后续详情型页面优先复用圆形返回图标；设置流程或返回目标不明显时保留文字说明。

### Metadata
- Source: user_feedback
- Related Files: src/App.svelte, src/style.css
- Tags: app-detail, navigation, macos

---

## [LRN-20260914-009] frontend

**Logged**: 2026-09-14T17:00:00+08:00
**Priority**: medium
**Status**: done
**Area**: frontend

### Summary
侧边栏全局入口与详情页局部返回不应同时使用相同的左箭头。

### Details
左侧“返回搜索”和详情页“返回列表”同时显示箭头，会让不同导航层级看起来重复。侧边栏顶部应使用应用标识和 VviTools 名称表达全局主页入口，详情内容区则保留圆形返回箭头表达局部层级返回。

### Suggested Action
全局导航入口优先使用品牌或主页语义；仅在内容层级返回时使用返回箭头。

### Metadata
- Source: user_feedback
- Related Files: src/App.svelte, src/style.css
- Tags: sidebar, navigation, hierarchy

---

## [LRN-20260914-007] plugin-detail-actions

**Logged**: 2026-09-14T14:30:00+08:00
**Priority**: high
**Status**: done
**Area**: frontend

### Summary
插件详情中的设置应使用轻量图标操作，应用描述应独立位于应用头部信息下方。

### Details
将“设置”做成与主操作同规格的文字边框按钮，会和“打开”争夺视觉层级。应用描述夹在名称和元数据之间也会让头部横向拥挤。详情头部应只突出名称、状态、元数据和主操作，描述放到图标所在首行下方并横向展开。

### Suggested Action
插件详情只保留一个明确的主按钮；设置等熟悉的次级操作使用带悬停提示的图标按钮，并紧跟在应用名右侧。捆绑插件不展示“内置”标签。描述作为独立信息行，从图标左边界开始对齐。

### Metadata
- Source: user_feedback
- Related Files: src/App.svelte, src/style.css
- Tags: plugin-detail, hierarchy, actions, description

---

## [LRN-20260914-006] installed-plugin-view

**Logged**: 2026-09-14T12:45:00+08:00
**Priority**: high
**Status**: done
**Area**: frontend

### Summary
VviTools 的已安装页应与探索、开发、自定义共用插件卡片和详情样式。

### Details
为已安装插件单独使用左侧列表加右侧详情的双栏管理界面，会破坏插件中心的一致性，并在插件数量少时产生大片无意义空白。已安装只是插件集合的一种筛选状态，不应拥有另一套浏览结构。

### Suggested Action
插件中心的集合页统一复用同一套卡片网格；点击任意来源的插件后进入同一详情组件，仅根据安装状态调整操作按钮和状态标签。

### Metadata
- Source: user_feedback
- Related Files: src/App.svelte, src/style.css
- Tags: ui, installed-plugins, consistency, plugin-market

---

## [LRN-20260914-005] plugin-center-ui

**Logged**: 2026-09-14T12:20:00+08:00
**Priority**: high
**Status**: done
**Area**: frontend

### Summary
VviTools 插件中心应采用紧凑、扁平的工具型界面，不展示面向开发者的插件协议字段。

### Details
用户认为大侧栏、大标题、装饰性预览卡和卡片套卡片的设置页过于笨重。插件列表应保持可扫描的紧凑尺寸，详情页只展示用户能理解的功能、版本、安装来源和权限用途；runtime、entry、原始权限 ID 等内部信息不应出现在普通用户页面。空状态需要提供与当前页面相关的有效操作。

### Suggested Action
后续插件中心 UI 调整继续沿用小尺寸、低饱和、弱阴影和分隔线分组；新增 manifest 字段时默认只用于内部逻辑，除非能转译为明确的用户价值。

### Metadata
- Source: user_feedback
- Related Files: src/App.svelte, src/style.css
- Tags: ui, plugin-market, settings, information-architecture

---

## [LRN-20260914-004] plugin-navigation

**Logged**: 2026-09-14T11:30:00+08:00
**Priority**: high
**Status**: done
**Area**: frontend

### Summary
VviTools 插件市场左侧上方分类只保留探索、开发、自定义。

### Details
效率、搜索工具、图像、系统等底层分类不再作为一级菜单展示，避免工具箱菜单过长、层级过细。探索展示全部市场插件，开发筛选开发类插件，自定义展示用户导入插件；已安装和设置继续作为底部独立入口。

### Suggested Action
后续新增插件分类时优先作为插件元数据和详情标签使用，不直接扩展一级菜单，除非形成稳定且高频的独立使用场景。

### Metadata
- Source: user_feedback
- Related Files: src/App.svelte
- Tags: navigation, plugin-market, categories

---

## [LRN-20260914-003] clipboard-feedback

**Logged**: 2026-09-14T11:14:36+08:00
**Priority**: high
**Status**: done
**Area**: frontend

### Summary
点击剪贴板 item 后，不应把第一项或列表区域临时改成状态提示。

### Details
无论辅助功能权限是否开启，剪贴板第一项都应保持原内容。未授权时由独立引导弹窗说明；已授权时直接关闭窗口并粘贴。窗口重新唤出前应先清空旧状态，避免状态条残留造成第一项闪动或位移。

### Suggested Action
关闭剪贴板窗口的操作不写入列表内状态提示；重新显示窗口时先分发重置事件，再调用 show。

### Metadata
- Source: user_feedback
- Related Files: src/App.svelte, src-tauri/src/commands.rs, src-tauri/src/main.rs
- Tags: clipboard, feedback, flicker, permission

---

## [LRN-20260914-002] macos-accessibility-signing

**Logged**: 2026-09-14T10:53:50+08:00
**Priority**: high
**Status**: done
**Area**: config

### Summary
VviTools 本地安装构建必须使用固定代码签名身份，才能跨重新编译保留 macOS 辅助功能授权。

### Details
Ad-hoc 签名的 Designated Requirement 绑定每次构建的 CDHash，重新编译后 TCC 会视为新的应用身份。固定自签名证书后，不同 CDHash 的构建仍共享由 Bundle ID 和证书根组成的 Designated Requirement。首次切换签名需重新授权一次，后续构建可复用授权。

### Suggested Action
本机先运行 `npm run signing:setup`，之后始终使用 `npm run build:install`。不要删除登录钥匙串中的 `VviTools Local Development` 身份，也不要随意变更 Bundle ID。

### Metadata
- Source: conversation
- Related Files: scripts/setup-macos-signing.sh, scripts/build-install-macos.sh, package.json
- Tags: macos, accessibility, tcc, codesign

---

## [LRN-20260914-001] macos-install

**Logged**: 2026-09-14T10:41:11+08:00
**Priority**: high
**Status**: done
**Area**: config

### Summary
VviTools 本地构建安装流程成功后只能保留 `/Applications` 中的应用。

### Details
用户指出复制构建产物到 `/Applications` 会同时留下 target 中的 `.app`，形成两个应用副本。正确流程是在安装成功后清理构建目录中的 `.app`，并确保运行的是 Applications 中的版本。

### Suggested Action
后续修改 macOS 本地安装流程时，应检查文件系统和运行中进程，确认只存在 `/Applications/VviTools.app`。

### Metadata
- Source: user_feedback
- Related Files: scripts/install-macos-app.sh, README.md
- Tags: macos, build, install, duplicate

---

## [LRN-20260803-001] plugin-cli-module-boundary

**Logged**: 2026-08-03T03:03:00Z
**Priority**: medium
**Status**: pending
**Area**: config

### Summary
VviTools 命令行工具应作为独立 workspace 模块维护，而不是直接挂在私有桌面应用根包的 `bin` 字段上。

### Details
用户纠正了首版实现：CLI 面向开发者，后续应具备独立发布、独立安装和独立演进能力；主应用根包是 `private` 桌面 app，只应通过 `npm run plugin` 提供仓库内便捷入口。当前项目采用根目录 `cli/` 放置命令行模块，包名可用 `vvitools-cli`，对外命令只使用更简洁的 `vvitools`，不保留 `vvitools-plugin-cli` 兼容别名。

### Suggested Action
后续维护 VviTools 命令行能力时优先放在根目录 `cli/` 模块中，根包只保留 workspace 声明和便捷脚本；不要再引入 `vvitools-plugin-cli` 这类兼容别名。

### Metadata
- Source: user_feedback
- Related Files: package.json, cli/package.json
- Tags: cli, workspace, plugin-system

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

---

## [LRN-20260717-001] launcher

**Logged**: 2026-07-17T04:18:09Z
**Priority**: medium
**Status**: pending
**Area**: frontend

### Summary
VviTools 首页“推荐”不能展示插件市场、已安装这类导航入口。

### Details
用户明确纠正“推荐，你显示个已安装和插件市场干毛”。首页推荐区如果没有真实推荐插件，应直接隐藏；插件市场、已安装属于导航能力，应保留在左上角入口或二级页侧栏，不应伪装成推荐内容。

### Suggested Action
后续恢复推荐区时必须由真实插件推荐数据驱动，例如热门插件、官方精选或新插件；没有数据时不展示推荐区。

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
