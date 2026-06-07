// 全局热键注册与管理 — DESIGN 6 / DESIGN 8.1 / DESIGN 9.2
//
// 阶段 12：基于 tauri-plugin-global-shortcut 实现热键注册、更新与状态机门控。
// 热键回调仅切换 runtime_status 并发送事件，不实际运行模拟（阶段 13 接入真实 worker）。

use crate::config::HotkeyConfig;
use crate::state::{RuntimeStatus, SharedState};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

/// 热键更新结果 — DESIGN 6.2
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyUpdateResult {
    /// 与已持久化配置对比是否有变化
    pub changed: bool,
    /// 新热键注册是否成功
    pub registered: bool,
    /// 是否成功写入 mimic.ini
    pub persisted: bool,
    /// 可选错误消息
    pub message: Option<String>,
}

/// scanCode → Code 枚举映射 — DESIGN 8.1
///
/// 第一版支持字母、数字、F1-F12、Space、Tab、Esc、Left/Right Shift/Ctrl/Alt。
/// 不支持方向键与组合键。
fn scan_code_to_code(scan_code: u16) -> Option<Code> {
    match scan_code {
        // F1-F12
        59 => Some(Code::F1),
        60 => Some(Code::F2),
        61 => Some(Code::F3),
        62 => Some(Code::F4),
        63 => Some(Code::F5),
        64 => Some(Code::F6),
        65 => Some(Code::F7),
        66 => Some(Code::F8),
        67 => Some(Code::F9),
        68 => Some(Code::F10),
        87 => Some(Code::F11),
        88 => Some(Code::F12),
        // 字母键 A-Z
        16 => Some(Code::KeyQ),
        17 => Some(Code::KeyW),
        18 => Some(Code::KeyE),
        19 => Some(Code::KeyR),
        20 => Some(Code::KeyT),
        21 => Some(Code::KeyY),
        22 => Some(Code::KeyU),
        23 => Some(Code::KeyI),
        24 => Some(Code::KeyO),
        25 => Some(Code::KeyP),
        30 => Some(Code::KeyA),
        31 => Some(Code::KeyS),
        32 => Some(Code::KeyD),
        33 => Some(Code::KeyF),
        34 => Some(Code::KeyG),
        35 => Some(Code::KeyH),
        36 => Some(Code::KeyJ),
        37 => Some(Code::KeyK),
        38 => Some(Code::KeyL),
        44 => Some(Code::KeyZ),
        45 => Some(Code::KeyX),
        46 => Some(Code::KeyC),
        47 => Some(Code::KeyV),
        48 => Some(Code::KeyB),
        49 => Some(Code::KeyN),
        50 => Some(Code::KeyM),
        // 数字键 0-9 (主键盘)
        11 => Some(Code::Digit0),
        2 => Some(Code::Digit1),
        3 => Some(Code::Digit2),
        4 => Some(Code::Digit3),
        5 => Some(Code::Digit4),
        6 => Some(Code::Digit5),
        7 => Some(Code::Digit6),
        8 => Some(Code::Digit7),
        9 => Some(Code::Digit8),
        10 => Some(Code::Digit9),
        // 功能键
        57 => Some(Code::Space),
        15 => Some(Code::Tab),
        1 => Some(Code::Escape),
        // Shift/Ctrl/Alt
        42 => Some(Code::ShiftLeft),
        54 => Some(Code::ShiftRight),
        29 => Some(Code::ControlLeft),
        56 => Some(Code::AltLeft),
        _ => None,
    }
}

