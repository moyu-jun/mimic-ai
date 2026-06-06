# Mimic 项目规划

## 目标概述

Mimic 是一个仅面向 Windows 的桌面按键/鼠标模拟工具，技术栈为 Tauri 2 + Rust + Vue 3 + TypeScript。应用通过 Interception 驱动完成游戏场景下的键盘按键和鼠标点击模拟，前端负责配置和状态展示，Rust 后端负责配置持久化、全局热键监听、驱动调用和模拟任务调度。

## 当前项目状态

- 主应用是标准 Tauri 2 + Vue 3 模板，核心文件位于 `src/` 和 `src-tauri/`。
- `src/App.vue` 仍是默认示例页面，需要整体替换为实际应用界面。
- `src-tauri/src/lib.rs` 只有 `greet` 示例命令，需要替换为应用服务层。
- `src-tauri/tauri.conf.json` 当前窗口为 800x600 默认窗口，需要调整为固定大小、无系统标题栏、透明/圆角相关配置。
- `demo/` 是独立 Rust 试验工程，已经验证 Interception 按键发送思路，可作为后续集成参考。

## 关键需求拆分

### 1. 应用窗口和外观

- 仅支持 Windows，构建和运行时不考虑 macOS/Linux。
- Tauri 窗口固定尺寸为 `600x400`，禁止缩放、最大化和拖拽改变大小。
- 使用自定义标题栏：
  - Tauri 配置关闭系统装饰：`decorations: false`。
  - 前端实现标题、运行状态摘要、最小化和关闭按钮。
  - 标题栏拖拽区域必须支持鼠标拖曳窗口，使用 Tauri window API 的 `startDragging()`。
- 应用最外层圆角：
  - 前端根容器使用圆角、深色背景和边框。
  - Windows 桌面透明圆角需要配合 Tauri `transparent: true`，实际效果需要在 Windows 上验证。
- 整体设计：
  - 深色现代风格，参考 Windows 11 暗色主题的质感和配色。
  - 使用 Windows 11 风格的深色背景、柔和的边框、微妙的阴影和圆角设计。
  - 主色使用 Windows 11 暗色主题配色：深灰背景（#202020）、中灰容器（#2b2b2b）、浅灰边框（#3b3b3b）、系统蓝色强调（#0078d4）+ 状态色（绿/黄/红）。
  - 交互使用 120ms-220ms 的 hover、active、page transition、focus 动画。

### 2. 前端页面结构

布局固定为：

- 顶部：自定义标题栏，固定不随页面切换。
- 中间：主区域，左右布局。
  - 左侧：菜单栏。
  - 右侧：当前页面内容。
- 底部：状态栏，固定显示当前运行状态。

菜单：

- 首页
- 按键模拟
- 鼠标模拟
- 设置，固定在左侧菜单底部区域

前端建议文件结构：

```text
src/
  App.vue
  main.ts
  styles/
    base.css
    theme.css
  types/
    config.ts
  stores/
    appStore.ts
  components/
    AppTitleBar.vue
    AppSidebar.vue
    AppStatusBar.vue
    KeyCaptureInput.vue
    IconButton.vue
  pages/
    HomePage.vue
    KeyboardPage.vue
    MousePage.vue
    SettingsPage.vue
```

状态管理可先使用 Vue `reactive/provide` 或轻量 composable，不必引入 Pinia，除非后续状态复杂度明显上升。

### 3. 数据模型

前端与 Rust 共享的核心配置模型：

```ts
type AppPage = 'home' | 'keyboard' | 'mouse' | 'settings'

interface KeyboardAction {
  id: string
  selected: boolean
  keyLabel: string
  scanCode: number
  intervalMs: number
}

interface MouseAction {
  id: string
  x: number | null
  y: number | null
  intervalMs: number
}

interface HotkeyConfig {
  start: CapturedKey
  stop: CapturedKey
}

interface CapturedKey {
  keyLabel: string
  scanCode: number
}

interface AppConfig {
  keyboardActions: KeyboardAction[]
  mouseActions: MouseAction[]
  hotkeys: HotkeyConfig
}
```

Rust 侧用等价 `serde` 结构体。`id` 可由前端生成，也可由 Rust 返回。

