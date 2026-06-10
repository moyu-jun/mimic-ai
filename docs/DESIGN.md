# Mimic 设计文档

> 本文档描述技术选型、模块划分、命令/事件协议、数据结构与线程模型。
> 功能需求见 [REQUIREMENTS.md](./REQUIREMENTS.md)，实施顺序见 [TASKS.md](./TASKS.md)。

## 1. 技术栈

- **前端框架**：Vue 3 + TypeScript + Vite
- **桌面运行时**：Tauri 2
- **后端语言**：Rust
- **底层驱动**：Interception（第三方键盘/鼠标模拟驱动）

## 2. 项目结构

```text
mimic-ai/
├── src/                          # 前端代码
│   ├── App.vue                   # 根组件，承载标题栏、菜单、路由容器、状态栏
│   ├── main.ts                   # 前端入口
│   ├── styles/
│   │   ├── base.css              # 全局基础样式、CSS reset
│   │   └── theme.css             # 主题变量（Windows 11 暗色配色）
│   ├── types/
│   │   └── config.ts             # TypeScript 类型定义
│   ├── stores/
│   │   └── appStore.ts           # 全局状态管理（页面、配置、运行状态）
│   ├── components/
│   │   ├── AppTitleBar.vue       # 自定义标题栏
│   │   ├── AppSidebar.vue        # 左侧菜单
│   │   ├── AppStatusBar.vue      # 底部状态栏
│   │   ├── KeyCaptureInput.vue   # 按键捕获输入框组件
│   │   └── IconButton.vue        # 图标按钮（最小化/关闭/删除等）
│   └── pages/
│       ├── HomePage.vue          # 首页
│       ├── KeyboardPage.vue      # 按键模拟页
│       ├── MousePage.vue         # 鼠标模拟页
│       └── SettingsPage.vue      # 设置页
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── lib.rs                # Tauri 主入口，注册命令
│   │   ├── config.rs             # INI 读写与默认配置
│   │   ├── hotkeys.rs            # 全局热键注册与管理
│   │   ├── state.rs              # 全局状态（页面、配置、运行状态、锁）
│   │   ├── input/
│   │   │   ├── mod.rs            # 输入模拟模块入口
│   │   │   ├── keyboard.rs       # 按键模拟（Interception）
│   │   │   └── mouse.rs          # 鼠标模拟（Interception）
│   │   ├── driver.rs             # Interception 驱动检测与安装
│   │   └── mouse_picker.rs       # 鼠标坐标拾取
│   ├── Cargo.toml
│   └── tauri.conf.json           # Tauri 配置
└── drivers/
    └── interception/             # 驱动安装文件外置目录（直接入仓库）
        ├── install-interception.exe
        └── interception.dll
```

## 3. Tauri 配置关键点

**tauri.conf.json** 需要的关键配置：

```json
{
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devPath": "http://localhost:5173",
    "distDir": "../dist"
  },
  "tauri": {
    "windows": [
      {
        "title": "Mimic",
        "width": 600,
        "height": 400,
        "resizable": false,
        "maximizable": false,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": false
      }
    ],
    "bundle": {
      "identifier": "com.mimic.app",
      "active": true,
      "targets": "all",
      "windows": {
        "nsis": {}
      }
    },
    "security": {
      "csp": null
    }
  }
}
```

关键说明：

- `resizable: false` → 禁止用户缩放窗口
- `maximizable: false` → 禁止最大化
- `decorations: false` → 关闭系统标题栏
- `transparent: true` → 支持 CSS 圆角和透明（需要实机验证效果）

## 4. 数据模型（前后端共享）

### 4.1 TypeScript 类型定义

**src/types/config.ts**

```typescript
export type AppPage = 'home' | 'keyboard' | 'mouse' | 'settings'

export interface KeyboardAction {
  id: string
  selected: boolean
  keyLabel: string
  scanCode: number
  intervalMs: number
}

export interface MouseAction {
  id: string
  x: number | null
  y: number | null
  intervalMs: number
}

export interface CapturedKey {
  keyLabel: string
  scanCode: number
}

export interface HotkeyConfig {
  start: CapturedKey
  stop: CapturedKey
}

export interface AppConfig {
  keyboardActions: KeyboardAction[]
  mouseActions: MouseAction[]
  hotkeys: HotkeyConfig
}

export type RuntimeStatus =
  | 'Idle'
  | 'ReadyKeyboard'
  | 'ReadyMouse'
  | 'RunningKeyboard'
  | 'RunningMouse'
  | 'PickingMouse'
  | 'Error'

export type DriverStatus =
  | 'NotInstalled'
  | 'InstalledNeedReboot'
  | 'Ready'
  | 'Error'
```

### 4.2 Rust 结构体定义

**src-tauri/src/config.rs**（部分示例）

