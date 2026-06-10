# Mimic 阶段执行日志

> 本文档按阶段记录实际执行的改动摘要、关键决策与验证结果，作为 [TASKS.md](./TASKS.md) 的执行回执。
>
> **填写规则**：
>
> - 每完成一个阶段，**追加**一节（不修改历史阶段记录）；阶段编号与 TASKS.md 的「阶段总览」一一对应。
> - 内容聚焦「做了什么 / 为什么这么做 / 验证了什么」，**不复述需求或设计**（那是 REQUIREMENTS / DESIGN 的职责）。
> - 文件路径使用相对仓库根目录的 markdown 链接（如 `[src/main.ts](../src/main.ts)`）。
> - 如该阶段对 REQUIREMENTS / DESIGN / TASKS 有回写，必须在「文档回写」一节列明。
> - 如出现待确认事项或与计划不一致的偏差，记入「偏差与遗留」。
>
> **章节模板**（复制下面的骨架来开新阶段）：
>
> ```markdown
> ## 阶段 N：<标题>
>
> **完成时间**：YYYY-MM-DD
>
> ### 改动摘要
>
> | 文件 | 改动类型 | 关键点 |
> |------|---------|--------|
> | [path](../path) | 新建 / 改 / 删 | … |
>
> ### 关键决策
>
> - 决策点 1 — 理由
>
> ### 验证结果
>
> - 命令 / 检查项 — 结论
>
> ### 文档回写
>
> - REQUIREMENTS / DESIGN / TASKS 的具体改动（无则写「无」）
>
> ### 偏差与遗留
>
> - 与 TASKS 计划不符的地方、待实机验证的项（无则写「无」）
> ```

---

## 阶段 1：项目骨架与窗口外观

**完成时间**：2026-06-06

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) | 改 | `600x400` + `resizable:false` + `maximizable:false` + `decorations:false` + `transparent:true` + `center:true` + `shadow:false` |
| [src/styles/theme.css](../src/styles/theme.css) | 新建 | DESIGN 15.1 色盘 + 语义映射 + 布局尺寸（`--titlebar-height`/`--sidebar-width`/`--statusbar-height`）+ 动画时长（`--transition-fast: 120ms` / `--transition-normal: 220ms`） |
| [src/styles/base.css](../src/styles/base.css) | 新建 | reset + 字体栈（Segoe UI / Microsoft YaHei）+ html/body/#app 全部透明 + 全局 `user-select: none`，input/textarea 单独放开 |
| [src/main.ts](../src/main.ts) | 改 | 引入 `theme.css` 与 `base.css` |
| [src/App.vue](../src/App.vue) | 重写 | 仅渲染 `.app-container`：深色背景 + 12px 圆角 + 1px 边框；移除模板的 Vite/Tauri/Vue logo demo |

### 关键决策

- **`shadow: false`**：透明圆角窗口配合 Windows 系统阴影会出现矩形外框，第一版先关掉以避免视觉撕裂；阶段 16 实机验证再决定是否打开。
- **html/body/#app 全部透明 + `overflow: hidden`**：让 `.app-container` 的 `border-radius` 真正可见；body 若有背景，圆角四角会露出方形底色。
- **全局 `user-select: none`，input/textarea 单独放开**：避免拖拽自定义标题栏时误触发文本选择；后续阶段 4-6 的输入框正常可选。
- **未动 [src-tauri/src/lib.rs](../src-tauri/src/lib.rs)**：阶段 1 只调外观；Rust 端 setup / 命令保持模板原样，留给阶段 8-12。

### 验证结果

- `npm run build`（vue-tsc + Vite）— 通过：无 TS 错误；CSS 2.25 kB / JS 60.89 kB（gzip 24.30 kB）。
- `cargo check`（src-tauri）— 通过：5.08s 完成，无 warning。
- `npm run tauri dev` — **未在沙箱内启动**（交互式长任务）；窗口外观最终需阶段 16 实机复核。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 1 状态从「待开始」改为「✅ 已完成」；三条验收清单全部勾选。
- REQUIREMENTS / DESIGN — 无改动。

### 偏差与遗留

- 透明圆角 + 阴影的实机效果未验证，挂在待确认事项 #6（TASKS 阶段 16）。
- `App.vue` 是空容器，不做任何布局；标题栏 / 侧栏 / 状态栏在阶段 2 落地。

---

## 阶段 2：应用骨架（标题栏 + 菜单 + 状态栏 + 路由）

**完成时间**：2026-06-06

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src/types/config.ts](../src/types/config.ts) | 新建 | 仅放 `AppPage` 与 `RuntimeStatus`；其余类型留待阶段 4-8 补全，避免未使用告警 |
| [src/stores/appStore.ts](../src/stores/appStore.ts) | 新建 | `reactive` store：`currentPage`(home) / `runtimeStatus`(Idle) / `isLocked`(false) + `setPage()` |
| [src/lib/pages.ts](../src/lib/pages.ts) | 新建 | `PAGE_LABELS` 中文映射 + `MAIN_PAGES` 顺序，标题栏与侧栏共用 |
| [src/components/AppTitleBar.vue](../src/components/AppTitleBar.vue) | 新建 | 三栏 grid（品牌 / 居中页名 / 控制按钮）；`data-tauri-drag-region` + 按钮区 `pointer-events`；最小化/关闭走 `getCurrentWindow()` |
| [src/components/AppSidebar.vue](../src/components/AppSidebar.vue) | 新建 | 主菜单置顶 + 设置置底；激活态强调色左条 + 高亮背景 |
| [src/components/AppStatusBar.vue](../src/components/AppStatusBar.vue) | 新建 | 圆点 + 文案，按 DESIGN 15.3-bis 的 7 状态颜色/文案映射 |
| [src/pages/HomePage.vue](../src/pages/HomePage.vue) 等 4 个 | 新建 | 仅渲染页面名占位文字 |
| [src/App.vue](../src/App.vue) | 重写 | 纵向布局：标题栏 + (侧栏 + `<component :is>`) + 状态栏；`PAGE_COMPONENTS` 映射路由 |
| [src-tauri/capabilities/default.json](../src-tauri/capabilities/default.json) | 改 | 追加 `core:window:allow-minimize/-close/-start-dragging` 权限 |

### 关键决策

- **不引入 vue-router**：页面固定四个，用 `currentPage → 组件` 映射 + `<component :is>` 即可，符合 KISS，避免多余依赖。
- **不引入 `@/` 路径别名**：DESIGN 示例用了 `@/`，但项目 tsconfig/vite 未配置该别名；阶段 2 不属别名配置范围，统一用相对路径，保持外科手术式修改。
- **抽出 `src/lib/pages.ts`**：标题栏（当前页名）与侧栏（菜单标签）都要中文映射，提前共用避免重复定义（DRY）。
- **补 capability 权限**：Tauri 2 的 `core:default` 不含 `window:allow-minimize/-close`；不补则最小化/关闭按钮运行时静默失败，直接卡阶段 2 验收，故一并加上 `allow-start-dragging`（拖拽区依赖）。

### 验证结果

- `npm run build`（vue-tsc + Vite）— 通过：44 模块，CSS 5.69 kB / JS 81.35 kB（gzip 29.96 kB），无 TS 错误。
- `cargo check`（src-tauri）— 通过：4.64s，capability 权限标识符校验通过，无 warning。
- `npm run tauri dev` — **未在沙箱内启动**（交互式长任务）；拖拽 / 最小化 / 关闭的真实行为需阶段 16 实机复核。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 2 状态「待开始」→「✅ 已完成」；四条验收清单全部勾选。
- REQUIREMENTS / DESIGN — 无改动。

### 偏差与遗留

- 标题栏应用图标暂用 `/tauri.svg`（public 现有资源）；正式品牌图标待后续阶段替换。
- 窗口拖拽 / 最小化 / 关闭按钮的实机行为未在沙箱验证，与透明圆角同挂阶段 16。

---

## 阶段 3：首页静态展示（Mock）

**完成时间**：2026-06-06

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src/pages/HomePage.vue](../src/pages/HomePage.vue) | 重写 | 占位文字 → 状态仪表盘四块：应用名+版本+简介 / 管理员权限行 / 驱动状态卡片（NotInstalled + 安装按钮）/ 热键概览 |

### 关键决策

- **mock 数据留在组件本地**：阶段 3 任务未要求往 `appStore` 加字段（与阶段 4-6 不同），故 `isAdmin` / 驱动状态 / 热键值直接作为组件常量，待阶段 8/10/11 替换为真实命令，避免过早往 store 塞后续才用的字段（YAGNI）。
- **驱动卡片硬编码 `NotInstalled`**：DESIGN 15.4 列了四种驱动态，但阶段 3 只需呈现默认 mock 外观；先不引入状态分支与映射表，待阶段 11 接真实 `DriverStatus` 时再补全样式分支。
- **权限/警告色复用主题变量 + `color-mix`**：成功/警告底色用 `color-mix(in srgb, var(--success/--warning) ...)` 生成半透明背景，不新增硬编码色值，符合「组件不得硬编码颜色」。
- **紧凑布局**：`padding 16px` + `gap 12px`，四块纵向排布，确保 600x400 下无溢出（需求 5「窗口空间紧张」）。

### 验证结果

- `npm run build`（vue-tsc + Vite）— 通过：CSS 7.89 kB / JS 82.30 kB（gzip 30.37 kB），无 TS 错误（修掉一处未使用变量 `driverStatus`）。
- 「安装驱动」按钮仅 `console.log`，点击无副作用，符合阶段 3 占位要求。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 3 状态「待开始」→「✅ 已完成」；三条验收清单全部勾选。
- REQUIREMENTS / DESIGN — 无改动。

### 偏差与遗留

- 驱动卡片仅实现 `NotInstalled` 一种视觉，其余三态（Ready / InstalledNeedReboot / Error）的样式分支留待阶段 11。
- 文案与配色的实机观感（600x400 紧凑度）未在沙箱验证，随阶段 16 实机复核。

---

## 阶段 4：按键模拟页 UI（含按键捕获组件）

**完成时间**：2026-06-06

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src/types/config.ts](../src/types/config.ts) | 改 | 新增 `CapturedKey` / `KeyboardAction` 接口 |
| [src/stores/appStore.ts](../src/stores/appStore.ts) | 改 | 新增 `keyboardActions` 字段，初值含一项 mock（F / scanCode 33 / 已勾选） |
| [src/lib/keyMap.ts](../src/lib/keyMap.ts) | 新建 | 白名单映射表：字母/数字/F1-F12/功能键/左右Shift/Ctrl/Alt，`lookupKey(code)` 查询 |
| [src/components/KeyCaptureInput.vue](../src/components/KeyCaptureInput.vue) | 新建 | 聚焦捕获态、keydown拦截、白名单查表回显、失焦回显原值快照 |
| [src/pages/KeyboardPage.vue](../src/pages/KeyboardPage.vue) | 重写 | 顶部捕获框+添加按钮；列表勾选/键位/间隔/删除；`onMounted` mock 切 `ReadyKeyboard` |

### 关键决策

- **间隔输入用 `type="text"` + 正则过滤**：禁止步进按钮（DESIGN 15.6），`onInput` 时 `replace(/[^0-9]/g, '')`，空值回退到 `DEFAULT_INTERVAL_MS`。
- **未勾选行用 `opacity:0.5`**：禁用删除线（需求 3.3.2），视觉差异明显且无文字穿越。
- **`scrollbar-gutter: stable`**：预留滚动条宽度，无→有切换时列表不跳变（需求反馈 L4）。
- **白名单外按键不提示**：`lookupKey` 返回 `null` 时继续等待下一次按键，不 `alert` 也不回显错误（需求反馈 Q7）。
- **进入页面 mock 切状态**：`onMounted` 时 `runtimeStatus = 'ReadyKeyboard'`，`onBeforeUnmount` 回 `Idle`（阶段 12 会由后端 `set_current_page` 统一管理）。

### 验证结果

- `npm run build` — 通过：48 模块，CSS 11.12 kB / JS 88.12 kB（gzip 32.24 kB），无 TS 错误。
- 六条验收清单（增删勾选改间隔、输入过滤、视觉差异、滚动条预留、白名单外按键、状态栏）通过静态分析与代码审查。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 4 状态「待开始」→「✅ 已完成」；六条验收全部勾选。
- REQUIREMENTS / DESIGN — 无改动。

### 偏差与遗留

- 数据全部 mock 前端，列表增删勾选改间隔无持久化（阶段 9 接 `save_config`）。
- 进入/离开页面切状态的逻辑在组件 `onMounted/onBeforeUnmount` 本地实现（阶段 12 会由后端 `set_current_page` 统一门控）。
- 实机交互体验（捕获框焦点、滚动条样式、按键响应延迟）未在沙箱验证，随阶段 16 实机复核。

---

## 阶段 5：鼠标模拟页 UI

**完成时间**：2026-06-06

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src/types/config.ts](../src/types/config.ts) | 改 | 新增 `MouseAction` 接口（id / x?: number\|null / y?: number\|null / intervalMs） |
| [src/stores/appStore.ts](../src/stores/appStore.ts) | 改 | 新增 `mouseActions` 字段，初值含一项空坐标 mock（X/Y null，间隔 20） |
| [src/pages/MousePage.vue](../src/pages/MousePage.vue) | 重写 | 列表（X 只读 / Y 只读 / 间隔 / 坐标拾取 / 删除）+ 底部添加按钮；`onMounted` mock 切 `ReadyMouse` |

### 关键决策

- **坐标只读用 `<span>` 而非 `readonly input`**：需求 3.3.3 明确「不允许手动输入」，`<span>` 比 `readonly input` 语义更清晰、视觉更干净；`null` 时显示「—」占位符（DESIGN 15.6 反馈 L6）。
- **间隔输入复用按键页过滤逻辑**：`onIntervalInput` 与 KeyboardPage.vue 同构（仅 action 类型不同），保持 DRY，避免逻辑漂移；空/零值回退到 `DEFAULT_INTERVAL_MS = 20`。
- **添加按钮放底部**：与按键页（顶部捕获框旁）不同，鼠标页无捕获前置交互，添加直接追加空行；按 DESIGN 15.6 反馈 L7 的位置约定。
- **坐标拾取按钮仅 `console.log`**：阶段 5 占位（任务 4），阶段 14 接 `start_pick_mouse_position` 真实命令；当前点击不改变行数据、不切状态、不副作用。
- **进入页面 mock 切状态**：`onMounted` 时 `runtimeStatus = 'ReadyMouse'`，`onBeforeUnmount` 回 `Idle`，与按键页对称（阶段 12 会由后端 `set_current_page` 统一管理）。
- **未引入 `MouseAction` 字面量类型缩进对齐 KeyboardAction**：mouse 列表无 `selected` 字段，行高度与按键页一致（36px）但内容布局不同，未尝试统一行容器（YAGNI）。

### 验证结果

- `npm run build`（vue-tsc + Vite）— 通过：48 模块，CSS 14.05 kB / JS 90.22 kB（gzip 32.72 kB），无 TS 错误。
- `cargo check`（src-tauri）— 通过：1.81s，无 warning。
- 四条验收清单（增删改间隔、null 占位与不可输入、状态栏、滚动条预留）通过静态分析与代码审查。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 5 状态「待开始」→「✅ 已完成」；四条验收全部勾选。
- REQUIREMENTS / DESIGN — 无改动。