`CapturedKey.modifiers` 结论：当前需求不需要。`modifiers` 原本用于支持 `Ctrl+F12`、`Alt+K` 这类组合热键；当前启动/停止热键允许单键且默认都是 `F12`，按键模拟也按单个按键处理，所以先从模型中移除，避免增加映射和注册复杂度。后续如需组合热键，再以独立需求补回。

### 4. INI 持久化

要求：

- 配置文件和 exe 同级，文件名固定为 `mimic.ini`。
- 启动时：
  - 如果文件存在，读取并解析。
  - 如果文件不存在，使用默认配置初始化并立即创建文件。
  - 如果文件损坏，直接使用默认配置覆盖 `mimic.ini`，不备份。
- 修改后实时持久化：
  - 前端数据变更后调用 `save_config`。
  - 前端增加短 debounce，例如 150ms，避免输入间隔时频繁写盘。

建议 INI 结构：

```ini
[hotkeys]
start_label=F12
start_scan_code=88
stop_label=F12
stop_scan_code=88

[keyboard]
actions=[{"id":"default-keyboard-1","selected":true,"keyLabel":"F","scanCode":33,"intervalMs":20}]

[mouse]
actions=[{"id":"...","x":100,"y":200,"intervalMs":20}]
```

说明：INI 天然不适合复杂列表。为了稳定和可维护，列表字段建议使用 JSON 字符串存入 INI；Rust 用 `serde_json` 解析。这样既满足“本地 ini 文件”，也避免自定义数组格式带来的解析风险。

Rust 依赖建议：

- `ini` 或 `configparser`：读写 INI。
- `serde` / `serde_json`：列表字段序列化。
- `anyhow` 或 `thiserror`：后端错误处理。

### 5. 全局热键

要求：

- 应用启动顺序必须是：先加载/初始化 `mimic.ini`，再用加载到的配置注册全局热键。
- 默认热键在无配置时注册，启动热键和停止热键默认都使用 `F12`。
- 启动热键和停止热键允许配置为同一个按键。
- 设置页修改热键后，实时更新全局热键注册。
- 应用最小化后仍能监听。
- 当前页面不是可触发页面时，启动/停止热键不生效。

实现方案：

- 优先使用 Tauri 2 官方全局快捷键插件 `tauri-plugin-global-shortcut` 处理全局热键。
- Rust 后端维护：
  - 当前页面 `current_page`
  - 可触发页面集合，例如 `["keyboard", "mouse"]`
  - 当前运行状态 `Idle | RunningKeyboard | RunningMouse | Stopping | Error`
  - 热键状态 `Idle | Started | Stopping`
  - 已注册热键
- 前端每次页面切换调用 `set_current_page(page)`，后端同步记录当前页面。
- 修改热键调用 `update_hotkeys(config.hotkeys)`：
  - 注销旧热键。
  - 注册新热键。
  - 保存配置。
- 热键生效前必须先判断当前状态：
  - `Idle` 时启动热键才允许生效。
  - `RunningKeyboard` / `RunningMouse` / `Stopping` 时重复启动热键忽略。
  - `RunningKeyboard` / `RunningMouse` 时停止热键才允许生效。
  - `Idle` 时停止热键忽略。
- 当前热键状态和模拟运行状态需要通过状态栏展示。

注意事项：

- Tauri 全局快捷键通常使用按键名称而不是 scan code。前端捕获时需要保存显示名称、Interception scan code，以及后端热键注册可用的 key name。
- 如果必须完全按 scan code 监听，需要在 Rust 侧使用 Windows Raw Input 或 keyboard hook，复杂度和权限风险更高。建议第一版用 Tauri global-shortcut，Interception 仅负责模拟输入。

### 6. 按键捕获

按键输入框交互：

- 输入框获得焦点后进入捕获状态。
- 监听 `keydown`，阻止默认输入。
- 捕获按键显示名和 `event.code`，当前不处理组合键 modifier。
- 对常用键建立映射：
  - 前端显示：`W`、`F12`、`Space`、`Left Shift`
  - Rust Interception scan code：用于模拟
  - Tauri hotkey accelerator：用于热键注册