> **命名约定（强制）**：所有跨边界（Tauri 命令、事件 payload、INI 内 JSON）共享的结构体必须标注 `#[serde(rename_all = "camelCase")]`，使 Rust snake_case 字段序列化为前端 camelCase（`key_label` → `keyLabel`）。否则 Rust↔前端的命令边界与 INI 中的 JSON 列表都会字段对不上。

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardAction {
    pub id: String,
    pub selected: bool,
    pub key_label: String,
    pub scan_code: u16,
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseAction {
    pub id: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedKey {
    pub key_label: String,
    pub scan_code: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyConfig {
    pub start: CapturedKey,
    pub stop: CapturedKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub keyboard_actions: Vec<KeyboardAction>,
    pub mouse_actions: Vec<MouseAction>,
    pub hotkeys: HotkeyConfig,
}
```

## 5. INI 配置文件格式

**mimic.ini** 结构：

```ini
[hotkeys]
start_label=F12
start_scan_code=88
stop_label=F12
stop_scan_code=88

[keyboard]
actions=[{"id":"default-keyboard-1","selected":true,"keyLabel":"F","scanCode":33,"intervalMs":20}]

[mouse]
actions=[{"id":"default-mouse-1","x":null,"y":null,"intervalMs":20}]
```

**说明**：

- `[hotkeys]` section：平铺字段，直接存取。
- `[keyboard]` 和 `[mouse]` section：`actions` 字段存储 JSON 字符串数组。
- 使用 `serde_json` 序列化/反序列化列表，避免自定义 INI 数组格式。

## 6. Tauri 命令（Rust → 前端调用）

以下命令在 `src-tauri/src/lib.rs` 中注册，前端通过 `invoke()` 调用：

```rust
#[tauri::command]
fn load_config() -> Result<AppConfig, String>

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String>

#[tauri::command]
fn set_current_page(page: String) -> Result<(), String>

#[tauri::command]
fn update_hotkeys(hotkeys: HotkeyConfig) -> Result<HotkeyUpdateResult, String>

#[tauri::command]
fn start_pick_mouse_position(row_id: String) -> Result<(), String>

#[tauri::command]
fn stop_simulation() -> Result<(), String>

#[tauri::command]
fn get_runtime_status() -> RuntimeStatus

#[tauri::command]
fn check_driver_status() -> DriverStatus

#[tauri::command]
fn install_driver() -> Result<(), String>
```

命令设计原则：

- 读写配置：`load_config` / `save_config`
- 页面切换：`set_current_page`（后端记录当前页面以判断热键是否可触发）
- 热键更新：`update_hotkeys`（注销旧热键 → 注册新热键 → 持久化）
- 坐标拾取：`start_pick_mouse_position`（隐藏窗口 → 监听全局鼠标点击）
- 停止模拟：`stop_simulation`（设置停止标记）
- 运行状态查询：`get_runtime_status`
- 驱动管理：`check_driver_status` / `install_driver`

### 6.1 运行态命令守卫（强制，对应需求 3.9）

模拟运行（`RunningKeyboard` / `RunningMouse`）或坐标拾取（`PickingMouse`）期间，所有会改变运行上下文的命令必须在后端直接拒绝并返回 `Err`，不能仅依赖前端蒙版：

- 受守卫命令：`save_config`、`update_hotkeys`、`set_current_page`、`start_pick_mouse_position`、`install_driver`。
- 始终放行：`stop_simulation`、`get_runtime_status`、`check_driver_status`、`load_config`（只读操作，不影响运行上下文）。
- 实现方式：每个受守卫命令进入时先读 `AppState.runtime_status`，若处于上述忙状态则返回固定错误（如 `"busy: simulation running"`），前端据此忽略或提示。

### 6.2 热键更新的双重反馈（对应需求 3.3.4）

设置页保存需要分别提示"注册成功/失败"和"持久化成功/失败"，因此 `update_hotkeys` 返回结构化结果：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyUpdateResult {
    pub changed: bool,        // 与已持久化配置对比是否有变化
    pub registered: bool,     // 新热键注册是否成功
    pub persisted: bool,      // 是否成功写入 mimic.ini
    pub message: Option<String>,
}
```

- `changed == false`：无变化，前端不提示。
- `changed == true`：前端依据 `registered` / `persisted` 显示对应的小文字成功/失败提示。
- 注册失败时不应覆盖原有已注册热键（保持旧热键可用）。

## 7. Tauri 事件（Rust → 前端推送）

Rust 通过 `app.emit()` 发送事件，前端通过 `listen()` 监听：

| 事件名 | Payload | 触发时机 |
|--------|---------|---------|
| `runtime_status_changed` | `{ status: RuntimeStatus }` | 运行状态变化（Idle → Running 等） |
| `mouse_position_picked` | `{ row_id: string, x: number, y: number }` | 坐标拾取完成 |
| `config_reloaded` | `{ config: AppConfig }` | 配置重新加载（如 INI 修复后） |
| `hotkey_registration_failed` | `{ error: string }` | 热键注册失败 |
| `simulation_error` | `{ error: string }` | 模拟运行出错 |
| `driver_status_changed` | `{ status: DriverStatus }` | 驱动状态变化 |

## 8. 按键映射表

前端 `KeyboardEvent.code` 到 Interception scan code 的映射需要维护在前端或后端。

### 8.1 常用按键映射示例

| 显示名 | KeyboardEvent.code | Interception Scan Code | Tauri Hotkey Accelerator |
|--------|---------------------|------------------------|---------------------------|
| A | KeyA | 30 | A |
| F | KeyF | 33 | F |
| Space | Space | 57 | Space |
| F12 | F12 | 88 | F12 |
| Left Shift | ShiftLeft | 42 | Shift |
| Right Shift | ShiftRight | 54 | Shift |
| Left Ctrl | ControlLeft | 29 | Ctrl |
| Right Ctrl | ControlRight | 285 (0x11D) | Ctrl |
| Left Alt | AltLeft | 56 | Alt |
| Right Alt | AltRight | 312 (0x138) | Alt |
| Esc | Escape | 1 | Escape |
| Tab | Tab | 15 | Tab |

**说明**：

- 第一版必须支持字母、数字、F1-F12、Space、Tab、Esc、左右 Shift/Ctrl/Alt。**不支持方向键和组合键。**
- **E0 前缀键处理**（对应反馈 E3）：部分扩展键（如 Right Ctrl、Right Alt）scan code 在 PS/2 协议中需要 E0 前缀。Interception 通过 `stroke.state` 字段的 `INTERCEPTION_KEY_E0` 标志位表达，而非在 scan code 中编码。表中 `285 (0x11D)` 和 `312 (0x138)` 是逻辑标识，发送时需拆分为：
  ```rust
  // Right Ctrl: scan_code=29, state |= INTERCEPTION_KEY_E0
  // Right Alt: scan_code=56, state |= INTERCEPTION_KEY_E0
  ```
- Tauri global-shortcut 使用字符串加速器（如 `"F12"`, `"Ctrl+A"`），不使用 scan code。

### 8.2 映射维护建议

- 前端维护 `code → { label, scanCode, accelerator }` 映射表。
- 按键捕获时查表得到完整信息。
- 热键注册时使用 `accelerator`。
- 模拟执行时使用 `scanCode`。

### 8.3 热键实现方案演进

#### 阶段 12（当前已落地）— `tauri-plugin-global-shortcut`

- 底层 API：Windows `RegisterHotKey`（USER32.dll）。
- 集成方式:`tauri-plugin-global-shortcut` v2 注册 `Shortcut::new(None, Code::*)`,通过插件回调切换 `runtime_status`。
- **能力**：稳定支持字母/数字/F1-F12/Space/Tab/Esc 等"主键"作为独立热键;支持"修饰键 + 主键"组合。
- **限制**(阶段 12 后续修复确认):
  - `RegisterHotKey` 要求至少有一个非修饰键,**不支持单独的 Ctrl / Alt / Shift 作为热键**(注册时会因 `key_to_vk()` 找不到对应 VKCode 而报 `unknown VKCode for ControlLeft / ShiftLeft / AltLeft` 错误,导致全局热键注册失败)。
  - 因此 `keyMap.ts` 与 `scan_code_to_code()` 在阶段 12 修复中临时移除了 ShiftLeft/ShiftRight/ControlLeft/ControlRight/AltLeft/AltRight 的映射,以避免"用户在设置页设置成功但全局热键注册失败"的尴尬状态。
  - 当前可选热键收窄为字母/数字/F1-F12/Space/Tab/Esc。

#### 阶段 13（已落地）— Interception 驱动直接监听 + 按键模拟

**重要纠正**：阶段 13 验收后发现的核心缺陷修复已在"阶段 12-13 核心缺陷修复"部分（CHANGELOG.md § 阶段 12-13 修复）完整记录。此处仅说明设计原则与已验证的正确实现。

- 与按键模拟共用 Interception 驱动：阶段 13 引入 `interception` crate 作为热键监听与按键模拟通道，同一个驱动实例同时承担"输入监听（热键） + 输出发送（模拟）"职责。
- **采用动机**：
  1. **能力完整**：Interception 在内核层拦截所有按键事件，Ctrl/Alt/Shift 作为独立热键自然可识别（无需借助 `RegisterHotKey` 的修饰键语义）。
  2. **架构统一**：热键监听与按键模拟共用同一个 `interception::create_context()`，**两者为独立 context**（非同一对象），避免 "`global-shortcut` + `interception` 双驱动栈"带来的初始化重复、生命周期错配。
  3. **门控更可靠**：Interception 内核层语义清晰，可在驱动层判断按键来源（过滤掉自身模拟产生的回声）。

**核心缺陷修复（阶段 12-13）**：

| 缺陷 | 原因 | 修复 |
|------|------|------|
| **根因 #1：Ready 检测失败** | `check_driver_windows()` 仅查注册表，未验证驱动加载 | 改为先尝试 `Interception::new()`（即 `create_context()`），成功 → `Ready`；失败再查注册表 |
| **根因 #2：热键监听阻塞** | `wait()` 前未调 `set_filter()`，导致 StubKeyStroke 永不返回 | 监听线程循环前调 `set_filter(Filter::KeyFilter(...))`，设置仅监听按键事件 |
| **根因 #3：ScanCode 构造错误** | `information` 字段被误用为 ScanCode，hardcode `ScanCode::Esc` 占位 | 改为 `ScanCode::try_from(u16)` 构造真实 Code；`information` 字段恢复为 0 |
| **根因 #4：死锁** | 监听与 worker 共用同一 context + Mutex，互相争用导致阻塞 | 创建**独立 context**：监听用 `interception_ctx`（含 filter + 阻塞 wait），worker 用 `worker_ctx`（仅 send） |

**关键设计原则**（已验证）：

- **两个独立 Interception context**：
  - `interception_ctx`：监听线程用，初始化时调 `set_filter()` 过滤仅按键，循环内阻塞 `wait()`
  - `worker_ctx`：模拟 worker 用，仅调 `send()` 发送键事件，不阻塞
  - 两者都由 `Arc<Mutex<>>` 持有，线程安全但互不争用
  
- **状态机与门控**：
  - 热键回调在 `config.hotkeys` 与 `current_page` 匹配时发送 `ActionEvent` 到 worker
  - Worker 检查 `stop_flag`，从 channel 接收事件并执行模拟
  - `handle_stop_hotkey` 设置 `stop_flag` 后等待 50ms，确保 worker 收到信号后再切状态

- **驱动未安装兜底策略**：
  - 驱动 `NotInstalled` / `InstalledNeedReboot` 时不启动监听线程
  - 前端首页与设置页文案明确"安装并重启驱动后热键可用"
  - 热键功能完全禁用，无回退方案

- **风险与当前状态**：
  - ✅ **编译验证通过**：`cargo check` + `cargo clippy` 无警告
  - ⏳ **实机测试待进行**：验证热键→模拟端到端工作、修饰键识别、E0 扩展键处理

## 9. 全局状态管理

### 9.1 前端状态

使用 Vue 3 `reactive` + `provide/inject`，或轻量 composable：

```typescript
// src/stores/appStore.ts
import { reactive } from 'vue'
import type { AppConfig, AppPage, RuntimeStatus, DriverStatus } from '@/types/config'

export const appStore = reactive({
  currentPage: 'home' as AppPage,
  config: null as AppConfig | null,
  runtimeStatus: 'Idle' as RuntimeStatus,
  driverStatus: 'NotInstalled' as DriverStatus,
  isLocked: false, // 运行时锁定界面
})
```

### 9.2 Rust 后端状态

**src-tauri/src/state.rs**

```rust
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use serde::{Deserialize, Serialize};
use crate::config::AppConfig;

// 序列化为 PascalCase 字符串，与前端 RuntimeStatus 联合类型一致
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuntimeStatus {
    Idle,
    ReadyKeyboard,
    ReadyMouse,
    RunningKeyboard,
    RunningMouse,
    PickingMouse,
    Error,
}

// 与前端 DriverStatus 联合类型一致
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriverStatus {
    NotInstalled,
    InstalledNeedReboot,
    Ready,
    Error,
}

pub struct AppState {
    pub config: AppConfig,
    pub current_page: String,
    pub runtime_status: RuntimeStatus,
    pub driver_status: DriverStatus,
    pub stop_flag: Arc<AtomicBool>, // 停止标记，worker 线程检查
}

pub type SharedState = Arc<Mutex<AppState>>;
```

> serde 默认对无字段枚举变体序列化为其名字字符串（如 `"RunningKeyboard"`），正好匹配前端的 PascalCase 联合类型，无需额外 rename。

**状态同步**：

- 前端每次页面切换调用 `set_current_page(page)`，后端更新 `current_page`。
- 热键触发时检查 `current_page` 是否属于可触发页面（`keyboard` / `mouse`）。
- **热键与运行状态匹配门控**（对应反馈 L1）：
  - `Idle` 状态时，只有启动热键生效，停止热键被忽略。
  - `RunningKeyboard` / `RunningMouse` 状态时，只有停止热键生效，启动热键被忽略。
  - 状态不匹配时热键回调直接 `return`，不触发任何动作。
- **全局热键不阻塞其他键盘键**（对应反馈 L2）：
  - 全局热键通过 `tauri-plugin-global-shortcut` 注册，走系统级 `RegisterHotKey` API。
  - Interception 仅在模拟 worker 内部用作输出通道，不开启系统级输入拦截。
  - 运行期间用户可正常使用其他键盘键（如聊天、输入）。
- 运行状态变化时通过 `runtime_status_changed` 事件推送给前端。

## 10. 线程模型

- **主线程**：Tauri 命令处理、全局热键监听。
- **Worker 线程**：模拟任务循环（按键/鼠标模拟）。
- **停止机制**：主线程设置 `Arc<AtomicBool>` 停止标记，worker 线程定期检查并退出。

### 10.1 按键模拟 worker 伪代码

```rust
fn run_keyboard_simulation(
    actions: Vec<KeyboardAction>,
    stop_flag: Arc<AtomicBool>,
) {
    let ctx = interception::create_context();
    let keyboard_device = /* 选择键盘设备 */;

    loop {
        for action in &actions {
            if !action.selected { continue; }
            if stop_flag.load(Ordering::Relaxed) { return; }

            send_key_down(ctx, keyboard_device, action.scan_code);
            send_key_up(ctx, keyboard_device, action.scan_code);
            std::thread::sleep(Duration::from_millis(action.interval_ms));
        }
    }
}
```

### 10.2 鼠标模拟 worker 伪代码

```rust
fn run_mouse_simulation(
    actions: Vec<MouseAction>,
    stop_flag: Arc<AtomicBool>,
) {
    let ctx = interception::create_context();
    let mouse_device = /* 选择鼠标设备 */;

    loop {
        for action in &actions {
            if action.x.is_none() || action.y.is_none() { continue; }
            if stop_flag.load(Ordering::Relaxed) { return; }

            move_mouse_to(ctx, mouse_device, action.x.unwrap(), action.y.unwrap());
            send_mouse_click(ctx, mouse_device, MouseButton::Left);
            std::thread::sleep(Duration::from_millis(action.interval_ms));
        }
    }
}
```

## 11. 坐标拾取实现

**设计原则**：接口稳定、底层可替换。

### 11.1 接口协议

- **命令**：`start_pick_mouse_position(row_id: String)`
- **事件**：`mouse_position_picked { row_id, x, y }`

### 11.2 第一版实现方案

- 进入拾取时将 `runtime_status` 置为 `PickingMouse` 并推送 `runtime_status_changed`；此状态下受守卫命令被拒绝（见 6.1）。
- 调用 Windows API `SetWindowsHookExW` 注册 low-level mouse hook (`WH_MOUSE_LL`)。
- 隐藏 Tauri 窗口（`app_handle.get_window("main").unwrap().hide()`）。
- 监听下一次鼠标**左键**点击事件（仅左键触发，右键/中键忽略），获取屏幕坐标（对应反馈 Q4）。
- 取消 hook，恢复显示窗口，状态恢复为 `ReadyMouse`，发送 `mouse_position_picked` 事件。
- 前端收到事件后回填对应行 X/Y 并持久化。
- **第一版约束**（对应反馈 Q10/L13）：仅支持单显示器、标准 DPI 场景，不考虑多显示器坐标错位问题。
- **拾取期间无取消机制**（对应反馈 Q5）：用户只能通过左键点击完成拾取，或关闭窗口退出应用。
- **异常处理**：若拾取过程中发生错误（如 hook 注册失败），状态应恢复为 `ReadyMouse` 并发送 `simulation_error` 事件。

### 11.3 替代方案（如第一版不可用）

- 使用 Interception 监听鼠标事件（需要确认 Interception 是否支持读取鼠标绝对坐标）。
- 使用第三方库如 `rdev` 监听全局鼠标事件。

## 12. Interception 驱动集成

### 12.1 依赖

**Cargo.toml**

```toml
[dependencies]
interception = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
ini = "1.3"
anyhow = "1.0"
tauri-plugin-global-shortcut = "2.0.0-beta"
tauri-plugin-log = "2.0.0-beta"
windows = { version = "0.52", features = ["Win32_UI_Input_KeyboardAndMouse", "Win32_Foundation"] }
```

### 12.2 驱动检测

检测 Interception 驱动是否已安装：

```rust
pub fn check_interception_driver() -> DriverStatus {
    match interception::create_context() {
        Some(_) => DriverStatus::Ready,
        None => {
            // 检查注册表或文件系统判断是否已安装但未重启
            if is_driver_installed_but_not_loaded() {
                DriverStatus::InstalledNeedReboot
            } else {
                DriverStatus::NotInstalled
            }
        }
    }
}
```

### 12.3 驱动安装（更新：runas 静默调用）

**安装流程**（对应反馈 Q2/Q3）：

外置目录：`<exe_dir>/drivers/interception/`，包含以下文件（直接入仓库）：
- `install-interception.exe`（官方安装器）
- `interception.dll`（驱动库文件）

```rust
pub fn install_interception_driver() -> Result<(), String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("No parent dir")?
        .to_path_buf();

    let installer_path = exe_dir
        .join("drivers")
        .join("interception")
        .join("install-interception.exe");

    if !installer_path.exists() {
        return Err("Driver installer not found".to_string());
    }

    // 通过 runas 以管理员身份静默调用安装器
    // 使用 Windows ShellExecuteW with "runas" verb
    // 安装完成后弹窗提示用户必须重启系统
    // 前端显示进度提示（"正在安装驱动..."）
    // ...

    Ok(())
}
```

### 12.4 设备选择策略

Interception 需要选择键盘/鼠标设备，但不能依赖用户先按任意键。

建议策略：

- 启动模拟前懒初始化 Interception context。
- 遍历设备 1-20，调用 `interception::is_keyboard(device)` / `interception::is_mouse(device)` 判断设备类型。
- 选择第一个检测到的键盘设备和鼠标设备。
- 如果无法检测到设备，返回错误并记录日志。

## 13. 日志系统

使用 `tauri-plugin-log` 或 `tracing` + `tracing-subscriber`。

**Cargo.toml**

```toml
[dependencies]
log = "0.4"
tauri-plugin-log = "2.0.0-beta"
```

**lib.rs**

```rust
use tauri_plugin_log::LogTarget;

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([LogTarget::Stdout, LogTarget::LogDir])
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Info
                } else {
                    log::LevelFilter::Error
                })
                .build(),
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

日志示例：

```rust
use log::{info, error};

info!("Application started, config path: {:?}", config_path);
error!("Failed to register hotkey: {}", err);
```

### 13.1 应用启动流程（强制顺序，对应需求 3.6）

Tauri `setup` 钩子中必须按以下顺序执行，热键注册依赖已加载的配置：

```rust
.setup(|app| {
    // 1. 初始化日志
    // 2. 加载/初始化 mimic.ini：存在则解析；不存在写默认；损坏用默认覆盖
    let config = config::load_or_init();          // info: config path
    // 3. 检测 Interception 驱动状态
    let driver_status = driver::check_interception_driver();
    // 4. 用加载到的配置注册全局热键（默认或用户配置）
    hotkeys::register(app.handle(), &config.hotkeys)?;   // 失败发 hotkey_registration_failed
    // 5. 将 config / driver_status / Idle 写入 SharedState
    // 6. 前端就绪后通过事件同步初始状态
    Ok(())
})
```

启动顺序约束：

- 必须先完成第 2 步（配置加载）再做第 4 步（热键注册），不得用硬编码默认热键抢先注册。
- 驱动不可用不阻塞启动；仅将 `driver_status` 标记为非 `Ready`，模拟相关命令在执行前再校验。

## 14. 权限策略与标记

代码中关键位置添加 `// ADMIN_POLICY: ...` 标记，方便后续手动调整策略。

> 当前定稿策略（2026-06-10 更新）：应用启动**不主动请求** UAC，仅驱动安装命令需要管理员；模拟运行（热键监听 + 按键/鼠标 worker）在普通权限下即可工作。下方标记用于后续若再调整策略时快速定位。

示例：

```rust
// ADMIN_POLICY: Application starts WITHOUT admin elevation (no startup UAC)
// ADMIN_POLICY: Driver installation requires admin rights (guarded in install command)
pub fn install_interception_driver() -> Result<(), String> {
    // ...
}

// ADMIN_POLICY: Simulation runtime does NOT require admin (after driver loaded)
pub fn run_keyboard_simulation() {
    // ...
}
```

### 14.1 启动提权配置（2026-06-10 更新：按需提权）

**策略变更**：启动时**不再**主动请求 UAC，应用以普通权限运行。仅「安装驱动」时按需引导提权。

- **启动行为**：
  - 应用进程以调用方权限启动（普通用户即可），不调用 `ShellExecuteW("runas")`。
  - manifest 不含 `requireAdministrator`。
  - 启动后通过 `is_admin()` 检测当前权限，仅作首页状态展示。

- **首页权限状态**：
  - 已授权 → 绿色「管理员权限已授予」。
  - 未授权 → 橙色「管理员权限受限，安装驱动需提权」+「以管理员身份重启」按钮（用户主动点击才触发 UAC）。

- **驱动安装**：
  - `install_interception_driver` 命令入口检查 `is_admin()`，未授权直接返回 `Err("permission_denied")`。
  - 前端收到 `permission_denied` → 提示「请点击上方"以管理员身份重启"按钮」。
  - 用户点击重启按钮 → `request_admin_restart` → `ShellExecuteW("runas")` 触发 UAC → 重启进程为管理员 → 再点「安装驱动」即可正常调度安装器。

- **模拟运行**：
  - 驱动安装并加载后，热键监听与按键/鼠标 worker 均通过 Interception 用户态接口工作，**不需要管理员权限**。
  - 即使应用是普通权限启动，只要驱动已就绪（`DriverStatus::Ready`），所有模拟功能可用。

**实现方式**：保留 `admin::is_admin()` / `admin::restart_as_admin()` 不变；移除 `pub fn run()` 入口处的启动期 UAC 请求逻辑；`install_interception_driver` 与 `reboot_system` 命令保留运行时权限守卫。

### 14.2 阶段 10 落地形态

- 后端模块 `src-tauri/src/admin.rs`，依赖 `windows-sys` 0.59（Win32_Foundation / Win32_Security / Win32_System_Threading / Win32_UI_Shell），仅在 `#[cfg(windows)]` 下提供真实实现。
- `is_admin()` → `OpenProcessToken(GetCurrentProcess, TOKEN_QUERY)` + `GetTokenInformation(TokenElevation)`；任一 API 失败均视为「非管理员」并 `log::warn!`。
- `restart_as_admin()` → `ShellExecuteW` 以 `runas` verb 拉起当前 exe 路径，失败返回错误字符串（用户取消 UAC 也算失败）。
- **启动期不再调用 UAC**（2026-06-10 调整）：`pub fn run()` 入口直接进入 `tauri::Builder`，不再 `is_admin()` + `restart_as_admin()`。setup 阶段仍调用 `is_admin()` 用于首页状态展示与命令守卫。
- Tauri 命令：
  - `get_admin_status() -> bool`（首页 onMounted 调用，用于判断显示绿色「已授予」还是橙色「受限」）
  - `request_admin_restart(app: AppHandle) -> Result<(), String>`：用户在拒绝首次 UAC 后改主意时点击「以管理员身份重启」触发；调度成功后由后端 spawn 一个延迟 200ms 的线程调用 `app.exit(0)`，给前端留出 UI 反馈窗口，避免双开。
- 关键点都加了 `// ADMIN_POLICY:` 标记：`admin.rs` 模块顶部、`run()` 入口启动期检测、`get_admin_status` 命令、`request_admin_restart` 命令。

## 15. 前端样式设计

### 15.1 主题变量

**src/styles/theme.css**

```css
:root {
  /* === 品牌色盘（来源：extra/color_palette.jpg） === */
  --color-graphite-900: #23262C;  /* RGB(35,38,44)   主背景 */
  --color-graphite-700: #3A3F47;  /* RGB(58,63,71)   容器/卡片 */
  --color-cloud-zinc:   #D1D5DB;  /* RGB(209,213,219) 次要文本 */
  --color-safety-orange:#FE7733;  /* RGB(254,119,51)  强调/警告 */
  --color-neon-sprout:  #B1FA63;  /* RGB(177,250,99)  成功/运行 */
  --color-paper-white:  #FFFFFF;  /* RGB(255,255,255) 主文本 */

  /* === 补充色（语义扩展） === */
  --color-graphite-elevated: #2C2F36;  /* 介于 900/700 之间，hover 用 */
  --color-border-default:    #4A4F58;  /* 由 700 提亮一档 */
  --color-border-subtle:     #2F333A;  /* 细分隔线 */
  --color-text-disabled:     #6B7079;  /* 未勾选行、禁用状态 */
  --color-error-warm:        #FF5A4A;  /* 错误状态专用（暖红） */
  --color-accent-hover:      #FF8A4D;  /* Safety Orange hover */
  --color-accent-pressed:    #E5611E;  /* Safety Orange pressed */

  /* === 语义映射 === */
  --bg-primary:     var(--color-graphite-900);
  --bg-secondary:   var(--color-graphite-700);
  --bg-elevated:    var(--color-graphite-elevated);
  --border-color:   var(--color-border-default);
  --border-subtle:  var(--color-border-subtle);
  --text-primary:   var(--color-paper-white);
  --text-secondary: var(--color-cloud-zinc);
  --text-disabled:  var(--color-text-disabled);
  --accent:         var(--color-safety-orange);
  --accent-hover:   var(--color-accent-hover);
  --accent-pressed: var(--color-accent-pressed);
  --success:        var(--color-neon-sprout);
  --warning:        var(--color-safety-orange);
  --danger:         var(--color-error-warm);

  /* Layout */
  --titlebar-height: 40px;
  --sidebar-width: 180px;
  --statusbar-height: 32px;

  /* Animation */
  --transition-fast: 120ms;
  --transition-normal: 220ms;
}
```

### 15.2 圆角与边框

```css
.app-container {
  width: 600px;
  height: 400px;
  background: var(--bg-primary);
  border-radius: 12px;
  border: 1px solid var(--border-color);
  overflow: hidden;
}
```

### 15.3 自定义标题栏

```html
<div class="titlebar" data-tauri-drag-region>
  <!-- 左：应用图标 + 应用名 -->
  <span class="title">
    <img class="app-icon" src="/icon.png" alt="" />
    <span class="app-name">Mimic</span>
  </span>
  <!-- 中：当前页面名，居中显示（需求 3.1） -->
  <span class="current-page">{{ currentPageLabel }}</span>
  <!-- 右：窗口控制按钮 -->
  <div class="titlebar-buttons">
    <button @click="minimizeWindow">−</button>
    <button @click="closeWindow">✕</button>
  </div>
</div>
```

**说明**：

- `data-tauri-drag-region` 属性标记拖拽区域，Tauri 自动处理窗口拖动；当前页面名为静态文本可纳入拖拽区，按钮需阻止拖拽（点击可用）。
- 标题栏布局自左到右：应用图标 + 应用名 `Mimic`、当前页面名（居中）、最小化 / 关闭按钮。
- `currentPageLabel` 为 `AppPage` 到中文页名的映射（如 `home` → "首页"、`keyboard` → "按键模拟"）。
- **运行状态摘要不在标题栏**，统一放在底部状态栏（见 15.4-bis / 需求 3.10）。

### 15.3-bis 底部状态栏

底部状态栏展示当前运行状态摘要（需求 3.1、3.10）：

- **形式**：彩色圆点 + 状态文案
- **颜色映射**（基于新色盘）：
  
  | 状态 | 圆点颜色 | 中文文案 |
  |------|---------|---------|
  | `Idle` | `--text-disabled` (#6B7079) | 待机 |
  | `ReadyKeyboard` | `--text-secondary` (Cloud Zinc) | 当前可启动按键模拟 |
  | `ReadyMouse` | `--text-secondary` (Cloud Zinc) | 当前可启动鼠标模拟 |
  | `RunningKeyboard` | `--success` (Neon Sprout) | 按键模拟运行中 |
  | `RunningMouse` | `--success` (Neon Sprout) | 鼠标模拟运行中 |
  | `PickingMouse` | `--warning` (Safety Orange) | 正在拾取鼠标坐标 |
  | `Error` | `--danger` (#FF5A4A) | 错误 |

- 通过 `runtime_status_changed` 事件实时更新。

### 15.4 首页内容（状态仪表盘，对应需求 3.3.1）

首页采用紧凑状态仪表盘，展示核心状态信息，**不包含步骤引导或跳转按钮**（左侧菜单已足够明显）：

- **顶部区域**：
  - 应用名 + 版本号
  - 固定文案简介（一句话）：如"Windows 按键与鼠标模拟工具"
  
- **管理员权限状态**：
  - ✅ **已授权**：绿色图标 + "管理员权限已授予"
  - ❌ **未授权**：橙色警告图标 + "管理员权限受限，部分功能不可用"
  - 未授权时不阻止应用启动，仅在首页明确标识
  
- **驱动状态卡片**：
  - 展示 `DriverStatus`：
    - `Ready`：绿色 ✅ "Interception 驱动已加载"
    - `NotInstalled`：灰色 ⚠️ "驱动未安装" + **"安装驱动"按钮**（调用 `install_driver`）
    - `InstalledNeedReboot`：黄色 ⚠️ "驱动已安装，需重启系统"
    - `Error`：红色 ❌ "驱动错误"
  - 安装按钮点击后：
    - 后端通过 `runas` 静默调用 `install-interception.exe`
    - 显示进度提示（如"正在安装驱动..."）
    - 安装成功后弹窗提示**必须重启**
  
- **当前热键概览**：
  - 展示启动热键 / 停止热键的 `keyLabel`（只读，编辑在设置页）
  - 格式："启动：F12 | 停止：F12"

### 15.5 运行期锁定蒙版（对应需求 3.9）

```html
<!-- 蒙版仅覆盖中部主区域，绝不覆盖标题栏与状态栏 -->
<div class="main-area">
  <AppSidebar />
  <component :is="currentPageComponent" />
  <div v-if="appStore.isLocked" class="lock-overlay"></div>
</div>
```

- `lock-overlay` 为半透明灰色，绝对定位铺满 `.main-area`（菜单 + 内容），**不含任何文字、图标、按钮**。
- 通过 `pointer-events` 拦截点击，禁止菜单切换与数据编辑。
- 由 `appStore.isLocked` 控制；进入 `RunningKeyboard` / `RunningMouse` 时置真，确认 worker 停止后置假。
- 运行文案只出现在标题栏摘要与底部状态栏，不在蒙版上。

### 15.6 关键前端交互细节（对应需求 3.3.2 / 3.3.3 / 3.3.4）

- **间隔时间输入**：`type="text"` + 输入过滤（仅保留数字、剥离非数字字符），**不使用** `type="number"`（避免浏览器自带的加减步进按钮）；空值时回退到 `DEFAULT_INTERVAL_MS`。失焦时持久化（对应反馈 L4）。
- **滚动条宽度预留**：列表容器固定预留滚动条宽度（如 `scrollbar-gutter: stable;`），使无滚动条 → 有滚动条切换时列表宽度不突变。
- **设置页热键失焦回显**：捕获输入框聚焦时进入捕获并显示提示；若失焦时未捕获到新按键，则回显失焦前的原热键值（捕获前先快照原值）。
- **设置页热键捕获逻辑**（对应反馈 L3）：设置页热键输入框内**允许捕获已注册的全局热键**（如 F11/F12）。全局热键始终注册在系统级别，但输入框获得焦点时，`KeyboardEvent` 在到达 Tauri global-shortcut 监听器之前被前端 `preventDefault()` 拦截，因此可以正常捕获。前端在 `keydown` 事件处理中调用 `event.preventDefault()` + `event.stopPropagation()`，确保热键不会触发其他行为。
- **按键捕获范围外按键处理**（对应反馈 Q7）：用户按了白名单之外的按键（如方向键、PrintScreen）时，**不提示错误，继续等待**下一次按键。
- **热键与按键列表冲突校验**（对应反馈 Q6）：设置页保存热键时，后端检查启动热键和停止热键是否与 `keyboard_actions` 列表中任意 `scan_code` 冲突；冲突时返回错误并拒绝保存，前端显示"热键与按键模拟列表冲突，请更换"。
- **未勾选行视觉差异**（对应反馈 L5）：未勾选行降低透明度至 60% 或灰化背景色，**严禁使用** `text-decoration: line-through`（删除线）。
- **鼠标坐标输入框**（对应反馈 L6）：X/Y 坐标仅作为只读显示元素（`<span>` 或 `readonly` input），不允许手动输入，只能通过"坐标拾取"按钮修改。坐标为 `null` 时显示占位符（如"—"或"未设置"）。
- **按键模拟页添加按钮位置**（对应反馈 L7）：位于捕获输入框旁边；鼠标模拟页添加按钮位于列表下方。

## 16. 默认配置常量

**src-tauri/src/config.rs**

```rust
pub const DEFAULT_INTERVAL_MS: u64 = 20;

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
```

## 17. 错误处理

- 命令返回 `Result<T, String>`，前端捕获错误并展示提示。
- 关键操作失败时记录 error 级别日志。
- 驱动未安装/初始化失败时，不阻止应用启动，仅禁用模拟功能。

## 19. 方案变更记录

> 本节记录重大技术方案调整，与需求变更记录（REQUIREMENTS.md）交叉引用。

### DESIGN-CHANGE-001：热键实现从 global-shortcut 切换至 Interception

**关联需求变更**：REQ-CHANGE-001  
**变更时间**：2026-06-08  
**影响阶段**：阶段 12 → 阶段 13

#### 原方案（阶段 12）

- **技术选型**：`tauri-plugin-global-shortcut`（底层 Windows RegisterHotKey API）
- **能力范围**：支持字母/数字/F1-F12/Space/Tab/Esc 作为独立热键；支持修饰键+主键组合
- **API 限制**：RegisterHotKey 要求至少一个非修饰键，不支持纯修饰键作为独立热键
  - 尝试注册 Left Ctrl / Right Shift 等单独修饰键会失败，因为 `key_to_vk()` 无对应 VKCode
  - 阶段 12 修复中临时移除 keyMap.ts 与 scan_code_to_code() 中的修饰键映射，避免"设置成功但全局热键注册失败"的尴尬状态
- **当前可选热键**：字母/数字/F1-F12/Space/Tab/Esc（修饰键不支持）

#### 新方案（阶段 13）

- **技术选型**：Interception 驱动内核层监听
- **能力范围**：支持**所有按键**作为独立热键，包括修饰键（Left/Right Ctrl/Alt/Shift）
- **架构优势**：
  1. **统一底层**：热键监听与按键模拟共用同一 Interception context，避免"RegisterHotKey + Interception"双驱动栈的初始化重复、生命周期错配、潜在键事件干扰
  2. **完整能力**：内核层拦截所有按键，Ctrl/Alt/Shift 作为独立热键自然可识别，无需借助修饰键语义
  3. **可靠门控**：Interception 语义清晰，可过滤自身模拟产生的回声，阶段 13 worker 与监听共用 context 时此优势更明显
- **取舍与风险**：
  - 强依赖驱动：驱动未安装时热键完全不可用（阶段 12 的 global-shortcut 不依赖驱动）
  - 降级策略：驱动 `NotInstalled` / `InstalledNeedReboot` 时不启动监听线程，前端首页与设置页文案明确"安装并重启驱动后热键可用"
  - CPU 开销：监听所有按键事件相比注册特定热键略增，但 Interception 内核驱动延迟通常 <1ms，热键场景无感知影响

#### 实施细节

**代码变更**：

| 文件 | 改动 | 关键点 |
|------|------|--------|
| [src-tauri/src/hotkeys_interception.rs](../src-tauri/src/hotkeys_interception.rs) | 新建 181 行 | 完整的 Interception 热键监听实现，state_machine 门控，支持所有按键 |
| [src-tauri/src/state.rs](../src-tauri/src/state.rs) | 改 | SendInterception 包装器解决 Send/Sync 问题；AppState 新增 interception_context 字段 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | 导入 hotkeys_interception 模块；启动时初始化 Interception context 并启动监听线程；update_hotkeys 简化为仅做配置更新（无注册失败分支） |
| [src-tauri/src/hotkeys.rs](../src-tauri/src/hotkeys.rs) | 改 | 移除所有 tauri-plugin-global-shortcut API 调用；update_hotkeys 仅做冲突校验、持久化、内存更新 |
| [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) | 改 | 移除 `tauri-plugin-global-shortcut` 依赖；interception 版本确认为 0.1.2 |
| [src/lib/keyMap.ts](../src/lib/keyMap.ts) | 改 | 恢复 Shift/Ctrl/Alt 修饰键映射（Left/Right），scanCode 包含 E0 前缀位 |

**监听线程架构**：

```rust
// 伪代码
loop {
  let strokes = interception::wait(context);
  for stroke in strokes {
    if is_keyboard(stroke.device) {
      let key_code = stroke.code;
      let state = stroke.state;
      
      // 状态机门控
      if check_hotkey_match(&config.hotkeys, key_code, state) {
        handle_hotkey_triggered(key_code, current_page, runtime_status)?;
      }
    }
  }
}
```

#### 验证结果

**构建验证**：
- Rust 编译：`cargo check` 通过（10.25s），无 warning，Interception 链接成功
- TypeScript 编译：`npm run build` 成功，52 模块，CSS 18.61 kB / JS 98.20 kB（gzip 35.4 kB）
- 修饰键恢复：keyMap.ts 包含 ShiftLeft/ShiftRight/ControlLeft/ControlRight/AltLeft/AltRight

**接口兼容性**：
- `update_hotkeys` 命令签名不变，前端调用无感知
- 配置文件格式不变，阶段 12 的 mimic.ini 可直接兼容
- 事件协议不变，监听线程通过 `runtime_status_changed` 推送状态变更

#### 与阶段 13 按键模拟的集成

- **共用 context**：`interception::create_context()` 仅创建一次，为 `Arc<Mutex<Option<Context>>>` 持有
- **监听线程长驻**：贯穿应用生命周期，连续监听按键事件
- **模拟 worker 启停**：仅切换 `stop_flag`，不影响监听线程，可以实现"快速停止模拟"无延迟
- **事件分发**：驱动内核层保证键事件只分发一次，Interception crate 提供的 `consume` 能力可选择性拦截或转发

#### 风险与降级

**风险 1 — Interception 驱动崩溃 / 卸载**：
- 监听线程 `wait()` 返回错误时状态切到 `Error`，推送 `simulation_error` 事件
- 前端显示"驱动异常，请重启应用"提示，用户可选择卸载驱动重新安装或重启系统

**风险 2 — 驱动版本兼容性**：
- 使用 crates.io 提供的 `interception 0.1.2` 版本，已由多个项目验证
- 如需更新版本，需实机完整测试（按键事件、模拟可靠性、修饰键识别）

**风险 3 — 与其他 Interception 客户端的冲突**：
- Interception 设计支持多个客户端同时连接
- 内核驱动通过 `predicate` 机制分发事件，多个客户端可共存但事件分发顺序不确定
- 阶段 13 实机测试需验证是否与杀软、输入法等 Interception 用户进程产生干扰

#### 后续迭代计划

阶段 13 后如需进一步优化：

1. **热键组合支持**（如 Ctrl+Shift+A）：当前实现仅支持单键，若需组合可扩展 `HotkeyConfig` 结构并在监听线程中实现组合判断
2. **热键冲突检测**：与按键模拟列表冲突校验已实现，设置页保存时给出提示
3. **监听线程崩溃恢复**：当前失败时进入 `Error` 状态，后续可考虑自动重连机制（需防止重连风暴）

---