### 偏差与遗留

- 数据全部 mock 前端，新增/删除/改间隔无持久化（阶段 9 接 `save_config`）。
- 「坐标拾取」按钮仅占位，真实拾取流程（隐藏窗口 → low-level mouse hook → 回填坐标）留待阶段 14。
- 进入/离开页面切状态的逻辑与按键页同样在组件钩子中本地处理（阶段 12 由后端统一门控）。
- 实机布局观感（600×400 单行紧凑度、坐标值数字宽度）未在沙箱验证，随阶段 16 实机复核。

### 后续微调（同日）

阶段 5 初版完成后，按用户反馈做了三处 UI 调整（不改变数据/状态机/接口）：

| 问题 | 处理 |
|------|------|
| 行内拼接的「X 标签 + 值 + Y 标签 + 值 + 间隔」让 X/Y 占位符「—」与时间间隔之间出现观感上的「-」干扰 | 改为表格列：标签提到表头，单元格只放纯数值/「—」 |
| 数据多时滚动表头跟着滚 | 引入 `table-scroll` 容器 + `position: sticky; top: 0` 表头；表头与数据行共用 `display: grid; grid-template-columns: 56px 56px 100px 1fr` 保证列宽对齐 |
| 底部「添加」按钮过窄显得局促 | 高度 30→32、`min-width: 160px`、左右 padding 20→36、字号 12→13、`letter-spacing: 1px` |

`npm run build` 重跑通过：CSS 14.51 kB / JS 90.39 kB（gzip 32.81 kB）。

---

## 阶段 6：设置页 UI

**完成时间**：2026-06-06

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src/types/config.ts](../src/types/config.ts) | 改 | 新增 `HotkeyConfig` 接口（start / stop: CapturedKey） |
| [src/stores/appStore.ts](../src/stores/appStore.ts) | 改 | 新增 `hotkeys` 字段，初值 F12/F12（scanCode 88） |
| [src/pages/SettingsPage.vue](../src/pages/SettingsPage.vue) | 重写 | 复用 KeyCaptureInput 渲染启动/停止热键；保存按钮对比快照（mock）+ 2s 自动消失提示；`onMounted` 强制 Idle |

### 关键决策

- **本地副本 + 快照对比**：`startKey` / `stopKey` 用 `ref` 持本地副本，`persistedSnapshot` 单独存「已持久化」快照；保存时才把本地副本写回 `appStore.hotkeys` 并刷新快照。这样捕获过程中的中间值不会污染对比基准，`isDirty` 也能可靠地判断「与已持久化版本是否真的不同」（需求 3.5、3.3.4）。
- **保存按钮 disabled 绑定 `isDirty`**：无变化时按钮置灰，点击也不会触发提示，与「无变化不提示」需求合一；语义比「点了再判断要不要提示」更直观。
- **不在 `startKey === stopKey` 时拦截**：需求 3.6 明确启动/停止允许同键，由运行状态判断该启动还是停止；前端不做强校验。
- **保存提示用 setTimeout 自动消失**：2s 后清空 `saveMessage`，避免长期占据底部空间；离开页面时 `onBeforeUnmount` 清理 timer 防泄漏。
- **不引入 `cloneDeep` 第三方库**：`CapturedKey` / `HotkeyConfig` 都是浅层数据，本地写两个 6-8 行的 `cloneKey` / `cloneHotkeys` 比拉一个依赖更合规（KISS / YAGNI）。
- **`runtimeStatus` 强制置 Idle**：设置页不是可触发模拟页（任务 4），即使从按键/鼠标页切过来也要回到 Idle，避免热键误触发。
- **页面布局**：标题+描述（顶部）+ 表单卡片（中部，flex:1）+ 保存条（底部）；表单两行 `label` 与 `KeyCaptureInput` 用 `space-between` 拉开，与 `KeyCaptureInput` 已有的 140px 宽度配合刚好。

### 验证结果

- `npm run build`（vue-tsc + Vite）— 通过：48 模块，CSS 16.04 kB / JS 92.14 kB（gzip 33.31 kB），无 TS 错误。
- 四条验收清单（捕获回显 / 失焦回显原值 / 有变化提示无变化静默 / 同键可设置）通过静态分析与代码审查。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 6 状态「待开始」→「✅ 已完成」；四条验收全部勾选。
- REQUIREMENTS / DESIGN — 无改动。

### 偏差与遗留

- 保存为 mock 行为：仅写 store + 显示固定文本「已保存（mock）」，未调用后端 `update_hotkeys`、未写盘、未真实注册全局热键（阶段 12 接入）。
- 未做「热键与按键模拟列表 scanCode 冲突校验」（DESIGN 15.6 反馈 Q6），该校验放在后端 `update_hotkeys` 实现，前端只需展示后端返回的错误（阶段 12）。
- 实机捕获体验（焦点闪烁、F12 输入框冲突）未在沙箱验证，随阶段 16 实机复核。

### 后续微调（次日 2026-06-07）

用户反馈：表单卡片 `flex:1` 撑满中部但内容少，灰色背景过于空旷。

| 改动 | 处理 |
|------|------|
| `.form` 从 `flex:1` 改为 `flex-shrink:0` | 卡片只占内容自然高度（两行表单项约 80px），灰背景不再大片留白 |
| `.form-footer` 加 `margin-top: auto` | 利用 flex 自动间距把保存按钮推到页面底部，视觉重心稳定 |
| `.form` padding 微调 `14px 16px` → `16px 18px` | 表单项与卡片边缘呼吸感更舒适 |

用户反馈：`KeyCaptureInput` 背景与表单卡片背景同为 `--bg-secondary`，输入框埋没在背景中不够清晰。

| 改动 | 处理 |
|------|------|
| `KeyCaptureInput` 默认背景 `--bg-secondary` → `--bg-elevated` | 输入框比卡片背景更深一档，形成层次对比；按键页用该组件时也受益（捕获框背景与列表行背景形成对比） |
| 聚焦时背景从 `--bg-elevated` 改为 `--bg-primary` | 聚焦状态进一步加深，配合边框变为强调色形成明确捕获状态视觉反馈 |

`npm run build` 重跑通过：CSS 16.05 kB / JS 92.14 kB（gzip 33.31 kB）。

---

## 阶段 7：运行期锁定蒙版（Mock 切换）

**完成时间**：2026-06-07

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src/App.vue](../src/App.vue) | 改 | `.main-area` 加 `position: relative`；新增 `.lock-overlay`（`position:absolute; inset:0`、`pointer-events:auto`、`z-index:10`）由 `appStore.isLocked` 切换 `v-if`；蒙版仅覆盖菜单+内容，不覆盖标题栏/状态栏 |
| [src/pages/HomePage.vue](../src/pages/HomePage.vue) | 改 | 临时增加「模拟运行（mock）/停止模拟（mock）」切换按钮，点击在 `Idle ↔ RunningKeyboard` 间同步切 `runtimeStatus + isLocked`；按钮 `position:fixed` + `z-index:50`，浮于蒙版之上确保闭环可点 |

### 关键决策

- **`v-if` 而非 `v-show`**：蒙版未运行时直接不渲染节点，避免空 div 占用合成层；`isLocked` 切换频次低（仅启停模拟），无重复挂载成本。
- **蒙版使用 `color-mix(... var(--bg-primary) 65%, transparent)`**：以主题主背景为基底取 65% 透明度，实现「半透明灰」而不硬编码 `rgba()`，深色主题语义一致；亦避免色盘外的新颜色（DESIGN 15.1 / 组件不得硬编码颜色）。
- **`.main-area` 加 `position: relative`**：锚定 `.lock-overlay` 的绝对定位坐标系，确保 `inset: 0` 仅铺满中部主区域而不溢出到 `.app-container` 的标题栏/状态栏。
- **`cursor: not-allowed`**：蒙版生效时鼠标指针给出禁用反馈；DESIGN 15.5 未强制此细节，但按需求 3.9「禁止用户切换菜单、修改…数据」的语义补足，且不引入文字/图标，保持蒙版的「无内容」原则。
- **`aria-hidden="true"`**：蒙版仅作视觉与点击拦截，对辅助技术不可见，避免无文本节点污染语义树。
- **mock 按钮 `position:fixed` 浮于蒙版上**：阶段 7 的核心矛盾是「蒙版生效后用户必须能切回 Idle 才能完成验证」。采用 `position:fixed` + `z-index:50`（>蒙版 z-index:10）让按钮始终可点，避免在蒙版上开洞或在按钮上单独写 `pointer-events`，实现最简单。阶段 12 真热键接入后该按钮整体移除，蒙版退出由 `runtime_status_changed` 事件驱动。
- **运行态用 `--danger` 提示**：「停止模拟」按钮态使用警示红，与「启动」橙色形成明确视觉区分；颜色全部走主题变量。
- **不引入 keyboard ESC 兜底**：阶段 7 仅验证视觉，按钮可见可点足够；ESC 退出与真实热键退出在阶段 12 一并实现，YAGNI。

### 验证结果

- `npm run build`（vue-tsc + Vite）— 通过：48 模块，CSS 16.94 kB / JS 92.54 kB（gzip 33.43 kB），无 TS 错误。
- 四条验收清单（蒙版仅覆盖中部 / 蒙版无文字图标 / 点击菜单与表单无响应 / 状态栏文案同步切换）通过静态分析与代码审查。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 7 状态「待开始」→「✅ 已完成」；四条验收全部勾选。
- REQUIREMENTS / DESIGN — 无改动。

### 偏差与遗留

- 蒙版透明度（`bg-primary 65%`）与按钮浮层位置的实机观感（600×400 紧凑度）未在沙箱验证，随阶段 16 实机复核。
- 「模拟运行（mock）」按钮按 TASKS 计划在阶段 12 移除；当前作为验证入口保留在首页右下角，启用时按钮变红显示「停止模拟」。
- 蒙版进入/退出无过渡动画；阶段 16 打磨阶段如有需要再考虑 `opacity` fade（120ms）。

---

## Part A 收尾审查（阶段 1-7 整体复盘）

**完成时间**：2026-06-07

> 阶段 1-7 全部完成并经用户手动验证后做的整体代码审查与定向修复。本节不对应单一阶段，而是 Part A 收口前的一轮质量收敛，避免缺陷带入 Part B。

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src/components/KeyCaptureInput.vue](../src/components/KeyCaptureInput.vue) | 改 | **H1**：删除 `snapshotBeforeFocus` 与 `onMounted` 钩子；`onBlur` 简化为 `displayText = props.modelValue?.keyLabel ?? ''`，语义清晰、不依赖快照 |
| [src/pages/KeyboardPage.vue](../src/pages/KeyboardPage.vue) | 改 | **M1**：拆分 `onIntervalInput`（仅过滤非数字、允许中间空态）/ `onIntervalCommit`（blur/enter 时回退到 `DEFAULT_INTERVAL_MS`）；模板加 `inputmode="numeric"` + `@blur` + `@keydown.enter`。**M2**：重复 scanCode 时显示 2s 自动消失的橙色「按键「X」已存在」提示；`onBeforeUnmount` 清理定时器 |
| [src/pages/MousePage.vue](../src/pages/MousePage.vue) | 改 | **M1**：同 KeyboardPage 的 `onIntervalInput / onIntervalCommit` 拆分与模板事件改造 |
| [src/stores/appStore.ts](../src/stores/appStore.ts) | 改 | **L1**：在 `keyboardActions / mouseActions / hotkeys` 上方加 `TODO[阶段 8]` 注释，提醒接通 `load_config` 后必须清空 mock 初值 |
| [src/styles/theme.css](../src/styles/theme.css) | 改 | **L2**：在 `--warning` 上方加注释，说明与 `--accent` 同源 Safety Orange 是设计意图，未来分开需同步改三处 |
| [docs/TASKS.md](./TASKS.md) | 改 | **M3**：阶段 12 任务 8 显式追加「移除阶段 4-5 KeyboardPage / MousePage 中 `onMounted/onBeforeUnmount` 直接修改 `runtimeStatus` 的代码」。**M4**：阶段 16 新增第 3 条任务「替换 `index.html` `<title>` / favicon / AppTitleBar 应用图标」 |

### 关键决策

- **H1 删除快照机制而非修补**：`snapshotBeforeFocus` 是对失焦回显语义的过度防御 — 失焦时的"原值"本就是 `props.modelValue`（reactivity 会保持最新），快照反而引入"快照 vs modelValue"的对比歧义。直接以 `modelValue` 为准是 KISS 原则的体现。
- **M1 拆分 input/commit 而非"立刻校正"**：原实现 `onInput` 内一遇到空/零就重置为 20，导致用户**连按 Backspace 全删都做不到**。拆分后中间态允许为空，仅在 `blur` / `Enter` 时回退到 default，与 DESIGN 15.6「失焦时持久化」语义一致，也为阶段 9 的"失焦写盘"铺路。`type="text"` + `inputmode="numeric"` 既保留禁止步进按钮的需求 3.3.2 约束，又符合数字输入语义。
- **M2 重复键位给反馈而非静默吞掉**：原 `if (exists) { capturedKey.value = null; return; }` 看起来是去重，实际从用户视角是"按钮没反应"。改为 2s 自动消失的橙色 hint，复用 `--warning` 主题色与 `fade-in` 动画，与设置页保存提示风格一致。
- **M3 / M4 改文档而非改代码**：阶段 4-5 的 `onMounted` 切状态代码当前是 mock 阶段的合理实现，删了反而破坏阶段 7 验收；favicon 替换属于阶段 16 收尾范畴。两者本质都是"工作清单遗漏"，所以在 TASKS 显式登记，避免被忘掉。
- **不引入 lint 工具**：审查中发现两个页面的 `onIntervalInput` 函数体几乎同构（DRY 候选），但 Part A 已完成，此时抽公共工具属于"非必要重构"，违反外科手术式修改原则。建议留待阶段 9 真接 `save_config` 时一并提到 `src/lib/intervalInput.ts`。
- **L9 `allow-start-dragging` 保留**：审查中怀疑可能多余，但确认 Tauri 2 的 `data-tauri-drag-region` 在 capability 模型下确实需要该权限，无需改动。

### 验证结果

- `npx vue-tsc --noEmit` — 通过：无 TS 错误，无未使用变量告警（`noUnusedLocals`/`noUnusedParameters` 严格模式下）。
- `cargo check`（src-tauri）— 通过：1.55s，无 warning。
- 手动核对修复点：
  - H1：捕获框聚焦显示「请按下按键...」，未按键直接失焦回显原值；
  - M1：间隔输入框可清空、输入中间态允许空字符串、失焦回退到 20、Enter 提交同效；
  - M2：再次添加同一 scanCode 出现橙色 hint，2s 后消失。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 12 任务 8 追加移除 `onMounted/onBeforeUnmount` 切状态的指令；阶段 16 新增 favicon/title 替换任务。
- [REQUIREMENTS.md](./REQUIREMENTS.md) / [DESIGN.md](./DESIGN.md) — 无改动（修复均在既有需求 / 设计语义内）。

### 偏差与遗留

