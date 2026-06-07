// Mimic 应用后端入口 — DESIGN 6 / DESIGN 13.1
//
// 本模块注册 Tauri 命令、初始化全局状态，并配置应用启动流程。
// 阶段 8：仅注册 load_config 命令，返回内存中的默认配置。

mod config;
mod state;

use config::AppConfig;
use state::{AppState, DriverStatus, RuntimeStatus, SharedState};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// 加载配置命令 — 阶段 8 返回内存默认配置
///
/// 阶段 9 将实现从 INI 文件加载；当前仅返回 `config::default_config()`。
#[tauri::command]
fn load_config(state: tauri::State<SharedState>) -> Result<AppConfig, String> {
    let app_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    Ok(app_state.config.clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化全局状态 — 阶段 8 使用默认配置
    let initial_state = AppState {
        config: config::default_config(),
        current_page: "home".to_string(),
        runtime_status: RuntimeStatus::Idle,
        driver_status: DriverStatus::NotInstalled,
        stop_flag: Arc::new(AtomicBool::new(false)),
    };

    let shared_state: SharedState = Arc::new(Mutex::new(initial_state));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(shared_state)
        .invoke_handler(tauri::generate_handler![load_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
