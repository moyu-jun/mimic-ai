// 全局状态管理 — DESIGN 9.2
//
// 本模块定义应用的全局状态结构，用于在 Tauri 命令间共享运行时信息。
// 状态包括当前页面、配置、运行状态、驱动状态和停止标记。

use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;

// 为 Interception 创建 Send + Sync 包装器
// SAFETY: Interception 内部使用 Windows 内核对象指针，
// 该指针仅在创建它的线程内使用是安全的。
// 我们通过将其包装在 Arc<Mutex<>> 中，确保同一时刻只有一个线程访问它。
pub struct SendInterception(pub interception::Interception);
unsafe impl Send for SendInterception {}
unsafe impl Sync for SendInterception {}

/// 运行状态机 — DESIGN 9.2
///
/// serde 默认将无字段枚举序列化为其名字字符串（如 "RunningKeyboard"），
/// 正好匹配前端的 PascalCase 联合类型，无需额外 rename。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuntimeStatus {
    /// 待机
    Idle,
    /// 当前处于按键模拟页面，可启动按键模拟
    ReadyKeyboard,
    /// 当前处于鼠标模拟页面，可启动鼠标模拟
    ReadyMouse,
    /// 按键模拟运行中
    RunningKeyboard,
    /// 鼠标模拟运行中
    RunningMouse,
    /// 正在拾取鼠标坐标
    PickingMouse,
    /// 驱动或配置错误
    Error,
}

/// 驱动状态 — DESIGN 9.2
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriverStatus {
    /// 驱动未安装
    NotInstalled,
    /// 驱动已安装但需要重启系统
    InstalledNeedReboot,
    /// 驱动已加载，可以使用
    Ready,
    /// 驱动错误
    Error,
}

/// 应用全局状态 — DESIGN 9.2
///
/// 阶段 9 增加 `config_warning`：启动时若 INI 写盘失败，此字段记录原因，
/// 前端可通过 `get_init_warning` 命令读取并在首页展示小字提示。
/// 其余字段在阶段 10-13 接入真实功能后逐步使用，当前先 allow(dead_code)。
#[allow(dead_code)]
pub struct AppState {
    /// 当前配置（从 INI 加载或默认）
    pub config: AppConfig,
    /// 启动时配置写盘失败的原因；None 表示无问题
    pub config_warning: Option<String>,
    /// 当前页面（用于判断热键是否可触发）
    pub current_page: String,
    /// 运行状态机
    pub runtime_status: RuntimeStatus,
    /// 驱动状态
    pub driver_status: DriverStatus,
    /// 停止标记，供 worker 线程检查
    pub stop_flag: Arc<AtomicBool>,
    /// 当前坐标拾取的目标行 ID；Some 表示拾取进行中（PickingMouse 状态下由 listener 读取）
    pub pick_row_id: Option<String>,
    /// 监听专用 context（设置 filter + 阻塞 wait）
    pub interception_listener: Arc<Mutex<Option<SendInterception>>>,
    /// 模拟专用 context（仅 send，非阻塞）
    pub interception_worker: Arc<Mutex<Option<SendInterception>>>,
    /// 按键模拟事件发送器（阶段 13）— DESIGN 8.4
    pub action_tx: SyncSender<crate::keyboard_worker::ActionEvent>,
    /// 鼠标模拟事件发送器（阶段 15）— DESIGN 10.2
    pub mouse_tx: SyncSender<crate::mouse_worker::MouseEvent>,
}

/// 共享状态类型（Arc + Mutex 包装）
pub type SharedState = Arc<Mutex<AppState>>;