/// 初始注册全局热键 — DESIGN 13.1
///
/// 在 setup 阶段调用，基于 load_or_init 返回的配置注册热键。
/// 注册失败记录错误日志但不阻塞应用启动。
pub fn register_hotkeys(app: &AppHandle, hotkeys: &HotkeyConfig) -> Result<(), String> {
    let start_code = scan_code_to_code(hotkeys.start.scan_code).ok_or_else(|| {
        format!(
            "Unsupported start key scan code: {}",
            hotkeys.start.scan_code
        )
    })?;
    let stop_code = scan_code_to_code(hotkeys.stop.scan_code)
        .ok_or_else(|| format!("Unsupported stop key scan code: {}", hotkeys.stop.scan_code))?;

    info!(
        "[hotkeys] registering: start={:?} ({}), stop={:?} ({})",
        start_code, hotkeys.start.key_label, stop_code, hotkeys.stop.key_label
    );

    // 注册启动热键
    let start_shortcut = Shortcut::new(None, start_code);
    let app_for_start = app.clone();
    app.global_shortcut()
        .on_shortcut(start_shortcut, move |_app, shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                handle_start_hotkey(&app_for_start, shortcut);
            }
        })
        .map_err(|e| format!("Failed to register start hotkey: {}", e))?;

    // 注册停止热键（如果与启动热键不同）
    if start_code != stop_code {
        let stop_shortcut = Shortcut::new(None, stop_code);
        let app_for_stop = app.clone();
        app.global_shortcut()
            .on_shortcut(stop_shortcut, move |_app, shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    handle_stop_hotkey(&app_for_stop, shortcut);
                }
            })
            .map_err(|e| format!("Failed to register stop hotkey: {}", e))?;
    }

    info!("[hotkeys] registration complete");
    Ok(())
}

/// 启动热键回调 — 状态机门控 + 页面过滤
fn handle_start_hotkey(app: &AppHandle, _shortcut: &Shortcut) {
    let state = match app.try_state::<SharedState>() {
        Some(s) => s,
        None => {
            error!("[hotkeys] start: state not found");
            return;
        }
    };

    let mut app_state = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("[hotkeys] start: failed to lock state: {}", e);
            return;
        }
    };

    // 页面过滤 — REQUIREMENTS 3.6
    if app_state.current_page != "keyboard" && app_state.current_page != "mouse" {
        return;
    }

    // 状态机门控 — DESIGN 9.2：仅 Idle 时启动键生效
    match app_state.runtime_status {
        RuntimeStatus::Idle => {
            // 根据 current_page 切到对应 Running* 状态
            let new_status = if app_state.current_page == "keyboard" {
                RuntimeStatus::RunningKeyboard
            } else {
                RuntimeStatus::RunningMouse
            };

            info!(
                "[hotkeys] start triggered: Idle -> {:?} (page={})",
                new_status, app_state.current_page
            );
            app_state.runtime_status = new_status.clone();

            // 发送 runtime_status_changed 事件
            if let Err(e) = app.emit(
                "runtime_status_changed",
                serde_json::json!({ "status": new_status }),
            ) {
                error!("[hotkeys] failed to emit runtime_status_changed: {}", e);
            }
        }
        _ => {
            // 状态不匹配，忽略
        }
    }
}

/// 停止热键回调 — 状态机门控
fn handle_stop_hotkey(app: &AppHandle, _shortcut: &Shortcut) {
    let state = match app.try_state::<SharedState>() {
        Some(s) => s,
        None => {
            error!("[hotkeys] stop: state not found");
            return;
        }
    };

    let mut app_state = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("[hotkeys] stop: failed to lock state: {}", e);
            return;
        }
    };

    // 状态机门控 — DESIGN 9.2：仅 Running* 时停止键生效
    let new_status = match app_state.runtime_status {
        RuntimeStatus::RunningKeyboard => RuntimeStatus::Idle,
        RuntimeStatus::RunningMouse => RuntimeStatus::Idle,
        _ => {
            // 状态不匹配，忽略
            return;
        }
    };

    info!(
        "[hotkeys] stop triggered: {:?} -> {:?}",
        app_state.runtime_status, new_status
    );
    app_state.runtime_status = new_status.clone();

    // 发送 runtime_status_changed 事件
    if let Err(e) = app.emit(
        "runtime_status_changed",
        serde_json::json!({ "status": new_status }),
    ) {
        error!("[hotkeys] failed to emit runtime_status_changed: {}", e);
    }
}