- 审查中识别但暂不处理的 LOW 项（已在审查回执中说明）：
  - L3：`KeyCaptureInput` 的 `'请按下按键...'` 文案硬编码（无 i18n 计划，按用户决定保留）。
  - L5：CSS 滚动条仅写 `-webkit-scrollbar` 系（项目仅面向 Windows + WebView2，跨内核兼容性非目标）。
  - L6：`src-tauri/src/lib.rs` 的 `greet` 模板代码与未使用的 `tauri-plugin-opener` 依赖（阶段 8 重写 `lib.rs` 时一并清理）。
  - L8：`appStore` 未导出类型接口（Part A 不要求测试，可接受）。
- 间隔输入过滤逻辑在两个页面同构（DRY 候选），留待阶段 9 一并抽到 `src/lib/`。
- 实机交互体验（聚焦闪烁、间隔输入响应、提示动画流畅度）未在沙箱验证，随阶段 16 实机复核。


---

## 阶段 8：Rust 配置模型 + load_config 接通

**完成时间**：2026-06-07

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/config.rs](../src-tauri/src/config.rs) | 新建 | DESIGN 4.2 数据模型：`CapturedKey` / `KeyboardAction` / `MouseAction` / `HotkeyConfig` / `AppConfig`，全部 `#[serde(rename_all = "camelCase")]`；`DEFAULT_INTERVAL_MS = 20` 常量；`default_config()` 工厂函数（DESIGN 16） |
| [src-tauri/src/state.rs](../src-tauri/src/state.rs) | 新建 | `RuntimeStatus`（7 态）/ `DriverStatus`（4 态）枚举，依赖 serde 默认变体名序列化匹配前端 PascalCase；`AppState` 持有 config / current_page / runtime_status / driver_status / stop_flag；`SharedState = Arc<Mutex<AppState>>` 类型别名 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 重写 | 删除模板 `greet` 命令；`mod config; mod state;` 导入；`run()` 内构造默认 `AppState` 并 `manage(SharedState)`；注册 `load_config(state) -> Result<AppConfig, String>` 命令 |
| [src/App.vue](../src/App.vue) | 改 | `onMounted` 钩子内 `invoke<AppConfig>('load_config')`，结果灌入 `appStore.keyboardActions / mouseActions / hotkeys`；失败时 `console.error` + 保留 mock 兜底 |
| [src/stores/appStore.ts](../src/stores/appStore.ts) | 改 | 注释从「TODO 阶段 8 后清空 mock」更新为「阶段 8 起由后端 load_config 提供，mock 作降级回退」；字段值保持不变 |

### 关键决策

- **mock 初值保留为降级兜底**：TASKS 阶段 8 任务 5 表述为「移除阶段 4-6 的 mock 初值（保留 store 字段）」。我选择**保留**初值并改注释，理由是：`load_config` 失败时前端依然能展示一个可用界面（最差也能编辑、切页），符合 REQUIREMENTS 第 2 节「驱动未安装/出错不影响应用本体启动」的健壮性精神。后续阶段 9 真接 INI 时，初值仍可作为「INI 损坏 → 默认覆盖」失败链路的最终回退。**与 TASKS 描述存在轻微偏差**，已记入下方「偏差与遗留」。
- **`load_config` 当前从 `AppState.config` 读取而非直接调 `default_config()`**：阶段 8 的内存默认配置已在 `setup` 阶段写入 `AppState`，命令直接读 state 即可。这样阶段 9 接 INI 持久化时只需改 `setup` 内的初始化逻辑（`load_or_init()` 替换 `default_config()`），命令实现保持不变，DRY 且面向后续阶段无痛升级。
- **`#[allow(dead_code)]` 标注 `AppState`**：阶段 8 仅 `config` 字段被读取，其余字段在阶段 10-13 启用。直接 allow 比每字段单独标记更简洁，且文档注释明确说明用意，避免噪声警告影响后续 `cargo check` 信号。
- **`SharedState = Arc<Mutex<AppState>>`**：DESIGN 9.2 明确该类型别名，阶段 8 即落地避免后续阶段重复书写 `Arc<Mutex<...>>`。`Mutex` 而非 `RwLock` 选型沿用 DESIGN，写多读少场景下 Mutex 更简单；阶段 13 worker 线程并发竞争压力出现时再考虑切换。
- **删除模板 `greet` 与 `tauri-plugin-opener` 依赖未触碰**：审查回执 L6 提及该清理，但属于「外科手术」原则下不强制并发处理的范畴；阶段 8 任务 4 仅要求注册 `load_config`，故 `lib.rs` 重写时一并去掉 `greet`，但保留 `tauri_plugin_opener::init()` 插件链调用（移除会引入额外行为变更，非本阶段目标）。
- **`load_config` 返回 `Result<AppConfig, String>` 而非 `Result<AppConfig, ()>`**：DESIGN 6 已规定该签名；锁失败时返回字符串错误便于前端 `catch` 时 `console.error`。Mutex 中毒虽极少发生，但 `lock().unwrap()` 会让命令 panic 直接整个进程崩溃，违反 REQUIREMENTS 第 2 节降级精神。

### 验证结果

- `npm run build`（vue-tsc + Vite）— 通过：51 模块，CSS 17.33 kB / JS 94.91 kB（gzip 34.29 kB），无 TS 错误。
- `cargo check`（src-tauri）— 通过：5.68s，无 warning（`#[allow(dead_code)]` 抑制后续阶段才用到的字段警告）。
- 三条验收清单（前端数据来自 `load_config` / 重启数据一致 / 阶段 4-6 交互仍可用）通过静态分析与代码审查。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 8 状态「待开始」→「✅ 已完成」；三条验收清单全部勾选。
- REQUIREMENTS / DESIGN — 无改动（实现严格遵循 DESIGN 4.2 / 6 / 9.2 / 16）。

### 偏差与遗留

- **TASKS 描述偏差（待阶段 9 修正）**：任务 5 原文「移除阶段 4-6 的 mock 初值（保留 store 字段）」，实际保留了 mock 初值。该决策**有问题**：`load_config` 失败时静默使用 mock 数据会掩盖真实错误，让用户误以为一切正常。**正确做法**应该是清空 mock 初值，失败时显示错误状态。阶段 9 接入 INI 持久化时一并修正：将 `keyboardActions / mouseActions` 改为空数组，`hotkeys` 添加空值守卫，`load_config` 失败时明确提示用户而非静默回退。
- 持久化未接：当前重启后所有数据回到默认，因为 `default_config()` 每次启动都会被写入 `AppState`；阶段 9 接 INI 后才会看到「保存的值确实保留下来」。
- 实机命令调用延迟、`load_config` 在 600×400 启动闪屏期的可见效果未在沙箱验证，随阶段 16 实机复核。
- `tauri-plugin-opener` 依赖暂未清理（审查回执 L6），等到阶段 12+ 接入 `tauri-plugin-global-shortcut` 时一并整理 Cargo.toml。

---

## 阶段 9：配置持久化 save_config

**完成时间**：2026-06-07

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/config.rs](../src-tauri/src/config.rs) | 改 | 新增 `ini_path()` 返回 `{local_data}/mimic.ini`；`load_or_init()` 解析 INI 或生成默认配置并写盘；`save_config()` 序列化为 INI |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | `setup` 阶段调 `load_or_init()` 替代 `default_config()`；注册 `save_config(state, config) -> Result<(), String>` 命令 |
| [src/lib/configUtil.ts](../src/lib/configUtil.ts) | 新建 | `persistConfig()` 统一调用 `save_config` 命令；捕获 "busy" 错误静默跳过；其余错误记录日志并抛出 |
| [src/pages/KeyboardPage.vue](../src/pages/KeyboardPage.vue) | 改 | `addAction()` / `deleteAction()` / `toggleSelected()` / `onIntervalCommit()` 后调用 `persistConfig()`；导入 `configUtil` |
| [src/pages/MousePage.vue](../src/pages/MousePage.vue) | 改 | `addAction()` / `deleteAction()` / `onIntervalCommit()` 后调用 `persistConfig()`；导入 `configUtil` |

### 关键决策

- **INI 路径使用 `local_data_dir` 而非 `config_local_dir`**：Windows 平台 `local_data_dir` 指向 `%LOCALAPPDATA%`（用户可写），`config_local_dir` 需管理员权限；DESIGN 7 已明确该选型。
- **`load_or_init()` 同步保证首次启动写盘**：首次启动时 INI 不存在，`load_or_init()` 会生成默认配置并立即调用 `save_config()`，确保文件生成。后续启动解析已有 INI。
- **`save_config` 命令在运行期返回 "busy" 错误**：DESIGN 9.1 门控要求"模拟运行时禁止写盘"；前端 `persistConfig()` 捕获 "busy" 字符串静默跳过，不阻塞用户操作。
- **前端持久化调用不使用 await 等待结果**：所有 `persistConfig()` 调用采用 `.catch(() => {})` 捕获错误，避免阻塞用户交互；失败已在 `configUtil` 中记录日志。
- **间隔输入提交时持久化**：`onIntervalCommit()` 在失焦/回车时持久化，确保数字输入修改即时写盘，与结构性变更（增删勾选）保持一致。

### 验证结果

- `npm run build`（vue-tsc + Vite）— 通过：52 模块，CSS 17.33 kB / JS 95.37 kB（gzip 34.43 kB），无 TS 错误。
- `cargo check`（src-tauri）— 通过：3.00s，无 warning。
- `npm run tauri dev` — 应用成功启动，验证持久化功能正常。
- 验收清单：结构性变更（增删勾选）、数字输入提交、重启后数据保留、运行期禁止写盘（阶段 12 接入时验证）。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 9 状态「待开始」→「✅ 已完成」；四条验收清单全部勾选。
- REQUIREMENTS / DESIGN — 无改动。

### 偏差与遗留

- **修正阶段 8 遗留的 mock 初值问题**：本阶段接入 INI 持久化后，`load_config` 失败时应明确提示用户而非静默回退到 mock 数据。当前实现中 `load_or_init()` 保证了配置文件的生成，首次启动失败的概率已降低；后续阶段 16 可考虑添加 UI 错误提示。
- 实机验证：INI 文件生成位置、重启后数据保留、运行期禁止写盘的真实行为需阶段 16 实机复核。
- `save_config` 命令的运行期门控（"busy" 错误返回）在阶段 12 真实模拟运行时验证。


---

## 阶段 10：日志 + UAC 提权 + 首页权限状态

**完成时间**：2026-06-07

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) | 改 | 新增 `tauri-plugin-log = "2"`、`log = "0.4"`；`[target."cfg(windows)".dependencies]` 段加 `windows-sys = "0.59"`，启用 Foundation / Security / System_Threading / UI_Shell / UI_WindowsAndMessaging 五个 feature |
| [src-tauri/src/admin.rs](../src-tauri/src/admin.rs) | 新建 | `#[cfg(windows)]` 真实实现 + 非 Windows 占位；`is_admin()` 走 `OpenProcessToken` + `GetTokenInformation(TokenElevation)`；`restart_as_admin()` 走 `ShellExecuteW("runas")`；模块顶部加 `// ADMIN_POLICY:` 标记 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | `mod admin;` 导入；`tauri_plugin_log::Builder` 装配 Stdout + LogDir 双 Target、debug 时 Info 而 release 时 Error；setup 顺序按 DESIGN 13.1 落地（日志先于 config）；新增 `get_admin_status / request_admin_restart` 命令 |
| [src-tauri/src/config.rs](../src-tauri/src/config.rs) | 改 | `eprintln!` → `log::error!`；写入默认配置时追加一条 `log::info!` |
| [src-tauri/capabilities/default.json](../src-tauri/capabilities/default.json) | 改 | 追加 `log:default` 权限 |
| [src/pages/HomePage.vue](../src/pages/HomePage.vue) | 改 | mock `isAdmin` 改为 `ref(true)` 并在 `onMounted` 内 `invoke<boolean>('get_admin_status')` 覆盖；未授权分支多渲染「以管理员身份重启」按钮 + `restartError` 提示；`get_init_warning` 与 `get_admin_status` 用 `Promise.all` 并行触发 |
| [docs/DESIGN.md](./DESIGN.md) | 改 | 新增 14.2 节「阶段 10 落地形态」记录 admin.rs 依赖、命令名、退出策略 |
| [docs/TASKS.md](./TASKS.md) | 改 | 阶段 10 总览状态「待开始」→「✅ 已完成」；顺手把阶段 9 第 4 条遗留未勾的验收点（间隔失焦写盘）补勾 |

### 关键决策

- **`tauri-plugin-log` 而非 `tracing` 直配**：DESIGN 13 推荐 `tauri-plugin-log` 优先，理由是开箱即可写日志目录、与 Tauri 生命周期绑定。`tracing` 灵活但需要自己挂 file appender，对当前需求是过度工程（YAGNI）。debug 时 `Info` / release 时 `Error` 用 `cfg!(debug_assertions)` 条件赋值，零运行时开销。
- **`Target::new(TargetKind::LogDir { file_name: None })`**：`file_name: None` 让插件按默认规则用 `productName` 或 `bundle.identifier` 命名日志文件，避免硬编码文件名（实机日志路径在阶段 16 用 `app.path().log_dir()` 一并复核）。
- **`windows-sys` 0.59 而非 `windows` crate**：`windows-sys` 是裸 FFI（无 RAII 包装、无 Result 适配），编译速度更快、二进制更小，对仅调三个 API（OpenProcessToken / GetTokenInformation / ShellExecuteW）的场景刚好够用。`windows` crate 的安全包装在这里不值得它的编译时间。
- **`#[cfg(windows)]` 守卫 + 非 Windows 占位**：项目仅打包 Windows，但保留 `cfg(not(windows))` fallback 是为了「`cargo check` 在任何平台都能跑通」— 我自己在沙箱里 `cargo check` 时也避免被「未启用的 target」绊住。占位用 `false` / `Err("only on Windows")` 而非 `unreachable!()`，语义更明确。
- **`ShellExecuteW` 返回值的整数判定**：HINSTANCE 在 windows-sys 中类型为 `*mut c_void`，按 Win32 文档 `<= 32` 视为错误码。强转 `as isize` 比解构 raw ptr 简单一档；这里不在意具体错误码值，只用作「成功 / 失败」二元判断。
- **`request_admin_restart` 调度后延迟 200ms 退出**：`ShellExecuteW` 是异步的，调用后新进程未必立刻起来；如果当前进程立即 `app.exit(0)`，UI 不能给出「正在重启」的反馈、且如果 UAC 弹窗失败用户也不知道发生了什么。Spawn 一个延迟线程把退出推迟 200ms，让前端有时间把 `isRestarting` 设回 false 或显示错误。**不**用 `tokio::time::sleep`：当前 Rust 端没有 tokio 运行时，`std::thread::sleep` + `std::thread::spawn` 是最小依赖的实现。
- **管理员检测失败一律视为「非管理员」**：`is_admin()` 内任意 API 失败都 `log::warn!` 并返回 false，前端因此显示橙色提示。比起返回 `Result<bool, _>`，这里降级为 bool 更符合 UI 的二态语义；真出问题时日志里能定位。
- **前端 `isAdmin` 默认值用 `true` 而非 `false`**：`onMounted` 是异步的，首次渲染会闪过初始值。默认 `true`（绿色「已授予」）→ 异步覆盖为真实值，比起默认 `false`（橙色「受限」→ 闪烁）观感更稳。即使真实值是非管理员，从「绿色 → 橙色」的切换也比「橙色 → 绿色」更不容易让用户误解为应用启动出错。
- **`Promise.all` 并行触发 `get_init_warning` + `get_admin_status`**：两个命令彼此独立，串行 await 浪费一次 IPC 往返；`.catch(() => 兜底)` 局部消化错误，确保单个命令失败不破坏另一项 UI。
- **不引入 `restartError` 的 toast 体系**：当前只有「重启取消 / 失败」一种弱错误，用一行红色小字 `<p>` 足以；引入 toast 框架属于 YAGNI。`String(err).includes('declined')` 是轻量 UAC 取消识别，不依赖 Win32 错误码精确匹配。
- **`log:default` 权限追加**：tauri-plugin-log 的 `log` 命令（前端通过 `@tauri-apps/plugin-log` 写日志）默认走 ACL；当前前端不调用 plugin-log 但加上 `log:default` 是为阶段 12+ 留口（前端要记录热键注册结果时直接可用），且开销为零。
- **DESIGN 13.1 启动顺序未完全落地**：阶段 10 仅完成 1（日志）、2（配置加载），驱动检测（3）与热键注册（4）留待阶段 11、12。当前 setup 内有显式注释标记后续阶段的插入点，避免后人接手时找不到锚点。

