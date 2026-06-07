// Mimic 应用后端入口 — DESIGN 6 / DESIGN 13.1
//
// 本模块注册 Tauri 命令、初始化全局状态，并配置应用启动流程。
// 阶段 9：
//   - 初始化移入 setup 钩子（TASKS 9.2 / DESIGN 13.1 顺序基础）
//   - 使用 load_or_init_graceful()：写盘失败不 panic，降级为内存默认配置
//   - 新增 get_init_warning 命令：前端首页读取启动警告

mod config;
mod state;

use config::AppConfig;
use state::{AppState, DriverStatus, RuntimeStatus, SharedState};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// 加载配置命令 — 返回内存中的当前配置
#[tauri::command]
fn load_config(state: tauri::State<SharedState>) -> Result<AppConfig, String> {
    let app_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    Ok(app_state.config.clone())
}

/// 保存配置命令 — 先落盘成功，再更新内存
///
/// TODO(阶段 12): 在此处添加运行态守卫 — DESIGN 6.1
#[tauri::command]
fn save_config(config: AppConfig, state: tauri::State<SharedState>) -> Result<(), String> {
    // 先持久化，失败时内存状态不变
    config::save(&config)?;

    // 写盘成功后才更新内存
    let mut app_state = state.lock().map_err(|e| format!("Failed to lock state: {}", e))?;
    app_state.config = config;

    Ok(())
}

/// 读取启动时配置写盘失败的警告 — 阶段 9
///
/// 首页 onMounted 调用此命令；返回 None 表示无问题，不显示提示。
/// 返回 Some(msg) 时首页展示小字警告。
#[tauri::command]
fn get_init_warning(state: tauri::State<SharedState>) -> Option<String> {
    state.lock().ok()?.config_warning.clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // DESIGN 13.1 启动顺序：阶段 9 仅含配置加载
            // 阶段 10 追加：日志初始化、权限检测
            // 阶段 12 追加：驱动检测、热键注册
            let (loaded_config, config_warning) = config::load_or_init_graceful();

            let initial_state = AppState {
                config: loaded_config,
                config_warning,
                current_page: "home".to_string(),
                runtime_status: RuntimeStatus::Idle,
                driver_status: DriverStatus::NotInstalled,
                stop_flag: Arc::new(AtomicBool::new(false)),
            };

            let shared_state: SharedState = Arc::new(Mutex::new(initial_state));
            app.manage(shared_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![load_config, save_config, get_init_warning])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
