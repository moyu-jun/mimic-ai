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

[此处省略阶段 1 到阶段 18.7 的历史记录，保持原文不变]

---

## 阶段 18.8：DLL 加载路径修复

**完成时间**：2026-06-12

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/build.rs](../src-tauri/build.rs) | 改 | 重写为完整的 `copy_extra_to_target()` 实现：递归复制 `extra/` 目录所有内容到 `target/{debug,release}/`，保留子目录结构；添加 `rerun-if-changed=../extra` 触发条件 |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | 移除 `SetDllDirectoryW` 设置 `driver/` 子目录的逻辑（L492-512），改为注释说明 `interception.dll` 现通过 `build.rs` 复制到 exe 同级目录，依赖 Windows 标准 DLL 搜索顺序 |
| [docs/DESIGN.md](../docs/DESIGN.md) | 改 | § 2 项目结构：`drivers/interception/` 改为 `extra/{interception.dll, audio/, driver/}`；§ 12.3 新增「DLL 部署策略」小节，说明 `build.rs` 自动复制、`interception.dll` 必须在 exe 同级、不再使用 `SetDllDirectoryW` |

### 关键决策

- **问题根因**：`interception.dll` 是 exe 的隐式导入依赖，Windows 加载器在进入 `main` 之前就解析导入表。原设计将 DLL 放在 `<exe_dir>/driver/` 子目录，并在 `lib.rs::run()` 里调用 `SetDllDirectoryW` 添加搜索路径——但该调用在 `main` 已启动后才执行，对启动期隐式依赖不起作用。`npm run tauri dev` 能工作是因为 cargo 将 `interception-sys` 的 `OUT_DIR` 加入子进程 PATH；双击 exe 时没有这种环境变量扩展，导致加载失败。
- **修复方案**：将 `interception.dll` 放到 exe 同级目录（Windows DLL 搜索顺序最高优先级：应用程序所在目录），通过 `build.rs` 在编译时自动复制 `extra/` 内容到 `target/{debug,release}/`，保留子目录结构（`driver/install-interception.exe` 仍在子目录）。
- **资源目录重组**：`extra/interception.dll`（exe 同级）、`extra/driver/install-interception.exe`（安装器）、`extra/audio/*.wav`（提示音），全部通过 `copy_dir_all` 递归复制。
- **移除 `SetDllDirectoryW`**：该调用仅对显式 `LoadLibrary` 有效，对隐式导入无效且时机已晚；DLL 直接在 exe 同级后该逻辑冗余，直接移除并留注释说明。

### 验证结果

- `cargo build`（src-tauri 目录）— 通过，build.rs 输出 `warning: Copied extra/ to target/debug`。
- 目录结构验证：
  - `target/debug/interception.dll` 存在（exe 同级）
  - `target/debug/driver/install-interception.exe` 存在
  - `target/debug/audio/{按键开启.wav, 按键关闭.wav}` 存在
- 直接执行验证：`cd target/debug && ./mimic-ai.exe` 成功启动，日志显示完整启动流程无 DLL 加载错误。

### 文档回写

- DESIGN § 2：项目结构更新为 `extra/` 目录布局。
- DESIGN § 12.3：新增「DLL 部署策略」说明 `build.rs` 自动复制、exe 同级加载、移除 `SetDllDirectoryW`。

### 偏差与遗留

- 原设计将驱动文件放在 `<exe_dir>/driver/` 子目录，现调整为 `interception.dll` 在 exe 同级、仅安装器在子目录，与 DESIGN 初版有出入但已同步更新文档。
- Release 构建与最终打包（NSIS installer）的 `extra/` 复制需在后续打包阶段验证。

---

## 阶段 19：提示音延迟优化（内存常驻 + SND_MEMORY）

**完成时间**：2026-06-12

### 改动摘要