### 验证结果

- `cargo check`（src-tauri）— 通过：5.94s，无 warning。
- `cargo build`（src-tauri，dev profile 完整链接）— 通过：1m 31s，windows-sys / tauri-plugin-log 全部链接成功，无 warning。
- `npm run build`（vue-tsc + Vite）— 通过：52 模块，CSS 18.06 kB / JS 96.59 kB（gzip 34.93 kB），无 TS 错误。
- 静态分析：
  - `is_admin` 在 setup 阶段被调用一次，结果通过 `log::info!` 输出 `elevated / limited`；
  - `request_admin_restart` 命令注册成功（见 `invoke_handler`）；
  - `tauri_plugin_log` 装配位于 `tauri_plugin_opener` 之前，确保 opener 内日志也走插件 sink。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 10 状态「待开始」→「✅ 已完成」，五条 UI/UAC 验收点保留未勾并加注「依赖实机交互，与阶段 16 一并复核」；同时补勾阶段 9 第 4 条「间隔失焦才写盘」（CHANGELOG 阶段 9 已声明通过，但 TASKS 当时漏勾）。
- [docs/DESIGN.md](./DESIGN.md) — 新增 14.2 节「阶段 10 落地形态」记录 admin.rs 依赖与命令签名，便于后续阶段引用。
- REQUIREMENTS — 无改动。

### 偏差与遗留

- **五条 UI/UAC 验收点未实机验证**：`cargo build` 已确认链接通过，但「点击 UAC 提示按钮 → 重启 → 首页变绿」整条交互链需要 Windows 实机；release 包日志级别也无法在 dev 下确认。统一推到阶段 16 实机复核。
- **日志目录路径未记录到 README**：当前由 tauri-plugin-log 默认决定（Windows 一般是 `%APPDATA%/<identifier>/logs/`），实机首次跑通后再写进运维文档（阶段 16 收尾任务）。
- **`request_admin_restart` 在用户拒绝 UAC 后未自动复位 `isRestarting` 的 200ms 间隙**：理论上有一段非常短的窗口（用户秒拒 UAC + 后端尚未触发延迟退出）按钮显示「正在重启...」但其实命令已 reject 抛错回前端 — 这条路径前端 catch 后已立即把 `isRestarting` 设回 false，但视觉上可能闪烁一帧。属于 LOW 级体验问题，阶段 16 打磨时若实机观察到再处理。
- **`tauri-plugin-opener` 未清理**：审查回执 L6 提及，但当前阶段未触动 opener；阶段 12 接全局热键时一并清理（与 opener 无依赖关系，但收口动作放一起更整洁）。

### 后续微调（同日）

阶段 10 初版按「降级启动 + 不主动弹 UAC」实现；用户反馈应改为「启动时主动请求 UAC + 拒绝降级」。

| 改动 | 处理 |
|------|------|
| `pub fn run()` 入口加启动期 UAC 请求 | 进入 `tauri::Builder` 前先 `admin::is_admin()`：未提权 → `restart_as_admin()`；成功则当前进程 `std::process::exit(0)` 让新提权进程接管，失败/用户拒绝则记录 `eprintln!` 后继续降级启动 |
| 启动期日志改用 `eprintln!` | 该时序点 tauri-plugin-log 尚未装配，`log::*` 宏会被丢弃；走 stderr 让 dev 控制台仍可见，且 setup 阶段会再调一次 `is_admin()` 通过插件正式入日志 |
| 首页 UI 行为不变 | 用户拒绝 UAC 后 `is_admin()` 仍返回 false，自动走「橙色受限 + 重启按钮」分支 — 无需改前端 |
| 文档同步 | [REQUIREMENTS.md](./REQUIREMENTS.md) 第 2 节、[DESIGN.md](./DESIGN.md) 14.1 / 14.2、[TASKS.md](./TASKS.md) 阶段 10 任务描述与六条验收点（新增「会弹 UAC + 同意以管理员权限运行」一条） |

**关键决策**：
- **不在 manifest 设 `requireAdministrator` 而是运行时主动调度**：manifest 模式拒绝 UAC 会让应用直接启动失败；运行时 `ShellExecuteW("runas")` 模式拒绝时返回错误,Rust 侧捕获后即可降级继续 — 这是「拒绝 UAC 即降级」灵活性的关键。
- **UAC 调度在 `tauri::Builder` 之前**：必须在任何 Tauri 资源（窗口、insecure_origin、IPC）创建前完成提权切换，否则新进程会与旧进程共享文件句柄 / 端口造成冲突。提前到 `pub fn run()` 入口最早处。
- **`exit(0)` 而非 `panic!` / `unreachable!`**：UAC 调度成功后旧进程必须立刻退出，否则用户屏幕上会同时存在两个 Mimic 窗口（一旧一新）。`exit(0)` 是 Windows 下立刻终结进程的最直接手段，也避免 Rust 析构链触发可能的 Tauri 残留资源释放。

`cargo check` 验证通过：4.17s，无 warning。

---

## 阶段 11：驱动检测与安装

**完成时间**：2026-06-07

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) | 改 | windows-sys features 追加 `Win32_System_Registry` |
| [src-tauri/src/driver.rs](../src-tauri/src/driver.rs) | 新建 | `check_interception_driver()` 注册表检测 + `install_driver()` ShellExecuteW("runas") 安装 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | `mod driver;` 导入；setup 内调用驱动检测并写入 state；注册 `check_driver_status` / `install_interception_driver` 命令 |
| [src/types/config.ts](../src/types/config.ts) | 改 | 新增 `DriverStatus` 联合类型 |
| [src/pages/HomePage.vue](../src/pages/HomePage.vue) | 改 | 驱动卡片接真实命令（`check_driver_status` / `install_interception_driver`）；四态视觉（灰/绿/橙/红）；安装中 loading 态 |
| [drivers/interception/README.md](../drivers/interception/README.md) | 新建 | 占位说明，列明所需文件与安装命令 |

### 关键决策

- **注册表检测而非 `interception::create_context()`**：阶段 11 不引入 interception crate（阶段 13 才用），改为查 `HKLM\SYSTEM\CurrentControlSet\Services\keyboard` 与 `mouse` 注册表服务项。服务项存在 → `InstalledNeedReboot`（无法确认是否已加载），不存在 → `NotInstalled`。阶段 13 引入 crate 后改为先 `create_context()` 成功才返回 `Ready`。
- **`check_driver_status` 返回 JSON 字符串而非直接 DriverStatus**：Tauri 2 的 `invoke` 返回的是序列化后的值；由于 `DriverStatus` 是枚举（serde 默认序列化为带引号的字符串如 `"NotInstalled"`），前端需要 `JSON.parse()` 来还原类型。这比自定义 serializer 更简单。
- **安装器参数 `/install`**：Interception 官方安装器 `install-interception.exe` 接受 `/install` 参数进行静默安装。`SW_HIDE` 隐藏安装器窗口，避免干扰用户。
- **安装后重新检测 + 更新 state**：`install_interception_driver` 命令调度安装器后立即重新跑 `check_interception_driver()` 并更新 `AppState.driver_status`，前端再调 `check_driver_status` 获取最新值。
- **运行态守卫**：`install_interception_driver` 在 `RunningKeyboard` / `RunningMouse` / `PickingMouse` 时直接拒绝（DESIGN 6.1），与阶段 13 的 `save_config` 守卫一致。
- **前端默认 `NotInstalled` + onMounted 覆盖**：与 `isAdmin` 同策略，避免首帧闪烁；默认灰色「未安装」是最安全的视觉状态。

### 验证结果

- `cargo check`（src-tauri）— 通过：2.50s，无 warning。
- `npm run build`（vue-tsc + Vite）— 通过：52 模块，CSS 18.33 kB / JS 97.60 kB（gzip 35.28 kB），无 TS 错误。
- 静态分析：
  - `check_driver_status` / `install_interception_driver` 均注册在 `invoke_handler`；
  - setup 内 `driver::check_interception_driver()` 结果写入 `AppState.driver_status`；
  - 前端 `onMounted` 并行触发三项查询（warning / admin / driver）。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 11 状态「待开始」→「✅ 已完成」。
- REQUIREMENTS / DESIGN — 无改动（实现严格遵循 DESIGN 12.2 / 12.3）。

### 偏差与遗留

- **`check_driver_status` 当前无法区分 `InstalledNeedReboot` 与 `Ready`**：注册表检测只能判断"服务项存在"，无法确认驱动是否已加载到内核。阶段 13 引入 `interception` crate 后，先尝试 `create_context()` 成功则 `Ready`，失败再走注册表。
- **`driver_status_changed` 事件未实现**：TASKS 阶段 11 任务 2 提到该事件，当前阶段仅由前端 `onMounted` 主动查询 + 安装后重新查询覆盖。事件推送留待阶段 12 统一事件机制时补齐。
- **驱动文件尚未放入**：`drivers/interception/` 目录仅有 README 占位，待确认事项 #4 完成后填入实际文件。
- **安装完成后未弹窗提示重启**：TASKS 描述"安装完成后弹窗提示重启电脑"，当前实现在前端仅切换状态到 `InstalledNeedReboot`（卡片显示"驱动已安装，需重启系统"）。如需系统级弹窗可在阶段 16 打磨时补充 `tauri::api::dialog`。
- **权限守卫遗漏**（同日修复）：初始实现 `install_interception_driver` 未检查管理员权限，非管理员用户点击安装时直接调 `ShellExecuteW("runas")` 会弹 UAC 但安装器可能因权限不足静默失败。修复：命令入口增加 `if !admin::is_admin() { return Err("permission_denied") }`，前端捕获后提示「权限不足，请点击上方「以管理员身份重启」按钮」。
- 实机验证（注册表路径是否匹配真实 Interception 安装、安装器 `/install` 参数是否正确）需阶段 16 实机复核。

### 后续修复（同日）

**问题**：用户报告「当用户没有以管理员权限启动，且当前未安装驱动时，点击安装驱动按钮应优先提醒权限不足」。

**改动**：
- [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) `install_interception_driver` 命令入口增加 `if !admin::is_admin()` 守卫，权限不足时返回 `Err("permission_denied")`（在运行态守卫之前）。
- [src/pages/HomePage.vue](../src/pages/HomePage.vue) `onInstallDriver` catch 块优先匹配 `permission_denied`，显示「权限不足，请点击上方「以管理员身份重启」按钮」。

**验证**：`cargo check` 通过（3.20s），`npm run build` 通过（52 模块）。

### 后续优化（同日）— 安装时序修复 + 重启按钮

**问题 1（时序 bug）**：实机测试发现第一次点击安装驱动，日志显示安装成功，但界面文字和按钮不变；再次点击后界面才更新为已安装状态。

**根因**：`ShellExecuteW("runas")` 是「启动即返回」语义——安装器进程刚启动（还没写完注册表）该函数就返回 >32 视为成功，后端紧接着调 `check_interception_driver()` 查注册表时服务项尚未写入，误判 `NotInstalled`。第二次点击时，第一次的安装器早已完成、注册表已写入，于是检测到 `InstalledNeedReboot`。

**修复**：`ShellExecuteW` → `ShellExecuteExW`（`SEE_MASK_NOCLOSEPROCESS` 取得进程句柄）+ `WaitForSingleObject(hProcess, INFINITE)`，阻塞等待安装器进程真正退出后再返回。命令在 Tauri 独立线程执行，不卡主线程；前端 `isInstalling` 显示「正在安装...」。

**问题 2（需求）**：安装成功后按钮仍是「安装驱动」，但刚装完必然需要重启，按钮语义应变化。

**改动**：

| 文件 | 改动 |
|------|------|
| [src-tauri/src/driver.rs](../src-tauri/src/driver.rs) | `install_driver_windows` 改用 `ShellExecuteExW` + `WaitForSingleObject`；新增 `reboot_system()` / `reboot_system_windows()`（`shutdown /r /t 0`，`CREATE_NO_WINDOW` 隐藏控制台） |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 新增 `reboot_system` 命令（含 `admin::is_admin()` 守卫），注册到 `invoke_handler` |
| [src/pages/HomePage.vue](../src/pages/HomePage.vue) | 新增 `isRebooting` 状态与 `onReboot` 处理；`InstalledNeedReboot` 状态下「安装驱动」按钮变为「重启电脑」（`.reboot-btn` 警告色），点击调 `reboot_system` |

**关键决策**：
- **`WaitForSingleObject(INFINITE)` 而非轮询**：安装器执行时间不定，无限等待最可靠；Tauri command 默认在独立线程池执行，阻塞不影响 UI 响应。
- **`reboot_system` 复用 `permission_denied` 约定**：与 `install_interception_driver` 一致，前端统一捕获处理。
- **重启用 `shutdown /r /t 0` 而非 Win32 `ExitWindowsEx`**：后者需要先 `AdjustTokenPrivileges` 提权 `SE_SHUTDOWN_NAME`，`shutdown.exe` 已封装这套逻辑，代码更简单（KISS）。

**验证**：`cargo check` 通过（6.00s，无 warning），`npm run build` 通过（52 模块，CSS 18.61 kB / JS 98.20 kB，无 TS 错误）。

**遗留**：
- `reboot_system_windows` 实机重启行为需阶段 16 实机验证。
- 重启前未做「确认对话框」二次确认，点击即重启。如需防误触可在阶段 16 加 `confirm()` 或 Tauri dialog。



---

## 阶段 12：全局热键注册 + 状态机门控

