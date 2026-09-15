# Errors

命令失败、集成异常与排障记录。

---

## [ERR-20260915-007] tauri-build-generic-failure

**Logged**: 2026-09-15T13:16:02+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary
最终 `npm run build:install` 在 Rust Release 编译阶段返回无细节的 Tauri 构建失败，详细重跑确认进程以退出码 `137` 被系统终止。

### Error
```text
failed to build app: failed to build app
Error failed to build app: failed to build app
```

### Context
- 同一代码已通过 `cargo check`、9 个测试、前端构建和 TypeScript 检查
- 此前一次安装构建已成功，失败发生在加入旧 macOS 激活兼容分支后的最终重打包
- `cargo build --release -vv` 未出现 Rust 编译错误，最终退出码为 `137`，符合系统因内存压力发送 `SIGKILL` 的表现
- 磁盘仍有约 51 GiB 可用空间；部署目标变化导致大量依赖重新编译，默认并行度会显著增加峰值资源占用

### Suggested Fix
关闭正在运行的安装版 VviTools，并设置 `CARGO_BUILD_JOBS=2` 降低 Release 编译并行度后重跑构建安装。

### Resolution
降低并行度后 Release 构建、签名和安装成功，安装版签名校验通过。退出码 `137` 的具体系统原因未独立确认。

### Metadata
- Reproducible: unknown
- Related Files: src-tauri/src/commands.rs, scripts/build-install-macos.sh

---

## [ERR-20260915-006] cargo-fmt-check

**Logged**: 2026-09-15T12:44:09+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary
新增 AppKit 主线程显示闭包后，首次 `cargo fmt --check` 未通过。

### Error
```text
Diff in src-tauri/src/commands.rs: run_on_main_thread 闭包缩进不符合 rustfmt
```

### Context
- Rust 编译已经通过
- 仅为格式差异，不影响逻辑

### Suggested Fix
编辑 Rust 闭包后先运行 `cargo fmt`，再执行 `cargo fmt --check`。

### Resolution
已运行 `cargo fmt --manifest-path src-tauri/Cargo.toml` 并通过复查。

### Metadata
- Reproducible: yes
- Related Files: src-tauri/src/commands.rs

---

## [ERR-20260915-005] launchservices-minus-600

**Logged**: 2026-09-15T12:44:09+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary
替换并重启安装版 VviTools 时，macOS LaunchServices 首次启动返回 `-600`。

### Error
```text
_LSOpenURLsWithCompletionHandler() failed with error -600.
```

### Context
- 刚执行 `pkill -x vvitools` 后立即调用 `open /Applications/VviTools.app`
- 应用已通过签名构建并安装

### Suggested Fix
确认旧进程退出后重试启动；若 LaunchServices 仍未恢复，则直接启动 bundle 内二进制完成本轮验证。

### Resolution
使用 `open -n /Applications/VviTools.app` 重试后成功启动。

### Metadata
- Reproducible: unknown
- Related Files: scripts/install-macos-app.sh

---

## [ERR-20260915-004] cua-vvitools-timeout

**Logged**: 2026-09-15T12:15:53+08:00
**Priority**: low
**Status**: pending
**Area**: infra

### Summary
在 TextEdit 全屏状态下获取刚启动的 VviTools 辅助功能树时，Computer Use 服务超时。

### Error
```text
Computer Use server error -10005: timeoutReached
```

### Context
- 已通过终端启动 `/Applications/VviTools.app`
- 目的：检查主窗口是否覆盖当前全屏 Space
- 后续截图显示启动窗口未停留在全屏界面，可能因当前“失焦即隐藏”行为立即收起，需通过全局快捷键继续验证
- 最终构建安装后再次读取 VviTools 辅助功能树仍超时，因此改用快捷键后焦点状态与原生窗口配置交叉验证

### Suggested Fix
改用当前全屏 App 的屏幕截图确认视觉结果，并在 VviTools 稳定运行后重试其 bundle identifier。

### Metadata
- Reproducible: unknown
- Related Files: src-tauri/src/main.rs, src-tauri/tauri.conf.json

---

## [ERR-20260915-003] missing-app-icon

**Logged**: 2026-09-15T12:11:47+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary
误尝试读取安装包中不存在的 `Contents/Resources/icon.icns`。

### Error
```text
unable to locate image at /Applications/VviTools.app/Contents/Resources/icon.icns
```

### Context
- 与全屏弹窗验证无关
- 没有修改应用或文件

### Suggested Fix
需要检查应用资源时先用 `find` 确认实际文件名，不猜测资源路径。

### Metadata
- Reproducible: yes
- Related Files: none

---

## [ERR-20260915-002] write-stdin-finished-session

**Logged**: 2026-09-15T12:11:47+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary
误对已经结束的构建会话再次调用 `write_stdin`。

### Error
```text
write_stdin failed: Unknown process id 96597
```

### Context
- 构建会话此前已正常结束，随后又误用了不存在的 session id `0`
- 两次误轮询均不影响构建和安装结果

### Suggested Fix
长任务返回 `Process exited` 后不再轮询相同 session id。

### Metadata
- Reproducible: yes
- Related Files: none

---

## [ERR-20260915-001] cua-get-app

**Logged**: 2026-09-15T12:11:47+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary
通过 Computer Use 获取 TextEdit 做 macOS 全屏验证时，辅助功能接口返回 `AXError.cannotComplete`。

### Error
```text
Accessibility error: AXError.cannotComplete
app is not defined
```

