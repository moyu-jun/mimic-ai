// 热键提示音 — 启动/停止热键生效时播放
//
// 仅 Windows。使用 PlaySoundW(SND_ASYNC | SND_FILENAME) 异步播放，立即返回；
// 当一个声音正在播放时再次调用 PlaySoundW 会自动打断旧声音并播放新声音，
// 天然满足「短时间内连续触发时优先播放后者、前者直接被打断」的需求。
//
// 声音文件位于 exe 同级 audio 目录：
//   - audio/按键开启.wav —— 启动热键生效（进入 Running*）时播放
//   - audio/按键关闭.wav —— 停止热键生效（Running* → Idle）时播放
//
// 低延迟策略（见 sound::warmup / sound::start_keepalive）：
//   PlaySoundW 底层使用 waveOut API，首次调用（或空闲 30s+ 后）需初始化音频设备，
//   耗时 50~200ms，造成明显可感知的延迟。通过启动时同步预热 + 5s 周期保活，
//   使设备始终处于就绪状态，实际触发时延降至 <20ms。

#[cfg(windows)]
fn play_file(file_name: &str) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Media::Audio::{
        PlaySoundW, SND_ASYNC, SND_FILENAME, SND_NODEFAULT,
    };

    let path = match std::env::current_exe() {
        Ok(exe) => match exe.parent() {
            Some(dir) => dir.join("audio").join(file_name),
            None => {
                log::error!("[sound] exe has no parent dir");
                return;
            }
        },
        Err(e) => {
            log::error!("[sound] current_exe failed: {}", e);
            return;
        }
    };

    if !path.exists() {
        log::warn!("[sound] sound file not found: {}", path.display());
        return;
    }

    // 转 UTF-16 宽字符串（含结尾 NUL），PlaySoundW 要求宽字符路径
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // SND_ASYNC:    异步播放，立即返回；新调用会打断正在播放的旧声音
    // SND_FILENAME: 第一参数按文件路径解析
    // SND_NODEFAULT: 失败时不退回系统默认提示音
    let ok = unsafe {
        PlaySoundW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            SND_ASYNC | SND_FILENAME | SND_NODEFAULT,
        )
    };
    if ok == 0 {
        log::error!("[sound] PlaySoundW failed for {}", path.display());
    }
}

/// 播放启动提示音（按键开启.wav）。
#[cfg(windows)]
pub fn play_start() {
    play_file("按键开启.wav");
}

/// 播放停止提示音（按键关闭.wav）。
#[cfg(windows)]
pub fn play_stop() {
    play_file("按键关闭.wav");
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
                audio_dir.join("按键开启.wav").exists(),
                audio_dir.join("按键关闭.wav").exists(),
            )
        }
        None => (false, false),
    }
}