- 点击添加按钮后写入按键列表，默认 `selected: false`、`intervalMs: DEFAULT_INTERVAL_MS`。
- 列表中勾选和未勾选行需要有明显样式区分，例如透明度、背景色、左侧强调线或 checkbox 状态，但禁止使用文字删除线。

风险：

- 浏览器 `KeyboardEvent.code` 到 Interception scan code 需要维护映射表，尤其是功能键、方向键、小键盘、修饰键。
- 第一版应先支持游戏常用键：字母、数字、F1-F12、Space、Tab、Esc、Shift、Ctrl、Alt、方向键。

### 7. 鼠标坐标拾取

需求：点击坐标拾取按钮后监听鼠标点击位置，并填入该行 X/Y。

建议实现：

- 前端点击“坐标拾取”调用 Rust `start_pick_mouse_position(row_id)`。
- 调用后应用窗口立即隐藏，避免遮挡目标区域。
- Rust 临时启用全局鼠标监听：
  - 可以使用 Windows API low-level mouse hook 获取屏幕坐标。
  - 捕获下一次鼠标点击后停止监听。
  - 通过 Tauri event 发送 `mouse_position_picked` 给前端。
- 拾取完成后 Rust/前端恢复显示应用窗口，并填入对应行的 X/Y，随后持久化。
- 坐标拾取必须先设计稳定接口，再选择底层实现：
  - 第一版实现可采用简单方案，例如隐藏窗口后短暂延迟，再监听下一次全局鼠标点击。
  - 如果第一版方案在游戏窗口、权限、焦点或全屏场景下不可用，只替换 `mouse_picker` 模块内部实现，不改变前端命令和事件协议。
  - 后端接口保持为 `start_pick_mouse_position(row_id)`，事件保持为 `mouse_position_picked`。

注意：

- 坐标应使用屏幕坐标，后续 Interception 鼠标移动/点击也基于屏幕坐标或明确转换。
- 暂时不需要支持多显示器和高 DPI 缩放下的精确点击。

### 8. 模拟执行逻辑

启动热键触发：

- 后端读取当前页面。
- 如果当前页面是 `keyboard`：
  - 获取已勾选的按键列表。
  - 按列表顺序执行 key down / key up。
  - 每个动作后等待该行动作的 `intervalMs`。
  - 循环执行直到停止热键触发。
- 如果当前页面是 `mouse`：
  - 获取有效坐标行。
  - 按列表顺序移动/左键点击。
  - 每个动作后等待该行动作的 `intervalMs`。
  - 循环执行直到停止热键触发。
- 如果当前页面不是可触发页面，则忽略启动/停止热键。

线程模型：

- Rust 使用一个后台 worker thread 运行模拟循环。
- 用 `Arc<AtomicBool>` 或 channel 控制停止。
- 使用 `Arc<Mutex<AppState>>` 存储配置和状态。
- 防止重复启动：运行中再次按启动热键应忽略或更新状态提示。
- 停止热键设置停止标记，worker 尽快退出。
- 启动热键生效后前端进入锁定状态：
  - 显示灰色半透明蒙版。
  - 蒙版只覆盖中间主区域，包括左侧菜单和右侧主内容区。
  - 蒙版不覆盖顶部自定义标题栏和底部状态栏。
  - 蒙版上不显示任何文字、按钮、图标或提示。
  - 运行状态只在底部状态栏显示。
  - 禁止切换菜单。
  - 禁止修改按键模拟、鼠标模拟和设置数据。
  - 停止热键触发并确认 worker 停止后恢复操作。
  - 后端也必须根据运行态拒绝配置保存、热键更新、坐标拾取、页面切换等会改变运行上下文的命令，避免只依赖前端蒙版。

状态栏事件：

- `Idle`：待机
- `ReadyKeyboard`：当前页面可启动按键模拟
- `ReadyMouse`：当前页面可启动鼠标模拟
- `RunningKeyboard`：按键模拟运行中
- `RunningMouse`：鼠标模拟运行中
- `PickingMouse`：正在拾取鼠标坐标
- `Error`：驱动或配置错误

按键模拟规则：

- 每个按键只执行一次 `down + up`。
- `up` 后等待该行配置的间隔时间，再执行下一个按键。
- 列表执行完成后，如果未停止，则继续从第一条重新循环。

