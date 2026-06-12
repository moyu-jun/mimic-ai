// 热键提示音 — 启动/停止热键生效时播放
//
// 仅 Windows。使用 waveOut API 直接播放内存 PCM，预先打开设备 + 预先准备缓冲，
// 触发时仅需 waveOutReset + waveOutWrite，端到端延迟 < 15ms。
//
// 声音文件位于 exe 同级 audio 目录：
//   - audio/按键开启.wav —— 启动热键生效（进入 Running*）时播放
//   - audio/按键关闭.wav —— 停止热键生效（Running* → Idle）时播放
//
// 低延迟策略：
//   1. 启动时 waveOutOpen 打开设备（44100/16-bit/mono），设备常驻不关闭。
//   2. 加载 wav 文件后解析出 PCM 数据，waveOutPrepareHeader 预备缓冲。
//   3. 触发时 waveOutReset（打断旧播放）+ waveOutWrite（队列新缓冲），~5ms 完成。
//   4. 无需 keepalive — 设备始终处于打开状态，无冷启动开销。
//   5. 录制覆盖 wav 后由 reload_cache 重新解析 + 替换缓冲。

#[cfg(windows)]
mod inner {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    use windows_sys::Win32::Media::Audio::{
        waveOutClose, waveOutOpen, waveOutPrepareHeader, waveOutReset,
        waveOutUnprepareHeader, waveOutWrite, CALLBACK_NULL, HWAVEOUT, WAVEHDR,
        WAVEFORMATEX, WAVE_FORMAT_PCM, WAVE_MAPPER,
    };

    pub const FILE_START: &str = "按键开启.wav";
    pub const FILE_STOP: &str = "按键关闭.wav";

    struct PreparedBuf {
        hdr: Box<WAVEHDR>,
        _pcm: Arc<Vec<u8>>,
    }

    // SAFETY: WAVEHDR + HWAVEOUT are thread-safe when access is serialized by Mutex.
    unsafe impl Send for PreparedBuf {}

    struct WaveDevice {
        handle: HWAVEOUT,
        bufs: HashMap<&'static str, PreparedBuf>,
    }

    unsafe impl Send for WaveDevice {}

    static DEVICE: OnceLock<Mutex<Option<WaveDevice>>> = OnceLock::new();

