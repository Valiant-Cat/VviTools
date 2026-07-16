use std::{
    borrow::Cow,
    fs,
    io::Write,
    path::PathBuf,
    process::Command,
    sync::{Mutex, OnceLock},
    thread,
    time::Duration,
};

use arboard::ImageData;
use chrono::Utc;
use image::{ImageBuffer, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewWindow};
use vvitools_core::plugin::{
    bundled_plugins_dir, default_plugins_dir, ensure_action_allowed, install_plugin_from_zip,
    load_available_plugins_from, load_plugins, run_plugin_command, search_commands, CommandInput,
    CommandMatch, InstalledPlugin, MarketplaceEntry, PermissionDecision, RpcAction, RpcResult,
};

#[derive(Debug, Serialize)]
pub struct PluginView {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: String,
    pub runtime: String,
    pub entry: String,
    pub bundled: bool,
    pub permissions: Vec<String>,
    pub commands: usize,
}

#[derive(Debug, Deserialize)]
pub struct RunRequest {
    pub plugin_id: String,
    pub command_id: String,
    pub query: String,
    #[serde(default)]
    pub context: Value,
}

#[derive(Debug, Deserialize)]
pub struct InstallRequest {
    pub entry: MarketplaceEntry,
    pub approved: bool,
}

#[derive(Debug, Deserialize)]
pub struct ActionRequest {
    pub plugin_id: String,
    pub action: RpcAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub preview: String,
    pub copied_at: String,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub image_path: String,
    #[serde(default)]
    pub width: usize,
    #[serde(default)]
    pub height: usize,
    #[serde(default)]
    pub file_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClipboardCopyRequest {
    pub id: String,
}

const CLIPBOARD_HISTORY_LIMIT: usize = 200;
const CLIPBOARD_PREVIEW_LIMIT: usize = 160;
const APP_BUNDLE_ID: &str = "dev.vvicat.vvitools";

static PREVIOUS_FRONTMOST_APP: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[tauri::command]
pub fn list_plugins(app: tauri::AppHandle) -> Result<Vec<PluginView>, String> {
    load_available_plugins_for_app(&app)
        .map_err(to_message)?
        .into_iter()
        .map(plugin_to_view)
        .collect::<Vec<_>>()
        .pipe(Ok)
}

#[tauri::command]
pub fn search_plugin_commands(
    app: tauri::AppHandle,
    query: String,
) -> Result<Vec<CommandMatch>, String> {
    let plugins = load_available_plugins_for_app(&app).map_err(to_message)?;
    Ok(search_commands(&plugins, &query))
}

#[tauri::command]
pub fn execute_plugin_command(
    app: tauri::AppHandle,
    request: RunRequest,
) -> Result<RpcResult, String> {
    let plugins = load_available_plugins_for_app(&app).map_err(to_message)?;
    let plugin = plugins
        .iter()
        .find(|plugin| plugin.manifest.id == request.plugin_id)
        .ok_or_else(|| format!("插件不存在: {}", request.plugin_id))?;
    run_plugin_command(
        plugin,
        &request.command_id,
        CommandInput {
            query: request.query,
            context: request.context,
        },
    )
    .map_err(to_message)
}

#[tauri::command]
pub fn execute_plugin_action(app: tauri::AppHandle, request: ActionRequest) -> Result<(), String> {
    let plugins = load_available_plugins_for_app(&app).map_err(to_message)?;
    let plugin = plugins
        .iter()
        .find(|plugin| plugin.manifest.id == request.plugin_id)
        .ok_or_else(|| format!("插件不存在: {}", request.plugin_id))?;

    ensure_action_allowed(plugin, &request.action).map_err(to_message)?;
    match request.action {
        RpcAction::Copy { value } => {
            let mut clipboard = arboard::Clipboard::new().map_err(to_message)?;
            clipboard.set_text(value).map_err(to_message)?;
            Ok(())
        }
        RpcAction::OpenUrl { url } => open::that(url).map_err(to_message),
        RpcAction::Shell { .. } => Err("暂不允许插件通过结果动作执行 shell".into()),
    }
}

#[tauri::command]
pub fn list_clipboard_history() -> Result<Vec<ClipboardItem>, String> {
    load_clipboard_history().map_err(to_message)
}

#[tauri::command]
pub fn copy_clipboard_item(request: ClipboardCopyRequest) -> Result<(), String> {
    copy_clipboard_item_by_id(&request.id)
}

#[tauri::command]
pub fn paste_clipboard_item(
    app: tauri::AppHandle,
    request: ClipboardCopyRequest,
) -> Result<(), String> {
    copy_clipboard_item_by_id(&request.id)?;
    if let Some(window) = app.get_webview_window("clipboard") {
        let _ = window.hide();
    }
    paste_to_previous_frontmost_app();
    Ok(())
}

fn copy_clipboard_item_by_id(id: &str) -> Result<(), String> {
    let history = load_clipboard_history().map_err(to_message)?;
    let item = history
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "剪贴板记录不存在".to_string())?;
    if item.kind == "file" {
        write_files_to_clipboard(item)?;
        record_clipboard_files_from_item(item).map_err(to_message)
    } else if item.kind == "image" {
        write_image_to_clipboard(item)?;
        record_clipboard_image_from_path(item).map_err(to_message)
    } else {
        write_text_to_clipboard(&item.text)?;
        record_clipboard_text(&item.text).map_err(to_message)
    }
}