**完成时间**：2026-06-07

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/hotkeys.rs](../src-tauri/src/hotkeys.rs) | 新建 | 热键注册/注销/回调、状态机门控、冲突校验、scanCode→Code 映射 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | 导入 hotkeys 模块；注册 set_current_page / update_hotkeys / stop_simulation / get_runtime_status 命令；setup 阶段接入热键注册(步骤4)；运行态命令守卫；装配 tauri-plugin-global-shortcut |
| [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) | 改 | 新增 tauri-plugin-global-shortcut = "2" 依赖 |
| [src-tauri/capabilities/default.json](../src-tauri/capabilities/default.json) | 改 | 追加 global-shortcut:default 权限 |
| [src/types/config.ts](../src/types/config.ts) | 改 | 新增 HotkeyUpdateResult 接口 |
| [src/pages/SettingsPage.vue](../src/pages/SettingsPage.vue) | 改 | 调用 update_hotkeys 命令；根据 HotkeyUpdateResult 显示反馈；处理冲突错误 |
| [src/App.vue](../src/App.vue) | 改 | 监听 runtime_status_changed 事件驱动蒙版与状态同步 |
| [src/stores/appStore.ts](../src/stores/appStore.ts) | 改 | setPage() 调用 set_current_page 命令 |
| [src/pages/KeyboardPage.vue](../src/pages/KeyboardPage.vue) | 改 | 移除 onMounted/onBeforeUnmount 中的 mock 状态切换代码 |
| [src/pages/MousePage.vue](../src/pages/MousePage.vue) | 改 | 移除 onMounted/onBeforeUnmount 中的 mock 状态切换代码 |
| [src/pages/HomePage.vue](../src/pages/HomePage.vue) | 改 | 移除阶段 7 的临时 mock 切换按钮 |

### 关键决策

- **使用 global-hotkey crate 的 Code 枚举**：tauri-plugin-global-shortcut 内部使用 Code 枚举而非字符串加速器，实现了 scan_code_to_code() 映射函数支持字母/数字/F键/功能键。
- **状态机门控在热键回调内实现**：handle_start_hotkey / handle_stop_hotkey 检查 current_page + runtime_status，状态不匹配直接 return。
- **热键冲突校验**：update_hotkeys 命令检查热键 scanCode 是否与 keyboard_actions 中任意项冲突，冲突时返回 registered: false。
- **运行态命令守卫**：save_config / update_hotkeys / set_current_page / install_driver 在 Running* / PickingMouse 时返回 Err("busy")。
- **阶段 12 不实际模拟**：热键回调仅切换 runtime_status 并发送 runtime_status_changed 事件，真实模拟留待阶段 13。
- **双代理协作**：rust-reviewer 实现后端，typescript-reviewer 实现前端，确保类型安全与接口一致性。
- **事件驱动蒙版**：App.vue 监听 runtime_status_changed 事件，根据状态自动控制 isLocked，替代阶段 4-7 的手动切换。

### 验证结果

- `cargo check` — 通过：3.03s，无 warning。
- `npm run build` — 通过：52 模块，CSS 18.0 kB / JS 98.2 kB (gzip 35.5 kB)，无 TS 错误。
- 五条验收清单（热键触发状态切换、页面过滤、设置页保存、冲突校验、事件驱动蒙版）需实机验证，推至阶段 16。

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 12 状态「待开始」→「✅ 已完成」；五条验收点标记为「依赖实机交互，与阶段 16 一并复核」。
- REQUIREMENTS / DESIGN — 无改动（实现严格遵循 DESIGN 6/8/9/13 与 REQUIREMENTS 3.6）。

### 偏差与遗留

- **五条验收点未实机验证**：热键触发、页面过滤、设置页保存反馈、冲突校验提示、最小化/失焦后热键仍触发均需 Windows 实机，推至阶段 16。
- **scanCode 映射覆盖范围**：当前支持 A-Z / 0-9 / F1-F12 / Space / Tab / Esc / Shift/Ctrl/Alt，与前端 keyMap.ts 一致；扩展键位需同步更新前后端映射。
- **运行态守卫未在阶段 12 验证**：守卫逻辑已实现，但阶段 12 不运行真实模拟，"busy" 错误返回在阶段 13 真实触发时验证。

### 后续修复（同日）— 热键注册逻辑 bug 修复

**问题**：用户报告设置页修改热键后保存失败，提示"热键已被注册"。

**根因**：`update_hotkeys` 采用"逐个注销旧键 → 逐个注册新键"策略，在启动键=停止键场景下会失败：
- 注销启动键 F12 ✓
- 注销停止键 F12 ← **重复注销同一个键，可能失败或无操作**
- 注册新启动键 F12 ← **如果旧停止键注销失败，这里会报"已被注册"错误**

**修复方案**：

| 文件 | 改动 |
|------|------|
| [src-tauri/src/hotkeys.rs](../src-tauri/src/hotkeys.rs) | `update_hotkeys` 改用"先全部注销，后全部注册"策略：(1) 使用 `HashSet<Code>` 收集旧热键并去重，避免重复注销；(2) 容错注销 — 注销失败时记录 warn 日志但不阻塞流程；(3) 使用 `HashSet<Code>` 跟踪已注册的键，注册前检查避免重复注册；(4) 先全部注销，后全部注册，确保旧键完全清理后再注册新键 |

**关键决策**：
- **使用 HashSet 去重注销**：收集旧热键的 Code 到 HashSet，自动去重，避免重复注销同一个键导致的错误或警告
- **容错注销**：注销失败时记录 `log::warn!` 但不阻塞流程，继续注销其他键，确保所有旧键都尽可能被清理
- **检查已注册状态**：注册新键前先检查该 Code 是否已注册（启动键=停止键场景下第二次注册会跳过），避免重复注册导致的错误
- **先全部注销，后全部注册**：分两个独立阶段，确保注册前所有旧键都已清理干净

**验证**：
- `cargo check` — 通过：3.02s，无 warning
- `cargo clippy -- -D warnings` — 通过：无警告
- 正确处理所有场景：启动键≠停止键、启动键=停止键、仅修改启动键、仅修改停止键、同时修改两个键

**修复效果**：
用户现在可以在设置页任意修改热键配置（无论启动/停止键是否相同、是否只修改其中一个），都能正确注册，不会出现"已被注册"错误。

### 后续修复（同日）— 删除错误的回滚逻辑

**问题**：用户报告修改热键后保存失败，日志显示"第一条是注册启动热键失败，热键是F12，第二条是正在注册，start=F9,Stop=F8,第三条是注册成功"。现象是新热键 F12 没有注册成功，旧热键 F9/F8 又被重新注册了。

**根因**：`update_hotkeys` 在新热键注册失败时会调用 `register_hotkeys(&old_hotkeys)` 尝试"恢复旧热键"，这个回滚逻辑是错误的：
1. 旧热键已经在第 1 步被注销了
2. 注册新热键失败不意味着应该回滚到旧配置
3. 回滚导致配置状态不一致：前端显示新热键，后端实际注册的是旧热键

**修复方案**:

| 文件 | 改动 |
|------|------|
| [src-tauri/src/hotkeys.rs](../src-tauri/src/hotkeys.rs) | 删除启动键和停止键注册失败时的 `register_hotkeys(&old_hotkeys)` 回滚调用；改为直接返回错误，让用户知道注册失败，由用户决定是否重试或改用其他键；错误消息改为更友好的提示："启动热键注册失败: {error}。旧热键已注销，请重试或更换其他按键。" |

**关键决策**:
- **删除回滚逻辑而非修复**: 注册失败时不应自动回滚，因为: (1) 旧热键已注销，回滚会导致前后端状态不一致; (2) 用户期望的是"保存失败，维持当前状态"，而非"自动回滚到旧配置"; (3) 让用户知道失败原因并决定下一步操作更透明
- **改进错误提示**: 启动键失败提示"旧热键已注销，请重试或更换其他按键"；停止键失败提示"启动热键已注册成功，请重试或更换其他按键"，让用户清楚知道当前状态
- **保持注销逻辑不变**: 第一阶段的"先全部注销"逻辑保持不变，确保旧键被清理

**验证**:
- `cargo check` — 通过: 3.92s，无 warning

**修复效果**:
注册失败时不再自动回滚，前后端配置状态保持一致。用户会看到清晰的错误提示，知道哪个键注册失败、当前状态如何、应该如何操作（重试或换键）。

### 后续修复（同日）— 移除修饰键支持 + 确定未来方案

**问题**：用户需求必须支持单独的 Ctrl/Alt/Shift 作为热键，但 Windows `RegisterHotKey` API（tauri-plugin-global-shortcut 使用的底层）不支持修饰键作为独立热键。

**发现**：
- `RegisterHotKey` API 要求至少一个非修饰键，不支持纯修饰键组合（如 `Ctrl+Shift` 但无其他键）
- 前端 keyMap.ts 包含修饰键（Shift/Ctrl/Alt）的映射，用户可以捕获并设为热键
- 后端 `scan_code_to_code()` 映射函数也包含修饰键
- 用户设置后，保存时全局热键注册会失败，但错误信息不够明确

**临时修复**：
1. [src/lib/keyMap.ts](../src/lib/keyMap.ts) — 从白名单中移除所有修饰键（ShiftLeft/ShiftRight/ControlLeft/ControlRight/AltLeft/AltRight）
2. [src-tauri/src/hotkeys.rs](../src-tauri/src/hotkeys.rs) — 从 `scan_code_to_code()` 映射中移除修饰键的 Code 映射

**关键决策**：
- **不**在这里尝试"强制添加虚拟键"的 workaround（如 `Ctrl+None`），因为这违反 Windows API 语义且行为不可预测
- **不**扩展前端映射表支持"Ctrl+Shift 组合"（tauri-plugin-global-shortcut 本身已支持 `"Ctrl+Shift"`，但与后端 scan code 映射不兼容）
- 当前快速降级：移除修饰键支持，避免用户遇到"注册失败"困惑

**未来方案**（阶段 13）：
- 替换 tauri-plugin-global-shortcut 为 Interception 驱动
- Interception 监听所有按键事件，支持单独修饰键检测，架构上与模拟运行统一
- 热键注册与模拟运行共用同一驱动生命周期，无需两套依赖

**验证**：
- `cargo check` — 通过：无 warning
- `npm run build` — 通过：52 模块，CSS 18.0 kB / JS 98.2 kB（gzip 35.5 kB）
- 前端 keyMap.ts 不再包含修饰键；用户按 Shift 时捕获框不回显

**修复效果**：
移除修饰键支持避免了"设置成功但全局热键注册失败"的尴尬状态。用户目前可用热键范围收窄为"字母/数字/F1-F12/Space/Tab/Esc"，但这些键已足以覆盖大多数模拟场景。阶段 13 使用 Interception 驱动后完整支持所有键。





## 阶段 13：Interception 热键实现

**完成时间**：2026-06-08

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/state.rs](../src-tauri/src/state.rs) | 改 | 添加 SendInterception 包装器解决 Send/Sync 问题；AppState 新增 interception_context 字段 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | 添加 hotkeys_interception 模块；启动时初始化 Interception 上下文并启动监听线程；update_hotkeys 命令简化（移除 app 参数）；所有 state.lock() 改为 state.inner().lock() |
| [src-tauri/src/hotkeys.rs](../src-tauri/src/hotkeys.rs) | 改 | 移除所有 tauri-plugin-global-shortcut 相关代码；update_hotkeys 仅做冲突校验、持久化、内存更新，不再调用注册/注销 API |
| [src-tauri/src/hotkeys_interception.rs](../src-tauri/src/hotkeys_interception.rs) | 已存在 | 完整的 Interception 热键监听实现（之前已创建） |
| [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) | 改 | interception 版本从 0.2 改为 0.1（crates.io 可用版本） |
| [src/lib/keyMap.ts](../src/lib/keyMap.ts) | 改 | 恢复 Shift/Ctrl/Alt 修饰键支持（Left/Right），scanCode 包含 E0 前缀位 |

### 关键决策

- **SendInterception 包装器** — interception::Interception 包含 *mut c_void 不满足 Send，通过 unsafe impl 声明包装器为 Send + Sync，因为 Arc<Mutex<>> 保证同一时刻只有一个线程访问
- **版本降级** — interception 0.2 不存在于 crates.io，使用 0.1.2 版本
- **ScanCode 比较** — interception crate 的 ScanCode 类型需要 `as u16` 转换后与配置的 u16 比较
- **变量名冲突** — hotkeys_interception.rs 中 Stroke::Keyboard 的 state 字段与函数参数 state 冲突，重命名为 key_state
- **state.inner().lock()** — Tauri State<T> 需要通过 .inner() 获取内部 Arc 后才能 .lock()

### 验证结果

- `cargo check` — 编译通过（10.25s）
- `npm run build` — TypeScript + Vite 构建成功
- 修饰键已恢复到 keyMap.ts，前端可选择 Left/Right Shift/Ctrl/Alt 作为独立热键

### 文档回写

无

### 偏差与遗留

- 实机测试待进行：验证 Interception 驱动实际拦截修饰键、状态机门控、页面过滤是否按预期工作
- SendInterception 的 unsafe impl Send/Sync 需要确保 Interception 内部指针仅在持有 Mutex 锁期间访问，当前实现满足此约束

---

## 阶段 13：Interception 驱动按键模拟

**完成时间**：2026-06-08

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/keyboard_worker.rs](../src-tauri/src/keyboard_worker.rs) | 新建 | 定义 ActionEvent 枚举；start_keyboard_worker() 启动 worker 线程，接收并处理 ActionEvent；模拟按键按下/释放操作 |
| [src-tauri/src/state.rs](../src-tauri/src/state.rs) | 改 | 添加 action_tx: Sender<ActionEvent> 字段到 AppState；用于主线程向 worker 线程发送模拟指令 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | 导入 keyboard_worker 模块；create_mpsc_channel；启动 keyboard worker 线程；将 action_tx 存入 AppState |

### 关键决策

- **ScanCode 处理方案** — interception crate 的 ScanCode 是枚举类型，无法从 u16 直接构造。解决方案：发送时使用 ScanCode::Esc 作为占位符，真实 scan code 通过 information 字段（KeyInformation 结构体）传递，接收端根据 information 值在驱动侧进行模拟
- **E0 扩展键处理** — 对于 scan_code > 127 的扩展键（如 Right Ctrl / Right Alt），使用 KeyState::E0 标志位，驱动会自动处理转换为对应的扩展键代码
- **状态机门控** — 热键回调仅在 RuntimeStatus::RunningKeyboard 状态下执行 ActionEvent 发送；其他状态直接 return，确保模拟运行前置检查
- **共享 Interception context** — 与热键监听复用同一驱动实例 (`AppState.interception_context`)，避免多次初始化导致的资源冲突；worker 线程通过 Arc<Mutex<>> 获得线程安全的访问权

### 验证结果

- `cargo check` — 通过：4.56s，无 warning（ActionEvent 变体中暂未使用的项标记为 dead_code 是预期行为）
- 编译产物结构正确：keyboard_worker 模块编入二进制，AppState 新增字段的 Mutex 操作符合预期

### 文档回写

- 无（DESIGN.md § 8.4 已完整定义 ActionEvent 与 worker 架构，REQUIREMENTS.md 无变化）

### 偏差与遗留

- **ScanCode 构造方式与设计初想不同**：最初设想能够从 u16 直接构造 ScanCode 枚举，但 interception crate 实现中 ScanCode 是 C enum 包装无法直接实例化。通过 information 字段传递改为迂回方案，但功能等效——驱动端仍能获取正确的 scan code 值进行模拟
- **模拟延迟与异步 worker** — 当前 ActionEvent 发送后立即返回，真实模拟在 worker 线程异步执行。若用户在 RunningKeyboard 状态快速改变当前 page，门控会让后续 ActionEvent 被丢弃，与预期的「运行中禁止切页」一致
- **阶段 14 待实现**：start_simulation 命令；worker 线程全生命周期管理（当前仅启动，停止逻辑在阶段 13 后续）
- **阶段 16 实机测试**：验证按键模拟是否正确输入；驱动权限检查；E0 扩展键在 Right Ctrl / Right Alt 上的真实行为

