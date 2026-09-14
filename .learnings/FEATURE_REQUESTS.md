# Feature Requests

用户请求但当前系统尚未具备的能力。

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
