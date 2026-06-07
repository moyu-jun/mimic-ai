// Mimic 应用后端入口 — DESIGN 6 / DESIGN 13.1
//
// 本模块注册 Tauri 命令、初始化全局状态，并配置应用启动流程。
// 阶段 9：实现 INI 持久化 — load_or_init + save_config

mod config;
mod state;

use config::AppConfig;
use state::{AppState, DriverStatus, RuntimeStatus, SharedState};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// 加载配置命令 — 阶段 9 从 INI 文件加载
///
/// 返回当前内存中的配置（由 setup 钩子从 INI 加载）。
#[tauri::command]
fn load_config(state: tauri::State<SharedState>) -> Result<AppConfig, String> {
    let app_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    Ok(app_state.config.clone())
}

/// 保存配置命令 — 阶段 9
///
/// 将配置写入 INI 文件并更新内存状态。
#[tauri::command]
fn save_config(config: AppConfig, state: tauri::State<SharedState>) -> Result<(), String> {
    // 持久化到 INI
    config::save(&config)?;

    // 更新内存状态
    let mut app_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    app_state.config = config;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 阶段 9：从 INI 加载或初始化配置
    let loaded_config = config::load_or_init().expect("Failed to load or initialize config");

    // 初始化全局状态
    let initial_state = AppState {
        config: loaded_config,
        current_page: "home".to_string(),
        runtime_status: RuntimeStatus::Idle,
        driver_status: DriverStatus::NotInstalled,
        stop_flag: Arc::new(AtomicBool::new(false)),
    };

    let shared_state: SharedState = Arc::new(Mutex::new(initial_state));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(shared_state)
        .invoke_handler(tauri::generate_handler![load_config, save_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