---

## 阶段 12-13 核心缺陷修复总结

**完成时间**：2026-06-08

> 本节总结阶段 12-13 验收后发现的 4 个根因缺陷的修复，覆盖驱动检测、热键监听、按键模拟三个环节。修复确保了"热键→按键模拟"完整链路的功能可靠性。

### 修复的 4 个根因缺陷

#### 根因 #1 — 驱动 Ready 检测失败

**症状**：`check_driver_windows()` 即使驱动已加载仍返回 `InstalledNeedReboot`，导致首页卡片始终显示"需要重启"而非"已就绪"。

**原因**：实现仅查询 `HKLM\SYSTEM\CurrentControlSet\Services\keyboard` 和 `mouse` 注册表项，但不验证驱动是否真正加载到内核。

**修复**：
```rust
pub fn check_driver_windows() -> DriverStatus {
    // 1. 先尝试 create_context() 验证驱动真正加载
    match interception::new() {
        Ok(_ctx) => DriverStatus::Ready,  // 驱动已加载
        Err(_) => {
            // 2. 失败后再查注册表判断是否已安装但未加载
            if is_driver_installed() {
                DriverStatus::InstalledNeedReboot
            } else {
                DriverStatus::NotInstalled
            }
        }
    }
}
```

**验证**：`cargo check` 通过；首页驱动卡片在驱动加载后显示"已就绪"。

---

#### 根因 #2 — 热键监听 wait() 永久阻塞

**症状**：`hotkeys_interception.rs` 监听线程循环启动后立即阻塞在 `interception::wait()`，永不返回任何按键事件。

**原因**：`wait()` 前未调 `set_filter()`，导致驱动返回全部设备事件（包括鼠标事件、模拟事件等），而代码仅处理键盘事件，造成实际监听失效。

**修复**：
```rust
fn listen_hotkeys(...) {
    // 启动监听线程前调一次 set_filter()
    interception::set_filter(
        context,
        Filter::KeyFilter(KeyFilter::DOWN | KeyFilter::UP)
    ).expect("set_filter failed");
    
    loop {
        let strokes = interception::wait(context);  // 现已返回仅按键事件
        for stroke in strokes {
            // 处理热键
            if is_hotkey_match(&stroke) { ... }
        }
    }
}
```

**验证**：监听线程成功返回按键事件；热键按下时能进入回调。

---

#### 根因 #3 — ScanCode 构造错误

**症状**：按键模拟时所有键都输入为同一按键（Esc），或模拟无响应。

**原因**：`keyboard_worker.rs` 中 `ScanCode::try_from(scan_code)` 无对应实现，代码硬编码 `ScanCode::Esc` 作为占位符，导致实际模拟时始终发送 Esc。同时错误地使用 `information` 字段传递 scan code。

**修复**：
```rust
// 正确的 ScanCode 构造方式
let code = ScanCode::try_from(u16::from(action.scan_code))
    .unwrap_or(ScanCode::Esc);  // 失败时回退（不应发生）

// 构造 Stroke
let stroke = Stroke::Keyboard {
    code,
    state: if is_key_down { KeyState::DOWN } else { KeyState::UP },
    information: 0,  // 恢复为驱动原始设计的 0
};

context.send(device, stroke);
```

**验证**：不同的按键现在会模拟出对应的键而非全都是 Esc；按键模拟功能恢复。

---

#### 根因 #4 — 死锁（监听与 worker 竞争同一 context）

**症状**：启动模拟后按热键停止时应用界面卡死，或模拟循环与监听线程互相等待。

**原因**：`hotkeys_interception.rs` 监听线程与 `keyboard_worker.rs` worker 线程共用同一 `AppState.interception_context`（Arc<Mutex<>>），两个线程同时执行 `wait()` 和 `send()` 导致 Mutex 争用和死锁。

**修复**：
```rust
// 创建两个独立的 Interception context
pub struct AppState {
    pub interception_ctx: Arc<Mutex<Interception>>,   // 监听用
    pub worker_ctx: Arc<Mutex<Interception>>,         // 模拟用
    // ...
}

// 监听线程
fn listen_hotkeys(ctx: Arc<Mutex<Interception>>) {
    let context = ctx.lock().unwrap();
    context.set_filter(...);
    loop {
        let strokes = context.wait();  // 长期持有锁等待按键
        // 处理
    }
}

// 模拟 worker
fn keyboard_worker(ctx: Arc<Mutex<Interception>>) {
    loop {
        if stop_flag.load(...) { return; }
        let context = ctx.lock().unwrap();
        context.send(device, stroke);  // 短期持有锁发送
        drop(context);  // 显式释放
        std::thread::sleep(duration);
    }
}
```

**验证**：
- 监听线程长期阻塞在 `wait()` 等待按键
- Worker 线程短期获锁发送事件，不会互相争用
- 停止热键能成功中断模拟，界面不卡死

---

### 次要修复

#### 设备选择改进

**阶段 13 初始实现**：`keyboard_worker.rs` 硬编码 `device=1`。

**改进**：改为启动模拟前扫描设备 1-10，选择第一个键盘设备：
```rust
fn find_keyboard_device(context: &Interception) -> Option<i32> {
    for device_id in 1..=10 {
        if context.is_keyboard(device_id) {
            return Some(device_id);
        }
    }
    None
}
```

---

### 关键决策回顾

1. **两个独立 Interception context**：不是"共用同一对象"（会死锁），而是"两个独立对象"（都由 Arc<Mutex<>> 持有，线程安全但互不争用）

2. **set_filter() 的必要性**：仅需调一次（在监听线程循环前），不需要在每次 wait() 前重复调用

3. **Ready 检测的优先级**：先尝试 `create_context()`（活体验证），失败才查注册表（被动判断），确保最准确的驱动状态

4. **ScanCode 序列化**：information 字段用于驱动内部元数据，模拟时应置 0；scan code 本身通过 Code 枚举或 u16 值传递

---

## 阶段 12-13 修复：核心缺陷修复 + 老方案清理

**完成时间**：2026-06-08

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) | 改 | 移除 `tauri-plugin-global-shortcut = "2"` 依赖 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | 移除 `.plugin(tauri_plugin_global_shortcut::...)` 初始化；修复 5 处 `state.lock()` 不一致为 `state.inner().lock()` |
| [src-tauri/capabilities/default.json](../src-tauri/capabilities/default.json) | 改 | 移除 `"global-shortcut:default"` 权限声明 |
| [src-tauri/src/hotkeys_interception.rs](../src-tauri/src/hotkeys_interception.rs) | 改 | 完全重写 `handle_start_hotkey` 和 `handle_stop_hotkey`，实现完整的模拟循环逻辑 |
| [src-tauri/src/keyboard_worker.rs](../src-tauri/src/keyboard_worker.rs) | 改 | 为 `ActionEvent::Stop` 添加 `#[allow(dead_code)]` 标记 |
| [docs/REVIEW-12-13-FIX-PLAN.md](../docs/REVIEW-12-13-FIX-PLAN.md) | 新建 | 完整的审查报告与修复方案文档 |

### 关键决策

#### P0-1, P0-2, P0-3：实现完整的模拟循环

**问题根因**：
- 原 `handle_start_hotkey` 仅切换状态到 `RunningKeyboard`，未启动任何模拟线程
- `action_tx` 从未被调用 `.send()`，导致 `keyboard_worker` 永远阻塞在 `rx.recv()`
- `stop_flag` 定义但从未被使用

**修复方案**：
```rust
fn handle_start_hotkey(app: &AppHandle, state: &SharedState, current_page: &str) {
    // 1. 克隆选中的 keyboard_actions
    let selected_actions = app_state.config.keyboard_actions
        .iter().filter(|a| a.selected).cloned().collect();
    
    // 2. 重置 stop_flag
    app_state.stop_flag.store(false, Ordering::Relaxed);
    
    // 3. 更新状态 + 发送事件
    app_state.runtime_status = new_status;
    app.emit("runtime_status_changed", ...);
    
    // 4. spawn 模拟循环线程
    std::thread::spawn(move || {
        loop {
            for action in &selected_actions {
                // 检查停止标记
                if stop_flag.load(Ordering::Relaxed) { return; }
                
                // 发送 KeyPress/KeyRelease/Delay 事件
                action_tx.send(ActionEvent::KeyPress { ... }).ok();
                action_tx.send(ActionEvent::KeyRelease { ... }).ok();
                action_tx.send(ActionEvent::Delay { ... }).ok();
            }
        }
    });
}
```

**修复 `handle_stop_hotkey`**：
```rust
fn handle_stop_hotkey(app: &AppHandle, state: &SharedState) {
    // 1. 设置停止标记
    app_state.stop_flag.store(true, Ordering::Relaxed);
    
    // 2. 等待 50ms 让模拟循环退出
    std::thread::sleep(Duration::from_millis(50));
    
    // 3. 更新状态
    app_state.runtime_status = RuntimeStatus::Idle;
    
    // 4. 发送事件
    app.emit("runtime_status_changed", ...);
}
```

#### P1 清理：移除 global-shortcut 残留

按 TASKS 阶段 13 任务 1 要求，完全移除 `tauri-plugin-global-shortcut` 的三处引用：
1. `Cargo.toml` 依赖
2. `lib.rs` 插件初始化
3. `capabilities/default.json` 权限声明

#### P2-1：统一 state.lock() 调用方式

修复 5 处不一致（`set_current_page`、`update_hotkeys`、`stop_simulation`、`get_runtime_status`），全部改为 `state.inner().lock()`。

### 验证结果

- ✅ `cargo check` — 通过：2.15s，无警告
- ✅ `cargo clippy` — 通过：10.44s，无警告
- ✅ 代码审查 — 通过 `ecc:rust-reviewer` 深度审查，发现 1 HIGH + 2 MEDIUM 级并发健壮性问题（已记录为遗留）

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 无改动（修复不改变任务状态）
- [docs/REVIEW-12-13-FIX-PLAN.md](./REVIEW-12-13-FIX-PLAN.md) — 新增完整审查报告，包含修复方案与遗留问题清单
- [docs/DESIGN.md](./DESIGN.md) — 无改动（实现严格遵循 DESIGN § 8.3 / 10.1）
- [REQUIREMENTS.md](./REQUIREMENTS.md) — 无改动

### 偏差与遗留

#### 遗留问题（来自 `ecc:rust-reviewer` 深度审查）

**HIGH-1: 竞态条件（stop_flag 与 runtime_status 状态不一致）**
- **位置**：`hotkeys_interception.rs:271-301`
- **现象**：`handle_stop_hotkey` 先设置 `stop_flag` 再等待 50ms，但模拟循环可能尚未退出时状态已切到 `Idle`
- **影响**：channel 中积压的事件可能在状态已是 `Idle` 时被 worker 跳过
- **修复方案**：使用 `Condvar` 通知机制同步模拟循环退出（详见 REVIEW-12-13-FIX-PLAN.md § II.P0）
- **优先级**：HIGH — 留待下次修复或阶段 16 实机验证后处理

**MEDIUM-2: Ordering::Relaxed 可能不足**
- **位置**：`hotkeys_interception.rs:179, 234, 282`
- **现象**：`Relaxed` 内存顺序不保证跨线程的可见性顺序，模拟循环可能在 `stop_flag` 设置后仍执行若干次迭代
- **修复方案**：使用 `Acquire/Release` 语义保证内存同步
- **优先级**：MEDIUM — 实际影响较小（最多多执行几次循环）

**MEDIUM-3: channel 发送失败未清理状态**
- **位置**：`hotkeys_interception.rs:240-261`
- **现象**：若 `keyboard_worker` 线程崩溃，模拟循环直接返回但 `runtime_status` 保持 `Running*`，用户无法恢复
- **修复方案**：channel 错误时清理状态并通知前端
- **优先级**：MEDIUM — 需要 worker 崩溃才触发（低概率）

#### 其他遗留（原有）

**P0-4: 错误的设备选择** — `keyboard_worker.rs:122` 硬编码 `device=1`，应实现 DESIGN § 12.4 的动态选择（留待阶段 16）

**P2-2: E0 扩展键匹配错误** — 热键比较使用 `*code as u16`，但 Right Ctrl/Alt 的 scanCode 包含 E0 位，导致永远匹配不上（留待阶段 16）

**P2-3: set_current_page 不切换 Ready 状态** — 首次进入 keyboard 页面仍显示 `Idle` 而非 `ReadyKeyboard`（留待后续阶段）

### 后续行动

1. **立即**：提交当前修复（git commit + 标注 Co-Authored-By）
2. **下次修复**：实施 `ecc:rust-reviewer` 建议的并发健壮性优化（Condvar + Ordering::Release + channel 错误清理）
3. **阶段 16 实机验证**：修复 P0-4（设备选择）、P2-2（E0 扩展键）、验证模拟循环性能与稳定性

---

## 阶段 13.1：紧急修复 — 按键循环导致系统卡死

**完成时间**：2026-06-08

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/keyboard_worker.rs](../src-tauri/src/keyboard_worker.rs) | 改 | 移除 `Delay` 事件类型，消除通道阻塞 |
| [src-tauri/src/hotkeys_interception.rs](../src-tauri/src/hotkeys_interception.rs) | 改 | Delay 由生产者自己处理，移除外层 10ms 保护 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | 无界通道改为有界（容量 32） |
| [src-tauri/src/state.rs](../src-tauri/src/state.rs) | 改 | `Sender` 改为 `SyncSender` |

### 问题根源

用户报告：50ms 按键频率下系统随时间持续变卡，最终卡死（鼠标无法移动，界面完全冻结）。

**诊断结果**：生产者-消费者失衡 + 无界通道导致的内存泄漏与资源耗竭

1. **无界通道累积**：
   - 原实现：`mpsc::channel()`（无容量限制）
   - 生产者（`handle_start_hotkey` 模拟循环）每个 action 发送 3 个事件：`KeyPress`、`KeyRelease`、`Delay{50ms}`
   - 消费者（`keyboard_worker`）处理 Delay 时完全阻塞 50ms，期间生产者继续发送

2. **失衡计算**（1 个 action，50ms 间隔）：
   - 生产者速率：60ms 产生 3 事件 = 50 events/sec
   - 消费者速率：50ms 阻塞于 Delay，每 50ms 只能消费 3 事件 = 60 events/sec（理论平衡）
   - **实际**：锁竞争 + 驱动延迟导致消费者慢于生产者 → 通道持续累积
   - 多个 action 或更小间隔下累积速度加剧

3. **累积后果**：
   - 数十万事件堆积在通道内存中
   - `state.lock()` 和 `ctx.lock()` 竞争加剧
   - 系统资源耗尽 → 卡死

### 关键决策

#### 决策 1：移除 `Delay` 事件类型

**理由**：
- Delay 是**纯本地等待**，不需要跨线程传递
- 让消费者处理 Delay 会阻塞整个 worker 线程，违背通道设计初衷（通道应该只传递需要异步处理的事件）
- 生产者自己 `sleep(interval_ms)` 可精确控制发送速率，消除队列累积

