// 管理员权限检测与提权重启 — DESIGN 14.1 / TASKS 阶段 10
//
// 本模块仅在 Windows 平台具有真实实现：
//   - is_admin(): 通过 OpenProcessToken + GetTokenInformation(TokenElevation) 检测
//   - restart_as_admin(): 通过 ShellExecuteW + "runas" verb 触发 UAC 重启自身
//
// 非 Windows 平台仅作为占位（编译期不会进入），因为 tauri.conf.json 仅打包 Windows。
// 但保留 cfg 守卫便于 `cargo check` 在其他平台不报错。
//
// ADMIN_POLICY: 启动时检测，不在 manifest 中强制 requireAdministrator（DESIGN 14.1）。

#[cfg(windows)]
mod windows_impl {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_NORMAL;

    /// 检查当前进程是否以管理员（已提权 / Elevated）身份运行。
    ///
    /// 任何 API 调用失败均视为「非管理员」并记录日志 — 我们只在 UI 上呈现一个布尔标志，
    /// 不需要为查询失败本身向上传播错误。
    pub fn is_admin() -> bool {
        unsafe {
            let mut token: HANDLE = null_mut();
            // OpenProcessToken 成功返回非零；失败返回 0
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                log::warn!("[admin] OpenProcessToken failed; treating as non-admin");
                return false;
            }

            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut return_length: u32 = 0;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_length,
            );
            CloseHandle(token);

            if ok == 0 {
                log::warn!("[admin] GetTokenInformation failed; treating as non-admin");
                return false;
            }
            elevation.TokenIsElevated != 0
        }
    }

    /// 以管理员身份重启自身。
    ///
    /// 使用 ShellExecuteW + "runas" verb 触发 UAC 提示；用户取消或失败时返回 Err。
    /// 调用方负责在重启进程启动后退出当前进程（避免双开）。
    pub fn restart_as_admin() -> Result<(), String> {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get current exe path: {}", e))?;

        let exe_wide: Vec<u16> = OsStr::new(&exe_path)
            .encode_wide()
            .chain(once(0))
            .collect();
        let verb_wide: Vec<u16> = OsStr::new("runas").encode_wide().chain(once(0)).collect();

        // ShellExecuteW 返回 HINSTANCE，按惯例 > 32 表示成功
        let result = unsafe {
            ShellExecuteW(
                null_mut(),
                verb_wide.as_ptr(),
                exe_wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_NORMAL,
            )
        };
        // HINSTANCE 是 *mut c_void;按 Win32 文档，<= 32 视为错误码
        if (result as isize) <= 32 {
            Err(format!(
                "ShellExecuteW failed (code {}); user may have declined UAC",
                result as isize
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
pub use windows_impl::{is_admin, restart_as_admin};

// 非 Windows 平台占位 — 真实运行环境只会是 Windows，此处仅让 cargo check 在其他平台通过。
#[cfg(not(windows))]
pub fn is_admin() -> bool {
    false
}

#[cfg(not(windows))]
pub fn restart_as_admin() -> Result<(), String> {
    Err("restart_as_admin is only supported on Windows".to_string())
}
