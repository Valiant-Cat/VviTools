# Feature Requests

用户请求但当前系统尚未具备的能力。

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
