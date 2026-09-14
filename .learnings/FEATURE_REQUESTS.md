# Feature Requests

用户请求但当前系统尚未具备的能力。

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
