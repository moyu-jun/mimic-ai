// 热键提示音 — 启动/停止热键生效时播放
//
// 仅 Windows。使用 PlaySoundW(SND_ASYNC | SND_FILENAME) 异步播放，立即返回；
// 当一个声音正在播放时再次调用 PlaySoundW 会自动打断旧声音并播放新声音，
// 天然满足「短时间内连续触发时优先播放后者、前者直接被打断」的需求。
//
// 声音文件位于 exe 同级目录：
//   - 按键开启.wav —— 启动热键生效（进入 Running*）时播放
//   - 按键关闭.wav —— 停止热键生效（Running* → Idle）时播放

#[cfg(windows)]
fn play_file(file_name: &str) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Media::Audio::{
        PlaySoundW, SND_ASYNC, SND_FILENAME, SND_NODEFAULT,
    };

    let path = match std::env::current_exe() {
        Ok(exe) => match exe.parent() {
            Some(dir) => dir.join(file_name),
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