    fn device_mutex() -> &'static Mutex<Option<WaveDevice>> {
        DEVICE.get_or_init(|| Mutex::new(None))
    }

    fn audio_file_path(file_name: &str) -> Option<std::path::PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|d| d.join("audio").join(file_name)))
    }

    fn read_wav_bytes(file_name: &str) -> Option<Vec<u8>> {
        let path = audio_file_path(file_name)?;
        if !path.exists() {
            log::warn!("[sound] file not found: {}", path.display());
            return None;
        }
        match std::fs::read(&path) {
            Ok(bytes) => {
                log::info!("[sound] loaded {} ({} bytes)", path.display(), bytes.len());
                Some(bytes)
            }
            Err(e) => {
                log::error!("[sound] read failed {}: {}", path.display(), e);
                None
            }
        }
    }

    /// 从 wav 字节中解析 PCM 数据起始偏移和长度,同时验证格式。
    /// 返回 (data_offset, data_len) 或 None。
    fn parse_wav(raw: &[u8]) -> Option<(usize, usize)> {
        if raw.len() < 44 {
            return None;
        }
        if &raw[0..4] != b"RIFF" || &raw[8..12] != b"WAVE" {
            return None;
        }
        // 扫描 "data" chunk（跳过可能的 LIST/INFO 等元数据 chunk）
        let mut pos = 12;
        while pos + 8 <= raw.len() {
            let chunk_id = &raw[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                raw[pos + 4],
                raw[pos + 5],
                raw[pos + 6],
                raw[pos + 7],
            ]) as usize;
            if chunk_id == b"fmt " && chunk_size >= 16 {
                let fmt_tag = u16::from_le_bytes([raw[pos + 8], raw[pos + 9]]);
                let channels = u16::from_le_bytes([raw[pos + 10], raw[pos + 11]]);
                let sample_rate =
                    u32::from_le_bytes([raw[pos + 12], raw[pos + 13], raw[pos + 14], raw[pos + 15]]);
                let bits = u16::from_le_bytes([raw[pos + 22], raw[pos + 23]]);
                if fmt_tag != 1 || channels != 1 || sample_rate != 44100 || bits != 16 {
                    log::warn!(
                        "[sound] unsupported format: tag={} ch={} rate={} bits={}",
                        fmt_tag,
                        channels,
                        sample_rate,
                        bits
                    );
                    return None;
                }
            }
            if chunk_id == b"data" {
                let data_offset = pos + 8;
                let data_len = chunk_size.min(raw.len() - data_offset);
                return Some((data_offset, data_len));
            }
            // 下一个 chunk（对齐到偶数字节）
            pos += 8 + ((chunk_size + 1) & !1);
        }
        None
    }

    fn open_device() -> Option<HWAVEOUT> {
        let fmt = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM as u16,
            nChannels: 1,
            nSamplesPerSec: 44100,
            nAvgBytesPerSec: 44100 * 2,
            nBlockAlign: 2,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        let mut handle: HWAVEOUT = std::ptr::null_mut();
        let result = unsafe {
            waveOutOpen(
                &mut handle,
                WAVE_MAPPER,
                &fmt,
                0,
                0,
                CALLBACK_NULL,
            )
        };
        if result == 0 {
            Some(handle)
        } else {
            log::error!("[sound] waveOutOpen failed: error {}", result);
            None
        }
    }

    fn prepare_buf(handle: HWAVEOUT, pcm_data: Arc<Vec<u8>>) -> Option<PreparedBuf> {
        let mut hdr = Box::new(WAVEHDR {
            lpData: pcm_data.as_ptr() as *mut u8,
            dwBufferLength: pcm_data.len() as u32,
            dwBytesRecorded: 0,
            dwUser: 0,
            dwFlags: 0,
            dwLoops: 0,
            lpNext: std::ptr::null_mut(),
            reserved: 0,
        });
        let result = unsafe {
            waveOutPrepareHeader(
                handle,
                hdr.as_mut() as *mut WAVEHDR,
                std::mem::size_of::<WAVEHDR>() as u32,
            )
        };
        if result != 0 {
            log::error!("[sound] waveOutPrepareHeader failed: {}", result);
            return None;
        }
        Some(PreparedBuf { hdr, _pcm: pcm_data })
    }

    fn unprepare_buf(handle: HWAVEOUT, buf: &mut PreparedBuf) {
        unsafe {
            waveOutReset(handle);
            waveOutUnprepareHeader(
                handle,
                buf.hdr.as_mut() as *mut WAVEHDR,
                std::mem::size_of::<WAVEHDR>() as u32,
            );
        }
    }

    fn load_single(handle: HWAVEOUT, file_name: &'static str) -> Option<PreparedBuf> {
        let raw = read_wav_bytes(file_name)?;
        let (offset, len) = parse_wav(&raw)?;
        let pcm: Vec<u8> = raw[offset..offset + len].to_vec();
        let pcm_arc = Arc::new(pcm);
        prepare_buf(handle, pcm_arc)
    }

    /// 打开 waveOut 设备并加载两个提示音缓冲。
    pub fn init() {
        let handle = match open_device() {
            Some(h) => h,
            None => return,
        };
        log::info!("[sound] waveOut device opened");

        let mut bufs = HashMap::new();
        for &name in &[FILE_START, FILE_STOP] {
            if let Some(buf) = load_single(handle, name) {
                bufs.insert(name, buf);
                log::info!("[sound] prepared buffer for {}", name);
            }
        }

        let mut guard = device_mutex().lock().unwrap();
        *guard = Some(WaveDevice { handle, bufs });
    }

    pub fn play_file(file_name: &str) {
        let mut guard = match device_mutex().lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let dev = match guard.as_mut() {
            Some(d) => d,
            None => {
                log::warn!("[sound] device not initialized");
                return;
            }
        };
        let buf = match dev.bufs.get_mut(file_name) {
            Some(b) => b,
            None => {
                log::warn!("[sound] no buffer for {}", file_name);
                return;
            }
        };
        unsafe {
            // 打断正在播放的旧声音
            waveOutReset(dev.handle);
            // 重置 flags 以便重新提交（WHDR_DONE 清除）
            buf.hdr.dwFlags &= !0x01; // clear WHDR_DONE
            buf.hdr.dwFlags |= 0x02; // ensure WHDR_PREPARED stays set
            waveOutWrite(
                dev.handle,
                buf.hdr.as_mut() as *mut WAVEHDR,
                std::mem::size_of::<WAVEHDR>() as u32,
            );
        }
    }

    /// 重新加载指定 wav 进缓冲 — 录制覆盖后调用。
    pub fn reload_cache(file_name: &'static str) {
        let mut guard = match device_mutex().lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let dev = match guard.as_mut() {
            Some(d) => d,
            None => return,
        };
        // 停止播放后卸载旧缓冲
        unsafe { waveOutReset(dev.handle); }
        if let Some(mut old) = dev.bufs.remove(file_name) {
            unprepare_buf(dev.handle, &mut old);
        }
        // 加载新文件
        if let Some(buf) = load_single(dev.handle, file_name) {
            dev.bufs.insert(file_name, buf);
            log::info!("[sound] reloaded buffer for {}", file_name);
        }
    }

    /// 停止当前播放。
    pub fn purge_playing() {
        let guard = match device_mutex().lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(dev) = guard.as_ref() {
            unsafe { waveOutReset(dev.handle); }
        }
    }

    /// 查询提示音文件是否存在。
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

    impl Drop for WaveDevice {
        fn drop(&mut self) {
            unsafe {
                waveOutReset(self.handle);
                for (_, buf) in self.bufs.iter_mut() {
                    waveOutUnprepareHeader(
                        self.handle,
                        buf.hdr.as_mut() as *mut WAVEHDR,
                        std::mem::size_of::<WAVEHDR>() as u32,
                    );
                }
                waveOutClose(self.handle);
            }
        }
    }
}