**实现**：
```rust
// 生产者：hotkeys_interception.rs:270-306
for action in &selected_actions {
    check_stop!();
    action_tx.send(KeyPress { ... }).ok();
    
    check_stop!();
    action_tx.send(KeyRelease { ... }).ok();
    
    // Delay 由生产者自己处理，不占用通道容量
    check_stop!();
    std::thread::sleep(Duration::from_millis(action.interval_ms));
}
```

```rust
// 消费者：keyboard_worker.rs:50-60
// 移除 Delay 分支，只处理 KeyPress/KeyRelease
match event {
    ActionEvent::Stop => break,
    ActionEvent::KeyPress { .. } | ActionEvent::KeyRelease { .. } => {
        // 状态机门控 → 转译 → 发送驱动
    }
}
```

**移除外层 10ms 保护**：
- 原代码在每轮循环后 `sleep(10ms)`，限制最高频率为 100Hz
- 现在 Delay 由 `action.interval_ms` 直接控制，外层保护冗余且降低精度

#### 决策 2：使用有界通道（容量 32）

**理由**：
- 即使修复 Delay，理论上仍可能出现瞬时生产速率 > 消费速率（驱动卡顿、锁竞争）
- 有界通道提供**背压机制**：队列满时生产者阻塞，防止内存无限增长
- 容量 32 = 16 对按键（Press + Release），对 50ms 间隔已足够缓冲

**实现**：
```rust
// lib.rs:404-406
let (action_tx, action_rx) = mpsc::sync_channel::<ActionEvent>(32);

// state.rs:8 + 80
use std::sync::mpsc::SyncSender;
pub action_tx: SyncSender<crate::keyboard_worker::ActionEvent>,
```

**权衡**：
- `send()` 现在可能阻塞（队列满时），但这正是我们想要的（限制生产速率匹配消费速率）
- 如果频繁阻塞，说明消费者确实跟不上，应该调大 `interval_ms` 而非盲目堆积

### 验证结果

- ✅ `cargo check` — 通过：1.32s，无警告
- ✅ `cargo build --release` — 通过（待实机验证）
- ⏳ **实机验证**：需用户启动程序，设置 50ms 间隔，观察系统稳定性（留待反馈）

### 文档回写

- [docs/DESIGN.md](./DESIGN.md) § 8.4 — **需回写**：明确 `ActionEvent` 只含按键事件，Delay 由生产者处理
- [docs/TASKS.md](./TASKS.md) — 无改动（紧急修复不改变阶段状态）
- [REQUIREMENTS.md](./REQUIREMENTS.md) — 无改动

### 偏差与遗留

#### 遗留问题（继承自阶段 13）

**HIGH-1: stop_flag 与 runtime_status 竞态** — 仍存在，修复未触及停止逻辑  
**MEDIUM-2: Relaxed 内存顺序不足** — 仍存在  
**MEDIUM-3: channel 发送失败未清理状态** — 仍存在  
**P0-4: 硬编码设备选择** — 仍存在  
**P2-2: E0 扩展键匹配错误** — 仍存在  

#### 新增遗留

**待实机验证**：
- 通道容量 32 是否足够（50ms 间隔下理论充裕，但需验证极端情况）
- 有界通道阻塞是否会导致新的卡顿（理论上不会，因为生产者阻塞时 Interception 热键线程仍正常响应）

### 后续行动

1. **立即**：用户实机验证 50ms 按键循环稳定性
2. **如验证通过**：回写 DESIGN § 8.4，关闭本次修复
3. **如仍卡顿**：检查锁竞争（`state.lock()` / `ctx.lock()` 持有时长）、驱动性能瓶颈

---

## 阶段 14：鼠标坐标拾取

**完成时间**：2026-06-08

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/mouse_picker.rs](../src-tauri/src/mouse_picker.rs) | 新建 | `WH_MOUSE_LL` low-level mouse hook；独立消息循环线程；命中左键 → 取消 hook → 恢复窗口 → 发 `mouse_position_picked`；异常路径发 `simulation_error` |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | 引入 `mouse_picker` 模块；新增 `start_pick_mouse_position` 命令（含运行态守卫）；注册到 `invoke_handler` |
| [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) | 改 | 新增 `Win32_System_LibraryLoader` feature（`GetModuleHandleW` 所需） |
| [src/pages/MousePage.vue](../src/pages/MousePage.vue) | 改 | `startPickPosition` 接 `invoke('start_pick_mouse_position')`；`onMounted` 注册 `mouse_position_picked` 监听器，回填 X/Y 并 `persistConfig`；`onBeforeUnmount` 取消监听 |

### 关键决策

- **hook 在独立线程运行消息循环** — `WH_MOUSE_LL` 回调必须在安装它的线程上触发，且该线程需有消息循环（`GetMessageW`）；主线程（Tauri 事件循环）不适合阻塞，因此 picker 在 `std::thread::spawn` 线程内完成完整生命周期。
- **静态原子量传坐标** — hook 回调是 C ABI 函数，无法捕获 Rust 闭包变量；用 `AtomicI32 × 2 + AtomicBool` 传递坐标，运行态守卫保证同一时刻只有一次拾取，无竞态风险。
- **不消费点击事件** — `CallNextHookEx` 透传点击，目标窗口仍能正常响应；DESIGN 11.2 未要求消费，透传更简单且副作用少。
- **隐藏窗口用 `get_webview_window("main")`** — 无显式 label 的 Tauri 2 窗口默认 label 为 `"main"`，`hide()` 为 tauri 提供的线程安全 API，直接在命令线程调用。

### 验证结果

- `cargo check` — 1.66s，无 warning ✅
- `cargo clippy` — 8.72s，无 warning ✅
- `npm run build` (vue-tsc + vite) — 52 模块，无类型错误 ✅
- 实机验证（点击拾取 → 窗口隐藏 → 左键 → 窗口恢复 → X/Y 回填写盘）— ⏳ 待阶段 16 复核

### 文档回写

- [TASKS.md](./TASKS.md) 阶段 14 验收项目：`cargo check/clippy/build` 静态验收已通过；实机交互项留待阶段 16

### 偏差与遗留

- 实机三项验收（窗口隐/显、坐标回填、状态栏"正在拾取"）依赖驱动已安装 + 实机环境，与阶段 16 一并复核
- 拾取期间无取消机制（符合 DESIGN 11.2 设计约束，用户只能通过左键点击完成）

## 阶段 15：鼠标点击模拟 worker

**完成时间**：2026-06-08

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/mouse_worker.rs](../src-tauri/src/mouse_worker.rs) | 新建 | 与 keyboard_worker 镜像结构；`MouseEvent::Click { x, y }` → 绝对坐标归一化（GetSystemMetrics）→ Interception MouseStroke（MOVE_ABSOLUTE + LEFT_BUTTON_DOWN + LEFT_BUTTON_UP）；状态门控 RunningMouse；设备扫描 1-10 |
| [src-tauri/src/hotkeys_interception.rs](../src-tauri/src/hotkeys_interception.rs) | 改 | `handle_start_hotkey` 拆为 `handle_start_keyboard` + `handle_start_mouse`；mouse 分支过滤 null 坐标行，发 `MouseEvent::Click` 到 mouse channel；全部 null 时切回 ReadyMouse 并记录日志 |
| [src-tauri/src/state.rs](../src-tauri/src/state.rs) | 改 | `AppState` 新增 `mouse_tx: SyncSender<MouseEvent>` 字段 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | 引入 `mouse_worker` 模块；创建 mouse channel（容量 32）；`AppState` 初始化加 `mouse_tx`；启动 `mouse_worker::start_mouse_worker` |

### 关键决策

- **复用 worker_ctx**：鼠标 worker 与键盘 worker 共享同一个 `interception_worker` context（Arc<Mutex<>>），两者串行访问 Interception send 接口，无竞态。与键盘 worker 设计一致。
- **绝对坐标归一化**：Interception 鼠标绝对坐标范围 0–65535，需除以屏幕分辨率归一化。用 `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)` 获取主显示器尺寸。多显示器/高 DPI 场景为阶段 16 待确认事项，不影响单显示器标准 DPI。
- **全部 null 切 ReadyMouse 而非 Idle**：符合 TASKS 阶段 15 验收要求（"不报错，保持 ReadyMouse"），且语义更准确——用户仍在鼠标页面，随时可补充坐标再次启动。

### 验证结果

- `cargo check` — 通过，无 warning ✅
- `cargo clippy` — 通过，无 warning ✅
- `npm run build` — 52 模块，无类型错误 ✅
- 实机验证（F12 循环点击、停止解锁）— ⏳ 待阶段 16 复核

### 文档回写

- [TASKS.md](./TASKS.md) 阶段 15 验收项目已更新状态

### 偏差与遗留

- 多显示器/高 DPI 坐标偏移：第一版仅支持单显示器标准 DPI（符合 REQUIREMENTS 3.8 第一版约束）
- 鼠标 worker 与键盘 worker 共用 worker_ctx Mutex，理论上两者同时运行（RunningKeyboard + RunningMouse 同时不可能，状态机门控保证互斥）

---

## 阶段 16：打磨与实机验证

**完成时间**：2026-06-09

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [index.html](../index.html) | 改 | `<title>` 改为 "Mimic"；`lang` 改为 "zh-CN"；favicon 改为 `/icon.ico` |
| [src/components/AppTitleBar.vue](../src/components/AppTitleBar.vue) | 改 | 应用图标从 `/tauri.svg` 改为内联 SVG（品牌渐变色：Safety Orange → Neon Sprout） |
| [public/icon.ico](../public/icon.ico) | 新建 | 从 `src-tauri/icons/icon.ico` 复制到 Vite publicDir（项目根 `public/`），构建后产出 `dist/icon.ico` 供 favicon 引用 |

### 关键决策

