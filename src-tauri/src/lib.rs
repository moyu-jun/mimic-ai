// Mimic 应用后端入口 — DESIGN 6 / DESIGN 13.1
//
// 阶段 10：
//   - 接入 tauri-plugin-log（开发 info / release error，targets: Stdout + LogDir）
//   - setup 顺序按 DESIGN 13.1 微调：日志先于配置加载，便于后者出错时被记录
//   - 新增 admin 模块与命令：get_admin_status / request_admin_restart
//   - 关键启动事件改用 log::{info,error,warn}

mod admin;
mod config;
mod driver;
mod hotkeys;
mod hotkeys_interception;
mod state;

use config::AppConfig;
use hotkeys::HotkeyUpdateResult;
use state::{AppState, RuntimeStatus, SendInterception, SharedState};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_log::{Target, TargetKind};

/// 加载配置命令 — 返回内存中的当前配置
#[tauri::command]
fn load_config(state: tauri::State<SharedState>) -> Result<AppConfig, String> {
    let app_state = state
        .inner()
        .lock()
        .map_err(|e| format!("Failed to lock state: {}", e))?;
    Ok(app_state.config.clone())
}

/// 保存配置命令 — 先落盘成功,再更新内存
///
/// 阶段 12: 增加运行态守卫 — DESIGN 6.1
#[tauri::command]
fn save_config(config: AppConfig, state: tauri::State<SharedState>) -> Result<(), String> {
    // 运行态守卫 — DESIGN 6.1
    {
        let app_state = state
            .inner()
            .lock()
            .map_err(|e| format!("Failed to lock state: {}", e))?;
        match app_state.runtime_status {
            RuntimeStatus::RunningKeyboard
            | RuntimeStatus::RunningMouse
            | RuntimeStatus::PickingMouse => {
                return Err("busy: simulation running".to_string());
            }
            _ => {}
        }
    }

    // 先持久化，失败时内存状态不变
    config::save(&config).map_err(|e| {
        log::error!("[save_config] persist failed: {}", e);
        e
    })?;

    // 写盘成功后才更新内存
    let mut app_state = state
        .inner()
        .lock()
        .map_err(|e| format!("Failed to lock state: {}", e))?;
    app_state.config = config;

    Ok(())
}

/// 读取启动时配置写盘失败的警告 — 阶段 9
#[tauri::command]
fn get_init_warning(state: tauri::State<SharedState>) -> Option<String> {
    state.inner().lock().ok()?.config_warning.clone()
}

/// 当前进程是否以管理员身份运行 — DESIGN 14.1 / 阶段 10
///
/// 失败一律视为非管理员（admin 模块内部已记录 warn 日志）。
// ADMIN_POLICY: Runtime detection only — no requireAdministrator manifest entry.
#[tauri::command]
fn get_admin_status() -> bool {
    admin::is_admin()
}

/// 检测 Interception 驱动状态 — DESIGN 12.2 / 阶段 11
#[tauri::command]
fn check_driver_status(state: tauri::State<SharedState>) -> Result<String, String> {
    let app_state = state
        .inner()
        .lock()
        .map_err(|e| format!("Failed to lock state: {}", e))?;
    Ok(serde_json::to_string(&app_state.driver_status)
        .unwrap_or_else(|_| "\"NotInstalled\"".to_string()))
}

/// 安装 Interception 驱动 — DESIGN 12.3 / 阶段 11
///
/// 通过 ShellExecuteW("runas") 以管理员身份调用外置安装器。
/// 成功调度后返回 Ok，调用方应重新调 check_driver_status 刷新。
///
/// 前置条件：必须以管理员权限运行（否则返回 Err 提示用户重启）。
#[tauri::command]
fn install_interception_driver(state: tauri::State<SharedState>) -> Result<(), String> {
    // 权限守卫 — 驱动安装必须管理员权限（阶段 11 遗漏修复）
    if !admin::is_admin() {
        log::warn!("[install_driver] rejected: not running as admin");
        return Err("permission_denied".to_string());
    }

    // 运行态守卫 — DESIGN 6.1
    {
        let app_state = state
            .inner()
            .lock()
            .map_err(|e| format!("Failed to lock state: {}", e))?;
        match app_state.runtime_status {
            RuntimeStatus::RunningKeyboard
            | RuntimeStatus::RunningMouse
            | RuntimeStatus::PickingMouse => {
                return Err("busy: simulation running".to_string());
            }
            _ => {}
        }
    }

    driver::install_driver()?;

    // 安装后重新检测并更新 state
    let new_status = driver::check_interception_driver();
    if let Ok(mut app_state) = state.inner().lock() {
        app_state.driver_status = new_status;
    }

    Ok(())
}

