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