### 9. Interception 集成

需要把 `demo/` 中验证过的 Interception 逻辑迁移到 `src-tauri`：

- 在 `src-tauri/Cargo.toml` 加入 `interception` 依赖。
- 创建输入模拟模块：

```text
src-tauri/src/
  lib.rs
  config.rs
  hotkeys.rs
  input/
    mod.rs
    keyboard.rs
    mouse.rs
  state.rs
```

关键点：

- Interception driver 未安装时，应用应正常打开配置界面，但模拟功能不可用，并进入驱动安装引导流程。
- 执行模拟前初始化 Interception；初始化失败返回清晰错误。
- 键盘设备选择不能依赖用户先按任意键。需要设计设备选择策略：
  - 第一版可在启动模拟前懒初始化并尝试选择可用键盘设备。
  - 如 Interception API 必须 wait 设备，可提供初始化提示或后台捕获设备。
- 鼠标模拟需确认 Interception 是否适合绝对坐标点击；如果绝对坐标复杂，可能需要 Windows `SendInput` 移动鼠标，Interception 负责点击。此点实现前需要验证。
- 鼠标模拟第一版只支持左键。

驱动安装和权限要求：

- 驱动安装文件采用 exe 同级目录外置方案，不直接隐藏在主应用安装包内部。
  - 推荐目录：`<exe_dir>/drivers/interception/`。
  - 应用启动时检测该目录下的驱动安装文件是否存在。
  - 这种方案更透明，用户和杀毒软件更容易识别文件来源，也便于你手动替换驱动文件版本。
  - 相比把驱动安装器嵌入二进制或静默释放到临时目录，外置同级目录方案更稳妥，降低被杀毒软件误判为可疑释放驱动/静默安装行为的概率。
- 首次启动时检测 Interception 驱动是否已安装。
- 如果未安装：
  - 应用引导并执行 Interception 驱动安装。
  - 安装完成后弹窗提示用户必须重启电脑。
  - 在用户重启前，模拟功能不可用，状态栏显示驱动待重启/不可用状态。
- 如果驱动检测失败或安装失败，记录错误日志并在界面提示。
- UAC 管理员权限采用可切换策略：
  - 驱动安装流程需要管理员权限。
  - 按键/鼠标模拟运行是否必须管理员权限，需要你后续实机验证。
  - 代码中必须在权限检测、驱动安装、模拟启动位置添加清晰标记，例如 `// ADMIN_POLICY: ...`，方便你手动修改策略。
  - 默认实现建议：驱动安装时请求/要求管理员权限；模拟执行前检测当前权限并记录日志，如后续验证不需要管理员权限，可关闭运行时强提醒。
  - 不建议在无法确认必要性前，每次启动都强制 UAC 弹窗，避免降低使用体验。

### 10. 日志记录

要求：

- Rust 后端需要添加日志，方便后续排查配置文件、热键注册、驱动检测/安装、模拟执行等问题。
- 开发环境打印 `info` 及以上日志。
- 生产环境 release 打包只打印 `error` 级别日志。
- 建议使用 `log` + `tauri-plugin-log` 或 `tracing` + 文件输出。
- 日志建议至少覆盖：
  - 应用启动和配置文件路径。
  - `mimic.ini` 创建、读取、解析失败、覆盖默认配置。
  - 全局热键注册/注销/更新结果。
  - Interception 驱动检测和安装结果。
  - 模拟任务启动、忽略重复启动、停止、异常退出。
  - 鼠标坐标拾取开始、完成、失败。

### 11. Tauri 命令/API 设计

建议暴露命令：

```rust
load_config() -> AppConfig
save_config(config: AppConfig) -> Result<()>
set_current_page(page: String) -> Result<()>
update_hotkeys(hotkeys: HotkeyConfig) -> Result<()>
start_pick_mouse_position(row_id: String) -> Result<()>
stop_simulation() -> Result<()>
get_runtime_status() -> RuntimeStatus
check_interception_driver() -> DriverStatus
install_interception_driver() -> Result<()>
```

建议事件：

```text
runtime_status_changed
mouse_position_picked
config_reloaded
hotkey_registration_failed
simulation_error
driver_status_changed
```

### 12. 实施顺序