/// 重启系统 — 驱动安装后需重启加载（阶段 11 优化）
///
/// 需管理员权限；非管理员返回 `permission_denied`。
#[tauri::command]
fn reboot_system() -> Result<(), String> {
    if !admin::is_admin() {
        log::warn!("[reboot] rejected: not running as admin");
        return Err("permission_denied".to_string());
    }
    log::info!("[reboot] user requested system reboot");
    driver::reboot_system()
}

/// 设置当前页面 — 阶段 12
///
/// 后端记录当前页面，用于判断热键是否可触发（REQUIREMENTS 3.6）。
#[tauri::command]
fn set_current_page(page: String, state: tauri::State<SharedState>) -> Result<(), String> {
    // 运行态守卫 — DESIGN 6.1
    {
        let app_state = state
            .lock()
            .map_err(|e| format!("Failed to lock state: {}", e))?;
        match app_state.runtime_status {
            RuntimeStatus::RunningKeyboard
            | RuntimeStatus::RunningMouse
            | RuntimeStatus::PickingMouse => {
                return Err("busy: simulation running".to_string());
            }
            _ => {}
        }
    }

    let mut app_state = state
        .lock()
        .map_err(|e| format!("Failed to lock state: {}", e))?;
    app_state.current_page = page.clone();
    log::info!("[set_current_page] page changed to: {}", page);
    Ok(())
}

/// 更新热键配置 — 阶段 13 / DESIGN 6.2
///
/// 流程：对比变化 → 持久化 → 更新内存。
/// Interception 热键由后台监听线程统一处理，不需要注册/注销。
/// 返回结构化结果供前端分别提示持久化成功/失败。
#[tauri::command]
fn update_hotkeys(
    hotkeys: config::HotkeyConfig,
    state: tauri::State<SharedState>,
) -> Result<HotkeyUpdateResult, String> {
    // 运行态守卫 — DESIGN 6.1
    {
        let app_state = state
            .lock()
            .map_err(|e| format!("Failed to lock state: {}", e))?;
        match app_state.runtime_status {
            RuntimeStatus::RunningKeyboard
            | RuntimeStatus::RunningMouse
            | RuntimeStatus::PickingMouse => {
                return Err("busy: simulation running".to_string());
            }
            _ => {}
        }
    }

    hotkeys::update_hotkeys(&state, hotkeys)
}

/// 停止模拟 — 阶段 12（仅切换状态）
///
/// 当前阶段仅将状态从 Running* 切回 Idle,不涉及真实 worker 停止（阶段 13 接入）。
#[tauri::command]
fn stop_simulation(state: tauri::State<SharedState>, app: tauri::AppHandle) -> Result<(), String> {
    let new_status = {
        let mut app_state = state
            .lock()
            .map_err(|e| format!("Failed to lock state: {}", e))?;

        match app_state.runtime_status {
            RuntimeStatus::RunningKeyboard | RuntimeStatus::RunningMouse => {
                app_state.runtime_status = RuntimeStatus::Idle;
                RuntimeStatus::Idle
            }
            _ => {
                return Err("Not running".to_string());
            }
        }
    };

    // 发送 runtime_status_changed 事件
    if let Err(e) = app.emit(
        "runtime_status_changed",
        serde_json::json!({ "status": new_status }),
    ) {
        log::error!("[stop_simulation] failed to emit event: {}", e);
    }

    log::info!("[stop_simulation] simulation stopped");
    Ok(())
}