#[tauri::command]
pub fn toggle_clipboard_favorite(id: String) -> Result<(), String> {
    let mut history = load_clipboard_history().map_err(to_message)?;
    let item = history
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| "剪贴板记录不存在".to_string())?;
    item.favorite = !item.favorite;
    save_clipboard_history(&history).map_err(to_message)
}

pub fn remember_frontmost_app() {
    #[cfg(target_os = "macos")]
    {
        if let Some(bundle_id) = frontmost_bundle_id() {
            if bundle_id != APP_BUNDLE_ID {
                if let Ok(mut previous) = previous_frontmost_app().lock() {
                    *previous = Some(bundle_id);
                }
            }
        }
    }
}

fn previous_frontmost_app() -> &'static Mutex<Option<String>> {
    PREVIOUS_FRONTMOST_APP.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "macos")]
fn frontmost_bundle_id() -> Option<String> {
    let output = Command::new("osascript")
        .args([
            "-e",
            "id of application (path to frontmost application as text)",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if is_safe_bundle_id(&bundle_id) {
        Some(bundle_id)
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn is_safe_bundle_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}

fn paste_to_previous_frontmost_app() {
    #[cfg(target_os = "macos")]
    {
        let bundle_id = previous_frontmost_app()
            .lock()
            .ok()
            .and_then(|previous| previous.clone());
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(120));
            let mut script = String::new();
            if let Some(bundle_id) = bundle_id {
                script.push_str(&format!(
                    "tell application id \"{}\" to activate\n",
                    bundle_id
                ));
                script.push_str("delay 0.08\n");
            }
            script.push_str(
                "tell application \"System Events\" to keystroke \"v\" using command down",
            );
            let _ = Command::new("osascript").args(["-e", &script]).output();
        });
    }
}

#[tauri::command]
pub fn delete_clipboard_item(id: String) -> Result<(), String> {
    let mut history = load_clipboard_history().map_err(to_message)?;
    if let Some(item) = history
        .iter()
        .find(|item| item.id == id && item.kind == "image")
    {
        let _ = fs::remove_file(&item.image_path);
    }
    history.retain(|item| item.id != id);
    save_clipboard_history(&history).map_err(to_message)
}

#[tauri::command]
pub fn clear_clipboard_history() -> Result<(), String> {
    let _ = fs::remove_dir_all(clipboard_images_dir());
    save_clipboard_history(&[]).map_err(to_message)
}