/// 启动提示音文件名。
pub const FILE_START: &str = "按键开启.wav";
/// 停止提示音文件名。
pub const FILE_STOP: &str = "按键关闭.wav";

/// 初始化音频设备并加载提示音缓冲 — 在 setup 阶段调用一次。
#[cfg(windows)]
pub fn init() {
    inner::init();
}

#[cfg(not(windows))]
pub fn init() {}

/// 播放启动提示音。
#[cfg(windows)]
pub fn play_start() {
    inner::play_file(inner::FILE_START);
}

/// 播放停止提示音。
#[cfg(windows)]
pub fn play_stop() {
    inner::play_file(inner::FILE_STOP);
}

#[cfg(not(windows))]
pub fn play_start() {}

#[cfg(not(windows))]
pub fn play_stop() {}

/// 停止当前播放 — 录制覆盖前调用。
#[cfg(windows)]
pub fn purge_playing() {
    inner::purge_playing();
}

#[cfg(not(windows))]
pub fn purge_playing() {}

/// 重新加载指定 wav 进缓冲 — 录制覆盖后调用。
#[cfg(windows)]
pub fn reload_cache(file_name: &'static str) {
    inner::reload_cache(file_name);
}

#[cfg(not(windows))]
pub fn reload_cache(_file_name: &'static str) {}

/// 查询提示音文件是否存在。
pub fn sound_files_exist() -> (bool, bool) {
    #[cfg(windows)]
    {
        inner::sound_files_exist()
    }
    #[cfg(not(windows))]
    {
        (false, false)
    }
}
