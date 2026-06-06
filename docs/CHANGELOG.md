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
