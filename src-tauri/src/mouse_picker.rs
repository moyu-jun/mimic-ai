// 鼠标坐标拾取 — DESIGN 11.2 / TASKS 阶段 14
//
// 拾取机制（2026-06-10 改用 Interception）：
//   - 原方案用 WH_MOUSE_LL 用户态 hook，独占全屏游戏（DirectX exclusive fullscreen）
//     直接从驱动层取输入，绕过用户态 hook，导致拾取在全屏游戏内失效。
//   - 新方案复用 Interception 内核驱动监听鼠标：在驱动层捕获左键，全屏游戏同样有效，
//     且普通权限即可（驱动已工作在内核态，不需要调用方提权）。
//
// 流程：
//   - 进入拾取时状态置 PickingMouse 并推送 runtime_status_changed
//   - 隐藏主窗口，避免遮挡目标点击区域
//   - 在 worker context 上设置鼠标 filter（仅 LEFT_BUTTON_DOWN），循环 wait_with_timeout
//   - 捕获到左键：用 GetCursorPos 读屏幕坐标 → 透传点击 → 清除 filter
//     → 恢复窗口 → 状态回 ReadyMouse → 发 mouse_position_picked
//   - context 不可用或异常：清除 filter（best-effort）+ 恢复窗口 + 发 simulation_error
//
// 设备/坐标说明：
//   - Interception 鼠标 stroke 的 x/y 是相对/绝对移动量，不是屏幕坐标，
//     故命中后用 GetCursorPos 读取系统光标位置作为拾取结果。
//   - 拾取期间状态为 PickingMouse，mouse_worker / keyboard_worker 的状态门控
//     使其不发送，故拾取线程独占 worker context 设置 filter 是安全的。

use crate::state::{RuntimeStatus, SharedState};
use tauri::{AppHandle, Emitter, Manager};

/// 拾取入口 — 切状态 + 隐藏窗口 + 启动 Interception 监听线程。
///
/// 运行态守卫由调用方（lib.rs 命令）负责，此处假定已处于可拾取状态。
pub fn start_pick_mouse_position(
    app: AppHandle,
    state: SharedState,
    row_id: String,
) -> Result<(), String> {
    log::info!("[mouse_picker] start picking for row {}", row_id);

    // 拾取需要 worker context（Interception 监听）。提前取出，None 直接报错返回。
    #[cfg(windows)]
    let worker_ctx = {
        let app_state = state
            .lock()
            .map_err(|e| format!("Failed to lock state: {}", e))?;
        app_state.interception_worker.clone()
    };

    // 1. 状态置 PickingMouse
    {
        let mut app_state = state
            .lock()
            .map_err(|e| format!("Failed to lock state: {}", e))?;
        app_state.runtime_status = RuntimeStatus::PickingMouse;
    }
    emit_status(&app, RuntimeStatus::PickingMouse);

    // 2. 隐藏主窗口（best-effort：拿不到窗口时记录但不中断拾取）
    if let Some(win) = app.get_webview_window("main") {
        if let Err(e) = win.hide() {
            log::warn!("[mouse_picker] failed to hide window: {}", e);
        }
    } else {
        log::warn!("[mouse_picker] main window not found, picking without hiding");
    }

    // 3. 启动 Interception 监听线程
    #[cfg(windows)]
    {
        std::thread::spawn(move || {
            windows_impl::run_picker(app, state, row_id, worker_ctx);
        });
    }

    // 非 Windows 平台不可能进入真实运行环境，直接恢复以免界面卡在 PickingMouse。
    #[cfg(not(windows))]
    {
        restore_with_error(&app, &state, "mouse picking is only supported on Windows");
        let _ = row_id;
    }

    Ok(())
}

/// 发送 runtime_status_changed 事件。
fn emit_status(app: &AppHandle, status: RuntimeStatus) {
    if let Err(e) = app.emit(
        "runtime_status_changed",
        serde_json::json!({ "status": status }),
    ) {
        log::error!("[mouse_picker] failed to emit runtime_status_changed: {}", e);
    }
}

/// 恢复窗口 + 状态回 ReadyMouse（拾取成功路径调用）。
///
/// 窗口显示/聚焦操作 marshal 到主线程执行：Windows 窗口有线程亲和性，
/// 从拾取后台线程直接调用 show()/set_focus() 在隐藏后不可靠（前台锁定限制）。
fn restore_window_and_ready(app: &AppHandle, state: &SharedState) {
    restore_window_on_main(app);
    if let Ok(mut app_state) = state.lock() {
        app_state.runtime_status = RuntimeStatus::ReadyMouse;
    }
    emit_status(app, RuntimeStatus::ReadyMouse);
}

