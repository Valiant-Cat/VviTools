# 系统剪贴板

这是 VviTools 随应用分发的捆绑内置插件。

它不是只有 `plugin.json` 的占位入口：插件目录负责声明插件身份、命令、关键词、权限、图标和内置桥接契约；高权限能力仍由 Tauri/Rust 宿主实现，避免普通插件直接访问系统剪贴板、全局快捷键和前台应用粘贴控制。

## 目录结构

```text
plugins/system-clipboard/
  plugin.json
  builtin.ts
  README.md
  assets/icon.svg
  ui/
    ClipboardWindow.svelte
    ClipboardSettings.svelte
    clipboard.css
    types.ts
```

## 运行方式

- `runtime: "builtin"` 表示由 VviTools 宿主加载。
- `entry: "builtin.ts"` 是内置插件的契约说明和类型化入口。
- `builtin.bridge: "host.clipboard"` 绑定到宿主剪贴板桥接。
- `builtin.host_commands` 声明该插件允许调用的宿主命令。
- `ui.window` 声明独立剪贴板窗口组件。
- `ui.settings` 声明剪贴板设置组件。
- `ui.styles` 声明插件专属样式入口。

## 能力边界

当前宿主侧提供：

- 后台监听剪贴板文本、图片、文件。
- 本地历史记录持久化。
- 收藏、删除、清空、搜索和筛选。
- 独立底部剪贴板窗口。
- 选择记录后复制并尝试粘贴回原前台应用。
- `accessibility:paste` 声明自动粘贴能力；真正的 macOS 辅助功能授权由宿主在首次需要自动粘贴时向系统申请。

插件侧提供：

- 插件市场/已安装页中的应用信息。
- 搜索命令 `clipboard` / `剪贴板`。
- 内置桥接契约和图标资产。
- 独立剪贴板窗口和设置页的 Svelte UI。

主应用只负责路由、状态和调用宿主命令，不再直接维护剪贴板页面模板。当前 UI 入口为一方插件的编译期组件，不对普通导入插件开放任意前端代码执行。
