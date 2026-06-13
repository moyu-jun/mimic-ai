// 配置模型与默认配置 — DESIGN 4.2 / DESIGN 16
//
// 本模块定义前后端共享的配置结构体，并提供默认配置初始化。
// 所有结构体必须标注 #[serde(rename_all = "camelCase")] 确保 Rust snake_case
// 字段序列化为前端 camelCase（key_label → keyLabel）。
//
// 阶段 9：增加 INI 持久化功能 — config_path / load_or_init / save

use ini::Ini;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

/// 全局默认间隔时间（毫秒）
pub const DEFAULT_INTERVAL_MS: u64 = 20;

/// 模拟动作的最小间隔时间（毫秒）
///
/// 用于防御 sleep(0) 烧 CPU + flood 目标程序的退化情况。
/// 加载 INI 与 save 时统一对 interval_ms 做下限 clamp。
/// 数值选 5 是因为 Windows 调度精度普遍 ~15ms，5 已经低于人感知阈值，
/// 但仍保留正向 sleep（避免被 OS 折叠为 yield）。
pub const MIN_INTERVAL_MS: u64 = 5;

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

/// 净化配置：clamp interval_ms 下限 + 修复重复 id
///
/// 防御以下退化情况：
/// 1. interval_ms < MIN_INTERVAL_MS（含 0）：手改 INI 或前端漏校验时，
///    sleep(0) 等价于 yield，模拟循环会以 OS 调度极限频率烧 CPU + flood 目标程序。
///    统一 clamp 到 MIN_INTERVAL_MS。
/// 2. 重复 id：手改 INI 复制粘贴可能产生同 id 行，前端 v-for :key 会出现重复 key
///    告警，删除/编辑可能误操作另一行。检测到重复时给后续重复行追加 `-dup-N` 后缀。
///
/// 调用点：load_or_init 解析成功后、save 写盘前各调用一次，确保磁盘与内存状态都净化。
pub fn sanitize_config(config: &mut AppConfig) {
    for action in &mut config.keyboard_actions {
        if action.interval_ms < MIN_INTERVAL_MS {
            log::warn!(
                "[config] keyboard action {} intervalMs {} < {}, clamped",
                action.id,
                action.interval_ms,
                MIN_INTERVAL_MS
            );
            action.interval_ms = MIN_INTERVAL_MS;
        }
    }
    for action in &mut config.mouse_actions {
        if action.interval_ms < MIN_INTERVAL_MS {
            log::warn!(
                "[config] mouse action {} intervalMs {} < {}, clamped",
                action.id,
                action.interval_ms,
                MIN_INTERVAL_MS
            );
            action.interval_ms = MIN_INTERVAL_MS;
        }
    }

    dedupe_ids(
        &mut config.keyboard_actions,
        |a| &a.id,
        |a, new_id| a.id = new_id,
        "keyboard",
    );
    dedupe_ids(
        &mut config.mouse_actions,
        |a| &a.id,
        |a, new_id| a.id = new_id,
        "mouse",
    );
}

/// 通用 ID 去重：首次出现保留原 id，后续重复行追加 `-dup-N` 后缀直到唯一。
fn dedupe_ids<T>(
    actions: &mut [T],
    get_id: impl Fn(&T) -> &str,
    mut set_id: impl FnMut(&mut T, String),
    kind: &str,
) {
    let mut seen: HashSet<String> = HashSet::new();
    for action in actions.iter_mut() {
        let original = get_id(action).to_string();
        if seen.insert(original.clone()) {
            continue;
        }
        // 已重复，找一个唯一后缀
        let mut counter = 1u32;
        let new_id = loop {
            let candidate = format!("{}-dup-{}", original, counter);
            if !seen.contains(&candidate) {
                break candidate;
            }
            counter += 1;
        };
        log::warn!(
            "[config] {} action duplicate id `{}`, renamed to `{}`",
            kind,
            original,
            new_id
        );
        seen.insert(new_id.clone());
        set_id(action, new_id);
    }
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
        Ok(mut config) => {
            sanitize_config(&mut config);
            Ok(config)
        }
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
/// 写入前调用 sanitize_config 防御退化值（interval_ms < 5、重复 id）。
/// 采用 `mimic.ini.tmp` + `fs::rename` 原子替换，写入中途崩溃不会损坏旧文件。
pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path()?;
    let mut sanitized = config.clone();
    sanitize_config(&mut sanitized);

    let mut ini = Ini::new();

    // 写入 [hotkeys] section
    ini.with_section(Some("hotkeys"))
        .set("start_label", &sanitized.hotkeys.start.key_label)
        .set(
            "start_scan_code",
            sanitized.hotkeys.start.scan_code.to_string(),
        )
        .set("stop_label", &sanitized.hotkeys.stop.key_label)
        .set(
            "stop_scan_code",
            sanitized.hotkeys.stop.scan_code.to_string(),
        );

    // 写入 [keyboard] section
    let keyboard_json = serde_json::to_string(&sanitized.keyboard_actions)
        .map_err(|e| format!("Failed to serialize keyboard actions: {}", e))?;
    ini.with_section(Some("keyboard"))
        .set("actions", keyboard_json);

    // 写入 [mouse] section
    let mouse_json = serde_json::to_string(&sanitized.mouse_actions)
        .map_err(|e| format!("Failed to serialize mouse actions: {}", e))?;
    ini.with_section(Some("mouse")).set("actions", mouse_json);

    // 原子写：先写到 mimic.ini.tmp，再 rename 替换原文件。
    // Rust std::fs::rename 在 Windows 使用 MoveFileExW + MOVEFILE_REPLACE_EXISTING，
    // 同卷内是原子操作，可覆盖目标文件。
    let mut tmp_os = path.clone().into_os_string();
    tmp_os.push(".tmp");
    let tmp_path = PathBuf::from(tmp_os);

    if let Err(e) = ini.write_to_file(&tmp_path) {
        // 写 tmp 失败时尝试清理残留，不阻塞错误返回
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("Failed to write INI tmp file: {}", e));
    }

    if let Err(e) = std::fs::rename(&tmp_path, &path) {
        // rename 失败时清理 tmp，避免下次启动残留
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("Failed to atomically replace mimic.ini: {}", e));
    }

    Ok(())
}
