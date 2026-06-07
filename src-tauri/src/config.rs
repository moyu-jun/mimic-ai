// 配置模型与默认配置 — DESIGN 4.2 / DESIGN 16
//
// 本模块定义前后端共享的配置结构体，并提供默认配置初始化。
// 所有结构体必须标注 #[serde(rename_all = "camelCase")] 确保 Rust snake_case
// 字段序列化为前端 camelCase（key_label → keyLabel）。
//
// 阶段 9：增加 INI 持久化功能 — config_path / load_or_init / save

use ini::Ini;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

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

/// 获取配置文件路径 — DESIGN 5 / 阶段 9
///
/// 返回 exe 同级的 `mimic.ini` 路径。
pub fn config_path() -> Result<PathBuf, String> {
    let exe_path = env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "Failed to get exe directory".to_string())?;
    Ok(exe_dir.join("mimic.ini"))
}

/// 优雅初始化配置 — 写盘失败也不 panic，返回 (配置, 可选警告)
///
/// 任何 IO 错误均降级为内存默认配置，应用仍可正常启动。
/// 警告非空时说明本次配置无法写盘，修改将不会持久化到磁盘。
pub fn load_or_init_graceful() -> (AppConfig, Option<String>) {
    match load_or_init() {
        Ok(config) => (config, None),
        Err(e) => {
            log::error!("[config] fallback to in-memory default: {}", e);
            (default_config(), Some(e))
        }
    }
}

/// 从 INI 加载配置，不存在或失败时写入默认配置 — 阶段 9
///
/// - 文件存在且解析成功 → 返回解析结果
/// - 文件不存在 → 创建默认配置并写入，返回默认配置
/// - 文件损坏（解析失败）→ 用默认配置覆盖，不备份，返回默认配置
/// - 写盘失败 → 返回 Err（由 load_or_init_graceful 捕获降级）
pub fn load_or_init() -> Result<AppConfig, String> {
    let path = config_path()?;

    if !path.exists() {
        // 文件不存在，写入默认配置
        log::info!("[config] mimic.ini not found, writing default");
        let default = default_config();
        save(&default)?;
        return Ok(default);
    }

    // 尝试解析现有文件
    match load_from_ini(&path) {
        Ok(config) => Ok(config),
        Err(e) => {
            // 解析失败，用默认配置覆盖
            log::error!(
                "[config] failed to parse INI, overwriting with default: {}",
                e
            );
            let default = default_config();
            save(&default)?;
            Ok(default)
        }
    }
}

/// 从 INI 文件加载配置 — 阶段 9 内部辅助函数
fn load_from_ini(path: &PathBuf) -> Result<AppConfig, String> {
    let ini = Ini::load_from_file(path).map_err(|e| format!("Failed to load INI file: {}", e))?;

    // 解析 [hotkeys] section
    let hotkeys_section = ini
        .section(Some("hotkeys"))
        .ok_or_else(|| "Missing [hotkeys] section".to_string())?;

    let start_label = hotkeys_section
        .get("start_label")
        .ok_or_else(|| "Missing start_label".to_string())?;
    let start_scan_code: u16 = hotkeys_section
        .get("start_scan_code")
        .ok_or_else(|| "Missing start_scan_code".to_string())?
        .parse()
        .map_err(|e| format!("Invalid start_scan_code: {}", e))?;

    let stop_label = hotkeys_section
        .get("stop_label")
        .ok_or_else(|| "Missing stop_label".to_string())?;
    let stop_scan_code: u16 = hotkeys_section
        .get("stop_scan_code")
        .ok_or_else(|| "Missing stop_scan_code".to_string())?
        .parse()
        .map_err(|e| format!("Invalid stop_scan_code: {}", e))?;

    let hotkeys = HotkeyConfig {
        start: CapturedKey {
            key_label: start_label.to_string(),
            scan_code: start_scan_code,
        },
        stop: CapturedKey {
            key_label: stop_label.to_string(),
            scan_code: stop_scan_code,
        },
    };

    // 解析 [keyboard] section
    let keyboard_section = ini
        .section(Some("keyboard"))
        .ok_or_else(|| "Missing [keyboard] section".to_string())?;
    let keyboard_actions_json = keyboard_section
        .get("actions")
        .ok_or_else(|| "Missing keyboard actions".to_string())?;
    let keyboard_actions: Vec<KeyboardAction> = serde_json::from_str(keyboard_actions_json)
        .map_err(|e| format!("Failed to parse keyboard actions: {}", e))?;

    // 解析 [mouse] section
    let mouse_section = ini
        .section(Some("mouse"))
        .ok_or_else(|| "Missing [mouse] section".to_string())?;
    let mouse_actions_json = mouse_section
        .get("actions")
        .ok_or_else(|| "Missing mouse actions".to_string())?;
    let mouse_actions: Vec<MouseAction> = serde_json::from_str(mouse_actions_json)
        .map_err(|e| format!("Failed to parse mouse actions: {}", e))?;

    Ok(AppConfig {
        keyboard_actions,
        mouse_actions,
        hotkeys,
    })
}

/// 保存配置到 INI 文件 — 阶段 9
///
/// 将 AppConfig 写入 exe 同级的 mimic.ini，格式按 DESIGN 5。
pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path()?;
    let mut ini = Ini::new();

    // 写入 [hotkeys] section
    ini.with_section(Some("hotkeys"))
        .set("start_label", &config.hotkeys.start.key_label)
        .set(
            "start_scan_code",
            config.hotkeys.start.scan_code.to_string(),
        )
        .set("stop_label", &config.hotkeys.stop.key_label)
        .set("stop_scan_code", config.hotkeys.stop.scan_code.to_string());

    // 写入 [keyboard] section
    let keyboard_json = serde_json::to_string(&config.keyboard_actions)
        .map_err(|e| format!("Failed to serialize keyboard actions: {}", e))?;
    ini.with_section(Some("keyboard"))
        .set("actions", keyboard_json);

    // 写入 [mouse] section
    let mouse_json = serde_json::to_string(&config.mouse_actions)
        .map_err(|e| format!("Failed to serialize mouse actions: {}", e))?;
    ini.with_section(Some("mouse")).set("actions", mouse_json);

    // 写入文件
    ini.write_to_file(&path)
        .map_err(|e| format!("Failed to write INI file: {}", e))?;

    Ok(())
}