pub fn start_clipboard_watcher() {
    thread::spawn(|| {
        let mut last_text = String::new();
        let mut last_image_id = String::new();
        let mut last_file_id = String::new();
        loop {
            if let Ok(file_paths) = read_files_from_clipboard() {
                if !file_paths.is_empty() {
                    let file_id = clipboard_files_id(&file_paths);
                    if file_id != last_file_id {
                        let _ = record_clipboard_files(file_paths, file_id.clone());
                        last_file_id = file_id;
                    }
                    thread::sleep(Duration::from_millis(900));
                    continue;
                }
            }
            if let Ok(text) = read_text_from_clipboard() {
                let trimmed = text.trim();
                if !trimmed.is_empty() && text != last_text {
                    let _ = record_clipboard_text(&text);
                    last_text = text;
                }
            }
            if let Ok(image) = read_image_from_clipboard() {
                let image_id = clipboard_image_id(&image);
                if image_id != last_image_id {
                    let _ = record_clipboard_image(image, image_id.clone());
                    last_image_id = image_id;
                }
            }
            thread::sleep(Duration::from_millis(900));
        }
    });
}

#[tauri::command]
pub fn load_marketplace(app: tauri::AppHandle) -> Result<Vec<MarketplaceEntry>, String> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let market_path = resource_dir.join("marketplace.json");
        if market_path.exists() {
            let data = fs::read(&market_path).map_err(to_message)?;
            return serde_json::from_slice(&data).map_err(to_message);
        }
    }
    bundled_marketplace(&bundled_plugins_dir_for_app(&app)).map_err(to_message)
}

#[tauri::command]
pub fn install_marketplace_plugin(request: InstallRequest) -> Result<PluginView, String> {
    let decision = if request.approved {
        PermissionDecision::Approved
    } else {
        PermissionDecision::Denied
    };
    install_plugin_from_zip(&request.entry, &default_plugins_dir(), decision)
        .map(plugin_to_view)
        .map_err(to_message)
}

#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    open::that(url).map_err(to_message)
}

#[tauri::command]
pub fn open_clipboard_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("clipboard")
        .ok_or_else(|| "剪贴板窗口不存在".to_string())?;
    remember_frontmost_app();
    apply_launcher_window(&window, "clipboard")?;
    window.show().map_err(to_message)?;
    let _ = window.emit("open-clipboard", ());
    let _ = window.eval("window.dispatchEvent(new CustomEvent('vvitools-open-clipboard'))");
    window.set_focus().map_err(to_message)
}

#[tauri::command]
pub fn hide_launcher(app: tauri::AppHandle) -> Result<(), String> {
    hide_window(app, "main".into())
}

#[tauri::command]
pub fn hide_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window.hide().map_err(to_message)?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_launcher_view(app: tauri::AppHandle, view: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        apply_launcher_window(&window, &view)?;
    }
    Ok(())
}

pub fn apply_launcher_window(window: &WebviewWindow, view: &str) -> Result<(), String> {
    let size = match view {
        "feature" => LogicalSize::new(980.0, 640.0),
        "clipboard" => LogicalSize::new(940.0, 238.0),
        _ => LogicalSize::new(720.0, 420.0),
    };
    window.set_size(Size::Logical(size)).map_err(to_message)?;
    if let Ok(Some(monitor)) = window.current_monitor() {
        let scale = monitor.scale_factor();
        let work_area = monitor.work_area();
        let width = size.width * scale;
        let height = size.height * scale;
        let x = work_area.position.x as f64 + (work_area.size.width as f64 - width) / 2.0;
        let y = if view == "clipboard" {
            work_area.position.y as f64 + work_area.size.height as f64 - height - 24.0 * scale
        } else {
            work_area.position.y as f64 + (work_area.size.height as f64 - height) / 2.0
        };
        window
            .set_position(Position::Physical(PhysicalPosition::new(
                x.round() as i32,
                y.round() as i32,
            )))
            .map_err(to_message)?;
    } else {
        let _ = window.center();
    }
    let _ = window.set_focus();
    Ok(())
}

