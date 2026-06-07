// Interception 驱动检测与安装 — DESIGN 12.2 / 12.3 / TASKS 阶段 11
//
// 检测策略：
//   1. 查 Windows 注册表 HKLM\SYSTEM\CurrentControlSet\Services\keyboard 与 mouse 服务项
//   2. 服务项存在 → 驱动已安装（可能需重启才能加载）
//   3. 服务项不存在 → 驱动未安装
//
// 安装策略：
//   定位 <exe_dir>/drivers/interception/install-interception.exe
//   通过 ShellExecuteW("runas") 以管理员身份静默调用 `/install` 参数
//   安装完成后返回 InstalledNeedReboot
//
// 阶段 13 接入 interception crate 后，改为先尝试 create_context()，
// 成功则 Ready，失败再走注册表判断 InstalledNeedReboot vs NotInstalled。

use crate::state::DriverStatus;

/// 检测 Interception 驱动当前状态
pub fn check_interception_driver() -> DriverStatus {
    #[cfg(windows)]
    {
        check_driver_windows()
    }
    #[cfg(not(windows))]
    {
        DriverStatus::NotInstalled
    }
}

/// 执行驱动安装（需管理员权限）
///
/// 成功调度安装器后返回 Ok(())；安装器本身的执行结果无法同步获取，
/// 调用者应在安装后重新 check_interception_driver() 判断状态。
pub fn install_driver() -> Result<(), String> {
    #[cfg(windows)]
    {
        install_driver_windows()
    }
    #[cfg(not(windows))]
    {
        Err("Driver installation is only supported on Windows".to_string())
    }
}

// ─── Windows 实现 ───────────────────────────────────────────────────────────

#[cfg(windows)]
fn check_driver_windows() -> DriverStatus {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    // Interception 注册两个服务：keyboard 和 mouse
    // 只要有一个存在就认为驱动已安装
    let keyboard_path = encode_wide("SYSTEM\\CurrentControlSet\\Services\\keyboard");
    let mouse_path = encode_wide("SYSTEM\\CurrentControlSet\\Services\\mouse");
    let service_paths: &[&[u16]] = &[&keyboard_path, &mouse_path];

    for path in service_paths {
        let mut hkey = std::ptr::null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                path.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            )
        };
        if status == 0 {
            // 打开成功 → 服务项存在
            unsafe { RegCloseKey(hkey) };
            log::info!("[driver] registry service key found, driver installed (may need reboot)");
            // 阶段 13 接入 interception crate 后,这里改为尝试 create_context():
            //   成功 → Ready
            //   失败 → InstalledNeedReboot
            // 当前阶段 11 无 interception 依赖，统一返回 InstalledNeedReboot
            return DriverStatus::InstalledNeedReboot;
        }
    }

    log::info!("[driver] no registry service key found, driver not installed");
    DriverStatus::NotInstalled
}

#[cfg(windows)]
fn install_driver_windows() -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or_else(|| "No parent directory for exe".to_string())?
        .to_path_buf();

    let installer_path = exe_dir
        .join("drivers")
        .join("interception")
        .join("install-interception.exe");

    if !installer_path.exists() {
        return Err(format!(
            "Driver installer not found at: {}",
            installer_path.display()
        ));
    }

    let verb = encode_wide("runas");
    let file = encode_wide(&installer_path.to_string_lossy());
    let params = encode_wide("/install");

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            SW_HIDE as i32,
        )
    };

    // ShellExecuteW 返回 > 32 表示成功
    let result_code = result as isize;
    if result_code > 32 {
        log::info!("[driver] installer launched successfully via runas");
        Ok(())
    } else {
        let err_msg = format!(
            "ShellExecuteW failed with code {} (user may have declined UAC)",
            result_code
        );
        log::error!("[driver] {}", err_msg);
        Err(err_msg)
    }
}

/// 将 Rust 字符串编码为 null 结尾的 UTF-16 宽字符序列
#[cfg(windows)]
fn encode_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