### Context
- 动作：先用英文显示名调用 `cua.getApp("TextEdit")`，后用 bundle identifier 重试时复用了未成功创建的变量
- 目的：验证 VviTools 在其他 App 的原生全屏 Space 中能否弹出
- 环境：macOS，本地安装版 VviTools 0.1.2

### Suggested Fix
通过 Computer Use 枚举当前可用 App 后使用本地化名称或 bundle identifier 重试，并用新的 `var` 绑定保存对象。本轮已按该方式成功获取 TextEdit。

### Metadata
- Reproducible: unknown
- Related Files: src-tauri/src/main.rs, src-tauri/tauri.conf.json

---

## [ERR-20260914-005] ui-ux-pro-max-script-pointer

**Logged**: 2026-09-14T14:00:00+08:00
**Priority**: low
**Status**: resolved

### Summary
本地 ui-ux-pro-max skill 的 scripts 入口是文本指针，不能直接作为目录执行。

### Details
按 skill 文档调用 `~/.codex/skills/ui-ux-pro-max/scripts/search.py` 时返回 `Not a directory`。通过查找已安装插件源码中的真实 `search.py` 后完成设计系统和 Svelte 指南查询。

### Suggested Action
后续先解析 skill 目录中的文本指针，或从插件安装目录定位真实脚本路径。

### Metadata
- Source: error
- Related Files: /Users/liam/.codex/skills/ui-ux-pro-max/scripts
- Tags: skill, ui-ux-pro-max, path

---

## [ERR-20260914-004] plugin-detail-open

**Logged**: 2026-09-14T12:55:00+08:00
**Priority**: high
**Status**: resolved
**Area**: frontend

### Summary
主窗口插件详情的“打开”按钮错误复用了独立剪贴板窗口的内部初始化函数。

### Details
`openClipboardPanel()` 只负责在剪贴板 WebView 收到打开事件后初始化页面状态。主窗口直接调用它会把 `view` 改成 `clipboard`，但主窗口模板没有对应分支，因此落入默认的设置页分支。正确行为是调用 Tauri 命令 `open_clipboard_window`，显示独立剪贴板窗口。

### Suggested Action
区分“请求宿主打开插件窗口”和“插件窗口收到事件后初始化内容”两个动作；主窗口、搜索命令等外部入口统一调用宿主命令。

### Metadata
- Source: user_feedback
- Related Files: src/App.svelte, src-tauri/src/commands.rs
- Tags: plugin-detail, clipboard, routing, window

---

## [ERR-20260914-003] osascript-key-event

**Logged**: 2026-09-14T11:44:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary
使用 System Events 发送全局快捷键被 macOS 辅助功能策略拒绝。

### Details
为了唤出隐藏的 VviTools 主窗口进行截图验收，尝试通过 osascript 发送 Option + Space，系统返回“不允许发送按键”。应用构建和运行未受影响。

### Suggested Action
工具型应用的 UI 验收优先使用应用可访问性接口；不要依赖 System Events 模拟快捷键。

### Metadata
- Source: error
- Related Files: none
- Tags: macos, ui-test, accessibility

---

## [ERR-20260914-001] exec_command

**Logged**: 2026-09-14T10:48:59+08:00
**Priority**: medium
**Status**: resolved
**Area**: config

### Summary
内联证书生成命令因包含递归删除临时目录而被命令安全规则拒绝。

### Details
首次尝试以内联 shell 创建本地代码签名证书，清理 trap 中的 `rm -rf` 被执行工具拒绝，命令未运行且未创建证书。

### Suggested Action
证书生成使用项目脚本封装，并通过删除已知临时文件后 `rmdir` 的方式清理，不使用递归强制删除。

### Metadata
- Source: error
- Related Files: scripts/setup-macos-signing.sh
- Tags: codesign, keychain, command-safety

---

## [ERR-20260914-002] security-import

**Logged**: 2026-09-14T10:49:33+08:00
**Priority**: medium
**Status**: resolved
**Area**: config

### Summary
登录钥匙串路径包含前导空格，导致证书导入报告找不到钥匙串。

### Details
`security default-keychain -d user` 输出包含缩进和引号。脚本最初只移除了引号，保留的前导空格使 `security import` 无法定位实际存在的 `login.keychain-db`。

### Suggested Action
解析 security 命令输出时同时移除首尾空白和包裹引号，并在导入前使用真实路径。

### Metadata
- Source: error
- Related Files: scripts/setup-macos-signing.sh, scripts/build-install-macos.sh
- Tags: codesign, keychain, parsing

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

## [ERR-20260803-001] cargo-test-stale-tauri-target

**Logged**: 2026-08-03T02:56:34Z
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary
项目路径迁移后，`src-tauri/target` 中残留的 Tauri build script 缓存可能引用旧路径，导致 `npm run test` 读取 autogenerated permissions 失败。

### Details
本轮首次执行 `npm run test` 时报错：Tauri build script 尝试读取 `/Volumes/Disk_APFS/Work/AI/vvitools/.../app_hide.toml`。当前项目实际路径是 `/Volumes/Disk_APFS/Work/XiaMao/Tools/vvitools`，`rg` 可见 target 缓存中大量旧路径。执行 `cargo clean --manifest-path src-tauri/Cargo.toml` 后重跑 `npm run test` 通过。

### Suggested Action
遇到 Tauri/Rust 测试报旧项目路径、旧 autogenerated permission 文件缺失时，先清理 Cargo 构建缓存，再重跑测试。

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
