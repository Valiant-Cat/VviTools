# Feature Requests

用户请求但当前系统尚未具备的能力。

---

## [FEAT-20260915-001] macos_fullscreen_overlay

**Logged**: 2026-09-15T11:51:52+08:00
**Priority**: high
**Status**: verified
**Area**: backend | config

### Requested Capability
VviTools 主面板和剪贴板窗口应能在其他 macOS 应用处于全屏状态时正常弹出。

### User Context
仅设置窗口置顶不能跨越 macOS 的全屏 Space，导致全屏工作时快捷键已触发但看不到窗口。

### Complexity Estimate
simple

### Suggested Implementation
窗口默认加入所有工作区，并在 AppKit 层声明 `CanJoinAllSpaces` 与 `FullScreenAuxiliary` collection behavior。

### Verification
最终验收：用户确认全屏弹出和扩展屏跟随已生效。最终版本统一原生窗口定位，并在 AX 权限不可用时通过前台进程的可见窗口边界选屏。构建、10 项测试和安装版签名检查通过。以下保留排障经过。

第一版普通 NSWindow 加工作区标记、激活应用的方案虽然通过构建和测试，但用户明确反馈全屏下没有生效，不能视为修复完成。当前改用固定版本的 tauri-nspanel，将窗口转换为可接收键盘输入的非激活式 NSPanel，并移除显示时激活整个应用的逻辑。全屏和多屏焦点跟随仍需真实键盘验收，不能以编译通过代替行为验证。

用户后续确认 NSPanel 版本在全屏下可以弹出，但焦点位于扩展屏时仍在主屏弹出。已将 `NSScreen::mainScreen` 选屏改为通过 AX 读取前台应用焦点窗口，以逻辑坐标转换及最大重叠面积选择显示器；权限不足或读取失败时回退鼠标所在屏幕。新增单元测试覆盖左右、上方和跨屏窗口，扩展屏实际效果待验收。

用户再次反馈仍在主屏。检查锁定版本 tao 的 macOS 实现发现，`set_outer_position`、`set_inner_size` 会异步派发 AppKit 操作，而当前原生面板定位直接同步执行；原有 `apply_launcher_window` 排队的旧屏位置存在覆盖新位置的竞态。macOS 路径现统一使用主线程原生尺寸和位置操作；AX 读取改为明确从 NSWorkspace 前台进程创建应用元素。缓存目录 `window-placement.json` 仅保留最近一次坐标诊断，不包含窗口标题或剪贴板内容。算法测试不覆盖操作系统消息队列，实际跨屏效果仍需验收。

### Metadata
- Frequency: first_time
- Related Features: launcher, system-clipboard, global-shortcut

---

## [FR-20260914-003] 主面板 Esc 分层返回

**Logged**: 2026-09-14T16:30:00+08:00
**Priority**: medium
**Status**: implemented
**Area**: frontend

### Summary
主窗口在任意焦点位置支持按 Esc 按当前界面层级返回。

### Details
此前 Esc 仅绑定在搜索输入框，焦点位于插件卡片、操作按钮或设置控件时无法返回。现在由窗口级键盘事件统一处理，并依次关闭弹窗或确认状态、从剪贴板设置返回插件详情、从插件详情返回列表、从功能页返回搜索面板。

### Suggested Action
后续增加主窗口内的模态层或子页面时，应同步纳入 Esc 返回层级，并继续保持独立剪贴板窗口按 Esc 直接关闭。

### Metadata
- Source: conversation
- Related Files: src/App.svelte
- Tags: keyboard, escape, navigation

---

## [FR-20260914-002] 系统剪贴板独立设置

**Logged**: 2026-09-14T14:00:00+08:00
**Priority**: high
**Status**: implemented
**Area**: backend | frontend

### Summary
为系统剪贴板提供记录范围、保留策略和本地存储管理。

### Details
剪贴板此前固定记录文本、图片和文件，最多保留 200 条，用户无法暂停记录、控制保留时间、查看存储位置或占用。

### Suggested Action
剪贴板设置独立保存，并让记录总开关、类型开关、保留时长、历史容量、存储位置、占用统计和清空历史都对应真实宿主行为。

### Metadata
- Source: conversation
- Related Files: src/App.svelte, src/style.css, src-tauri/src/commands.rs
- Tags: clipboard, settings, retention, storage

---

## [FR-20260914-001] 本地构建后安装到 Applications

**Logged**: 2026-09-14T10:38:56+08:00
**Priority**: medium
**Status**: completed
**Area**: config

### Summary
提供可重复执行的 macOS 本地构建并安装命令。

### Details
Tauri 的普通 build 只在 target 目录生成 `.app`，不会自动安装到 `/Applications`。项目需要独立命令完成本地构建与安装，同时避免 CI 打包流程修改系统应用目录。安装成功后应清理 target 中的应用副本，避免本机出现两个 VviTools。

### Suggested Action
使用 `npm run build:install` 构建并安装，或使用 `npm run install:macos` 安装已有构建产物。

### Metadata
- Source: conversation
- Related Files: package.json, scripts/install-macos-app.sh, README.md
- Tags: macos, build, install

---

## [FEAT-20260804-001] clipboard_accessibility_permission_prompt

**Logged**: 2026-08-04T11:11:31+08:00
**Priority**: high
**Status**: implemented
**Area**: backend | frontend

### Requested Capability
剪贴板自动粘贴需要在实际使用时触发 macOS 辅助功能授权提示，并在未授权时给出用户可见反馈。

### User Context
用户选择剪贴板记录后只能复制，无法直接粘贴到原输入框，且当前缺少授权引导。

### Complexity Estimate
medium

### Suggested Implementation
插件 manifest 声明 `accessibility:paste` 能力；宿主在首次执行自动粘贴时检查并请求 macOS 辅助功能授权；前端在剪贴板窗口和设置页展示授权状态。

### Metadata
- Frequency: first_time
- Related Features: system-clipboard

---
