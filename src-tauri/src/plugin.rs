use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginRuntime {
    Node,
    Shell,
    Builtin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginCommand {
    pub id: String,
    pub title: String,
    pub keyword: String,
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub runtime: PluginRuntime,
    pub entry: String,
    #[serde(default)]
    pub builtin: Option<BuiltinPluginSpec>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuiltinPluginSpec {
    pub module: String,
    pub bridge: String,
    #[serde(default)]
    pub host_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandMatch {
    pub plugin_id: String,
    pub plugin_name: String,
    pub command_id: String,
    pub title: String,
    pub keyword: String,
    pub score: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandInput {
    pub query: String,
    pub context: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RpcResult {
    Text { text: String },
    Markdown { markdown: String },
    List { items: Vec<RpcListItem> },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RpcListItem {
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub action: Option<RpcAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RpcAction {
    Copy { value: String },
    OpenUrl { url: String },
    Shell { command: String },
}

impl RpcAction {
    pub fn required_permissions(&self) -> &'static [&'static str] {
        match self {
            RpcAction::Copy { .. } => &["clipboard", "clipboard:write"],
            RpcAction::OpenUrl { .. } => &["open-url"],
            RpcAction::Shell { .. } => &["shell"],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub runtime: String,
    #[serde(default)]
    pub entry: String,
    #[serde(default)]
    pub bundled: bool,
    pub download_url: String,
    pub sha256: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionDecision {
    Approved,
    Denied,
}

pub fn default_plugins_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VviTools")
        .join("plugins")
}

pub fn bundled_plugins_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("plugins")
}

pub fn load_available_plugins() -> Result<Vec<InstalledPlugin>> {
    load_available_plugins_from(&bundled_plugins_dir())
}

pub fn load_available_plugins_from(bundled_root: &Path) -> Result<Vec<InstalledPlugin>> {
    let mut plugins = load_plugins(bundled_root)?;
    let bundled_ids = plugins
        .iter()
        .map(|plugin| plugin.manifest.id.clone())
        .collect::<std::collections::HashSet<_>>();
    plugins.extend(
        load_plugins(&default_plugins_dir())?
            .into_iter()
            .filter(|plugin| !bundled_ids.contains(&plugin.manifest.id)),
    );
    plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(plugins)
}

pub fn load_plugins(root: &Path) -> Result<Vec<InstalledPlugin>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut plugins = Vec::new();
    for entry in
        fs::read_dir(root).with_context(|| format!("读取插件目录失败: {}", root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let dir = entry.path();
        let manifest_path = dir.join("plugin.json");
        if !manifest_path.exists() {
            continue;
        }
        let manifest: PluginManifest = serde_json::from_slice(
            &fs::read(&manifest_path)
                .with_context(|| format!("读取插件清单失败: {}", manifest_path.display()))?,
        )
        .with_context(|| format!("解析插件清单失败: {}", manifest_path.display()))?;
        validate_manifest(&manifest)?;
        plugins.push(InstalledPlugin { manifest, dir });
    }

    plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(plugins)
}

pub fn install_plugin_manifest(
    manifest: PluginManifest,
    target_root: &Path,
) -> Result<InstalledPlugin> {
    validate_manifest(&manifest)?;
    if manifest.runtime == PluginRuntime::Builtin {
        return Err(anyhow!("自定义插件暂不允许导入 builtin 运行时"));
    }

    fs::create_dir_all(target_root)
        .with_context(|| format!("创建插件目录失败: {}", target_root.display()))?;
    let install_dir = target_root.join(&manifest.id);
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)
            .with_context(|| format!("清理旧插件目录失败: {}", install_dir.display()))?;
    }
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("创建插件目录失败: {}", install_dir.display()))?;
    fs::write(
        install_dir.join("plugin.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )
    .with_context(|| format!("写入插件清单失败: {}", install_dir.display()))?;

    Ok(InstalledPlugin {
        manifest,
        dir: install_dir,
    })
}

pub fn delete_user_plugin(plugin_id: &str, target_root: &Path) -> Result<()> {
    if plugin_id.trim().is_empty() || plugin_id.contains("..") || plugin_id.contains('/') {
        return Err(anyhow!("插件 id 不合法"));
    }
    let plugin_dir = target_root.join(plugin_id);
    if !plugin_dir.exists() {
        return Err(anyhow!("自定义插件不存在: {}", plugin_id));
    }
    fs::remove_dir_all(&plugin_dir)
        .with_context(|| format!("删除插件目录失败: {}", plugin_dir.display()))
}

pub fn search_commands(plugins: &[InstalledPlugin], query: &str) -> Vec<CommandMatch> {
    let q = query.trim().to_lowercase();
    let mut matches = Vec::new();

    for plugin in plugins {
        for command in &plugin.manifest.commands {
            let haystacks = [
                command.keyword.to_lowercase(),
                command.title.to_lowercase(),
                plugin.manifest.name.to_lowercase(),
                plugin.manifest.description.to_lowercase(),
                plugin.manifest.keywords.join(" ").to_lowercase(),
            ];
            let score = if q.is_empty() {
                1
            } else if command.keyword.to_lowercase() == q {
                100
            } else if haystacks.iter().any(|s| s.starts_with(&q)) {
                70
            } else if haystacks.iter().any(|s| s.contains(&q)) {
                40
            } else {
                0
            };

            if score > 0 {
                matches.push(CommandMatch {
                    plugin_id: plugin.manifest.id.clone(),
                    plugin_name: plugin.manifest.name.clone(),
                    command_id: command.id.clone(),
                    title: command.title.clone(),
                    keyword: command.keyword.clone(),
                    score,
                });
            }
        }
    }

    matches.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.title.cmp(&b.title)));
    matches
}

pub fn run_plugin_command(
    plugin: &InstalledPlugin,
    command_id: &str,
    input: CommandInput,
) -> Result<RpcResult> {
    if !plugin
        .manifest
        .commands
        .iter()
        .any(|cmd| cmd.id == command_id)
    {
        return Err(anyhow!("插件命令不存在: {}", command_id));
    }

    if plugin.manifest.runtime == PluginRuntime::Builtin {
        return run_builtin_command(plugin, command_id, input);
    }

    let entry = plugin.dir.join(&plugin.manifest.entry);
    if !entry.exists() {
        return Err(anyhow!("插件入口不存在: {}", entry.display()));
    }

    let payload = serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0",
        "method": "run",
        "params": {
            "command": command_id,
            "query": input.query,
            "context": input.context,
        }
    }))?;

    let mut command = match plugin.manifest.runtime {
        PluginRuntime::Node => {
            let mut cmd = Command::new("node");
            cmd.arg(entry);
            cmd
        }
        PluginRuntime::Shell => {
            let mut cmd = Command::new("sh");
            cmd.arg(entry);
            cmd
        }
        PluginRuntime::Builtin => {
            unreachable!("builtin runtime is handled before script entry resolution")
        }
    };

    let output = command
        .current_dir(&plugin.dir)
        .env("VVITOOLS_RPC_INPUT", &payload)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("执行插件失败: {}", plugin.manifest.id))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("插件执行失败: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: RpcResult =
        serde_json::from_str(stdout.trim()).with_context(|| "插件返回值不是合法 JSON-RPC 结果")?;
    Ok(result)
}

fn run_builtin_command(
    plugin: &InstalledPlugin,
    command_id: &str,
    _input: CommandInput,
) -> Result<RpcResult> {
    let Some(spec) = &plugin.manifest.builtin else {
        return Err(anyhow!(
            "内置插件缺少 builtin 桥接声明: {}",
            plugin.manifest.id
        ));
    };

    if !spec
        .host_commands
        .iter()
        .any(|command| command == command_id)
    {
        return Err(anyhow!(
            "内置插件 {} 未声明宿主命令: {}",
            plugin.manifest.id,
            command_id
        ));
    }

    match (plugin.manifest.id.as_str(), command_id) {
        ("dev.vvicat.system-clipboard", "clipboard.open") => Ok(RpcResult::Text {
            text: "使用 Alt + V 打开系统剪贴板。".into(),
        }),
        _ => Err(anyhow!("未知内置插件命令: {}", command_id)),
    }
}

pub fn action_allowed(plugin: &InstalledPlugin, action: &RpcAction) -> bool {
    action.required_permissions().iter().any(|required| {
        plugin
            .manifest
            .permissions
            .iter()
            .any(|permission| permission == required)
    })
}

pub fn ensure_action_allowed(plugin: &InstalledPlugin, action: &RpcAction) -> Result<()> {
    if action_allowed(plugin, action) {
        return Ok(());
    }

    Err(anyhow!(
        "插件 {} 未声明权限: {}",
        plugin.manifest.id,
        action.required_permissions().join(" 或 ")
    ))
}

pub fn install_plugin_from_zip(
    entry: &MarketplaceEntry,
    target_root: &Path,
    decision: PermissionDecision,
) -> Result<InstalledPlugin> {
    if !entry.permissions.is_empty() && decision != PermissionDecision::Approved {
        return Err(anyhow!(
            "安装插件需要确认权限: {}",
            entry.permissions.join(", ")
        ));
    }

    let bytes = read_marketplace_bytes(&entry.download_url)?;
    if let Some(expected) = &entry.sha256 {
        verify_sha256(&bytes, expected)?;
    }

    fs::create_dir_all(target_root)
        .with_context(|| format!("创建插件目录失败: {}", target_root.display()))?;

    let install_dir = target_root.join(&entry.id);
    let tmp_dir = target_root.join(format!(".{}.installing", entry.id));
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir)?;
    }
    fs::create_dir_all(&tmp_dir)?;
    unzip_bytes(&bytes, &tmp_dir)?;

    let manifest_path = find_manifest(&tmp_dir)?;
    let manifest: PluginManifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    validate_manifest(&manifest)?;
    if manifest.id != entry.id {
        return Err(anyhow!("市场插件 id 与清单不一致"));
    }

    let source_dir = manifest_path
        .parent()
        .ok_or_else(|| anyhow!("插件清单路径无父目录"))?
        .to_path_buf();

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)?;
    }
    fs::create_dir_all(&install_dir)?;
    copy_dir_contents(&source_dir, &install_dir)?;
    fs::remove_dir_all(&tmp_dir)?;

    Ok(InstalledPlugin {
        manifest,
        dir: install_dir,
    })
}

fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
    if manifest.id.trim().is_empty() {
        return Err(anyhow!("插件 id 不能为空"));
    }
    if manifest.name.trim().is_empty() {
        return Err(anyhow!("插件名称不能为空"));
    }
    if manifest.entry.contains("..") {
        return Err(anyhow!("插件入口不能包含上级目录"));
    }
    if manifest.icon.contains("..") {
        return Err(anyhow!("插件图标路径不能包含上级目录"));
    }
    if manifest.runtime == PluginRuntime::Builtin {
        let spec = manifest
            .builtin
            .as_ref()
            .ok_or_else(|| anyhow!("内置插件必须声明 builtin 配置"))?;
        if spec.module.trim().is_empty() || spec.bridge.trim().is_empty() {
            return Err(anyhow!(
                "内置插件 builtin.module 和 builtin.bridge 不能为空"
            ));
        }
        if spec.module.contains("..") || spec.bridge.contains("..") {
            return Err(anyhow!("内置插件 builtin 路径不能包含上级目录"));
        }
        if spec.host_commands.is_empty() {
            return Err(anyhow!("内置插件至少需要声明一个宿主命令"));
        }
    }
    if manifest.commands.is_empty() {
        return Err(anyhow!("插件至少需要声明一个命令"));
    }
    Ok(())
}

