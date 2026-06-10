// Interception 驱动热键监听 — DESIGN 8.3
//
// 基于 Interception 驱动实现全局热键监听，支持所有按键包括修饰键（Ctrl/Alt/Shift）。
// 替代 tauri-plugin-global-shortcut 以突破 Windows RegisterHotKey API 限制。

use crate::state::{RuntimeStatus, SendInterception, SharedState};
use interception::{KeyState, ScanCode, Stroke};
use log::{error, info};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

/// 启动热键监听线程 — DESIGN 8.3
///
/// 长生命周期线程：循环调用 wait() → receive() → 匹配热键 → 处理 → send()（或阻断）。
/// 热键配置从 AppState.config.hotkeys 动态读取，无需重启监听线程。
/// 实现状态机门控（Idle → Running*）与页面过滤（keyboard/mouse）。
pub fn start_hotkey_listener(
    app: AppHandle,
    state: SharedState,
    ctx: Arc<Mutex<Option<SendInterception>>>,
) -> Result<(), String> {
    std::thread::spawn(move || {
        info!("[hotkeys_interception] listener thread started");

        // 设置键盘事件过滤器（仅一次，在循环外）
        let filter_set = {
            let ctx_guard = match ctx.lock() {
                Ok(g) => g,
                Err(e) => {
                    error!("[hotkeys_interception] failed to lock context for filter: {}", e);
                    return;
                }
            };

            match ctx_guard.as_ref() {
                Some(i) => {
                    use interception::{Filter, KeyFilter};
                    // Predicate: 匹配所有键盘设备
                    extern "C" fn is_keyboard_device(device: i32) -> bool {
                        interception::is_keyboard(device)
                    }
                    // 设置过滤器: DOWN + UP 事件
                    i.0.set_filter(
                        is_keyboard_device,
                        Filter::KeyFilter(KeyFilter::DOWN | KeyFilter::UP),
                    );
                    info!("[hotkeys_interception] keyboard filter set");
                    true
                }
                None => {
                    error!("[hotkeys_interception] context not available for filter setup");
                    false
                }
            }
        };

        if !filter_set {
            return;
        }

        loop {
            // 检查 context 是否可用
            let ctx_guard = match ctx.lock() {
                Ok(g) => g,
                Err(e) => {
                    error!("[hotkeys_interception] failed to lock context: {}", e);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            let interception = match ctx_guard.as_ref() {
                Some(i) => &i.0,
                None => {
                    // Context 未初始化（驱动未就绪），休眠后重试
                    drop(ctx_guard);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            // 等待键盘事件
            let device = interception.wait();
            if !interception::is_keyboard(device) {
                // 非键盘设备，直接透传
                continue;
            }

            // 接收事件
            let mut strokes = [Stroke::Keyboard {
                code: ScanCode::Esc,
                state: KeyState::empty(),
                information: 0,
            }; 16];

            let count = interception.receive(device, &mut strokes);
            if count == 0 {
                continue;
            }

            // 处理每个 stroke
            for stroke in strokes.iter().take(count as usize) {
                match stroke {
                    Stroke::Keyboard {
                        code,
                        state: key_state,
                        ..
                    } => {
                        // 仅处理按下事件（忽略抬起）— DESIGN 8.3
                        if key_state.contains(KeyState::UP) {
                            interception.send(device, &[*stroke]);
                            continue;
                        }

                        // 读取当前热键配置
                        let (start_scan_code, stop_scan_code, current_page, runtime_status) = {
                            let app_state = match state.lock() {
                                Ok(s) => s,
                                Err(e) => {
                                    error!("[hotkeys_interception] failed to lock state: {}", e);
                                    interception.send(device, &[*stroke]);
                                    continue;
                                }
                            };
                            (
                                app_state.config.hotkeys.start.scan_code,
                                app_state.config.hotkeys.stop.scan_code,
                                app_state.current_page.clone(),
                                app_state.runtime_status.clone(),
                            )
                        };

                        // 统一热键匹配逻辑 — 支持启动和停止键相同的 toggle 场景
                        let is_start_key = *code as u16 == start_scan_code;
                        let is_stop_key = *code as u16 == stop_scan_code;

                        if is_start_key || is_stop_key {
                            // 诊断日志 — 热键匹配成功时记录上下文
                            info!(
                                "[hotkeys_interception] hotkey matched: code={}, start_code={}, stop_code={}, page={}, status={:?}",
                                *code as u16, start_scan_code, stop_scan_code, current_page, runtime_status
                            );

                            // 页面过滤 — REQUIREMENTS 3.6
                            if current_page.as_str() != "keyboard"
                                && current_page.as_str() != "mouse"
                            {
                                info!(
                                    "[hotkeys_interception] hotkey blocked by page filter: current_page={}",
                                    current_page
                                );
                                interception.send(device, &[*stroke]);
                                continue;
                            }

                            // 状态机门控：根据当前状态决定行为（支持 toggle）
                            match runtime_status {
                                RuntimeStatus::Idle if is_start_key => {
                                    // Idle 状态下按启动键 → 启动模拟
                                    handle_start_hotkey(&app, &state, current_page.as_str());
                                    // 阻断热键事件，不透传到系统
                                    continue;
                                }
                                RuntimeStatus::RunningKeyboard | RuntimeStatus::RunningMouse
                                    if is_stop_key =>
                                {
                                    // Running 状态下按停止键 → 停止模拟
                                    handle_stop_hotkey(&app, &state);
                                    // 阻断热键事件
                                    continue;
                                }
                                RuntimeStatus::Idle if is_stop_key => {
                                    // Idle 状态下按停止键 → 阻断（不透传）
                                    info!("[hotkeys_interception] stop key pressed in Idle state, ignoring");
                                    continue;
                                }
                                _ => {
                                    // 状态不匹配（如 Running 时按启动键），透传
                                    interception.send(device, &[*stroke]);
                                    continue;
                                }
                            }
                        }

                        // 非热键事件，透传到系统
                        interception.send(device, &[*stroke]);
                    }
                    _ => {
                        // 非键盘事件（理论上不会到达这里），透传
                        interception.send(device, &[*stroke]);
                    }
                }
            }
        }
    });

    Ok(())
}

/// 启动热键回调 — 状态机门控 + 页面过滤 — DESIGN 8.3 / 阶段 15
fn handle_start_hotkey(app: &AppHandle, state: &SharedState, current_page: &str) {
    if current_page == "keyboard" {
        handle_start_keyboard(app, state);
    } else {
        handle_start_mouse(app, state);
    }
}

/// 按键模拟启动分支
fn handle_start_keyboard(app: &AppHandle, state: &SharedState) {
    let new_status = RuntimeStatus::RunningKeyboard;
    info!("[hotkeys_interception] start triggered: Idle -> RunningKeyboard");

    let (selected_actions, action_tx, stop_flag) = {
        let mut app_state = match state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("[hotkeys_interception] start_keyboard: failed to lock state: {}", e);
                return;
            }
        };
        app_state.runtime_status = new_status.clone();
        app_state.stop_flag.store(false, std::sync::atomic::Ordering::Relaxed);
        let selected = app_state
            .config
            .keyboard_actions
            .iter()
            .filter(|a| a.selected)
            .cloned()
            .collect::<Vec<_>>();
        (selected, app_state.action_tx.clone(), app_state.stop_flag.clone())
    };

    if let Err(e) = app.emit("runtime_status_changed", serde_json::json!({ "status": new_status })) {
        error!("[hotkeys_interception] failed to emit runtime_status_changed: {}", e);
    }

    let app_clone = app.clone();
    let state_clone = state.clone();
    std::thread::spawn(move || {
        info!(
            "[hotkeys_interception] keyboard simulation loop started, {} actions",
            selected_actions.len()
        );

        if selected_actions.is_empty() {
            info!("[hotkeys_interception] no selected keyboard actions, stopping immediately");
            if let Ok(mut s) = state_clone.lock() { s.runtime_status = RuntimeStatus::Idle; }
            let _ = app_clone.emit("runtime_status_changed", serde_json::json!({ "status": RuntimeStatus::Idle }));
            return;
        }

        loop {
            for action in &selected_actions {
                macro_rules! check_stop {
                    () => {
                        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("[hotkeys_interception] stop_flag detected, exiting keyboard loop");
                            return;
                        }
                    };
                }
                check_stop!();
                if let Err(e) = action_tx.send(crate::keyboard_worker::ActionEvent::KeyPress { scan_code: action.scan_code }) {
                    error!("[hotkeys_interception] failed to send KeyPress: {}", e);
                    return;
                }
                check_stop!();
                if let Err(e) = action_tx.send(crate::keyboard_worker::ActionEvent::KeyRelease { scan_code: action.scan_code }) {
                    error!("[hotkeys_interception] failed to send KeyRelease: {}", e);
                    return;
                }
                check_stop!();
                std::thread::sleep(std::time::Duration::from_millis(action.interval_ms));
            }
        }
    });
}

/// 鼠标模拟启动分支 — DESIGN 10.2 / 阶段 15
fn handle_start_mouse(app: &AppHandle, state: &SharedState) {
    let new_status = RuntimeStatus::RunningMouse;
    info!("[hotkeys_interception] start triggered: Idle -> RunningMouse");

    let (valid_actions, mouse_tx, stop_flag) = {
        let mut app_state = match state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("[hotkeys_interception] start_mouse: failed to lock state: {}", e);
                return;
            }
        };
        app_state.runtime_status = new_status.clone();
        app_state.stop_flag.store(false, std::sync::atomic::Ordering::Relaxed);
        // 跳过坐标为 null 的行（DESIGN 10.2 / REQUIREMENTS 3.9）
        let valid = app_state
            .config
            .mouse_actions
            .iter()
            .filter(|a| a.x.is_some() && a.y.is_some())
            .cloned()
            .collect::<Vec<_>>();
        (valid, app_state.mouse_tx.clone(), app_state.stop_flag.clone())
    };

    if let Err(e) = app.emit("runtime_status_changed", serde_json::json!({ "status": new_status })) {
        error!("[hotkeys_interception] failed to emit runtime_status_changed: {}", e);
    }

    let app_clone = app.clone();
    let state_clone = state.clone();
    std::thread::spawn(move || {
        info!(
            "[hotkeys_interception] mouse simulation loop started, {} valid actions",
            valid_actions.len()
        );

        if valid_actions.is_empty() {
            // 全部坐标无效：日志明确，保持 ReadyMouse（不报错）— TASKS 阶段 15 验收
            info!("[hotkeys_interception] all mouse actions have null coords, keeping ReadyMouse");
            if let Ok(mut s) = state_clone.lock() { s.runtime_status = crate::state::RuntimeStatus::ReadyMouse; }
            let _ = app_clone.emit(
                "runtime_status_changed",
                serde_json::json!({ "status": crate::state::RuntimeStatus::ReadyMouse }),
            );
            return;
        }

        loop {
            for action in &valid_actions {
                macro_rules! check_stop {
                    () => {
                        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("[hotkeys_interception] stop_flag detected, exiting mouse loop");
                            return;
                        }
                    };
                }
                let (x, y) = (action.x.unwrap(), action.y.unwrap());
                check_stop!();
                if let Err(e) = mouse_tx.send(crate::mouse_worker::MouseEvent::Click { x, y }) {
                    error!("[hotkeys_interception] failed to send MouseClick: {}", e);
                    return;
                }
                check_stop!();
                std::thread::sleep(std::time::Duration::from_millis(action.interval_ms));
            }
        }
    });
}

/// 停止热键回调 — 状态机门控 — DESIGN 8.3
fn handle_stop_hotkey(app: &AppHandle, state: &SharedState) {
    info!("[hotkeys_interception] stop triggered: Running* -> Idle");

    // 设置停止标记
    {
        let app_state = match state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("[hotkeys_interception] stop: failed to lock state: {}", e);
                return;
            }
        };
        app_state
            .stop_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    // 等待一小段时间让模拟循环退出
    std::thread::sleep(std::time::Duration::from_millis(50));

    // 更新状态
    {
        let mut app_state = match state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!(
                    "[hotkeys_interception] stop: failed to lock state after wait: {}",
                    e
                );
                return;
            }
        };
        app_state.runtime_status = RuntimeStatus::Idle;
    }

    // 发送 runtime_status_changed 事件
    if let Err(e) = app.emit(
        "runtime_status_changed",
        serde_json::json!({ "status": RuntimeStatus::Idle }),
    ) {
        error!(
            "[hotkeys_interception] failed to emit runtime_status_changed: {}",
            e
        );
    }
}
