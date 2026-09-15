use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
    thread,
    time::Duration,
};

use arboard::ImageData;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use image::{ImageBuffer, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, WebviewWindow};
#[cfg(not(target_os = "macos"))]
use tauri::{PhysicalPosition, Position, Size};
use tauri_plugin_autostart::ManagerExt;
use vvitools_core::plugin::{
    bundled_plugins_dir, default_plugins_dir, delete_user_plugin, ensure_action_allowed,
    install_plugin_from_zip, install_plugin_manifest, load_available_plugins_from, load_plugins,
    run_plugin_command, search_commands, CommandInput, CommandMatch, InstalledPlugin,
    MarketplaceEntry, PermissionDecision, PluginManifest, PluginRuntime, RpcAction, RpcResult,
};

#[cfg(target_os = "macos")]
use core_foundation::{
    base::TCFType,
    boolean::CFBoolean,
    dictionary::{CFDictionary, CFDictionaryRef},
    string::CFString,
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
    pub categories: Vec<String>,
    pub permissions: Vec<String>,
    pub commands: usize,
}

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_url: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
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

#[derive(Debug, Deserialize)]
pub struct CustomPluginImportRequest {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct DeletePluginRequest {
    pub plugin_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AutostartRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct DockVisibilityRequest {
    pub enabled: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AppSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dock_visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status_bar_mode: Option<bool>,
}

impl AppSettings {
    fn dock_visible(&self) -> bool {
        self.dock_visible
            .or_else(|| self.status_bar_mode.map(|enabled| !enabled))
            .unwrap_or_else(default_dock_visible)
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CustomPluginConfig {
    Single(PluginManifest),
    List(Vec<PluginManifest>),
    Object { plugins: Vec<PluginManifest> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Serialize)]
pub struct AccessibilityPermissionStatus {
    pub granted: bool,
    pub supported: bool,
    pub app_path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ClipboardPasteResult {
    pub copied: bool,
    pub paste_requested: bool,
    pub needs_accessibility_permission: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardSettings {
    #[serde(default = "default_clipboard_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub retention_days: u32,
    #[serde(default = "default_clipboard_max_items")]
    pub max_items: usize,
    #[serde(default = "default_clipboard_capture")]
    pub capture_text: bool,
    #[serde(default = "default_clipboard_capture")]
    pub capture_images: bool,
    #[serde(default = "default_clipboard_capture")]
    pub capture_files: bool,
}

impl Default for ClipboardSettings {
    fn default() -> Self {
        Self {
            enabled: default_clipboard_enabled(),
            retention_days: 0,
            max_items: default_clipboard_max_items(),
            capture_text: default_clipboard_capture(),
            capture_images: default_clipboard_capture(),
            capture_files: default_clipboard_capture(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ClipboardStorageInfo {
    pub directory: String,
    pub total_bytes: u64,
    pub item_count: usize,
    pub image_count: usize,
}

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
pub fn is_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(to_message)
}

#[tauri::command]
pub fn set_autostart_enabled(
    app: tauri::AppHandle,
    request: AutostartRequest,
) -> Result<bool, String> {
    if request.enabled {
        app.autolaunch().enable().map_err(to_message)?;
    } else {
        app.autolaunch().disable().map_err(to_message)?;
    }
    app.autolaunch().is_enabled().map_err(to_message)
}

#[tauri::command]
pub fn is_dock_visible_enabled() -> Result<bool, String> {
    Ok(load_app_settings().map_err(to_message)?.dock_visible())
}

#[tauri::command]
pub fn set_dock_visible_enabled(
    app: AppHandle,
    request: DockVisibilityRequest,
) -> Result<bool, String> {
    apply_dock_visibility(&app, request.enabled)?;
    let mut settings = load_app_settings().map_err(to_message)?;
    settings.dock_visible = Some(request.enabled);
    settings.status_bar_mode = None;
    save_app_settings(&settings).map_err(to_message)?;
    Ok(request.enabled)
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
    cleanup_clipboard_history().map_err(to_message)
}

#[tauri::command]
pub fn get_clipboard_settings() -> Result<ClipboardSettings, String> {
    load_clipboard_settings().map_err(to_message)
}

#[tauri::command]
pub fn set_clipboard_settings(settings: ClipboardSettings) -> Result<ClipboardSettings, String> {
    let settings = normalize_clipboard_settings(settings);
    save_clipboard_settings(&settings).map_err(to_message)?;
    cleanup_clipboard_history_with_settings(&settings).map_err(to_message)?;
    Ok(settings)
}

#[tauri::command]
pub fn get_clipboard_storage_info() -> Result<ClipboardStorageInfo, String> {
    let history = cleanup_clipboard_history().map_err(to_message)?;
    let directory = clipboard_data_dir();
    let total_bytes = directory_size(&directory).map_err(to_message)?;
    let image_count = history.iter().filter(|item| item.kind == "image").count();
    Ok(ClipboardStorageInfo {
        directory: directory.to_string_lossy().into_owned(),
        total_bytes,
        item_count: history.len(),
        image_count,
    })
}

#[tauri::command]
pub fn open_clipboard_storage_location() -> Result<(), String> {
    let directory = clipboard_data_dir();
    fs::create_dir_all(&directory).map_err(to_message)?;
    open::that(directory).map_err(to_message)
}

#[tauri::command]
pub fn copy_clipboard_item(request: ClipboardCopyRequest) -> Result<(), String> {
    copy_clipboard_item_by_id(&request.id)
}

#[tauri::command]
pub fn paste_clipboard_item(
    app: tauri::AppHandle,
    request: ClipboardCopyRequest,
) -> Result<ClipboardPasteResult, String> {
    copy_clipboard_item_by_id(&request.id)?;
    if !is_accessibility_trusted(false) {
        let status = accessibility_status(false);
        return Ok(ClipboardPasteResult {
            copied: true,
            paste_requested: false,
            needs_accessibility_permission: !status.granted,
            message: if status.granted {
                "已复制，请再次选择记录以自动粘贴".into()
            } else {
                status.message
            },
        });
    }
    if let Some(window) = app.get_webview_window("clipboard") {
        let _ = window.hide();
    }
    paste_to_previous_frontmost_app()?;
    Ok(ClipboardPasteResult {
        copied: true,
        paste_requested: true,
        needs_accessibility_permission: false,
        message: "已复制并自动粘贴".into(),
    })
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

#[tauri::command]
pub fn accessibility_permission_status() -> Result<AccessibilityPermissionStatus, String> {
    Ok(accessibility_status(false))
}

#[tauri::command]
pub fn request_accessibility_permission() -> Result<AccessibilityPermissionStatus, String> {
    Ok(accessibility_status(true))
}

fn accessibility_status(prompt: bool) -> AccessibilityPermissionStatus {
    #[cfg(target_os = "macos")]
    {
        let granted = is_accessibility_trusted(prompt);
        AccessibilityPermissionStatus {
            granted,
            supported: true,
            app_path: current_app_bundle_path()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
            message: if granted {
                "自动粘贴权限已开启".into()
            } else {
                accessibility_permission_message(prompt)
            },
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        AccessibilityPermissionStatus {
            granted: true,
            supported: false,
            app_path: current_app_bundle_path()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
            message: "当前系统不需要辅助功能授权".into(),
        }
    }
}

#[tauri::command]
pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .status()
            .map_err(to_message)?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

#[tauri::command]
pub fn reveal_current_app_in_finder() -> Result<(), String> {
    let app_path = current_app_bundle_path()
        .or_else(|| std::env::current_exe().ok())
        .ok_or_else(|| "无法识别当前应用路径".to_string())?;
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg("-R")
            .arg(&app_path)
            .status()
            .map_err(to_message)?;
        if status.success() {
            Ok(())
        } else {
            Err("无法在 Finder 中定位当前应用".into())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        open::that(app_path).map_err(to_message)
    }
}

fn current_app_bundle_path() -> Option<PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    for ancestor in current_exe.ancestors() {
        if ancestor
            .extension()
            .is_some_and(|extension| extension == "app")
        {
            return Some(ancestor.to_path_buf());
        }
    }
    Some(current_exe)
}

#[cfg(target_os = "macos")]
fn accessibility_permission_message(prompted: bool) -> String {
    if prompted {
        return "已复制。请在系统设置中允许当前 VviTools 使用辅助功能权限，开启后可自动粘贴到原输入框。".into();
    }
    "已复制。VviTools 当前仍未取得有效辅助功能权限；如果系统设置里已经开启，通常是本地重新打包后 macOS 授权记录失效，请移除 VviTools 后重新添加当前应用，或使用稳定签名的安装包。".into()
}

fn is_accessibility_trusted(prompt: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_accessibility_trusted(prompt)
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
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

fn paste_to_previous_frontmost_app() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let bundle_id = previous_frontmost_app().lock().map_err(to_message)?.clone();
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
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn macos_accessibility_trusted(prompt: bool) -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    }

    if !prompt {
        return unsafe { AXIsProcessTrusted() };
    }

    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let value = CFBoolean::true_value();
    let options = CFDictionary::from_CFType_pairs(&[(key, value)]);
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
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
            let settings = load_clipboard_settings().unwrap_or_default();
            if !settings.enabled {
                thread::sleep(Duration::from_millis(900));
                continue;
            }
            if let Ok(file_paths) = read_files_from_clipboard() {
                if !file_paths.is_empty() {
                    let file_id = clipboard_files_id(&file_paths);
                    if settings.capture_files && file_id != last_file_id {
                        let _ = record_clipboard_files(file_paths, file_id.clone());
                        last_file_id = file_id;
                    }
                    thread::sleep(Duration::from_millis(900));
                    continue;
                }
            }
            if settings.capture_text {
                if let Ok(text) = read_text_from_clipboard() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() && text != last_text {
                        let _ = record_clipboard_text(&text);
                        last_text = text;
                    }
                }
            }
            if settings.capture_images {
                if let Ok(image) = read_image_from_clipboard() {
                    let image_id = clipboard_image_id(&image);
                    if image_id != last_image_id {
                        let _ = record_clipboard_image(image, image_id.clone());
                        last_image_id = image_id;
                    }
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
pub fn import_custom_plugin_config(
    request: CustomPluginImportRequest,
) -> Result<Vec<PluginView>, String> {
    let raw = if !request.content.trim().is_empty() {
        request.content
    } else if !request.url.trim().is_empty() {
        reqwest::blocking::get(request.url.trim())
            .map_err(to_message)?
            .error_for_status()
            .map_err(to_message)?
            .text()
            .map_err(to_message)?
    } else {
        return Err("请选择本地 JSON 文件或输入远程 URL".into());
    };

    let config: CustomPluginConfig = serde_json::from_str(&raw).map_err(to_message)?;
    let manifests = match config {
        CustomPluginConfig::Single(manifest) => vec![manifest],
        CustomPluginConfig::List(manifests) => manifests,
        CustomPluginConfig::Object { plugins } => plugins,
    };
    if manifests.is_empty() {
        return Err("配置文件中没有插件".into());
    }

    manifests
        .into_iter()
        .map(|manifest| install_plugin_manifest(manifest, &default_plugins_dir()))
        .map(|result| result.map(plugin_to_view).map_err(to_message))
        .collect()
}

#[tauri::command]
pub fn delete_custom_plugin(request: DeletePluginRequest) -> Result<(), String> {
    delete_user_plugin(&request.plugin_id, &default_plugins_dir()).map_err(to_message)
}

#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    open::that(url).map_err(to_message)
}

#[tauri::command]
pub fn check_for_update() -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let response = reqwest::blocking::Client::new()
        .get("https://api.github.com/repos/Valiant-Cat/VviTools/releases/latest")
        .header(reqwest::header::USER_AGENT, "VviTools")
        .send()
        .map_err(to_message)?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateInfo {
            current_version: current_version.clone(),
            latest_version: current_version,
            has_update: false,
            release_url: "https://github.com/Valiant-Cat/VviTools/releases".into(),
            message: "暂无发布版本".into(),
        });
    }

    let release: GitHubRelease = response
        .error_for_status()
        .map_err(to_message)?
        .json()
        .map_err(to_message)?;

    if release.draft || release.prerelease {
        return Err("暂无稳定更新版本".into());
    }

    let latest_version = normalize_version(&release.tag_name);
    let has_update = is_newer_version(&latest_version, &current_version);
    Ok(UpdateInfo {
        message: if has_update {
            format!("发现新版本 {}", latest_version)
        } else {
            "已是最新版本".into()
        },
        has_update,
        current_version,
        latest_version,
        release_url: release.html_url,
    })
}

#[tauri::command]
pub fn open_clipboard_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("clipboard")
        .ok_or_else(|| "剪贴板窗口不存在".to_string())?;
    remember_frontmost_app();
    apply_launcher_window(&window, "clipboard")?;
    let _ = window.emit("open-clipboard", ());
    let _ = window.eval("window.dispatchEvent(new CustomEvent('vvitools-open-clipboard'))");
    show_overlay_window(&window, "clipboard")
}

#[tauri::command]
pub fn open_accessibility_permission_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(clipboard_window) = app.get_webview_window("clipboard") {
        let _ = clipboard_window.hide();
    }
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    apply_launcher_window(&window, "permission")?;
    let _ = window.emit("open-accessibility-permission", ());
    let _ = window
        .eval("window.dispatchEvent(new CustomEvent('vvitools-open-accessibility-permission'))");
    show_overlay_window(&window, "permission")
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

fn launcher_size(view: &str) -> LogicalSize<f64> {
    match view {
        "feature" => LogicalSize::new(980.0, 640.0),
        "permission" => LogicalSize::new(500.0, 340.0),
        "clipboard" => LogicalSize::new(940.0, 238.0),
        _ => LogicalSize::new(720.0, 420.0),
    }
}

#[cfg(target_os = "macos")]
fn position_native_overlay(
    window: &objc2_app_kit::NSWindow,
    screen: &objc2_app_kit::NSScreen,
    size: LogicalSize<f64>,
    bottom_aligned: bool,
) {
    use objc2_foundation::{NSPoint, NSSize};
    // 不混用 Tauri 的异步尺寸/位置 API，防止旧坐标在显示后覆盖目标屏幕。
    window.setContentSize(NSSize::new(size.width, size.height));
    let area = screen.visibleFrame();
    let frame = window.frame();
    let x = area.origin.x + (area.size.width - frame.size.width) / 2.0;
    let y = if bottom_aligned {
        area.origin.y + 24.0
    } else {
        area.origin.y + (area.size.height - frame.size.height) / 2.0
    };
    window.setFrameOrigin(NSPoint::new(x, y));
}

#[cfg(target_os = "macos")]
pub fn apply_launcher_window(window: &WebviewWindow, view: &str) -> Result<(), String> {
    use tauri_nspanel::ManagerExt;
    let panel = window
        .app_handle()
        .get_webview_panel(window.label())
        .map_err(|_| "原生面板尚未初始化".to_string())?;
    let size = launcher_size(view);
    let bottom_aligned = view == "clipboard";
    window
        .run_on_main_thread(move || {
            let native = panel.as_panel();
            if let Some(screen) = native.screen() {
                position_native_overlay(native, &screen, size, bottom_aligned);
            }
        })
        .map_err(to_message)
}

#[cfg(not(target_os = "macos"))]
pub fn apply_launcher_window(window: &WebviewWindow, view: &str) -> Result<(), String> {
    let size = launcher_size(view);
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
    Ok(())
}

#[cfg(target_os = "macos")]
fn focused_window_frame() -> Option<objc2_foundation::NSRect> {
    use core_foundation::base::{CFType, CFTypeRef};
    use core_foundation::string::CFStringRef;
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use std::ffi::c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
        fn AXUIElementCopyAttributeValue(
            element: CFTypeRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> i32;
        fn AXUIElementSetMessagingTimeout(element: CFTypeRef, timeout: f32) -> i32;
        fn AXValueGetTypeID() -> usize;
        fn AXValueGetValue(value: CFTypeRef, kind: i32, output: *mut c_void) -> u8;
    }

    fn attribute(element: &CFType, name: &str) -> Option<CFType> {
        let mut value = std::ptr::null();
        let name = CFString::new(name);
        unsafe {
            AXUIElementSetMessagingTimeout(element.as_CFTypeRef(), 0.15);
            if AXUIElementCopyAttributeValue(
                element.as_CFTypeRef(),
                name.as_concrete_TypeRef(),
                &mut value,
            ) != 0
                || value.is_null()
            {
                return None;
            }
            Some(CFType::wrap_under_create_rule(value))
        }
    }

    if !macos_accessibility_trusted(false) {
        return None;
    }
    unsafe {
        let frontmost = objc2_app_kit::NSWorkspace::sharedWorkspace().frontmostApplication()?;
        let application = AXUIElementCreateApplication(frontmost.processIdentifier());
        if application.is_null() {
            return None;
        }
        let app = CFType::wrap_under_create_rule(application);
        let window =
            attribute(&app, "AXFocusedWindow").or_else(|| attribute(&app, "AXMainWindow"))?;
        let position = attribute(&window, "AXPosition")?;
        let size = attribute(&window, "AXSize")?;
        if position.type_of() != AXValueGetTypeID() || size.type_of() != AXValueGetTypeID() {
            return None;
        }
        let mut origin = NSPoint::default();
        let mut extent = NSSize::default();
        if AXValueGetValue(
            position.as_CFTypeRef(),
            1,
            (&mut origin as *mut NSPoint).cast(),
        ) == 0
            || AXValueGetValue(size.as_CFTypeRef(), 2, (&mut extent as *mut NSSize).cast()) == 0
        {
            return None;
        }
        Some(NSRect::new(origin, extent))
    }
}

#[cfg(target_os = "macos")]
fn frontmost_visible_window_frame() -> Option<objc2_foundation::NSRect> {
    use core_foundation::{
        array::{CFArray, CFArrayRef},
        base::CFType,
        number::CFNumber,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowListCopyWindowInfo(options: u32, relative_to: u32) -> CFArrayRef;
    }

    fn number(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<f64> {
        dict.find(&CFString::new(key))?
            .downcast::<CFNumber>()?
            .to_f64()
    }

    let app = objc2_app_kit::NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let pid = app.processIdentifier() as f64;
    // 仅查询当前可见窗口元数据，不获取图像或窗口标题。
    let raw = unsafe { CGWindowListCopyWindowInfo(1 | 16, 0) };
    if raw.is_null() {
        return None;
    }
    let windows: CFArray<CFDictionary<CFString, CFType>> =
        unsafe { CFArray::wrap_under_create_rule(raw) };
    for window in windows.iter() {
        if number(&window, "kCGWindowOwnerPID") != Some(pid)
            || number(&window, "kCGWindowLayer") != Some(0.0)
            || number(&window, "kCGWindowAlpha").unwrap_or(1.0) <= 0.0
        {
            continue;
        }
        let Some(bounds) = window
            .find(&CFString::new("kCGWindowBounds"))
            .and_then(|value| value.downcast::<CFDictionary>())
        else {
            continue;
        };
        let bounds: CFDictionary<CFString, CFType> =
            unsafe { CFDictionary::wrap_under_get_rule(bounds.as_concrete_TypeRef()) };
        let geometry = (
            number(&bounds, "X"),
            number(&bounds, "Y"),
            number(&bounds, "Width"),
            number(&bounds, "Height"),
        );
        if let (Some(x), Some(y), Some(width), Some(height)) = geometry {
            if width > 0.0 && height > 0.0 {
                return Some(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn focused_screen_index(
    window: objc2_foundation::NSRect,
    screens: &[objc2_foundation::NSRect],
) -> Option<usize> {
    // AX 使用主屏左上角为原点，AppKit 使用主屏左下角；两者均为逻辑坐标。
    let primary_top = screens.first()?.origin.y + screens.first()?.size.height;
    let top = primary_top - window.origin.y;
    let bottom = top - window.size.height;
    let mut best = None;
    let mut best_area = 0.0;
    for (index, screen) in screens.iter().enumerate() {
        let width = (window.origin.x + window.size.width).min(screen.origin.x + screen.size.width)
            - window.origin.x.max(screen.origin.x);
        let height = top.min(screen.origin.y + screen.size.height) - bottom.max(screen.origin.y);
        let area = width.max(0.0) * height.max(0.0);
        if area > best_area {
            best = Some(index);
            best_area = area;
        }
    }
    best
}

#[cfg(target_os = "macos")]
pub fn show_overlay_window(window: &WebviewWindow, view: &str) -> Result<(), String> {
    use tauri_nspanel::ManagerExt;
    let panel = window
        .app_handle()
        .get_webview_panel(window.label())
        .map_err(|_| "原生面板尚未初始化".to_string())?;
    let bottom_aligned = view == "clipboard";
    let size = launcher_size(view);
    let diagnostic_path = window
        .app_handle()
        .path()
        .app_cache_dir()
        .ok()
        .map(|dir| dir.join("window-placement.json"));
    let label = window.label().to_string();
    window
        .run_on_main_thread(move || {
            use objc2::MainThreadMarker;
            use objc2_app_kit::{NSEvent, NSScreen};

            if let Some(mtm) = MainThreadMarker::new() {
                let ns_window = panel.as_panel();
                let screens = NSScreen::screens(mtm);
                let frames: Vec<_> = screens.iter().map(|screen| screen.frame()).collect();
                let focused_frame = focused_window_frame();
                let visible_frame = if focused_frame.is_none() {
                    frontmost_visible_window_frame()
                } else {
                    None
                };
                let focused_index = focused_frame
                    .or(visible_frame)
                    .and_then(|frame| focused_screen_index(frame, &frames));
                let mouse = NSEvent::mouseLocation();
                let target_index = focused_index.or_else(|| {
                    frames.iter().position(|frame| {
                        mouse.x >= frame.origin.x
                            && mouse.x < frame.origin.x + frame.size.width
                            && mouse.y >= frame.origin.y
                            && mouse.y < frame.origin.y + frame.size.height
                    })
                });
                let screen = target_index
                    .map(|index| screens.objectAtIndex(index))
                    .or_else(|| ns_window.screen())
                    .or_else(|| NSScreen::mainScreen(mtm));
                if let Some(screen) = screen {
                    position_native_overlay(ns_window, &screen, size, bottom_aligned);
                }

                // 非激活式面板直接接收键盘焦点，不激活应用以免切回桌面空间。
                panel.show_and_make_key();
                if let Some(path) = diagnostic_path {
                    let rect = |r: objc2_foundation::NSRect| {
                        [r.origin.x, r.origin.y, r.size.width, r.size.height]
                    };
                    let snapshot = serde_json::json!({
                        "time": Utc::now().to_rfc3339(),
                        "window": label,
                        "accessibility_trusted": macos_accessibility_trusted(false),
                        "focused_window_ax": focused_frame.map(rect),
                        "frontmost_window_cg": visible_frame.map(rect),
                        "screens_appkit": frames.into_iter().map(rect).collect::<Vec<_>>(),
                        "mouse_appkit": [mouse.x, mouse.y],
                        "focused_screen": focused_index,
                        "target_screen": target_index,
                        "actual_frame_appkit": rect(ns_window.frame())
                    });
                    // 仅保留最后一次坐标诊断，不记录窗口标题或剪贴板内容。
                    if let Some(parent) = path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(path, snapshot.to_string());
                }
            }
        })
        .map_err(to_message)
}

#[cfg(not(target_os = "macos"))]
pub fn show_overlay_window(window: &WebviewWindow, _view: &str) -> Result<(), String> {
    window.show().map_err(to_message)?;
    window.set_focus().map_err(to_message)
}

fn bundled_marketplace(
    bundled_root: &std::path::Path,
) -> Result<Vec<MarketplaceEntry>, Box<dyn std::error::Error>> {
    let entries = load_plugins(bundled_root)?
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
            categories: plugin.manifest.categories,
            permissions: plugin.manifest.permissions,
        })
        .collect::<Vec<_>>();
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
    let settings = load_clipboard_settings()?;
    if !settings.enabled || !settings.capture_text {
        return Ok(());
    }
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
    save_cleaned_clipboard_history(history, &settings)
}

fn record_clipboard_image(
    image: ImageData<'static>,
    id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let settings = load_clipboard_settings()?;
    if !settings.enabled || !settings.capture_images {
        return Ok(());
    }
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
    save_cleaned_clipboard_history(history, &settings)
}

fn record_clipboard_image_from_path(
    item: &ClipboardItem,
) -> Result<(), Box<dyn std::error::Error>> {
    let settings = load_clipboard_settings()?;
    if !settings.enabled || !settings.capture_images {
        return Ok(());
    }
    let mut history = load_clipboard_history()?;
    history.retain(|entry| entry.id != item.id);
    let mut restored = item.clone();
    restored.copied_at = Utc::now().to_rfc3339();
    history.insert(0, restored);
    save_cleaned_clipboard_history(history, &settings)
}

fn record_clipboard_files(
    file_paths: Vec<String>,
    id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let settings = load_clipboard_settings()?;
    if !settings.enabled || !settings.capture_files {
        return Ok(());
    }
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
    save_cleaned_clipboard_history(history, &settings)
}

fn record_clipboard_files_from_item(
    item: &ClipboardItem,
) -> Result<(), Box<dyn std::error::Error>> {
    let settings = load_clipboard_settings()?;
    if !settings.enabled || !settings.capture_files {
        return Ok(());
    }
    let mut history = load_clipboard_history()?;
    history.retain(|entry| entry.id != item.id);
    let mut restored = item.clone();
    restored.copied_at = Utc::now().to_rfc3339();
    history.insert(0, restored);
    save_cleaned_clipboard_history(history, &settings)
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

fn save_cleaned_clipboard_history(
    history: Vec<ClipboardItem>,
    settings: &ClipboardSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let cleaned = prune_clipboard_history(history.clone(), settings, Utc::now());
    remove_discarded_clipboard_images(&history, &cleaned);
    save_clipboard_history(&cleaned)
}

fn cleanup_clipboard_history() -> Result<Vec<ClipboardItem>, Box<dyn std::error::Error>> {
    let settings = load_clipboard_settings()?;
    cleanup_clipboard_history_with_settings(&settings)
}

fn cleanup_clipboard_history_with_settings(
    settings: &ClipboardSettings,
) -> Result<Vec<ClipboardItem>, Box<dyn std::error::Error>> {
    let history = load_clipboard_history()?;
    let cleaned = prune_clipboard_history(history.clone(), settings, Utc::now());
    if cleaned != history {
        remove_discarded_clipboard_images(&history, &cleaned);
        save_clipboard_history(&cleaned)?;
    }
    Ok(cleaned)
}

fn prune_clipboard_history(
    history: Vec<ClipboardItem>,
    settings: &ClipboardSettings,
    now: DateTime<Utc>,
) -> Vec<ClipboardItem> {
    let cutoff = (settings.retention_days > 0)
        .then(|| now - ChronoDuration::days(i64::from(settings.retention_days)));
    let filtered = history
        .into_iter()
        .filter(|item| {
            if item.favorite {
                return true;
            }
            cutoff.map_or(true, |minimum| {
                DateTime::parse_from_rfc3339(&item.copied_at)
                    .map(|copied_at| copied_at.with_timezone(&Utc) >= minimum)
                    .unwrap_or(true)
            })
        })
        .collect::<Vec<_>>();

    if filtered.len() <= settings.max_items {
        return filtered;
    }

    let mut keep = vec![false; filtered.len()];
    let mut kept = 0;
    for (index, item) in filtered.iter().enumerate() {
        if item.favorite && kept < settings.max_items {
            keep[index] = true;
            kept += 1;
        }
    }
    for (index, item) in filtered.iter().enumerate() {
        if !item.favorite && kept < settings.max_items {
            keep[index] = true;
            kept += 1;
        }
    }

    filtered
        .into_iter()
        .enumerate()
        .filter_map(|(index, item)| keep[index].then_some(item))
        .collect()
}

fn remove_discarded_clipboard_images(original: &[ClipboardItem], retained: &[ClipboardItem]) {
    for item in original {
        if item.kind == "image"
            && !item.image_path.is_empty()
            && !retained
                .iter()
                .any(|retained_item| retained_item.image_path == item.image_path)
        {
            let _ = fs::remove_file(&item.image_path);
        }
    }
}

fn clipboard_history_path() -> PathBuf {
    clipboard_data_dir().join("history.json")
}

fn load_clipboard_settings() -> Result<ClipboardSettings, Box<dyn std::error::Error>> {
    let path = clipboard_settings_path();
    if !path.exists() {
        return Ok(ClipboardSettings::default());
    }
    let data = fs::read(path)?;
    Ok(normalize_clipboard_settings(serde_json::from_slice(&data)?))
}

fn save_clipboard_settings(settings: &ClipboardSettings) -> Result<(), Box<dyn std::error::Error>> {
    let path = clipboard_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(settings)?)?;
    Ok(())
}

fn clipboard_settings_path() -> PathBuf {
    clipboard_data_dir().join("settings.json")
}

fn clipboard_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VviTools")
        .join("clipboard")
}

fn directory_size(path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(0);
    }
    let mut total = 0;
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

pub fn load_dock_visible_setting() -> bool {
    load_app_settings()
        .map(|settings| settings.dock_visible())
        .unwrap_or_else(|_| default_dock_visible())
}

pub fn apply_dock_visibility(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("vvitools") {
        tray.set_visible(true).map_err(to_message)?;
    }

    #[cfg(target_os = "macos")]
    {
        if enabled {
            app.set_activation_policy(tauri::ActivationPolicy::Regular)
                .map_err(to_message)?;
            app.set_dock_visibility(true).map_err(to_message)?;
        } else {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory)
                .map_err(to_message)?;
            app.set_dock_visibility(false).map_err(to_message)?;
        }
    }

    Ok(())
}

fn load_app_settings() -> Result<AppSettings, Box<dyn std::error::Error>> {
    let path = app_settings_path();
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let data = fs::read(path)?;
    Ok(serde_json::from_slice(&data)?)
}

fn save_app_settings(settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
    let path = app_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(settings)?)?;
    Ok(())
}

fn app_settings_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VviTools")
        .join("settings.json")
}

fn default_dock_visible() -> bool {
    false
}

fn default_clipboard_enabled() -> bool {
    true
}

fn default_clipboard_max_items() -> usize {
    200
}

fn default_clipboard_capture() -> bool {
    true
}

fn normalize_clipboard_settings(mut settings: ClipboardSettings) -> ClipboardSettings {
    settings.retention_days = settings.retention_days.min(3650);
    settings.max_items = settings.max_items.clamp(10, 5000);
    settings
}

fn normalize_version(version: &str) -> String {
    version.trim().trim_start_matches('v').to_string()
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_parts = version_parts(latest);
    let current_parts = version_parts(current);
    for index in 0..latest_parts.len().max(current_parts.len()) {
        let latest_part = *latest_parts.get(index).unwrap_or(&0);
        let current_part = *current_parts.get(index).unwrap_or(&0);
        if latest_part != current_part {
            return latest_part > current_part;
        }
    }
    false
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

fn clipboard_images_dir() -> PathBuf {
    clipboard_data_dir().join("images")
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
    let bundled =
        plugin.manifest.runtime == PluginRuntime::Builtin || plugin.dir.starts_with(&bundled_root);
    PluginView {
        id: plugin.manifest.id,
        name: plugin.manifest.name,
        version: plugin.manifest.version,
        description: plugin.manifest.description,
        icon: plugin.manifest.icon,
        runtime: format!("{:?}", plugin.manifest.runtime).to_lowercase(),
        entry: plugin.manifest.entry,
        bundled,
        categories: plugin.manifest.categories,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn focused_screen_tracks_external_displays_and_spanning_windows() {
        use objc2_foundation::{NSPoint, NSRect, NSSize};
        let rect = |x, y, w, h| NSRect::new(NSPoint::new(x, y), NSSize::new(w, h));
        let screens = [
            rect(0.0, 0.0, 1440.0, 900.0),
            rect(1440.0, 0.0, 1920.0, 1080.0),
            rect(-1920.0, 0.0, 1920.0, 1080.0),
            rect(0.0, 900.0, 1440.0, 900.0),
        ];
        assert_eq!(
            focused_screen_index(rect(1500.0, 0.0, 800.0, 600.0), &screens),
            Some(1)
        );
        assert_eq!(
            focused_screen_index(rect(-1800.0, 0.0, 800.0, 600.0), &screens),
            Some(2)
        );
        assert_eq!(
            focused_screen_index(rect(100.0, -800.0, 800.0, 600.0), &screens),
            Some(3)
        );
        assert_eq!(
            focused_screen_index(rect(1300.0, 0.0, 800.0, 600.0), &screens),
            Some(1)
        );
        assert_eq!(
            focused_screen_index(rect(100.0, 100.0, 800.0, 600.0), &screens),
            Some(0)
        );
        assert_eq!(
            focused_screen_index(rect(5000.0, 0.0, 800.0, 600.0), &screens),
            None
        );
        assert_eq!(
            focused_screen_index(rect(0.0, 0.0, 800.0, 600.0), &[]),
            None
        );
    }

    fn clipboard_item(id: &str, copied_at: DateTime<Utc>, favorite: bool) -> ClipboardItem {
        ClipboardItem {
            id: id.into(),
            kind: "text".into(),
            text: id.into(),
            preview: id.into(),
            copied_at: copied_at.to_rfc3339(),
            favorite,
            image_path: String::new(),
            width: 0,
            height: 0,
            file_paths: Vec::new(),
        }
    }

    #[test]
    fn clipboard_retention_keeps_favorites() {
        let now = Utc::now();
        let settings = ClipboardSettings {
            retention_days: 7,
            ..ClipboardSettings::default()
        };
        let history = vec![
            clipboard_item("recent", now - ChronoDuration::days(2), false),
            clipboard_item("expired", now - ChronoDuration::days(12), false),
            clipboard_item("favorite", now - ChronoDuration::days(30), true),
        ];

        let cleaned = prune_clipboard_history(history, &settings, now);

        assert_eq!(
            cleaned
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["recent", "favorite"]
        );
    }

    #[test]
    fn clipboard_capacity_prioritizes_favorites_without_reordering() {
        let now = Utc::now();
        let settings = ClipboardSettings {
            max_items: 2,
            ..ClipboardSettings::default()
        };
        let history = vec![
            clipboard_item("newest", now, false),
            clipboard_item("favorite", now - ChronoDuration::minutes(1), true),
            clipboard_item("older", now - ChronoDuration::minutes(2), false),
        ];

        let cleaned = prune_clipboard_history(history, &settings, now);

        assert_eq!(
            cleaned
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["newest", "favorite"]
        );
    }

    #[test]
    fn clipboard_settings_are_bounded() {
        let settings = normalize_clipboard_settings(ClipboardSettings {
            retention_days: u32::MAX,
            max_items: 0,
            ..ClipboardSettings::default()
        });

        assert_eq!(settings.retention_days, 3650);
        assert_eq!(settings.max_items, 10);
    }
}
