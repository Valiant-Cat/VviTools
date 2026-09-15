#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod updater;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewWindow, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const TRAY_TOGGLE_ID: &str = "toggle";
const TRAY_SETTINGS_ID: &str = "settings";
const TRAY_QUIT_ID: &str = "quit";

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(LauncherPanel {
        config: {
            can_become_key_window: true,
            can_become_main_window: false,
            is_floating_panel: true,
            hides_on_deactivate: false,
            becomes_key_only_if_needed: false
        }
    })
}

fn main() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    builder
        .manage(updater::UpdateState::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_plugins,
            commands::search_plugin_commands,
            commands::execute_plugin_command,
            commands::execute_plugin_action,
            commands::list_clipboard_history,
            commands::get_clipboard_settings,
            commands::set_clipboard_settings,
            commands::get_clipboard_storage_info,
            commands::open_clipboard_storage_location,
            commands::copy_clipboard_item,
            commands::paste_clipboard_item,
            commands::accessibility_permission_status,
            commands::request_accessibility_permission,
            commands::open_accessibility_settings,
            commands::reveal_current_app_in_finder,
            commands::toggle_clipboard_favorite,
            commands::delete_clipboard_item,
            commands::clear_clipboard_history,
            commands::load_marketplace,
            commands::install_marketplace_plugin,
            commands::import_custom_plugin_config,
            commands::delete_custom_plugin,
            commands::open_external,
            updater::check_for_update,
            updater::app_update_version,
            updater::install_app_update,
            updater::restart_after_update,
            commands::open_clipboard_window,
            commands::open_accessibility_permission_window,
            commands::hide_launcher,
            commands::hide_window,
            commands::set_launcher_view,
            commands::is_autostart_enabled,
            commands::set_autostart_enabled,
            commands::is_dock_visible_enabled,
            commands::set_dock_visible_enabled,
        ])
        .on_window_event(|window, event| {
            if matches!(window.label(), "main" | "clipboard")
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let launcher_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            let clipboard_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyV);
            let handler_shortcut = launcher_shortcut.clone();
            let handler_clipboard_shortcut = clipboard_shortcut.clone();
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, event| {
                        if event.state() != ShortcutState::Pressed {
                            return;
                        }

                        if shortcut == &handler_shortcut {
                            if let Some(window) = app.get_webview_window("main") {
                                let visible = window.is_visible().unwrap_or(false);
                                if visible {
                                    reset_launcher_view(&window);
                                    let _ = window.hide();
                                } else {
                                    show_launcher(app);
                                }
                            }
                        } else if shortcut == &handler_clipboard_shortcut {
                            show_clipboard(app);
                        }
                    })
                    .build(),
            )?;
            app.global_shortcut().register(launcher_shortcut)?;
            app.global_shortcut().register(clipboard_shortcut)?;

            configure_overlay_windows(app)?;
            setup_tray(app.handle())?;
            commands::start_clipboard_watcher();

            if let Some(window) = app.get_webview_window("main") {
                let _ = commands::apply_launcher_window(&window, "launcher");
                commands::show_overlay_window(&window, "launcher")?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 VviTools 失败");
}

#[cfg(target_os = "macos")]
fn configure_overlay_windows(app: &tauri::App) -> tauri::Result<()> {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
    use tauri_nspanel::WebviewWindowExt;

    for label in ["main", "clipboard"] {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        let panel = window.to_panel::<LauncherPanel>()?;
        let ns_window = window.ns_window()?;

        unsafe {
            let ns_window: &NSWindow = &*ns_window.cast();
            let mut behavior = ns_window.collectionBehavior();
            behavior.remove(
                NSWindowCollectionBehavior::FullScreenPrimary
                    | NSWindowCollectionBehavior::FullScreenNone,
            );
            behavior.insert(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
            if objc2::available!(macos = 13.0) {
                behavior.remove(
                    NSWindowCollectionBehavior::Primary | NSWindowCollectionBehavior::Auxiliary,
                );
                behavior.insert(NSWindowCollectionBehavior::CanJoinAllApplications);
            }
            panel.set_style_mask(ns_window.styleMask() | NSWindowStyleMask::NonactivatingPanel);
            ns_window.setCollectionBehavior(behavior);
        }
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn configure_overlay_windows(_app: &tauri::App) -> tauri::Result<()> {
    Ok(())
}

fn reset_launcher_view(window: &WebviewWindow) {
    let _ = window.emit("show-launcher", ());
    let _ = window.eval("window.dispatchEvent(new CustomEvent('vvitools-show-launcher'))");
}

fn show_launcher(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = commands::apply_launcher_window(&window, "launcher");
        reset_launcher_view(&window);
        let _ = commands::show_overlay_window(&window, "launcher");
    }
}

fn show_clipboard(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("clipboard") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            commands::remember_frontmost_app();
            let _ = commands::apply_launcher_window(&window, "clipboard");
            let _ = window.emit("open-clipboard", ());
            let _ = window.eval("window.dispatchEvent(new CustomEvent('vvitools-open-clipboard'))");
            let _ = commands::show_overlay_window(&window, "clipboard");
        }
    }
}

fn toggle_launcher(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            reset_launcher_view(&window);
            let _ = window.hide();
        } else {
            show_launcher(app);
        }
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, TRAY_TOGGLE_ID, "显示/隐藏", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, TRAY_SETTINGS_ID, "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_QUIT_ID, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;
    let icon = tray_icon_image();

    let tray = TrayIconBuilder::with_id("vvitools")
        .tooltip("VviTools")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_TOGGLE_ID => toggle_launcher(app),
            TRAY_SETTINGS_ID => show_settings(app),
            TRAY_QUIT_ID => app.exit(0),
            _ => {}
        })
        .build(app)?;

    app.manage(tray);
    let _ = commands::apply_dock_visibility(app, commands::load_dock_visible_setting());
    Ok(())
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = commands::apply_launcher_window(&window, "feature");
        let _ = window.emit("open-settings", ());
        let _ = window.eval("window.dispatchEvent(new CustomEvent('vvitools-open-settings'))");
        let _ = commands::show_overlay_window(&window, "feature");
    }
}

fn tray_icon_image() -> tauri::image::Image<'static> {
    const SIZE: u32 = 18;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let left = x >= 4 && x <= 6 && y >= 3 && y <= 13;
            let right = x >= 12 && x <= 14 && y >= 3 && y <= 13;
            let bottom = y >= 12 && y <= 15 && x >= 6 && x <= 12;
            let accent = x >= 8 && x <= 10 && y >= 7 && y <= 9;
            let alpha = if left || right || bottom || accent {
                255
            } else {
                0
            };
            rgba.extend_from_slice(&[0, 0, 0, alpha]);
        }
    }
    tauri::image::Image::new_owned(rgba, SIZE, SIZE)
}
