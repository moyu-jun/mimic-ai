// Mimic 应用后端入口 — DESIGN 6 / DESIGN 13.1
//
// 阶段 10：
//   - 接入 tauri-plugin-log（开发 info / release error，targets: Stdout + LogDir）
//   - setup 顺序按 DESIGN 13.1 微调：日志先于配置加载，便于后者出错时被记录
//   - 新增 admin 模块与命令：get_admin_status / request_admin_restart
//   - 关键启动事件改用 log::{info,error,warn}

mod admin;
mod config;
mod state;

use config::AppConfig;
use state::{AppState, DriverStatus, RuntimeStatus, SharedState};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

/// 加载配置命令 — 返回内存中的当前配置
#[tauri::command]
fn load_config(state: tauri::State<SharedState>) -> Result<AppConfig, String> {
    let app_state = state
        .lock()
        .map_err(|e| format!("Failed to lock state: {}", e))?;
    Ok(app_state.config.clone())
}

/// 保存配置命令 — 先落盘成功，再更新内存
///
/// TODO(阶段 12): 在此处添加运行态守卫 — DESIGN 6.1
#[tauri::command]
fn save_config(config: AppConfig, state: tauri::State<SharedState>) -> Result<(), String> {
    // 先持久化，失败时内存状态不变
    config::save(&config).map_err(|e| {
        log::error!("[save_config] persist failed: {}", e);
        e
    })?;

    // 写盘成功后才更新内存
    let mut app_state = state
        .lock()
        .map_err(|e| format!("Failed to lock state: {}", e))?;
    app_state.config = config;

    Ok(())
}

/// 读取启动时配置写盘失败的警告 — 阶段 9
#[tauri::command]
fn get_init_warning(state: tauri::State<SharedState>) -> Option<String> {
    state.lock().ok()?.config_warning.clone()
}

/// 当前进程是否以管理员身份运行 — DESIGN 14.1 / 阶段 10
///
/// 失败一律视为非管理员（admin 模块内部已记录 warn 日志）。
// ADMIN_POLICY: Runtime detection only — no requireAdministrator manifest entry.
#[tauri::command]
fn get_admin_status() -> bool {
    admin::is_admin()
}

/// 以管理员身份重启自身 — DESIGN 14.1 / 阶段 10
///
/// 触发 UAC 提示；用户取消或 ShellExecuteW 失败时返回 Err 字符串。
/// 成功调度后由前端立即调用 `app.exit()` 关闭当前进程，避免双开。
// ADMIN_POLICY: 通过 ShellExecuteW("runas") 触发用户级 UAC,无静默提权。
#[tauri::command]
fn request_admin_restart(app: tauri::AppHandle) -> Result<(), String> {
    log::info!("[admin] user requested elevation restart");
    admin::restart_as_admin().map_err(|e| {
        log::error!("[admin] restart_as_admin failed: {}", e);
        e
    })?;
    // 调度成功后给前端 200ms 让其完成 UI 反馈，再退出当前进程
    let app_handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        app_handle.exit(0);
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // DESIGN 13.1 启动顺序（阶段 10 当前覆盖 1-2 + 权限检测）：
    //   1. 初始化日志   ← 由 plugin builder 在 setup 之前装配
    //   2. 加载/初始化 mimic.ini
    //   3. 检测驱动状态     ← 阶段 11 接入
    //   4. 注册全局热键     ← 阶段 12 接入
    //   5. 写入 SharedState
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Error
    };

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log_level)
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            log::info!("[setup] Mimic starting, version {}", env!("CARGO_PKG_VERSION"));

            // 配置加载（路径 + 结果均记录日志）
            match config::config_path() {
                Ok(p) => log::info!("[setup] config path: {}", p.display()),
                Err(e) => log::error!("[setup] resolve config path failed: {}", e),
            }
            let (loaded_config, config_warning) = config::load_or_init_graceful();
            if let Some(w) = &config_warning {
                log::error!("[setup] config write failed, falling back to in-memory default: {}", w);
            } else {
                log::info!(
                    "[setup] config loaded: {} keyboard / {} mouse actions, hotkeys {} / {}",
                    loaded_config.keyboard_actions.len(),
                    loaded_config.mouse_actions.len(),
                    loaded_config.hotkeys.start.key_label,
                    loaded_config.hotkeys.stop.key_label,
                );
            }

            // 权限检测仅记录日志,不阻断启动 — DESIGN 14.1 降级启动
            // ADMIN_POLICY: Detect at startup, render result on home page, never block launch.
            let admin = admin::is_admin();
            log::info!("[setup] admin status: {}", if admin { "elevated" } else { "limited" });

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

            log::info!("[setup] ready");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            get_init_warning,
            get_admin_status,
            request_admin_restart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