1. 基础窗口和 UI 骨架
   - 调整 Tauri 窗口固定尺寸、无系统标题栏。
   - 窗口尺寸固定为 `600x400`。
   - 实现自定义标题栏、可拖曳窗口、菜单、状态栏和四个页面壳。
   - 完成深色主题和基础动画。

2. 前端配置模型
   - 实现按键列表、鼠标列表、设置页热键输入。
   - 实现当前页面全局状态和可触发页面配置。
   - 先用前端本地默认数据跑通交互。

3. Rust 配置持久化
   - 实现 `mimic.ini` 同 exe 目录读写。
   - 损坏时使用默认配置直接覆盖。
   - 接入 `load_config` / `save_config`。
   - 前端修改后 debounce 实时保存。

4. 日志、权限和驱动检测
   - 接入日志系统，区分开发和生产日志级别。
   - 实现管理员权限检测和可切换权限策略，并在关键代码处添加 `ADMIN_POLICY` 标记。
   - 实现 Interception 驱动检测、安装引导和重启提示。
   - 驱动安装文件按 exe 同级 `drivers/interception/` 外置目录读取。

5. 全局热键
   - 接入 Tauri global shortcut 插件。
   - 启动加载默认/INI 热键并注册。
   - 设置页修改后重新注册。
   - 页面不匹配时忽略启动/停止。
   - 支持启动/停止热键为同一个按键，并通过运行状态判断启动或停止。

6. Interception 按键模拟
   - 迁移 demo 中按键发送逻辑。
   - 建立前端按键到 scan code 的映射。
   - 实现按键模拟 worker 和停止机制。
   - 运行中锁定界面并显示无文字蒙版，蒙版只覆盖中间主区域。

7. 鼠标坐标拾取和点击模拟
   - 实现全局鼠标坐标拾取。
   - 先固定坐标拾取接口和事件协议，底层实现可替换。
   - 拾取期间隐藏应用窗口，拾取完成后恢复显示。
   - 实现鼠标点击模拟 worker。
   - 鼠标模拟只支持左键。
   - 第一版暂不处理多显示器和高 DPI 精确点击。

8. 打磨和验证
   - 错误状态、驱动缺失提示、热键冲突提示。
   - 检查 UI 固定尺寸下文字不溢出。
   - Windows 上验证最小化后热键、圆角、透明、Interception 行为。
   - 构建 release 包，确认 `mimic.ini` 在 exe 同级创建。

## 默认配置建议

```json
{
  "keyboardActions": [
    {
      "id": "default-keyboard-1",
      "selected": true,
      "keyLabel": "F",
      "scanCode": 33,
      "intervalMs": 20
    }
  ],
  "mouseActions": [
    {
      "id": "default-mouse-1",
      "x": null,
      "y": null,
      "intervalMs": 20
    }
  ],
  "hotkeys": {
    "start": {
      "keyLabel": "F12",
      "scanCode": 88
    },
    "stop": {
      "keyLabel": "F12",
      "scanCode": 88
    }
  }
}
```

全局默认间隔时间：

```rust
const DEFAULT_INTERVAL_MS: u64 = 20;
```

## 需要提前确认的问题

1. Interception 驱动外置目录下具体包含哪些文件和安装命令，需要在你放入文件后确认。
2. 按键/鼠标模拟是否必须管理员权限，需要你实机测试后决定是否开启运行时强提醒。
3. 坐标拾取第一版方案如果在游戏窗口或全屏场景不可用，需要替换 `mouse_picker` 模块内部实现。

## 风险和约束

- 全局热键注册和 Interception scan code 不是同一个抽象层，必须维护清晰的按键映射。
- Interception 驱动依赖外部系统安装；驱动缺失时应提示而不是导致应用启动失败。
- 游戏和反作弊环境可能拦截或禁止模拟输入，应用应避免承诺所有游戏可用。
- 自定义透明圆角窗口在 Windows/Tauri/WebView2 下需要实机验证，CSS 圆角不一定等同系统窗口圆角。
- INI 存复杂列表建议使用 JSON 字符串，否则后期扩展和兼容成本高。
- `600x400` 窗口空间较小，页面需要采用紧凑布局，避免过多说明文字和大面积卡片。