/// 在主线程恢复并聚焦主窗口（show + unminimize + set_focus）。
fn restore_window_on_main(app: &AppHandle) {
    let app_clone = app.clone();
    let dispatched = app.run_on_main_thread(move || {
        match app_clone.get_webview_window("main") {
            Some(win) => {
                if let Err(e) = win.unminimize() {
                    log::warn!("[mouse_picker] unminimize failed: {}", e);
                }
                if let Err(e) = win.show() {
                    log::error!("[mouse_picker] window show failed: {}", e);
                }
                if let Err(e) = win.set_focus() {
                    log::warn!("[mouse_picker] set_focus failed: {}", e);
                }
                log::info!("[mouse_picker] window restored on main thread");
            }
            None => {
                log::error!("[mouse_picker] main window not found during restore");
            }
        }
    });
    if let Err(e) = dispatched {
        log::error!("[mouse_picker] run_on_main_thread dispatch failed: {}", e);
    }
}

/// 恢复窗口 + 状态回 ReadyMouse + 发 simulation_error（异常路径调用）。
#[cfg_attr(not(windows), allow(dead_code))]
fn restore_with_error(app: &AppHandle, state: &SharedState, error: &str) {
    log::error!("[mouse_picker] picking failed: {}", error);
    restore_window_and_ready(app, state);
    if let Err(e) = app.emit("simulation_error", serde_json::json!({ "error": error })) {
        log::error!("[mouse_picker] failed to emit simulation_error: {}", e);
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::{restore_window_and_ready, restore_with_error};
    use crate::state::{SendInterception, SharedState};
    use interception::{Filter, MouseFilter, MouseState, Stroke};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tauri::{AppHandle, Emitter};
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    /// 匹配所有鼠标设备的 predicate。
    extern "C" fn is_mouse_device(device: i32) -> bool {
        interception::is_mouse(device)
    }

    /// 在独立线程内执行：设置鼠标 filter → 等待左键按下 → 读坐标 → 清 filter → 回填。
    ///
    /// 复用 worker context（Interception 内核驱动），在驱动层捕获左键，
    /// 独占全屏游戏内同样有效，且不需要管理员权限。
    pub fn run_picker(
        app: AppHandle,
        state: SharedState,
        row_id: String,
        ctx: Arc<Mutex<Option<SendInterception>>>,
    ) {
        // 全程持有 context 锁：拾取期间状态为 PickingMouse，worker 不发送，独占安全。
        let ctx_guard = match ctx.lock() {
            Ok(g) => g,
            Err(e) => {
                restore_with_error(&app, &state, &format!("failed to lock context: {}", e));
                return;
            }
        };

        let interception = match ctx_guard.as_ref() {
            Some(i) => &i.0,
            None => {
                restore_with_error(
                    &app,
                    &state,
                    "Interception context not available (driver not ready)",
                );
                return;
            }
        };

        // 设置鼠标 filter：仅监听左键按下事件。
        interception.set_filter(
            is_mouse_device,
            Filter::MouseFilter(MouseFilter::LEFT_BUTTON_DOWN),
        );
        log::info!("[mouse_picker] mouse filter set, waiting for left click");

        // 循环等待左键按下；wait_with_timeout 返回 0 = 超时，>0 = 命中设备号。
        let mut captured: Option<(i32, i32)> = None;
        let mut error: Option<String> = None;

        loop {
            let device = interception.wait_with_timeout(Duration::from_millis(100));
            if device == 0 {
                // 超时，继续等待（用户尚未点击）。
                continue;
            }
            if !interception::is_mouse(device) {
                continue;
            }

            let mut strokes = [Stroke::Mouse {
                state: MouseState::empty(),
                flags: interception::MouseFlags::empty(),
                rolling: 0,
                x: 0,
                y: 0,
                information: 0,
            }; 16];

            let count = interception.receive(device, &mut strokes);
            if count == 0 {
                continue;
            }

            let mut hit = false;
            for stroke in strokes.iter().take(count as usize) {
                // 透传事件，保持目标窗口点击行为不变。
                interception.send(device, &[*stroke]);
                if let Stroke::Mouse { state: ms, .. } = stroke {
                    if ms.contains(MouseState::LEFT_BUTTON_DOWN) {
                        hit = true;
                    }
                }
            }

            if hit {
                // 用 GetCursorPos 读屏幕坐标（Interception stroke 不含屏幕坐标）。
                let mut pt = POINT { x: 0, y: 0 };
                let ok = unsafe { GetCursorPos(&mut pt) };
                if ok != 0 {
                    captured = Some((pt.x, pt.y));
                } else {
                    error = Some("GetCursorPos failed".to_string());
                }
                break;
            }
        }

        // 清除 filter，恢复 worker context 正常状态（关键：否则会持续拦截鼠标）。
        interception.set_filter(is_mouse_device, Filter::MouseFilter(MouseFilter::empty()));
        log::info!("[mouse_picker] mouse filter cleared");
        drop(ctx_guard);

        match captured {
            Some((x, y)) => {
                log::info!("[mouse_picker] picked ({}, {}) for row {}", x, y, row_id);
                restore_window_and_ready(&app, &state);
                if let Err(e) = app.emit(
                    "mouse_position_picked",
                    serde_json::json!({ "rowId": row_id, "x": x, "y": y }),
                ) {
                    log::error!("[mouse_picker] failed to emit mouse_position_picked: {}", e);
                }
            }
            None => {
                restore_with_error(
                    &app,
                    &state,
                    error.as_deref().unwrap_or("picking ended without capture"),
                );
            }
        }
    }
}
