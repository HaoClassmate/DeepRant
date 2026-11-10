use crate::store::initialize_settings;
use tauri::Manager;
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// 开机自启时附带的启动参数，用来区分「开机自动启动」和「用户手动打开」
const AUTOSTART_ARG: &str = "--autostart";
pub mod ai_translator;
pub mod screen_ocr;
pub mod shell_helper;
pub mod shortcut;
pub mod store;
pub mod tray;

#[tauri::command]
fn log_to_backend(message: String) {
    println!("Frontend Log: {}", message);
}

#[tauri::command]
fn get_version(app_handle: tauri::AppHandle) -> String {
    app_handle.package_info().version.to_string()
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn update_translator_shortcut(
    app_handle: tauri::AppHandle,
    keys: Vec<String>,
) -> Result<(), String> {
    shortcut::update_translator_shortcut(&app_handle, keys)
}

#[tauri::command]
async fn update_screen_shortcut(
    app_handle: tauri::AppHandle,
    which: String,
    keys: Vec<String>,
) -> Result<(), String> {
    shortcut::update_screen_shortcut(&app_handle, &which, keys)
}

#[tauri::command]
async fn get_settings(app_handle: tauri::AppHandle) -> Result<store::AppSettings, String> {
    store::get_settings(&app_handle).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_autostart(app_handle: tauri::AppHandle) -> Result<bool, String> {
    app_handle.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app_handle: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let launcher = app_handle.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| format!("设置开机自启动失败: {}", e))?;
    launcher.is_enabled().map_err(|e| e.to_string())
}

/// 这次是不是开机自启动起来的（主窗口隐藏在托盘）
#[tauri::command]
fn launched_by_autostart() -> bool {
    std::env::args().any(|a| a == AUTOSTART_ARG)
}

pub fn run() {
    println!("Starting application...");

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        // 剪贴板插件
        .plugin(tauri_plugin_clipboard_manager::init())
        // opener插件
        .plugin(tauri_plugin_opener::init())
        // 开机自启动插件
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .setup(|app| {
            // 初始化存储
            println!("Initializing...");
            match initialize_settings(&app.app_handle()) {
                Ok(_) => println!("应用设置初始化完成"),
                Err(e) => eprintln!("初始化设置失败: {}", e),
            }

            // 初始化所有快捷键
            println!("正在注册全局快捷键...");
            match shortcut::init_shortcuts(&app.app_handle()) {
                Ok(_) => println!("快捷键设置成功"),
                Err(e) => eprintln!("注册全局快捷键失败: {}", e),
            }

            // 创建AI模型托盘
            match tray::create_tray(&app.app_handle()) {
                Ok(_) => println!("托盘创建成功"),
                Err(e) => eprintln!("创建托盘失败: {}", e),
            }

            // 开机自启时不弹出主窗口，只留在托盘，快捷键照常可用
            if launched_by_autostart() {
                println!("开机自启动，主窗口隐藏到托盘");
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
                #[cfg(target_os = "macos")]
                let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            update_translator_shortcut,
            update_screen_shortcut,
            log_to_backend,
            get_settings,
            get_version,
            get_autostart,
            set_autostart,
            launched_by_autostart,
            screen_ocr::ocr_select_region,
            screen_ocr::ocr_translate_now,
            screen_ocr::ocr_region_selected,
            screen_ocr::ocr_cancel_select,
            screen_ocr::ocr_last_overlay,
            screen_ocr::ocr_test_push
        ]);

    // 只在非Windows系统上添加窗口事件监听
    #[cfg(not(target_os = "windows"))]
    {
        builder = builder.on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                #[cfg(target_os = "macos")]
                let _ = window
                    .app_handle()
                    .set_activation_policy(tauri::ActivationPolicy::Accessory);
                api.prevent_close();
            }
            _ => {}
        });
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
