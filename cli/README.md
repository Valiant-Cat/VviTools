# vvitools-cli

VviTools 命令行工具。当前优先提供插件开发能力，参考 rubick 插件 CLI 的开发体验，但生成和校验的是 VviTools 当前 `plugin.json` 协议。

## 使用

```bash
vvitools --help
```

创建插件模板：

```bash
vvitools plugin create hello-tools --runtime node --name "Hello 工具"
```

校验插件：

```bash
vvitools plugin validate hello-tools
```

打包插件：

```bash
vvitools plugin pack hello-tools
```

插件 CLI 默认面向第三方插件，只允许 `node` 和 `shell` runtime。`builtin` 只用于 VviTools 随应用分发的捆绑内置插件，内部校验时需要显式传入 `--allow-builtin`。
