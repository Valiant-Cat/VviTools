use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{ipc::Channel, AppHandle, State, WebviewWindow};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct UpdateState(Mutex<PendingUpdate>);

#[derive(Default)]
struct PendingUpdate {
    update: Option<Update>,
    installed: bool,
}

#[derive(Serialize)]
pub struct UpdateInfo {
    current_version: String,
    latest_version: String,
    has_update: bool,
    notes: String,
}

#[derive(Clone, Serialize)]
pub struct UpdateProgress {
    phase: &'static str,
    downloaded: u64,
    total: Option<u64>,
}

fn require_main(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("请从主面板设置页更新应用".into())
    }
}

fn update_error(error: tauri_plugin_updater::Error) -> String {
    use tauri_plugin_updater::Error;
    match error {
        Error::ReleaseNotFound => "更新源尚未就绪，请稍后重试或查看发布页".into(),
        Error::TargetNotFound(_) | Error::TargetsNotFound(_) => {
            "此版本尚未提供适用于当前系统架构的更新包".into()
        }
        Error::Minisign(_) | Error::Base64(_) | Error::SignatureUtf8(_) => {
            "更新包签名校验失败，已阻止安装，请重新检查更新".into()
        }
        Error::AuthenticationFailed => "安装授权已取消或失败，可重试安装".into(),
        other => format!("更新失败：{other}"),
    }
}

#[tauri::command]
pub fn app_update_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, UpdateState>,
) -> Result<UpdateInfo, String> {
    require_main(&window)?;
    let mut state = state.0.try_lock().map_err(|_| "更新操作正在进行中")?;
    if state.installed {
        return Err("更新已安装，请重启应用".into());
    }
    state.update = None;
    let update = app
        .updater_builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(update_error)?
        .check()
        .await
        .map_err(update_error)?;
    let current_version = app.package_info().version.to_string();
    let info = UpdateInfo {
        latest_version: update
            .as_ref()
            .map(|u| u.version.clone())
            .unwrap_or_else(|| current_version.clone()),
        has_update: update.is_some(),
        notes: update
            .as_ref()
            .and_then(|u| u.body.clone())
            .unwrap_or_default(),
        current_version,
    };
    state.update = update;
    Ok(info)
}

#[tauri::command]
pub async fn install_app_update(
    window: WebviewWindow,
    state: State<'_, UpdateState>,
    on_progress: Channel<UpdateProgress>,
) -> Result<(), String> {
    require_main(&window)?;
    let mut state = state.0.try_lock().map_err(|_| "更新操作正在进行中")?;
    if state.installed {
        return Err("更新已安装，请重启应用".into());
    }
    let mut update = state.update.clone().ok_or("请先检查更新")?;
    update.timeout = Some(Duration::from_secs(600));
    let mut downloaded = 0_u64;
    let mut last_sent = Instant::now() - Duration::from_secs(1);
    // 安装只接收宿主检查得到的更新对象，不接受前端传入下载地址或签名。
    let bytes = update
        .download(
            |chunk, total| {
                downloaded = downloaded.saturating_add(chunk as u64);
                if last_sent.elapsed() >= Duration::from_millis(100) || Some(downloaded) == total {
                    let _ = on_progress.send(UpdateProgress {
                        phase: "downloading",
                        downloaded,
                        total,
                    });
                    last_sent = Instant::now();
                }
            },
            || {},
        )
        .await
        .map_err(update_error)?;
    let _ = on_progress.send(UpdateProgress {
        phase: "installing",
        downloaded: bytes.len() as u64,
        total: Some(bytes.len() as u64),
    });
    tauri::async_runtime::spawn_blocking(move || update.install(bytes))
        .await
        .map_err(|e| format!("安装任务失败：{e}"))?
        .map_err(update_error)?;
    state.installed = true;
    state.update = None;
    Ok(())
}

#[tauri::command]
pub async fn restart_after_update(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, UpdateState>,
) -> Result<(), String> {
    require_main(&window)?;
    let state = state.0.try_lock().map_err(|_| "更新操作正在进行中")?;
    if !state.installed {
        return Err("尚未安装更新".into());
    }
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_feed_is_not_reported_as_latest() {
        assert!(update_error(tauri_plugin_updater::Error::ReleaseNotFound).contains("尚未就绪"));
    }

    #[test]
    fn missing_platform_is_actionable() {
        assert!(
            update_error(tauri_plugin_updater::Error::TargetsNotFound(vec![])).contains("系统架构")
        );
    }

    #[tokio::test]
    async fn state_rejects_parallel_operations_and_starts_without_install() {
        let state = UpdateState::default();
        let lock = state.0.try_lock().unwrap();
        assert!(!lock.installed);
        assert!(lock.update.is_none());
        assert!(state.0.try_lock().is_err());
        drop(lock);
        assert!(state.0.try_lock().is_ok());
    }
}