fn read_marketplace_bytes(url: &str) -> Result<Vec<u8>> {
    if url.starts_with("http://") || url.starts_with("https://") {
        let response = reqwest::blocking::get(url)?.error_for_status()?;
        Ok(response.bytes()?.to_vec())
    } else {
        Ok(fs::read(url).with_context(|| format!("读取插件包失败: {}", url))?)
    }
}

fn verify_sha256(bytes: &[u8], expected: &str) -> Result<()> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    let normalized = expected.strip_prefix("sha256:").unwrap_or(expected);
    if actual != normalized {
        return Err(anyhow!("插件包校验失败"));
    }
    Ok(())
}

fn unzip_bytes(bytes: &[u8], target: &Path) -> Result<()> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let out_path = target.join(file.mangled_name());
        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;
            fs::write(&out_path, contents)?;
        }
    }
    Ok(())
}

fn find_manifest(root: &Path) -> Result<PathBuf> {
    for entry in WalkDir::new(root).max_depth(3) {
        let entry = entry?;
        if entry.file_name() == "plugin.json" {
            return Ok(entry.path().to_path_buf());
        }
    }
    Err(anyhow!("插件包中未找到 plugin.json"))
}

fn copy_dir_contents(source: &Path, target: &Path) -> Result<()> {
    for entry in WalkDir::new(source).min_depth(1) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?;
        let destination = target.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}
