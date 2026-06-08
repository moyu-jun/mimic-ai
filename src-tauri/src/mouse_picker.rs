// 鼠标坐标拾取 — DESIGN 11.2 / TASKS 阶段 14
//
// 第一版方案（单显示器 + 标准 DPI）：
//   - 进入拾取时状态置 PickingMouse 并推送 runtime_status_changed
//   - 隐藏主窗口，避免遮挡目标点击区域
//   - 注册 WH_MOUSE_LL low-level mouse hook，仅左键触发（右/中键忽略）
//   - 捕获到左键点击：取消 hook → 恢复窗口 → 状态回 ReadyMouse → 发 mouse_position_picked
//   - hook 注册失败：恢复窗口 + 状态，发 simulation_error
//
// low-level 鼠标钩子由系统在「安装它的线程」上回调，且该线程必须持有消息循环。
// 因此拾取在独立线程内完成：安装 hook → GetMessageW 循环 → 命中后 PostQuitMessage 退出循环。
// 同一时刻只允许一次拾取（由 lib.rs 命令的运行态守卫保证），故用静态原子量在
// C 回调与循环线程间传递坐标是安全的。

use crate::state::{RuntimeStatus, SharedState};
use tauri::{AppHandle, Emitter, Manager};

/// 拾取入口 — 切状态 + 隐藏窗口 + 启动 hook 线程。
///
/// 运行态守卫由调用方（lib.rs 命令）负责，此处假定已处于可拾取状态。
pub fn start_pick_mouse_position(
    app: AppHandle,
    state: SharedState,
    row_id: String,
) -> Result<(), String> {
    log::info!("[mouse_picker] start picking for row {}", row_id);

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

    // 3. 启动 hook 线程
    #[cfg(windows)]
    {
        std::thread::spawn(move || {
            windows_impl::run_picker(app, state, row_id);
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
fn restore_window_and_ready(app: &AppHandle, state: &SharedState) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    if let Ok(mut app_state) = state.lock() {
        app_state.runtime_status = RuntimeStatus::ReadyMouse;
    }
    emit_status(app, RuntimeStatus::ReadyMouse);
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
    use crate::state::SharedState;
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
    use tauri::{AppHandle, Emitter};
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HC_ACTION, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN,
    };

    // C 回调与循环线程间的坐标传递。同一时刻只有一次拾取，静态量安全。
    static PICKED_X: AtomicI32 = AtomicI32::new(0);
    static PICKED_Y: AtomicI32 = AtomicI32::new(0);
    static CAPTURED: AtomicBool = AtomicBool::new(false);

    /// low-level 鼠标钩子回调 — 仅左键按下触发。
    ///
    /// 命中后记录屏幕坐标、置 CAPTURED，并 PostQuitMessage 使循环线程退出。
    /// 点击事件透传（不消费），保持目标窗口行为不变（DESIGN 11.2）。
    unsafe extern "system" fn low_level_mouse_proc(
        code: i32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        if code == HC_ACTION as i32 && wparam == WM_LBUTTONDOWN as usize {
            let info = &*(lparam as *const MSLLHOOKSTRUCT);
            let pt: POINT = info.pt;
            PICKED_X.store(pt.x, Ordering::SeqCst);
            PICKED_Y.store(pt.y, Ordering::SeqCst);
            CAPTURED.store(true, Ordering::SeqCst);
            windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }

    /// 在独立线程内执行：安装 hook → 消息循环 → 取消 hook → 回填结果。
    pub fn run_picker(app: AppHandle, state: SharedState, row_id: String) {
        CAPTURED.store(false, Ordering::SeqCst);

        unsafe {
            let hmodule = GetModuleHandleW(std::ptr::null());
            let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), hmodule, 0);
            if hook.is_null() {
                restore_with_error(&app, &state, "SetWindowsHookExW failed");
                return;
            }

            // 消息循环：hook 回调 PostQuitMessage 后 GetMessageW 返回 0，循环退出。
            let mut msg: MSG = std::mem::zeroed();
            loop {
                let ret = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                if ret <= 0 {
                    // 0 = WM_QUIT（正常命中退出）；-1 = 错误
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            UnhookWindowsHookEx(hook);
        }

        if CAPTURED.load(Ordering::SeqCst) {
            let x = PICKED_X.load(Ordering::SeqCst);
            let y = PICKED_Y.load(Ordering::SeqCst);
            log::info!(
                "[mouse_picker] picked ({}, {}) for row {}",
                x,
                y,
                row_id
            );
            restore_window_and_ready(&app, &state);
            if let Err(e) = app.emit(
                "mouse_position_picked",
                serde_json::json!({ "rowId": row_id, "x": x, "y": y }),
            ) {
                log::error!("[mouse_picker] failed to emit mouse_position_picked: {}", e);
            }
        } else {
            // 循环异常退出（GetMessageW 返回 -1）且未捕获坐标
            restore_with_error(&app, &state, "message loop exited without capture");
        }
    }
}
