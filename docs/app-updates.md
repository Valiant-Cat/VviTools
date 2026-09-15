# 应用内更新

设置 → 软件更新支持手动检查、更新说明、下载进度、签名验证、安装和确认重启。
不会在后台自动安装。只有主窗口可以调用更新命令；并行检查、安装和未安装时重启均被拒绝。

## 发布前置条件

- GitHub 仓库 `Valiant-Cat/VviTools` 的 Actions Secret `TAURI_SIGNING_PRIVATE_KEY` 必须保存与 `src-tauri/tauri.conf.json` 内公钥配对的私钥内容。
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 保存私钥密码；无密码时可为空。
- 本机首次生成的私钥位于 `~/.config/vvitools/updater.key`，目录权限 700，文件权限 600，未加入仓库。必须安全备份，不可重新生成后直接覆盖已发布客户端对应的密钥。
- 公钥可以公开；私钥不能写入代码、文档、日志或 Release。

## 发布流程

1. 同步提升 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 和 `src-tauri/tauri.conf.json` 版本。
2. 配置上述 Secrets 后，推送与版本一致的稳定标签，例如 `v0.1.3`。
3. CI 构建 universal macOS app、DMG、`.app.tar.gz` 和 `.sig`，生成 `latest.json`，上传到 Release。
4. 检查 `latest.json` 同时含有 `darwin-aarch64` 和 `darwin-x86_64`，并确认下载链接与签名文件存在。
5. 从安装在 `/Applications` 的较旧支持更新版本测试检查、下载、安装及重启，核对版本号和快捷键。

客户端固定读取 `https://github.com/Valiant-Cat/VviTools/releases/latest/download/latest.json`。
缺少清单时显示“更新源尚未就绪”，不当成已经是最新版本。签名不匹配时阻止安装。
发布签名包的构建使用 `src-tauri/tauri.updater.conf.json`；普通本地构建不需要私钥。
首次切换到支持应用内更新的版本，旧客户端仍须手动安装一次。

## macOS 签名说明

更新包签名与 Apple Developer ID 代码签名是两回事。
现有 CI 使用 ad-hoc 代码签名，并未配置 Apple 公证；本地构建使用本地开发签名。
从本地签名版本更新为 CI ad-hoc 版本可能需要重新授予辅助功能权限。
对外正式分发前应配置稳定的 Developer ID 签名及公证，不能把更新包验签当作 Apple 公证。

## 验证范围

单元测试覆盖发布清单、版本标签、平台字段、更新源错误及并发锁。
真实跨版本替换仍需在发布签名更新包后验证；仅构建成功不代表升级链路已验收。
