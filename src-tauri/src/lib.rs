mod capture;
mod clipboard;
mod commands;
mod config;
mod error;
mod history;
mod hotkey;
mod ocr;
mod translate;
mod tray;
mod window;

use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout))
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview))
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: None }))
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .manage(Mutex::new(window::PinImageStore::default()))
        .manage(Mutex::new(window::CachedScreenStore::default()))
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::write_clipboard_image,
            commands::read_clipboard_image,
            commands::write_clipboard_text,
            commands::close_pin_window,
            commands::get_pin_image,
            commands::capture_region_from_cache,
            commands::get_overlay_image,
            commands::store_pin_image,
            commands::translate_image,
            commands::get_api_key,
            commands::set_api_key,
            commands::test_api_connection
        ])
        .setup(|app| {
            let config_manager = config::ConfigManager::new(app.handle())?;
            let app_config = config_manager.load()?;

            // 初始化并缓存截图服务
            let capture_service = capture::CaptureService::new()
                .map_err(|e| {
                    log::error!("初始化截图服务失败: {}", e);
                    e
                })?;
            app.manage(std::sync::Mutex::new(capture_service));

            tray::create_tray(app.handle(), &app_config.shortcuts)?;

            #[cfg(desktop)]
            hotkey::register_hotkeys(app.handle(), &app_config.shortcuts)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
