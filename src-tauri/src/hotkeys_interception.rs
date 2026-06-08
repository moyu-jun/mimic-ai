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

                        // 匹配启动热键
                        if *code as u16 == start_scan_code {
                            // 页面过滤 — REQUIREMENTS 3.6
                            if current_page.as_str() != "keyboard"
                                && current_page.as_str() != "mouse"
                            {
                                interception.send(device, &[*stroke]);
                                continue;
                            }

                            // 状态机门控 — DESIGN 9.2：仅 Idle 时启动键生效
                            if runtime_status == RuntimeStatus::Idle {
                                handle_start_hotkey(&app, &state, current_page.as_str());
                                // 阻断热键事件，不透传到系统
                                continue;
                            } else {
                                // 状态不匹配，透传
                                interception.send(device, &[*stroke]);
                                continue;
                            }
                        }

                        // 匹配停止热键
                        if *code as u16 == stop_scan_code {
                            // 状态机门控 — DESIGN 9.2：仅 Running* 时停止键生效
                            if runtime_status == RuntimeStatus::RunningKeyboard
                                || runtime_status == RuntimeStatus::RunningMouse
                            {
                                handle_stop_hotkey(&app, &state);
                                // 阻断热键事件
                                continue;
                            } else {
                                // 状态不匹配，透传
                                interception.send(device, &[*stroke]);
                                continue;
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

/// 启动热键回调 — 状态机门控 + 页面过滤 — DESIGN 8.3
fn handle_start_hotkey(app: &AppHandle, state: &SharedState, current_page: &str) {
    let new_status = if current_page == "keyboard" {
        RuntimeStatus::RunningKeyboard
    } else {
        RuntimeStatus::RunningMouse
    };

    info!(
        "[hotkeys_interception] start triggered: Idle -> {:?} (page={})",
        new_status, current_page
    );

    // 克隆需要的数据，避免在 spawn 线程中长期持有 state 锁
    let (selected_actions, action_tx, stop_flag) = {
        let mut app_state = match state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("[hotkeys_interception] start: failed to lock state: {}", e);
                return;
            }
        };

        // 更新状态
        app_state.runtime_status = new_status.clone();

        // 重置停止标记
        app_state
            .stop_flag
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // 克隆选中的 actions
        let selected = app_state
            .config
            .keyboard_actions
            .iter()
            .filter(|a| a.selected)
            .cloned()
            .collect::<Vec<_>>();

        (
            selected,
            app_state.action_tx.clone(),
            app_state.stop_flag.clone(),
        )
    };

    // 发送 runtime_status_changed 事件
    if let Err(e) = app.emit(
        "runtime_status_changed",
        serde_json::json!({ "status": new_status }),
    ) {
        error!(
            "[hotkeys_interception] failed to emit runtime_status_changed: {}",
            e
        );
    }

    // 启动模拟循环线程
    let app_clone = app.clone();
    let state_clone = state.clone();
    std::thread::spawn(move || {
        info!(
            "[hotkeys_interception] simulation loop started, {} actions selected",
            selected_actions.len()
        );

        if selected_actions.is_empty() {
            info!("[hotkeys_interception] no selected actions, stopping immediately");
            // 立即切回 Idle
            if let Ok(mut app_state) = state_clone.lock() {
                app_state.runtime_status = RuntimeStatus::Idle;
            }
            let _ = app_clone.emit(
                "runtime_status_changed",
                serde_json::json!({ "status": RuntimeStatus::Idle }),
            );
            return;
        }

        // 循环模拟
        loop {
            for action in &selected_actions {
                // 检查停止标记
                if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    info!("[hotkeys_interception] stop_flag detected, exiting loop");
                    return;
                }

                // 发送按键按下
                if let Err(e) = action_tx.send(crate::keyboard_worker::ActionEvent::KeyPress {
                    scan_code: action.scan_code,
                }) {
                    error!("[hotkeys_interception] failed to send KeyPress: {}", e);
                    return;
                }

                // 发送按键释放
                if let Err(e) = action_tx.send(crate::keyboard_worker::ActionEvent::KeyRelease {
                    scan_code: action.scan_code,
                }) {
                    error!("[hotkeys_interception] failed to send KeyRelease: {}", e);
                    return;
                }

                // 发送延迟
                if let Err(e) = action_tx.send(crate::keyboard_worker::ActionEvent::Delay {
                    duration_ms: action.interval_ms,
                }) {
                    error!("[hotkeys_interception] failed to send Delay: {}", e);
                    return;
                }
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