pub fn ensure_sample_plugin() -> Result<(), Box<dyn std::error::Error>> {
    let root = default_plugins_dir();
    let sample = root.join("dev.vvicat.echo");
    if sample.join("plugin.json").exists() {
        return Ok(());
    }
    fs::create_dir_all(&sample)?;
    fs::write(
        sample.join("plugin.json"),
        r#"{
  "id": "dev.vvicat.echo",
  "name": "回显工具",
  "version": "1.0.0",
  "description": "内置示例插件，用于验证 JSON-RPC 执行链路。",
  "keywords": ["echo", "回显", "test"],
  "runtime": "shell",
  "entry": "main.sh",
  "permissions": [],
  "commands": [
    {
      "id": "echo.run",
      "title": "回显输入",
      "keyword": "echo",
      "input": "text"
    }
  ]
}"#,
    )?;
    fs::write(
        sample.join("main.sh"),
        r#"#!/bin/sh
QUERY=$(printf '%s' "$VVITOOLS_RPC_INPUT" | /usr/bin/python3 -c 'import json,sys; print(json.load(sys.stdin)["params"]["query"])' 2>/dev/null)
printf '{"type":"text","text":"回显：%s"}\n' "$QUERY"
"#,
    )?;
    Ok(())
}

fn bundled_marketplace(
    bundled_root: &std::path::Path,
) -> Result<Vec<MarketplaceEntry>, Box<dyn std::error::Error>> {
    let mut entries = load_plugins(bundled_root)?
        .into_iter()
        .map(|plugin| MarketplaceEntry {
            id: plugin.manifest.id,
            name: plugin.manifest.name,
            version: plugin.manifest.version,
            description: plugin.manifest.description,
            icon: plugin.manifest.icon,
            runtime: format!("{:?}", plugin.manifest.runtime).to_lowercase(),
            entry: plugin.manifest.entry,
            bundled: true,
            download_url: String::new(),
            sha256: None,
            permissions: plugin.manifest.permissions,
        })
        .collect::<Vec<_>>();

    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VviTools")
        .join("marketplace");
    fs::create_dir_all(&dir)?;
    let zip_path = dir.join("text-kit-1.0.0.zip");
    let file = fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    zip.start_file("plugin.json", opts)?;
    zip.write_all(
        r#"{
  "id": "dev.vvicat.text-kit",
  "name": "文本工具",
  "version": "1.0.0",
  "description": "提供大小写转换和长度统计。",
  "keywords": ["text", "case", "文本"],
  "runtime": "node",
  "entry": "main.js",
  "permissions": ["clipboard"],
  "commands": [
    {
      "id": "text.uppercase",
      "title": "转为大写",
      "keyword": "upper",
      "input": "text"
    }
  ]
}"#
        .as_bytes(),
    )?;
    zip.start_file("main.js", opts)?;
    zip.write_all(
        r#"const input = JSON.parse(process.env.VVITOOLS_RPC_INPUT || "{}");
const query = input.params?.query || "";
console.log(JSON.stringify({
  type: "list",
  items: [
    {
      title: query.toUpperCase(),
      subtitle: `${query.length} 个字符`,
      action: { type: "copy", value: query.toUpperCase() }
    }
  ]
}));
"#
        .as_bytes(),
    )?;
    zip.finish()?;

    entries.push(MarketplaceEntry {
        id: "dev.vvicat.text-kit".into(),
        name: "文本工具".into(),
        version: "1.0.0".into(),
        description: "静态市场样例插件，演示 zip 安装、权限确认和 Node JSON-RPC 执行。".into(),
        icon: String::new(),
        runtime: "node".into(),
        entry: "main.js".into(),
        bundled: false,
        download_url: zip_path.to_string_lossy().into_owned(),
        sha256: None,
        permissions: vec!["clipboard".into()],
    });
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

fn bundled_plugins_dir_for_app(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(resource_dir) = app.path().resource_dir() {
        for relative in ["plugins", "_up_/plugins"] {
            let resource_plugins = resource_dir.join(relative);
            if resource_plugins.exists() {
                return resource_plugins;
            }
        }
    }
    bundled_plugins_dir()
}

fn load_available_plugins_for_app(
    app: &tauri::AppHandle,
) -> Result<Vec<InstalledPlugin>, Box<dyn std::error::Error>> {
    load_available_plugins_from(&bundled_plugins_dir_for_app(app)).map_err(Into::into)
}