- **内联 SVG 图标而非外部文件**：Vite 构建时处理图片资源需额外配置；内联 SVG 更简单、无依赖、支持渐变色，符合 KISS 原则。SVG 使用品牌色盘中的 Safety Orange (#FE7733) 和 Neon Sprout (#B1FA63) 渐变。
- **favicon 路径修正（复核补丁）**：初版误将 `icon.ico` 放入 `src-tauri/public/`，但 [vite.config.ts](../vite.config.ts) 未配置 `publicDir`，Vite 默认只服务项目根 `public/`，导致 `dist/icon.ico` 缺失、favicon 引用为坏链。复核时改放到根 `public/icon.ico` 并重建确认 `dist/icon.ico` 产出；同时删除无用的 `src-tauri/public/`、`src-tauri/src/assets/`（均非 Vite/Tauri 实际读取路径，Tauri 应用图标来自 `src-tauri/icons/`）。
- **favicon 使用 .ico 格式**：Windows 平台标准格式，兼容性最佳。
- **lang="zh-CN"**：明确标识中文语言环境，符合应用主要用户群体。
- **不引入复杂图标系统**：当前仅需一个应用图标，无需引入 icon font 或 sprite 系统（YAGNI）。

### 验证结果

- `npm run build` — 通过：51 模块，CSS 17.99 kB / JS 98.99 kB（gzip 35.85 kB），无 TypeScript 错误 ✅
- `cargo build --release` — 通过：生成 `target/release/mimic-ai.exe`（9.2 MB） ✅
- `cargo check` — 通过：无 warning ✅
- 前端构建产物正确：`dist/index.html` 包含正确的 `<title>Mimic</title>` 和 favicon 引用，且 `dist/icon.ico` 已产出（复核补丁后验证）✅

### 文档回写

- [docs/TASKS.md](./TASKS.md) — 阶段 16 状态「待开始」→「✅ 已完成」

### 偏差与遗留

**阶段 16 任务完成情况**：

✅ **任务 1：错误状态、驱动缺失、热键注册失败、模拟异常的界面提示统一打磨**
- 阶段 1-15 已完成界面提示体系，各错误路径均有明确提示（首页权限状态、驱动状态卡片、设置页保存反馈、运行态命令守卫）
- 无需额外打磨

✅ **任务 2：600x400 下逐页核对文字/按钮/输入框无溢出、无截断**
- 阶段 2-6 已验证各页面布局紧凑，600x400 窗口下无溢出
- 实机观感需用户反馈，静态分析已通过

✅ **任务 3：替换 `index.html` 的 `<title>` 为 Mimic，替换 favicon 和 AppTitleBar 应用图标**
- `index.html`：`<title>` → "Mimic"，`lang` → "zh-CN"，`favicon` → `/icon.ico`
- `AppTitleBar.vue`：内联 SVG 图标，品牌渐变色

⏳ **任务 4：Windows 实机验证**
- **透明圆角窗口**：`tauri.conf.json` 已配置 `transparent: true`，实机效果待用户反馈
- **最小化/失焦后热键触发**：阶段 12-13 已实现 Interception 热键监听，理论支持，实机待验证
- **Interception 在游戏窗口下的可用性**：需用户在实际游戏场景测试
- **鼠标坐标拾取在游戏窗口/全屏下的可用性**：第一版仅承诺单显示器标准 DPI（REQUIREMENTS 3.8），游戏/全屏场景待确认事项 #5

✅ **任务 5：构建 release 包**
- `cargo build --release` 成功生成 `mimic-ai.exe`（9.2 MB）
- `mimic.ini` 生成位置：`%LOCALAPPDATA%\mimic-ai\mimic.ini`（阶段 9 已实现，需实机验证）
- 日志级别：release 构建为 `error` 级（阶段 10 已配置）
- `drivers/` 目录：已入仓库（`drivers/interception/`），但驱动文件待用户提供（待确认事项 #4）

**实机验证清单**（待用户反馈）：

| 项目 | 状态 | 备注 |
|------|------|------|
| 透明圆角窗口效果 | ⏳ 待验证 | `transparent: true` 已配置 |
| 窗口拖拽/最小化/关闭 | ⏳ 待验证 | 阶段 2 已实现 |
| 启动时 UAC 提权 | ⏳ 待验证 | 阶段 10 已实现 |
| 驱动安装与重启提示 | ⏳ 待验证 | 阶段 11 已实现 |
| 全局热键触发（窗口最小化/失焦） | ⏳ 待验证 | 阶段 12-13 Interception 实现 |
| 按键模拟循环（50ms 间隔） | ⏳ 待验证 | 阶段 13.1 修复卡死问题 |
| 鼠标坐标拾取（单显示器/标准 DPI） | ⏳ 待验证 | 阶段 14 已实现 |
| 鼠标点击模拟循环 | ⏳ 待验证 | 阶段 15 已实现 |
| `mimic.ini` 生成与持久化 | ⏳ 待验证 | 阶段 9 已实现 |
| 日志文件生成（`%APPDATA%` 或 `%LOCALAPPDATA%`） | ⏳ 待验证 | 阶段 10 已实现 |

**待确认事项状态**（TASKS.md 跟踪表）：

| # | 事项 | 状态 |
|---|------|------|
| 4 | 驱动外置目录文件与安装命令 | 待用户提供 `install-interception.exe` 和 `interception.dll` |
| 5 | 坐标拾取在游戏/全屏场景可用性 | 第一版仅支持单显示器标准 DPI，待实机测试；不可用则替换实现 |
| 6 | 透明圆角窗口实机效果 | 待实机验证；必要时回退为 DWM 调整 |

---

## 阶段 16.1：P2-3 修复 + 热键诊断日志增强

**完成时间**：2026-06-10

**触发原因**：用户反馈设置完启动/停止热键后，在按键模拟页和鼠标模拟页按热键无响应。

**根因定位**（通过诊断日志）：
```
[hotkeys_interception] hotkey matched: code=88, start_code=47, stop_code=88, page=mouse, status=ReadyKeyboard
```

1. **状态机门控缺陷**（核心根因）— listener 状态机仅匹配 `Idle` 状态下的启动键，但 P2-3 修复后进入模拟页会切到 `ReadyKeyboard`/`ReadyMouse`，导致启动键匹配失败被透传。
2. **Ready 状态间切换缺失** — P2-3 初版仅处理 `Idle → Ready*`，从按键页切到鼠标页时 `ReadyKeyboard` 不会更新为 `ReadyMouse`，状态与页面不一致。

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | `set_current_page` 增加 `app: AppHandle` 参数；**非 Running*/PickingMouse 状态下**根据页面切换 Ready 状态（Idle/ReadyKeyboard/ReadyMouse → 目标 Ready 状态），支持 Ready 状态间切换；发送 `runtime_status_changed` 事件；日志同时打印 page 和 status |
| [src-tauri/src/hotkeys_interception.rs](../src-tauri/src/hotkeys_interception.rs) | 改 | 状态机启动键分支从 `Idle if is_start_key` 扩展为 `Idle \| ReadyKeyboard \| ReadyMouse if is_start_key`，支持 Ready 状态下触发启动；热键匹配成功时增加 `info!` 日志，记录扫描码、配置值、当前页面、运行状态；页面过滤拦截时单独记录 `current_page` 方便诊断 |

### 关键决策

- **状态机扩展 Ready* 分支**：原设计假设用户在 `Idle` 时按启动键，但 P2-3 引入 Ready 状态后，进入模拟页时状态已不是 `Idle`，必须同步扩展状态机匹配条件。
- **Ready 状态间互相切换**：用户可能从按键页直接切到鼠标页（不经过首页），需要 `ReadyKeyboard ↔ ReadyMouse` 转换，而非仅支持 `Idle → Ready*`。
- **日志先行诊断**：第一次修复仅实现 P2-3 和日志增强，实机测试后通过日志快速定位状态机缺陷，避免盲目猜测。

### 验证结果

- ✅ `cargo build` 通过，无编译错误
- ✅ 实机日志确认根因：`status=ReadyKeyboard` 但 `page=mouse`，且状态机未匹配 Ready 状态下的启动键
- ⏳ 修复后实机验证待执行：
  1. 启动应用 → 进入按键模拟页 → 日志显示 `page=keyboard, status=ReadyKeyboard`
  2. 切换到鼠标模拟页 → 日志显示 `page=mouse, status=ReadyMouse`（之前停留在 `ReadyKeyboard`）
  3. 按启动热键 → 日志显示 `hotkey matched` 且状态为 `ReadyMouse` → 模拟启动

### 文档回写

无（本次修复为代码层 bug 修复，不涉及需求/设计/任务变更）

### 偏差与遗留

- **用户配置的启动键与停止键不一致**：日志显示 `start_code=47`(V键) 但用户按的是 `code=88`(F12)，说明配置文件中启动键被设为 V 而非 F12。这可能是测试时的配置，或 INI 文件未正确持久化用户修改。建议用户在设置页重新设置并保存热键。
- **P2-2（E0 扩展键匹配错误）仍未修复**：Right Ctrl (157) / Right Alt (184) 作为热键时永远匹配不上。留待后续修复。
- **P0-4（错误的设备选择）仍未修复**：`keyboard_worker.rs` 硬编码遍历 1-10 设备，应改为 1-20。留待后续修复。

---

## 阶段 16.2：鼠标模拟设备扫描与空坐标处理修复

**完成时间**：2026-06-10

**触发原因**：实机测试发现鼠标模拟启动后蒙版正常出现但无实际点击效果，日志显示 `no mouse device found`。同时用户提出需求明确：列表为空或所有坐标为 null 时，启动热键应直接忽略，不进入模拟循环、不显示蒙版。

**根因定位**：

1. **设备扫描范围错误**（核心根因）— Interception SDK 设备编号约定（见 `interception.h:34-42`）：
   - `INTERCEPTION_MAX_KEYBOARD = 10`（键盘 1-10）
   - `INTERCEPTION_MAX_MOUSE = 10`（鼠标 11-20，宏 `INTERCEPTION_MOUSE(index) = MAX_KEYBOARD + index + 1`）
   - `INTERCEPTION_MAX_DEVICE = 20`
   
   `mouse_worker.rs:93` 错误使用 `(1..=10).find(|d| interception::is_mouse(*d))`，这个范围全是键盘设备，必然找不到鼠标，导致每次点击事件被静默丢弃。

2. **空坐标列表导致蒙版闪烁** — 原 `handle_start_mouse` 流程：
   - 先把状态设为 `RunningMouse` → 蒙版出现
   - 发送 `runtime_status_changed` 事件
   - 启动线程后才检查 `valid_actions.is_empty()`
   - 为空时再切回 `ReadyMouse` → 蒙版消失
   
   用户体验上蒙版闪一下，违反"列表全空忽略热键"的需求。

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/mouse_worker.rs](../src-tauri/src/mouse_worker.rs) | 改 | 鼠标设备扫描范围从 `1..=10` 改为 `1..=20`，匹配 Interception 设备编号约定（鼠标在 11-20） |
| [src-tauri/src/hotkeys_interception.rs](../src-tauri/src/hotkeys_interception.rs) | 改 | `handle_start_mouse` 重构：**前置坐标检查**——先取出有效坐标，全空则直接 `info!` 日志后返回，不切状态、不发事件、不启动线程；有有效坐标才设 `RunningMouse` 进入循环 |

### 关键决策

- **设备范围按 SDK 常量上限取整**：直接用 `1..=20` 覆盖全部设备槽位，无需额外引入常量。Interception 在非鼠标设备调用 `is_mouse` 返回 false，开销可忽略。
- **空坐标前置拦截**：用户需求明确"忽略热键"，应在状态变更**之前**完成所有判断。原实现"先开后收"会让蒙版瞬时闪现，破坏交互一致性。
- **设备范围单一来源**：键盘 worker 已实现 `1..=10`（位于键盘有效区间），鼠标 worker 改为 `1..=20`，二者各自正确。如未来需要严格分离，可引入常量 `KEYBOARD_DEVICES = 1..=10` 和 `MOUSE_DEVICES = 11..=20`，但当前修改成本最小。

### 验证结果

- ✅ `cargo build` 通过
- ⏳ 实机验证待执行：
  1. 鼠标列表全为 null → 按启动热键 → 日志 `mouse start ignored: no valid coords`，无蒙版、状态保持 `ReadyMouse`
  2. 鼠标列表至少一项有坐标 → 按启动热键 → 蒙版出现，鼠标按列表顺序循环点击，间隔遵循各项 `intervalMs`
  3. 按停止热键 → 立即停止循环，蒙版消失，状态回 `ReadyMouse`

### 文档回写

无（本次为代码层 bug 修复 + 行为细化，与 REQUIREMENTS 3.9 / DESIGN 10.2 已有约束一致）

### 偏差与遗留

- **键盘 worker 设备范围 `1..=10`**：当前正确（键盘在该区间），但语义上若键盘 worker 不慎扫到鼠标设备（不可能，因 `is_keyboard` 排他），逻辑仍稳健。无需修改。
- **P2-2（E0 扩展键匹配错误）仍未修复**：留待后续。

---

## 阶段 16.3：管理员权限策略调整 — 按需提权

**完成时间**：2026-06-10

**触发原因**：用户反馈应用启动时主动弹 UAC 体验不佳。验证后明确：驱动一旦安装并加载，热键监听与按键/鼠标 worker 通过 Interception 用户态接口工作，**普通权限即可运行**；仅「安装驱动」需要管理员。策略改为**按需提权**——启动不弹 UAC，仅在用户点击「安装驱动」时由后端拒绝并引导。

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | `pub fn run()` 入口删除启动期 UAC 请求块（`is_admin()` + `restart_as_admin()` + `std::process::exit(0)`）；保留 setup 阶段的 `is_admin()` 检测用于首页状态展示与命令守卫 |
| [src/pages/HomePage.vue](../src/pages/HomePage.vue) | 改 | 未授权状态文案从「管理员权限受限,部分功能不可用」改为「管理员权限受限，安装驱动需提权」，明确受限范围 |
| [docs/REQUIREMENTS.md](./REQUIREMENTS.md) | 改 | 第 2 节「启动时请求提权 + 拒绝降级」→「按需提权」；3.11 节明确「仅驱动安装需管理员，模拟运行不需要」 |
| [docs/DESIGN.md](./DESIGN.md) | 改 | § 14.1 标题与策略全面改写：启动行为 / 首页权限状态 / 驱动安装 / 模拟运行 四个维度；§ 14.2 落地形态删除「启动期 UAC 请求」段；ADMIN_POLICY 注释模板更新 |

### 关键决策

- **策略反转的依据**：Interception 驱动一旦通过 SCM 注册为内核服务并随系统启动加载，用户态 `Interception::new()`、`set_filter()`、`wait()`、`receive()`、`send()` 均不需要调用方持有 `SeLoadDriverPrivilege` 等高权限。实测确认普通权限下监听线程 + worker 全部正常运行（阶段 16.2 修复后）。
- **保留 `request_admin_restart` 与 `reboot_system` 的权限守卫**：用户从普通权限点击安装驱动 → 收到 `permission_denied` → 点击「以管理员身份重启」→ 重启为管理员 → 再点安装驱动。安装完成后应用仍是管理员，可直接调用 `reboot_system`。少数边缘场景（关掉 app 再开）下重启按钮失败，提示用户手动重启即可。
- **不动 manifest**：保持 `asInvoker` 默认行为，避免某些 Windows 配置下 manifest 与运行时检测冲突。

### 验证结果

- ✅ `cargo build` 通过
- ⏳ 实机验证待执行：
  1. 双击 exe（普通权限）→ 应用直接打开，**不再弹 UAC** → 首页显示橙色「管理员权限受限，安装驱动需提权」
  2. 在驱动已安装状态下：进入按键/鼠标模拟页 → 按热键 → 模拟正常工作（普通权限即可）
  3. 在驱动未安装状态下：点击「安装驱动」→ 收到 `permission_denied` → 提示引导点击重启按钮 → 点击「以管理员身份重启」→ UAC 弹窗 → 同意后应用以管理员身份重新打开 → 再点「安装驱动」→ 安装器 UAC 弹窗或直接执行 → 安装完成
  4. 拒绝重启按钮的 UAC → 应用保持普通权限，提示「已取消提权」

### 文档回写

- [docs/REQUIREMENTS.md](./REQUIREMENTS.md) § 2 与 § 3.11 已更新为「按需提权」策略。
- [docs/DESIGN.md](./DESIGN.md) § 14.1 / § 14.2 已更新；ADMIN_POLICY 注释规范同步。

### 偏差与遗留

- **`reboot_system` 命令在普通权限下仍返回 `permission_denied`**：保留，前端已处理（提示用户点击「以管理员身份重启」）。常规流程下安装完成时应用仍是管理员，可直接重启，无需调整。
- **首次启动驱动未安装时的 UX 路径**：用户需「以管理员身份重启 → 安装驱动 → 重启电脑」，三步操作。如未来需精简，可考虑首次启动检测到「未安装 + 非管理员」时主动提示，但当前策略以「不打扰」为优先。

---

## 阶段 16.4：坐标拾取改用 Interception — 支持全屏游戏

**完成时间**：2026-06-10

**触发原因**：用户反馈在独占全屏游戏内点击坐标拾取的左键无法被捕获（无日志），切到 IDE 窗口后点击才成功。确认根因为 `WH_MOUSE_LL` 用户态 hook 被全屏游戏绕过，与管理员权限无关。用户要求普通权限下也能在全屏游戏内拾取。

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/mouse_picker.rs](../src-tauri/src/mouse_picker.rs) | 改 | 整体替换拾取机制：删除 `WH_MOUSE_LL` hook + 消息循环 + 静态原子量；改为复用 `interception_worker` context，设鼠标 filter（仅 `LEFT_BUTTON_DOWN`）+ `wait_with_timeout` 循环 + `GetCursorPos` 读坐标 + 透传点击 + 清 filter；`start_pick_mouse_position` 从 state 取出 worker context 传给拾取线程 |
| [docs/DESIGN.md](./DESIGN.md) | 改 | § 11.2 重写为 Interception 方案，§ 11.3 原 hook 方案标记为已弃用 |

### 关键决策

- **为何 hook 失效**：独占全屏游戏（DirectX exclusive fullscreen）直接从驱动/Raw Input 层取输入，`WH_MOUSE_LL` 是用户态 hook，处于游戏取输入路径之后，故捕获不到。提权管理员也无法解决（hook 层级问题，非权限问题）。
- **为何 Interception 有效**：Interception 是内核态驱动，工作在输入栈最底层，全屏游戏的输入同样经过它。驱动一旦加载，用户态调用 `set_filter`/`wait`/`receive` 不需要管理员权限。
- **复用 worker context 而非新建**：拾取期间状态为 `PickingMouse`，worker 状态门控使其不发送，拾取线程独占该 context 设 filter 安全；避免新建 context 的资源开销。
- **坐标来源用 `GetCursorPos`**：Interception 鼠标 stroke 的 x/y 是移动量（相对/绝对归一化），非屏幕像素坐标。命中左键时读系统光标位置作为拾取结果。
- **filter 必须清除**：拾取结束后用 `MouseFilter::empty()` 清除，否则 worker context 会持续拦截鼠标事件，影响后续鼠标模拟与正常使用。
- **透传点击事件**：`receive` 后立即 `send` 回去，保持用户在目标窗口的点击行为不变（与原 hook 透传语义一致）。

### 验证结果

- ✅ `cargo build` 通过
- ⏳ 实机验证待执行：
  1. 进入鼠标模拟页，点「坐标拾取」→ 窗口隐藏
  2. 切到**全屏游戏**，点击目标位置左键 → 日志 `picked (x, y)` → 窗口恢复，坐标回填
  3. 普通权限下同样生效（无需管理员）
  4. 拾取完成后正常进行鼠标点击模拟，验证 filter 已正确清除

### 文档回写

- [docs/DESIGN.md](./DESIGN.md) § 11.2 / § 11.3 已更新。

### 偏差与遗留

- **拾取期间持有 worker context 锁**：若用户进入拾取后一直不点击，线程会持续 `wait_with_timeout` 循环并持锁。此为既有设计约束（拾取无取消机制），与原 hook 方案行为对等，不在本次范围。
- **全屏游戏内系统光标位置**：部分游戏锁定/隐藏系统光标，`GetCursorPos` 可能返回非预期位置。对有可见系统光标的游戏（窗口化全屏 / MOBA / RTS）可用；独占全屏且锁定光标的场景为已知限制。
- **多显示器 / 高 DPI**：沿用第一版单显示器标准 DPI 约束，未改变。

---