/// 获取当前运行状态 — 阶段 12
#[tauri::command]
fn get_runtime_status(state: tauri::State<SharedState>) -> Result<RuntimeStatus, String> {
    let app_state = state
        .lock()
        .map_err(|e| format!("Failed to lock state: {}", e))?;
    Ok(app_state.runtime_status.clone())
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
    // ADMIN_POLICY: 启动时主动请求 UAC 提权（REQUIREMENTS 2 / DESIGN 14.1）
    // 若用户同意 → 当前进程退出，由提权后的新进程接管；
    // 若用户拒绝 → 降级启动，首页显示权限受限 + 重启按钮。
    //
    // 注意：此处尚在 tauri::Builder 之前，tauri-plugin-log 尚未装配，
    // log::* 宏会被丢弃 — 用 eprintln! 走 stderr，dev 模式控制台可见。
    // UAC 弹窗本身就是用户可见的反馈，日志主要服务于事后排查。
    if !admin::is_admin() {
        eprintln!("[startup] not elevated, requesting UAC");
        match admin::restart_as_admin() {
            Ok(_) => {
                // UAC 调度成功，新进程已启动，当前进程直接退出
                eprintln!("[startup] elevated process launched, exiting current process");
                std::process::exit(0);
            }
            Err(e) => {
                // 用户拒绝 UAC 或调度失败，继续以普通权限启动
                eprintln!(
                    "[startup] UAC declined or failed ({}), starting with limited privileges",
                    e
                );
            }
        }
    } else {
        eprintln!("[startup] already elevated");
    }

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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            log::info!(
                "[setup] Mimic starting, version {}",
                env!("CARGO_PKG_VERSION")
            );

            // 配置加载（路径 + 结果均记录日志）
            match config::config_path() {
                Ok(p) => log::info!("[setup] config path: {}", p.display()),
                Err(e) => log::error!("[setup] resolve config path failed: {}", e),
            }
            let (loaded_config, config_warning) = config::load_or_init_graceful();
            if let Some(w) = &config_warning {
                log::error!(
                    "[setup] config write failed, falling back to in-memory default: {}",
                    w
                );
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
            log::info!(
                "[setup] admin status: {}",
                if admin { "elevated" } else { "limited" }
            );

            // 驱动检测 — DESIGN 13.1 步骤 3 / 阶段 11
            let driver_status = driver::check_interception_driver();
            log::info!("[setup] driver status: {:?}", driver_status);

            // 初始化 Interception 上下文 — DESIGN 8.3 / 阶段 13
            let interception_ctx = if matches!(&driver_status, state::DriverStatus::Ready) {
                match interception::Interception::new() {
                    Some(ctx) => {
                        log::info!("[setup] Interception context created");
                        Arc::new(Mutex::new(Some(SendInterception(ctx))))
                    }
                    None => {
                        log::error!("[setup] Interception context creation failed");
                        Arc::new(Mutex::new(None))
                    }
                }
            } else {
                log::warn!("[setup] Interception not ready, context not created");
                Arc::new(Mutex::new(None))
            };

            // 启动 Interception 热键监听线程 — DESIGN 8.3 / 阶段 13
            let shared_state: SharedState = Arc::new(Mutex::new(AppState {
                config: loaded_config,
                config_warning,
                current_page: "home".to_string(),
                runtime_status: RuntimeStatus::Idle,
                driver_status: driver_status.clone(),
                stop_flag: Arc::new(AtomicBool::new(false)),
                interception_context: interception_ctx.clone(),
            }));

            if matches!(&driver_status, state::DriverStatus::Ready) {
                if let Err(e) = hotkeys_interception::start_hotkey_listener(
                    app.handle().clone(),
                    shared_state.clone(),
                    interception_ctx.clone(),
                ) {
                    log::error!("[setup] Interception hotkey listener failed: {}", e);
                } else {
                    log::info!("[setup] Interception hotkey listener started");
                }
            } else {
                log::warn!("[setup] Interception not ready, hotkeys disabled");
            }

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
            check_driver_status,
            install_interception_driver,
            reboot_system,
            set_current_page,
            update_hotkeys,
            stop_simulation,
            get_runtime_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
