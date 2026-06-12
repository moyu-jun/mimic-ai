// 热键提示音 — 启动/停止热键生效时播放
//
// 仅 Windows。使用 PlaySoundW(SND_ASYNC | SND_MEMORY) 异步播放，立即返回；
// 当一个声音正在播放时再次调用 PlaySoundW 会自动打断旧声音并播放新声音，
// 天然满足「短时间内连续触发时优先播放后者、前者直接被打断」的需求。
//
// 声音文件位于 exe 同级 audio 目录：
//   - audio/按键开启.wav —— 启动热键生效（进入 Running*）时播放
//   - audio/按键关闭.wav —— 停止热键生效（Running* → Idle）时播放
//
// 低延迟策略（DESIGN 18.4）：
//   1. 启动时通过 sound::load_cache 把两个 wav 文件一次性读入内存常驻缓存。
//   2. 每次触发走 PlaySoundW(SND_MEMORY | SND_ASYNC)，触发路径上零文件 I/O，
//      端到端延迟 < 5ms。
//   3. sound::warmup + sound::start_keepalive 仍用合成静音 wav 保活 waveOut 设备，
//      避免设备空闲后冷启动重新初始化（50~200ms）。
//   4. 录制覆盖 wav 后由 sound_recorder::save_trimmed_audio 调用 reload_cache 刷新。

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// 启动提示音文件名（exe 同级 audio 目录下）。
pub const FILE_START: &str = "按键开启.wav";
/// 停止提示音文件名。
pub const FILE_STOP: &str = "按键关闭.wav";

/// 内存常驻 wav 缓存 — 见模块顶部说明。
///
/// key 为文件名字面量（&'static str）；value 为整个 wav 文件的字节副本（含 RIFF/WAVE 头）。
/// `Arc<Vec<u8>>` 保证 SND_ASYNC 播放期间缓冲存活：cache 始终持有一份强引用，
/// 触发时 `Arc::clone` 出本地副本调用 PlaySoundW；PlaySoundW 内部已把数据排进
/// waveOut 队列后才返回，下一次 PlaySoundW 会自动 purge 旧播放，故无需手动追踪。
#[allow(clippy::type_complexity)]
static SOUND_CACHE: OnceLock<RwLock<HashMap<&'static str, Arc<Vec<u8>>>>> = OnceLock::new();

fn cache() -> &'static RwLock<HashMap<&'static str, Arc<Vec<u8>>>> {
    SOUND_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 解析 exe 同级 audio 目录下指定文件的绝对路径。
fn audio_file_path(file_name: &str) -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("audio").join(file_name)))
}

/// 从磁盘读 wav 文件全部字节；缺失或失败返回 None 并记录日志。
fn read_wav_bytes(file_name: &str) -> Option<Vec<u8>> {
    let path = audio_file_path(file_name)?;
    if !path.exists() {
        log::warn!("[sound] sound file not found: {}", path.display());
        return None;
    }
    match std::fs::read(&path) {
        Ok(bytes) => {
            log::info!("[sound] loaded {} ({} bytes)", path.display(), bytes.len());
            Some(bytes)
        }
        Err(e) => {
            log::error!("[sound] failed to read {}: {}", path.display(), e);
            None
        }
    }
}

/// 启动期一次性把两个 wav 文件加载进缓存 — DESIGN 18.4。
///
/// 应在 setup 阶段 warmup 之后调用一次。文件缺失的 key 留空，
/// 触发时 cache miss 静默跳过，与「文件不存在」分支语义一致。
#[cfg(windows)]
pub fn load_cache() {
    let map = cache();
    for &name in &[FILE_START, FILE_STOP] {
        if let Some(bytes) = read_wav_bytes(name) {
            if let Ok(mut w) = map.write() {
                w.insert(name, Arc::new(bytes));
            }
        }
    }
}

#[cfg(not(windows))]
pub fn load_cache() {}

/// 重新加载指定 wav 进缓存 — 录制覆盖文件后调用。
///
/// 调用方在调用前应已通过 `purge_playing` 释放可能的旧播放，
/// 此处只负责读盘 + 替换 cache 条目。读失败时保留旧缓存。
#[cfg(windows)]
pub fn reload_cache(file_name: &'static str) {
    if let Some(bytes) = read_wav_bytes(file_name) {
        if let Ok(mut w) = cache().write() {
            w.insert(file_name, Arc::new(bytes));
            log::info!("[sound] cache reloaded for {}", file_name);
        }
    } else {
        log::warn!(
            "[sound] reload_cache: failed to read {}, keeping old cache",
            file_name
        );
    }
}