fn read_text_from_clipboard() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(to_message)?;
    clipboard.get_text().map_err(to_message)
}

fn write_text_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(to_message)?;
    clipboard.set_text(text.to_string()).map_err(to_message)
}

fn read_image_from_clipboard() -> Result<ImageData<'static>, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(to_message)?;
    clipboard.get_image().map_err(to_message)
}

fn read_files_from_clipboard() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"import AppKit
import Foundation

let urls = NSPasteboard.general.readObjects(
    forClasses: [NSURL.self],
    options: [.urlReadingFileURLsOnly: true]
) as? [URL] ?? []
let paths = urls.map { $0.path }
let data = try JSONSerialization.data(withJSONObject: paths, options: [])
FileHandle.standardOutput.write(data)
"#;
        let output = Command::new("swift")
            .args(["-e", script])
            .output()
            .map_err(to_message)?;
        if !output.status.success() {
            return Ok(Vec::new());
        }
        serde_json::from_slice(&output.stdout).map_err(to_message)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

fn write_image_to_clipboard(item: &ClipboardItem) -> Result<(), String> {
    let bytes = fs::read(&item.image_path).map_err(to_message)?;
    let image = image::load_from_memory(&bytes)
        .map_err(to_message)?
        .to_rgba8();
    let width = image.width() as usize;
    let height = image.height() as usize;
    let mut clipboard = arboard::Clipboard::new().map_err(to_message)?;
    clipboard
        .set_image(ImageData {
            width,
            height,
            bytes: Cow::Owned(image.into_raw()),
        })
        .map_err(to_message)
}

fn record_clipboard_text(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut history = load_clipboard_history()?;
    let id = clipboard_item_id(text);
    history.retain(|item| item.id != id);
    history.insert(
        0,
        ClipboardItem {
            id,
            kind: "text".into(),
            text: text.into(),
            preview: clipboard_preview(text),
            copied_at: Utc::now().to_rfc3339(),
            favorite: false,
            image_path: String::new(),
            width: 0,
            height: 0,
            file_paths: Vec::new(),
        },
    );
    history.truncate(CLIPBOARD_HISTORY_LIMIT);
    save_clipboard_history(&history)
}

fn record_clipboard_image(
    image: ImageData<'static>,
    id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let image_path = save_clipboard_image(&id, image.width, image.height, image.bytes.as_ref())?;
    let mut history = load_clipboard_history()?;
    history.retain(|item| item.id != id);
    history.insert(
        0,
        ClipboardItem {
            id,
            kind: "image".into(),
            text: String::new(),
            preview: format!("{} x {} 图片", image.width, image.height),
            copied_at: Utc::now().to_rfc3339(),
            favorite: false,
            image_path: image_path.to_string_lossy().into_owned(),
            width: image.width,
            height: image.height,
            file_paths: Vec::new(),
        },
    );
    history.truncate(CLIPBOARD_HISTORY_LIMIT);
    save_clipboard_history(&history)
}

fn record_clipboard_image_from_path(
    item: &ClipboardItem,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut history = load_clipboard_history()?;
    history.retain(|entry| entry.id != item.id);
    let mut restored = item.clone();
    restored.copied_at = Utc::now().to_rfc3339();
    history.insert(0, restored);
    history.truncate(CLIPBOARD_HISTORY_LIMIT);
    save_clipboard_history(&history)
}

fn record_clipboard_files(
    file_paths: Vec<String>,
    id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut history = load_clipboard_history()?;
    history.retain(|item| item.id != id);
    let preview = clipboard_files_preview(&file_paths);
    history.insert(
        0,
        ClipboardItem {
            id,
            kind: "file".into(),
            text: file_paths.join("\n"),
            preview,
            copied_at: Utc::now().to_rfc3339(),
            favorite: false,
            image_path: String::new(),
            width: 0,
            height: 0,
            file_paths,
        },
    );
    history.truncate(CLIPBOARD_HISTORY_LIMIT);
    save_clipboard_history(&history)
}

