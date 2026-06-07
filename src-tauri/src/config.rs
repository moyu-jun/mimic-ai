// 配置模型与默认配置 — DESIGN 4.2 / DESIGN 16
//
// 本模块定义前后端共享的配置结构体，并提供默认配置初始化。
// 所有结构体必须标注 #[serde(rename_all = "camelCase")] 确保 Rust snake_case
// 字段序列化为前端 camelCase（key_label → keyLabel）。

use serde::{Deserialize, Serialize};

/// 全局默认间隔时间（毫秒）
pub const DEFAULT_INTERVAL_MS: u64 = 20;

/// 按键捕获结果 — DESIGN 4.2
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedKey {
    /// 按键显示名（如 "F12", "A"）
    pub key_label: String,
    /// Interception scan code
    pub scan_code: u16,
}

/// 按键模拟单项动作 — DESIGN 4.2
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardAction {
    /// 唯一标识符
    pub id: String,
    /// 是否勾选（运行时执行）
    pub selected: bool,
    /// 按键显示名
    pub key_label: String,
    /// Interception scan code
    pub scan_code: u16,
    /// 动作执行后的等待间隔（毫秒）
    pub interval_ms: u64,
}

/// 鼠标模拟单项动作 — DESIGN 4.2
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseAction {
    /// 唯一标识符
    pub id: String,
    /// 屏幕 X 坐标（可空）
    pub x: Option<i32>,
    /// 屏幕 Y 坐标（可空）
    pub y: Option<i32>,
    /// 动作执行后的等待间隔（毫秒）
    pub interval_ms: u64,
}

/// 热键配置 — DESIGN 4.2
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyConfig {
    /// 启动热键
    pub start: CapturedKey,
    /// 停止热键
    pub stop: CapturedKey,
}

/// 应用完整配置 — DESIGN 4.2
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// 按键动作列表
    pub keyboard_actions: Vec<KeyboardAction>,
    /// 鼠标动作列表
    pub mouse_actions: Vec<MouseAction>,
    /// 全局热键配置
    pub hotkeys: HotkeyConfig,
}

/// 生成默认配置 — DESIGN 16
///
/// 按键列表包含一项默认按键 F（scan code 33），间隔 20ms，已勾选；
/// 鼠标列表包含一项空坐标动作，间隔 20ms；
/// 启动/停止热键均为 F12（scan code 88）。
pub fn default_config() -> AppConfig {
    AppConfig {
        keyboard_actions: vec![KeyboardAction {
            id: "default-keyboard-1".to_string(),
            selected: true,
            key_label: "F".to_string(),
            scan_code: 33,
            interval_ms: DEFAULT_INTERVAL_MS,
        }],
        mouse_actions: vec![MouseAction {
            id: "default-mouse-1".to_string(),
            x: None,
            y: None,
            interval_ms: DEFAULT_INTERVAL_MS,
        }],
        hotkeys: HotkeyConfig {
            start: CapturedKey {
                key_label: "F12".to_string(),
                scan_code: 88,
            },
            stop: CapturedKey {
                key_label: "F12".to_string(),
                scan_code: 88,
            },
        },
    }
}
