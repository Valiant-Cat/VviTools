# Errors

命令失败、集成异常与排障记录。

---

## [ERR-20260714-001] rubick-source-run

**Logged**: 2026-07-14T03:42:00Z
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary
rubick 源码运行需要 Node 16、镜像源、Node headers 镜像和 Python 3.11，否则依赖安装/原生模块编译会失败。

### Details
rubick 4.3.8 的 `volta.node` 为 `16.20.2`。当前 Node 20/23 环境下 yarn/corepack 不兼容；yarn.lock 大量 resolved 到 `registry.yarnpkg.com` 会网络超时；`extract-file-icon` 编译需要 Node 头文件，默认 `nodejs.org` 下载可能 TLS 断开；node-gyp 使用 Python 3.13 会缺少 `distutils`。

### Suggested Action
运行 rubick 源码时使用：
`source ~/.nvm/nvm.sh && nvm use 16.20.2`；
`npm config set registry https://registry.npmmirror.com`；
`npm config set disturl https://npmmirror.com/mirrors/node`；
`npm_config_python=/opt/homebrew/bin/python3.11 npm install --legacy-peer-deps`。
开发版或安装版启动后仍需在 macOS “隐私与安全性 → 辅助功能”中授权 rubick，才能启用全局快捷键。

## [ERR-20260710-001] tool-command

**Logged**: 2026-07-10T03:34:51Z
**Priority**: low
**Status**: pending
**Area**: config

### Summary
MemHub skill 同步脚本可能偶发 SSL EOF，但 memhub CLI 请求可继续作为兜底。

### Details
本轮执行 `sync_memhub_skills.py` 时出现 `[SSL: UNEXPECTED_EOF_WHILE_READING]`，随后 `memhub --json request get /skills/enabled` 与知识库查询正常，说明应区分同步脚本网络异常和 MemHub CLI/API 可用性。

### Suggested Action
遇到同步脚本 SSL EOF 时，先用 memhub CLI 或完整 HTTP API 继续完成必要查询，再视需要重试同步脚本。

## [ERR-20260710-002] shell-command

**Logged**: 2026-07-10T03:34:51Z
**Priority**: low
**Status**: pending
**Area**: infra

### Summary
使用 `node - <<'NODE'` 读取管道输入会把脚本来源切到 stdin，导致前置管道 JSON 被当作源码解析。

### Details
本轮用 `curl ... | node - <<'NODE'` 解析 `package.json` 时，Node 将 stdin 中的 JSON 当作脚本执行并报 `Unexpected token ':'`。改用 `node -e` 后正常读取管道输入。

### Suggested Action
需要同时提供 Node 脚本和读取管道内容时，优先使用 `node -e "..."`，或把 JSON 先保存到临时变量/文件后再解析。

## [ERR-20260713-001] tauri-icon

**Logged**: 2026-07-13T07:12:00Z
**Priority**: low
**Status**: resolved
**Area**: frontend

### Summary
Tauri app 运行时会解析窗口图标，损坏或像素数据不匹配的 PNG 会导致启动崩溃。

### Details
本轮最小占位 PNG 让 `tauri build` 通过，但启动 release 二进制时报 `invalid icon: The specified dimensions (32x32) don't match the number of pixels supplied`。重新生成合法的 32x32 RGBA PNG 后，release app 可稳定运行。

### Suggested Action
Tauri 项目即使只是 MVP，也应使用真实有效 PNG/ICNS 图标；不要用未经验证的极简 base64 占位图。

## [ERR-20260713-002] svelte-tauri-runtime

**Logged**: 2026-07-13T10:39:00Z
**Priority**: medium
**Status**: resolved
**Area**: frontend

### Summary
Svelte 5 应使用 `mount(App, ...)` 启动组件，旧版 `new App(...)` 会导致 Tauri WebView 空白。

### Details
VviTools 初版 Tauri 窗口只显示标题栏和背景色，原因是 `src/main.ts` 使用了 Svelte 4 风格的 `new App({ target })`。生产构建可通过，但运行时前端入口崩溃，页面不渲染。改为 `import { mount } from "svelte"; mount(App, { target })` 后 UI 正常显示。

### Suggested Action
Svelte 5 项目创建入口必须使用 `mount`；如果 Tauri 窗口空白但背景色存在，优先检查前端入口运行时错误。

## [ERR-20260713-003] tauri-resource-dir

**Logged**: 2026-07-13T10:39:00Z
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary
Tauri command 中读取 `resource_dir()` 不能作为市场加载的硬依赖，应提供内置兜底。

### Details
VviTools UI 渲染后显示 `unknown path`，原因是 `load_marketplace` 强依赖 `app.path().resource_dir()`；当前运行方式下该路径不可用，导致刷新流程中断。改为资源目录存在时读取 `marketplace.json`，不可用时回退到内置静态市场样例。

### Suggested Action
桌面 app 的资源路径和运行方式相关，核心启动流程不应因资源目录不可用失败；本地默认数据应有代码级 fallback。

## [ERR-20260714-001] script-command

**Logged**: 2026-07-14T04:03:00Z
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary
解析 MemHub API 返回结构时，Python 条件表达式优先级容易导致 list 被误当 dict 使用。

### Details
本轮最终回复前枚举知识库元数据时使用了 `data.get(...) or data if isinstance(data, list) else []`，由于条件表达式优先级导致 list payload 进入 `.get` 分支并报 `AttributeError: 'list' object has no attribute 'get'`。改成显式 `as_items(payload)` 分支后正常枚举。

### Suggested Action
处理 API payload 兼容 list/dict 时使用独立 helper 和显式类型判断，不要把 `or` 与三元表达式混写在一行。