fn record_clipboard_files_from_item(
    item: &ClipboardItem,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut history = load_clipboard_history()?;
    history.retain(|entry| entry.id != item.id);
    let mut restored = item.clone();
    restored.copied_at = Utc::now().to_rfc3339();
    history.insert(0, restored);
    history.truncate(CLIPBOARD_HISTORY_LIMIT);
    save_clipboard_history(&history)
}

fn write_files_to_clipboard(item: &ClipboardItem) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let paths = serde_json::to_string(&item.file_paths).map_err(to_message)?;
        let script = r#"import AppKit
import Foundation

guard let raw = ProcessInfo.processInfo.environment["VVITOOLS_FILE_PATHS"],
      let data = raw.data(using: .utf8),
      let paths = try JSONSerialization.jsonObject(with: data) as? [String] else {
    exit(1)
}
let urls = paths.map { URL(fileURLWithPath: $0) as NSURL }
let pasteboard = NSPasteboard.general
pasteboard.clearContents()
pasteboard.writeObjects(urls)
"#;
        let output = Command::new("swift")
            .env("VVITOOLS_FILE_PATHS", paths)
            .args(["-e", script])
            .output()
            .map_err(to_message)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        write_text_to_clipboard(&item.text)
    }
}

fn save_clipboard_image(
    id: &str,
    width: usize,
    height: usize,
    bytes: &[u8],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    fs::create_dir_all(clipboard_images_dir())?;
    let path = clipboard_images_dir().join(format!("{id}.png"));
    let image: RgbaImage = ImageBuffer::from_raw(width as u32, height as u32, bytes.to_vec())
        .ok_or_else(|| format!("图片数据尺寸不匹配: {}x{}", width, height))?;
    image.save(&path)?;
    Ok(path)
}

fn load_clipboard_history() -> Result<Vec<ClipboardItem>, Box<dyn std::error::Error>> {
    let path = clipboard_history_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read(&path)?;
    Ok(serde_json::from_slice(&data)?)
}

fn save_clipboard_history(items: &[ClipboardItem]) -> Result<(), Box<dyn std::error::Error>> {
    let path = clipboard_history_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(items)?)?;
    Ok(())
}

fn clipboard_history_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VviTools")
        .join("clipboard")
        .join("history.json")
}

fn clipboard_images_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VviTools")
        .join("clipboard")
        .join("images")
}

fn clipboard_item_id(text: &str) -> String {
    format!("{:x}", sha2::Sha256::digest(text.as_bytes()))
}

fn clipboard_files_id(file_paths: &[String]) -> String {
    let mut hasher = sha2::Sha256::new();
    for path in file_paths {
        hasher.update(path.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn clipboard_image_id(image: &ImageData<'_>) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(image.width.to_le_bytes());
    hasher.update(image.height.to_le_bytes());
    hasher.update(image.bytes.as_ref());
    format!("{:x}", hasher.finalize())
}

fn clipboard_preview(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized.chars().take(CLIPBOARD_PREVIEW_LIMIT).collect()
}

fn clipboard_files_preview(file_paths: &[String]) -> String {
    let names = file_paths
        .iter()
        .filter_map(|path| {
            PathBuf::from(path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect::<Vec<_>>();
    if names.len() <= 1 {
        names.first().cloned().unwrap_or_else(|| "文件".into())
    } else {
        format!("{} 等 {} 个文件", names[0], names.len())
    }
}

fn plugin_to_view(plugin: InstalledPlugin) -> PluginView {
    let bundled_root = bundled_plugins_dir();
    let bundled = plugin.dir.starts_with(&bundled_root);
    PluginView {
        id: plugin.manifest.id,
        name: plugin.manifest.name,
        version: plugin.manifest.version,
        description: plugin.manifest.description,
        icon: plugin.manifest.icon,
        runtime: format!("{:?}", plugin.manifest.runtime).to_lowercase(),
        entry: plugin.manifest.entry,
        bundled,
        permissions: plugin.manifest.permissions,
        commands: plugin.manifest.commands.len(),
    }
}

fn to_message(error: impl std::fmt::Display) -> String {
    error.to_string()
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}
