# VviTools

VviTools 是一个基于 Tauri v2 + Svelte 的极简插件化桌面工具箱。它参考 rubick / uTools 的交互模型：通过全局快捷键呼出搜索框，搜索命令并执行；插件市场负责发现和安装插件；高权限系统能力由 Rust 宿主统一封装。

## 当前能力

- 桌面壳：Tauri v2
- 主进程：Rust
- 前端：Svelte + Vite
- 全局快捷键：
  - `Alt + Space`：显示或隐藏主搜索窗口
  - `Alt + V`：从底部打开独立剪贴板窗口
- 状态栏菜单：显示/隐藏、设置、退出
- 设置项：
  - 开机启动
  - Dock 栏显示
  - 全局快捷键展示
  - 点击外部关闭
- 插件系统：
  - `plugin.json` manifest
  - JSON RPC 风格输入输出
  - 第一版支持 `node`、`shell`、`builtin`
  - 静态 JSON + zip 包插件市场
  - manifest 权限声明 + 安装时确认
- 捆绑内置插件：
  - `系统剪贴板`
  - 文本、图片、文件剪贴板历史
  - 搜索、筛选、收藏、复制、删除、清空
  - 选择后复制并尝试粘贴回原前台应用

## 项目结构

```text
.
├── plugins/                         # 随应用分发的捆绑内置插件
│   └── system-clipboard/
├── src/                             # Svelte 前端
│   ├── App.svelte
│   └── style.css
├── src-tauri/                       # Tauri / Rust 主进程
│   ├── src/main.rs                  # 应用启动、状态栏、快捷键、窗口控制
│   ├── src/commands.rs              # Tauri 命令、剪贴板、设置、插件桥接
│   ├── src/plugin.rs                # 插件协议、安装、加载、执行
│   └── tauri.conf.json
└── package.json
```

## 开发

```bash
npm install
npm run tauri -- dev
```

前端单独构建：

```bash
npm run build
```

Rust 测试：

```bash
npm run test
```

正式打包：

```bash
npm run tauri -- build
```

macOS 打包产物：

```text
src-tauri/target/release/bundle/macos/VviTools.app
```

本机构建并安装到 `/Applications/VviTools.app`：

```bash
npm run signing:setup
npm run build:install
```

`signing:setup` 只需执行一次，它会在当前用户登录钥匙串中创建长期有效的 `VviTools Local Development` 代码签名身份。本地安装构建始终使用该身份，避免每次重新编译后 macOS 将 VviTools 识别成新的辅助功能授权对象。

首次从临时签名切换到稳定签名后，需要清理旧授权并对 `/Applications/VviTools.app` 重新授权一次：

```bash
tccutil reset Accessibility dev.vvicat.vvitools
```

后续只要保留当前钥匙串证书、Bundle ID 和签名身份，重新执行 `npm run build:install` 不需要重复移除和添加辅助功能权限。

安装已经生成的 `.app`：

```bash
npm run install:macos
```

安装成功后会清理 `src-tauri/target/release/bundle/macos/VviTools.app`，避免本机同时保留两个应用副本。

## 插件 manifest 示例

```json
{
  "id": "dev.example.echo",
  "name": "回显工具",
  "version": "1.0.0",
  "description": "返回输入内容",
  "keywords": ["echo", "回显"],
  "runtime": "shell",
  "entry": "main.sh",
  "permissions": [],
  "commands": [
    {
      "id": "echo.run",
      "title": "回显文本",
      "keyword": "echo",
      "input": "text"
    }
  ]
}
```

## 插件 CLI

仓库根目录的 `cli/` 提供独立 CLI 模块 `vvitools-cli`，安装后暴露 `vvitools` 命令。当前插件子命令用于对齐 rubick `rubick-plugin-cli create` 的开发体验，但生成的是 VviTools 当前协议的 `plugin.json` 插件。

本地使用：

```bash
npm run plugin -- --help
```

等价于调用：

```bash
vvitools plugin --help
```

创建插件模板：

```bash
npm run plugin -- create hello-tools --runtime node --name "Hello 工具"
```

校验插件：

```bash
npm run plugin -- validate hello-tools
```

打包插件：

```bash
npm run plugin -- pack hello-tools
```

CLI 默认面向第三方插件，只允许 `node` 和 `shell` runtime。`builtin` 仅用于 `plugins/` 下随应用分发的捆绑内置插件，内部校验时需要显式传入 `--allow-builtin`。

插件执行时通过 stdin 接收 JSON RPC 风格输入，stdout 输出 `RpcResult`：

```json
{
  "type": "text",
  "text": "hello"
}
```

## 捆绑内置插件

`plugins/system-clipboard` 是真实的捆绑内置插件目录，不是普通用户插件。它通过 manifest 声明插件身份、命令、权限、图标和宿主桥接契约；实际剪贴板监听、图片/文件读取、粘贴回前台应用等高权限能力由 Rust 宿主实现。

用户界面应展示插件名 `系统剪贴板`，不要展示内部 ID `system-clipboard`。

## 常用验证

提交前建议至少运行：

```bash
npm run build
npm run test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

涉及 Tauri、窗口、快捷键、状态栏、剪贴板、打包资源路径时，还需要运行：

```bash
npm run tauri -- build
```