#[cfg(not(windows))]
pub fn reload_cache(_file_name: &'static str) {}

#[cfg(windows)]
fn play_file(file_name: &str) {
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

    // cache miss = 文件未加载（启动时缺失或读失败），静默跳过
    let buf = match cache().read() {
        Ok(map) => map.get(file_name).cloned(),
        Err(e) => {
            log::error!("[sound] cache read lock poisoned: {}", e);
            return;
        }
    };
    let buf = match buf {
        Some(b) => b,
        None => {
            log::warn!(
                "[sound] cache miss: {} (file missing or not loaded)",
                file_name
            );
            return;
        }
    };

    // SND_MEMORY:   第一参数指向内存中的 wav 镜像（无文件 I/O）
    // SND_ASYNC:    异步播放，立即返回；新调用会打断正在播放的旧声音
    // SND_NODEFAULT: 失败时不退回系统默认提示音
    let ok = unsafe {
        PlaySoundW(
            buf.as_ptr() as *const u16,
            std::ptr::null_mut(),
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        )
    };
    if ok == 0 {
        log::error!("[sound] PlaySoundW(SND_MEMORY) failed for {}", file_name);
    }
}

/// 播放启动提示音（按键开启.wav）。
#[cfg(windows)]
pub fn play_start() {
    play_file(FILE_START);
}

/// 播放停止提示音（按键关闭.wav）。
#[cfg(windows)]
pub fn play_stop() {
    play_file(FILE_STOP);
}

#[cfg(not(windows))]
pub fn play_start() {}

#[cfg(not(windows))]
pub fn play_stop() {}

/// 停止当前所有正在播放的提示音，释放可能持有的文件句柄 — 阶段 18 录制覆盖前调用。
#[cfg(windows)]
pub fn purge_playing() {
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_PURGE};
    // 第一参数为 NULL + SND_PURGE：停止与本进程关联的所有播放
    unsafe {
        PlaySoundW(std::ptr::null(), std::ptr::null_mut(), SND_PURGE);
    }
}

#[cfg(not(windows))]
pub fn purge_playing() {}

/// 构建最小有效 WAV（44100Hz, 16-bit, mono, 10ms 静音）—— 内存中生成，无需文件。
///
/// 用于 warmup / keepalive：通过 SND_MEMORY 传给 PlaySoundW，绕过磁盘 I/O。
fn make_silent_wav() -> Vec<u8> {
    const SAMPLE_RATE: u32 = 44100;
    const N_SAMPLES: u32 = 441; // 10ms
    let data_size = N_SAMPLES * 2; // 16-bit = 2 bytes/sample
    let riff_size = 36 + data_size; // RIFF payload = "WAVE" + fmt chunk(24) + data chunk header(8) + data

    let mut v = Vec::with_capacity(44 + data_size as usize);
    // RIFF header
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&riff_size.to_le_bytes());
    v.extend_from_slice(b"WAVE");
    // fmt chunk
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    v.extend_from_slice(&1u16.to_le_bytes());  // PCM
    v.extend_from_slice(&1u16.to_le_bytes());  // mono
    v.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    v.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    v.extend_from_slice(&2u16.to_le_bytes());  // block align
    v.extend_from_slice(&16u16.to_le_bytes()); // bits/sample
    // data chunk
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_size.to_le_bytes());
    v.resize(v.len() + data_size as usize, 0u8); // 静音：全零
    v
}

/// 预热音频设备，消除首次触发热键时的冷启动延迟。
///
/// 在应用 setup 阶段调用一次：同步播放 10ms 静音，使 waveOut 设备进入就绪状态。
/// 此后热键触发时 PlaySoundW 直接复用已就绪的设备，延迟从 50~200ms 降至 <20ms。
#[cfg(windows)]
pub fn warmup() {
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_MEMORY, SND_NODEFAULT};
    // 不设 SND_ASYNC → 同步播放，函数返回后 wav 缓冲即可释放，无生命周期风险
    let wav = make_silent_wav();
    let ok = unsafe {
        PlaySoundW(
            wav.as_ptr() as *const u16,
            std::ptr::null_mut(),
            SND_MEMORY | SND_NODEFAULT,
        )
    };
    if ok == 0 {
        log::warn!("[sound] warmup: PlaySoundW failed (audio device may be unavailable)");
    } else {
        log::info!("[sound] audio device warmed up");
    }
}

#[cfg(not(windows))]
pub fn warmup() {}

/// 启动音频保活线程，防止设备长时间空闲后重新进入冷启动状态。
///
/// 每 5 秒尝试播放一次 10ms 静音（SND_MEMORY, 同步）：
/// - 若当前有 SND_ASYNC 声音正在播放（SND_NOSTOP），立即返回 FALSE，不打断真实提示音；
/// - 若设备空闲，同步播放 10ms 静音，使设备保持就绪。
///
/// 应在 setup 阶段调用一次，线程随进程退出自然结束。
#[cfg(windows)]
pub fn start_keepalive() {
    std::thread::spawn(|| {
        use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_MEMORY, SND_NODEFAULT, SND_NOSTOP};
        let wav = make_silent_wav();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            // SND_NOSTOP: 正在异步播放时立即返回 FALSE，不打断；否则同步播放 10ms 静音
            unsafe {
                PlaySoundW(
                    wav.as_ptr() as *const u16,
                    std::ptr::null_mut(),
                    SND_MEMORY | SND_NODEFAULT | SND_NOSTOP,
                );
            }
        }
    });
}

#[cfg(not(windows))]
pub fn start_keepalive() {}

/// 返回 exe 同级 audio 目录下提示音文件是否存在 — 阶段 18 设置页状态展示用。
pub fn sound_files_exist() -> (bool, bool) {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    match dir {
        Some(d) => {
            let audio_dir = d.join("audio");
            (
                audio_dir.join(FILE_START).exists(),
                audio_dir.join(FILE_STOP).exists(),
            )
        }
        None => (false, false),
    }
}