/// 更新热键配置 — DESIGN 6.2
///
/// 流程：对比变化 → 注销旧热键 → 校验冲突 → 注册新热键 → 持久化。
/// 注册失败时保留旧热键注册状态。
pub fn update_hotkeys(
    app: &AppHandle,
    state: &SharedState,
    new_hotkeys: HotkeyConfig,
) -> Result<HotkeyUpdateResult, String> {
    let old_hotkeys = {
        let app_state = state
            .lock()
            .map_err(|e| format!("Failed to lock state: {}", e))?;
        app_state.config.hotkeys.clone()
    };

    // 对比变化
    let changed = old_hotkeys.start.scan_code != new_hotkeys.start.scan_code
        || old_hotkeys.stop.scan_code != new_hotkeys.stop.scan_code;

    if !changed {
        return Ok(HotkeyUpdateResult {
            changed: false,
            registered: true,
            persisted: true,
            message: None,
        });
    }

    // 热键与按键列表冲突校验 — DESIGN 15.6 反馈 Q6
    let keyboard_scan_codes: HashSet<u16> = {
        let app_state = state
            .lock()
            .map_err(|e| format!("Failed to lock state: {}", e))?;
        app_state
            .config
            .keyboard_actions
            .iter()
            .map(|a| a.scan_code)
            .collect()
    };

    if keyboard_scan_codes.contains(&new_hotkeys.start.scan_code) {
        return Ok(HotkeyUpdateResult {
            changed: true,
            registered: false,
            persisted: false,
            message: Some(format!(
                "启动热键与按键模拟列表冲突: {}",
                new_hotkeys.start.key_label
            )),
        });
    }

    if keyboard_scan_codes.contains(&new_hotkeys.stop.scan_code) {
        return Ok(HotkeyUpdateResult {
            changed: true,
            registered: false,
            persisted: false,
            message: Some(format!(
                "停止热键与按键模拟列表冲突: {}",
                new_hotkeys.stop.key_label
            )),
        });
    }

    // 注销旧热键
    unregister_hotkeys(app, &old_hotkeys)?;

    // 注册新热键
    match register_hotkeys(app, &new_hotkeys) {
        Ok(_) => {
            // 注册成功，持久化
            let mut updated_config = {
                let app_state = state
                    .lock()
                    .map_err(|e| format!("Failed to lock state: {}", e))?;
                app_state.config.clone()
            };
            updated_config.hotkeys = new_hotkeys.clone();

            match crate::config::save(&updated_config) {
                Ok(_) => {
                    // 持久化成功，更新内存
                    let mut app_state = state
                        .lock()
                        .map_err(|e| format!("Failed to lock state: {}", e))?;
                    app_state.config.hotkeys = new_hotkeys;

                    Ok(HotkeyUpdateResult {
                        changed: true,
                        registered: true,
                        persisted: true,
                        message: None,
                    })
                }
                Err(e) => {
                    warn!("[hotkeys] persisted failed: {}, keeping in-memory only", e);
                    // 持久化失败但注册成功，内存保持新热键
                    let mut app_state = state
                        .lock()
                        .map_err(|e| format!("Failed to lock state: {}", e))?;
                    app_state.config.hotkeys = new_hotkeys;

                    Ok(HotkeyUpdateResult {
                        changed: true,
                        registered: true,
                        persisted: false,
                        message: Some(e),
                    })
                }
            }
        }
        Err(e) => {
            // 注册失败，恢复旧热键
            error!(
                "[hotkeys] registration failed: {}, restoring old hotkeys",
                e
            );
            if let Err(restore_err) = register_hotkeys(app, &old_hotkeys) {
                error!("[hotkeys] failed to restore old hotkeys: {}", restore_err);
            }

            Ok(HotkeyUpdateResult {
                changed: true,
                registered: false,
                persisted: false,
                message: Some(e),
            })
        }
    }
}

/// 注销指定热键配置
fn unregister_hotkeys(app: &AppHandle, hotkeys: &HotkeyConfig) -> Result<(), String> {
    let start_code = scan_code_to_code(hotkeys.start.scan_code).ok_or_else(|| {
        format!(
            "Unsupported start key scan code: {}",
            hotkeys.start.scan_code
        )
    })?;
    let stop_code = scan_code_to_code(hotkeys.stop.scan_code)
        .ok_or_else(|| format!("Unsupported stop key scan code: {}", hotkeys.stop.scan_code))?;

    info!(
        "[hotkeys] unregistering: start={:?}, stop={:?}",
        start_code, stop_code
    );

    // 注销启动热键
    let start_shortcut = Shortcut::new(None, start_code);
    if let Err(e) = app.global_shortcut().unregister(start_shortcut) {
        warn!("[hotkeys] failed to unregister start hotkey: {}", e);
    }

    // 注销停止热键（如果与启动热键不同）
    if start_code != stop_code {
        let stop_shortcut = Shortcut::new(None, stop_code);
        if let Err(e) = app.global_shortcut().unregister(stop_shortcut) {
            warn!("[hotkeys] failed to unregister stop hotkey: {}", e);
        }
    }

    Ok(())
}
