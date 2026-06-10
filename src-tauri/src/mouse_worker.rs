// 鼠标模拟 worker — DESIGN 10.2 / 阶段 15
//
// 从 channel 接收 MouseEvent，转译为 Interception MouseStroke 并发送。
// 复用独立的 Interception worker context（与键盘 worker context 相同实例，均为 send-only）。
//
// 与键盘 worker 保持一致的结构：
//   - 长生命周期线程：循环 recv() → 状态机门控 → 发送 stroke
//   - 状态门控：仅在 RunningMouse 时执行
//   - 设备选择：扫描 1-10 找第一个鼠标设备

use crate::state::{RuntimeStatus, SendInterception, SharedState};
use interception::{MouseFlags, MouseState, Stroke};
use log::{error, info, warn};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

/// 鼠标模拟事件类型 — DESIGN 10.2
#[derive(Debug, Clone)]
pub enum MouseEvent {
    /// 移动到绝对屏幕坐标并执行左键点击
    Click { x: i32, y: i32 },
    /// 间隔等待（由生产者线程 sleep 处理，此类型保留但不走 channel）
    #[allow(dead_code)]
    Stop,
}

/// 启动鼠标模拟 worker 线程 — DESIGN 10.2
///
/// 长生命周期线程：循环接收 MouseEvent → 状态机门控 → 转译为 Stroke → send()。
/// 绝对坐标范围 0–65535 对应屏幕宽/高（MOUSEEVENTF_ABSOLUTE 语义）。
pub fn start_mouse_worker(
    rx: Receiver<MouseEvent>,
    state: SharedState,
    ctx: Arc<Mutex<Option<SendInterception>>>,
) -> Result<(), String> {
    std::thread::spawn(move || {
        info!("[mouse_worker] worker thread started");

        loop {
            // 接收事件（阻塞）
            let event = match rx.recv() {
                Ok(e) => e,
                Err(e) => {
                    warn!("[mouse_worker] channel closed: {}", e);
                    break;
                }
            };

            if matches!(event, MouseEvent::Stop) {
                info!("[mouse_worker] received stop signal");
                break;
            }

            // 状态机门控：仅在 RunningMouse 状态下执行模拟
            let is_running = {
                let app_state = match state.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        error!("[mouse_worker] failed to lock state: {}", e);
                        continue;
                    }
                };
                matches!(app_state.runtime_status, RuntimeStatus::RunningMouse)
            };

            if !is_running {
                warn!("[mouse_worker] received event but not in RunningMouse state, skipping");
                continue;
            }

            let MouseEvent::Click { x, y } = event else {
                continue;
            };

            // 获取 Interception context
            let ctx_guard = match ctx.lock() {
                Ok(g) => g,
                Err(e) => {
                    error!("[mouse_worker] failed to lock context: {}", e);
                    continue;
                }
            };

            let interception = match ctx_guard.as_ref() {
                Some(i) => &i.0,
                None => {
                    error!("[mouse_worker] Interception context not available");
                    continue;
                }
            };

            // 扫描 1-20 找第一个鼠标设备
            // Interception 设备编号: 键盘 1-10, 鼠标 11-20 (INTERCEPTION_MAX_DEVICE = 20)
            let mouse_device = (1..=20).find(|d| interception::is_mouse(*d));
            let device = match mouse_device {
                Some(d) => d,
                None => {
                    error!("[mouse_worker] no mouse device found");
                    continue;
                }
            };

            // 1. 移动到绝对坐标
            // Interception 绝对坐标范围 0–65535，需按屏幕分辨率归一化。
            // 第一版单显示器标准 DPI：通过 GetSystemMetrics 获取屏幕尺寸转换。
            let (screen_w, screen_h) = get_screen_size();
            let norm_x = if screen_w > 0 {
                (x as i64 * 65535 / screen_w as i64) as i32
            } else {
                x
            };
            let norm_y = if screen_h > 0 {
                (y as i64 * 65535 / screen_h as i64) as i32
            } else {
                y
            };

            let move_stroke = Stroke::Mouse {
                state: MouseState::empty(),
                flags: MouseFlags::MOVE_ABSOLUTE,
                rolling: 0,
                x: norm_x,
                y: norm_y,
                information: 0,
            };
            interception.send(device, &[move_stroke]);

            // 2. 左键按下
            let down_stroke = Stroke::Mouse {
                state: MouseState::LEFT_BUTTON_DOWN,
                flags: MouseFlags::empty(),
                rolling: 0,
                x: 0,
                y: 0,
                information: 0,
            };
            interception.send(device, &[down_stroke]);

            // 3. 左键抬起
            let up_stroke = Stroke::Mouse {
                state: MouseState::LEFT_BUTTON_UP,
                flags: MouseFlags::empty(),
                rolling: 0,
                x: 0,
                y: 0,
                information: 0,
            };
            interception.send(device, &[up_stroke]);
        }

        info!("[mouse_worker] worker thread exited");
    });

    Ok(())
}

/// 获取主显示器分辨率（用于绝对坐标归一化）。
///
/// 仅 Windows 平台；其余平台返回 (0, 0) 触发原始坐标回退。
fn get_screen_size() -> (i32, i32) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
        };
        unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
    }
    #[cfg(not(windows))]
    {
        (0, 0)
    }
}