| 文件 | 改动类型 | 关键点 |
|------|---------|--------|
| [src-tauri/src/sound.rs](../src-tauri/src/sound.rs) | 改 | 全文重写：新增 `SOUND_CACHE: OnceLock<RwLock<HashMap<&'static str, Arc<Vec<u8>>>>>`，暴露 `load_cache` / `reload_cache`，改写 `play_file` 走 `PlaySoundW(SND_MEMORY \| SND_ASYNC)` |
| [src-tauri/src/lib.rs](../src-tauri/src/lib.rs) | 改 | setup 顺序：`warmup()` → `load_cache()` → `start_keepalive()`（L526-531） |
| [src-tauri/src/sound_recorder.rs](../src-tauri/src/sound_recorder.rs) | 改 | `save_trimmed_audio` 写盘成功后调 `crate::sound::reload_cache(file_name)`（L350-352）；`file_name` 从字面量改用模块常量 `crate::sound::FILE_START` / `FILE_STOP`（L302-306） |
| [docs/REQUIREMENTS.md](../docs/REQUIREMENTS.md) | 改 | § 3.13 末尾追加「响应延迟要求 < 10ms」条目，说明触发路径零同步 I/O 约束 |
| [docs/DESIGN.md](../docs/DESIGN.md) | 改 | § 18.4 新增「内存常驻缓存」小节，详述问题、方案、数据结构、生命周期、关键约束、预期收益 |
| [docs/TASKS.md](../docs/TASKS.md) | 改 | 追加阶段 19 描述、任务清单、验收标准 |

### 关键决策

- **问题定位**：现有 `PlaySoundW(SND_FILENAME | SND_ASYNC)` 每次触发在调用线程同步完成「`current_exe()` + `path.exists()` + UTF-16 转换 + 打开文件 + 读完整内容 + 解析 RIFF 头」，堆叠 10–50ms 可感知延迟（冷盘更糟）。设备虽已通过 warmup / keepalive 保活，但文件 I/O 在触发瞬间不可避免。
- **方案选型**：启动期一次性读两个 wav 进内存常驻缓存（`OnceLock<RwLock<HashMap>>`），触发路径直接走 `PlaySoundW(SND_MEMORY | SND_ASYNC | SND_NODEFAULT)` 从 `Arc<Vec<u8>>` 播放，零文件 I/O，端到端 < 5ms。备选「`waveOutOpen` + 预填 PCM 缓冲」可压到 <2ms 但代码量 ~150 行，通常无必要；先上方案 A，实测若仍嫌慢再上 waveOut。
- **生命周期安全**：`SND_MEMORY + SND_ASYNC` 要求播放期间 buffer 存活。通过 cache 常驻一份 `Arc` 强引用 + 触发时 `Arc::clone` 保证安全；录制覆盖后调 `reload_cache` 前已在 `save_trimmed_audio` 中 `purge_playing()` 停止旧播放，替换缓存 entry 时不会 use-after-free。
- **cache miss 语义**：文件启动时缺失或读失败 → key 留空 → 触发时 `log::warn` 静默跳过，与现有「文件不存在」分支一致，不报错、不阻塞模拟。
- **与 warmup / keepalive 正交**：后者用合成静音 wav 保活 waveOut 设备（避免冷启动重新初始化 50~200ms），与本阶段「避免文件 I/O 10~50ms」独立叠加，共同压到 < 5ms。

### 验证结果

- `cargo check`：通过（11.35s）。
- `cargo clippy -- -D warnings`：通过（4.63s）；SOUND_CACHE 类型复杂度用 `#[allow(clippy::type_complexity)]` 豁免（类型嵌套深但语义明确，不适合拆分 type alias）。
- 实机验收：⏳ 待用户实机测试热键触发与试听按钮延迟是否 < 10ms、录制覆盖后 reload_cache 生效、cache miss 静默跳过。

### 文档回写

- REQUIREMENTS § 3.13：追加「响应延迟 < 10ms」非功能性能要求。
- DESIGN § 18.4：新增内存常驻缓存策略完整设计。
- TASKS：新增阶段 19。

### 偏差与遗留

- 无。按 TASKS 阶段 19 任务清单完整实施，无偏差。
- `make_silent_wav` 函数内注释尾空格对齐问题（rustfmt diff）为历史遗留，保持原样；仅规整新写代码的 `log::warn!` 多行格式。

---
