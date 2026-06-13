// Interception 驱动检测与安装 — DESIGN 12.2 / 12.3 / TASKS 阶段 11
//
// 检测策略：
//   1. 查 Windows 注册表 HKLM\SYSTEM\CurrentControlSet\Services\keyboard 与 mouse 服务项
//   2. 服务项存在 → 驱动已安装（可能需重启才能加载）
//   3. 服务项不存在 → 驱动未安装
//
// 安装策略：
//   定位 <exe_dir>/driver/install-interception.exe
//   通过 ShellExecuteExW("runas") 以管理员身份静默调用 `/install` 参数，
//   并 WaitForSingleObject 等待安装器进程退出后再返回（否则注册表尚未写完，
//   后续 check_interception_driver() 会误判为 NotInstalled）。
//   安装完成后由 check_interception_driver() 检测得 InstalledNeedReboot。
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
        run_installer_windows("/install")
    }
    #[cfg(not(windows))]
    {
        Err("Driver installation is only supported on Windows".to_string())
    }
}

/// 执行驱动卸载（需管理员权限）
///
/// 与 install_driver() 对称：以管理员身份调用同一安装器的 `/uninstall` 参数。
/// 卸载后通常需重启系统才彻底移除，调用者应重新 check_interception_driver()。
pub fn uninstall_driver() -> Result<(), String> {
    #[cfg(windows)]
    {
        run_installer_windows("/uninstall")
    }
    #[cfg(not(windows))]
    {
        Err("Driver uninstallation is only supported on Windows".to_string())
    }
}

/// 触发系统重启 — 驱动安装后需重启才会加载
///
/// 通过 `shutdown /r /t 0` 立即重启。需管理员权限——
/// 函数顶部加 `is_admin()` 防御检查，避免外层守卫被回归改坏后悄悄退化。
pub fn reboot_system() -> Result<(), String> {
    if !crate::admin::is_admin() {
        log::warn!("[driver] reboot_system called without admin privileges, refused");
        return Err("permission_denied".to_string());
    }
    #[cfg(windows)]
    {
        reboot_system_windows()
    }
    #[cfg(not(windows))]
    {
        Err("Reboot is only supported on Windows".to_string())
    }
}

// ─── Windows 实现 ───────────────────────────────────────────────────────────

#[cfg(windows)]
fn check_driver_windows() -> DriverStatus {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    // 阶段 13：先尝试创建 context，成功则 Ready
    if let Some(_ctx) = interception::Interception::new() {
        log::info!("[driver] Interception context created successfully, driver ready");
        return DriverStatus::Ready;
    }

    // Context 创建失败，检查注册表判断是否已安装但需重启
    let keyboard_path = encode_wide("SYSTEM\\CurrentControlSet\\Services\\keyboard");
    let mouse_path = encode_wide("SYSTEM\\CurrentControlSet\\Services\\mouse");
    let service_paths: &[&[u16]] = &[&keyboard_path, &mouse_path];

    for path in service_paths {
        let mut hkey = std::ptr::null_mut();
        let status =
            unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.as_ptr(), 0, KEY_READ, &mut hkey) };
        if status == 0 {
            unsafe { RegCloseKey(hkey) };
            log::info!("[driver] registry service key found but context failed, need reboot");
            return DriverStatus::InstalledNeedReboot;
        }
    }

    log::info!("[driver] no registry service key found, driver not installed");
    DriverStatus::NotInstalled
}

#[cfg(windows)]
fn run_installer_windows(action_param: &str) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, WaitForSingleObject, INFINITE,
    };
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or_else(|| "No parent directory for exe".to_string())?
        .to_path_buf();

    let installer_path = exe_dir.join("driver").join("install-interception.exe");

    if !installer_path.exists() {
        return Err(format!(
            "Driver installer not found at: {}",
            installer_path.display()
        ));
    }

    // 路径直接走 OsStrExt::encode_wide，避免经 String 中转。
    // to_string_lossy 会把不能映射到 UTF-8 的 WTF-16 序列（如孤立代理对、某些 emoji）
    // 替换为 U+FFFD（�），导致 ShellExecuteExW 找不到文件。
    let verb = encode_wide("runas");
    let file: Vec<u16> = installer_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let params = encode_wide(action_param);

    // 用 ShellExecuteExW 而非 ShellExecuteW：
    // 后者是「启动即返回」，安装器还没写完注册表我们就检测 → 状态误判。
    // SEE_MASK_NOCLOSEPROCESS 让 hProcess 填入安装器进程句柄，
    // 配合 WaitForSingleObject 等其真正退出后再返回，确保后续检测拿到正确状态。
    //
    // 隐性约束（DESIGN 12.3）：SW_HIDE + INFINITE 假设 installer 是非交互控制台程序、
    // 始终能快速退出。当前 oblitum/Interception installer 满足此假设；若未来更换为
    // 带 GUI 对话框的 installer，需改为 SW_SHOWNORMAL 或加有限超时。
    let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = verb.as_ptr();
    sei.lpFile = file.as_ptr();
    sei.lpParameters = params.as_ptr();
    sei.nShow = SW_HIDE;

    let ok = unsafe { ShellExecuteExW(&mut sei) };
    if ok == 0 {
        // 取真实错误码区分场景：1223=用户拒绝 UAC、2=文件不存在、5=权限拒绝等。
        let err_code = unsafe { GetLastError() };
        let err_msg = format!(
            "ShellExecuteExW failed (GetLastError={}; 1223 = user declined UAC)",
            err_code
        );
        log::error!("[driver] {} (param={})", err_msg, action_param);
        return Err(err_msg);
    }

    if sei.hProcess.is_null() {
        // 极少数情况句柄为空，无法等待，退化为「已调度但状态未知」
        log::warn!("[driver] installer launched but no process handle to wait on");
        return Ok(());
    }

    // 阻塞等待安装器进程退出（INFINITE）。该命令在独立线程中执行，
    // 不会卡住 Tauri 主线程；前端通过 isInstalling/isUninstalling 显示进度。
    let wait = unsafe { WaitForSingleObject(sei.hProcess, INFINITE) };

    if wait != WAIT_OBJECT_0 {
        unsafe { CloseHandle(sei.hProcess) };
        let err_msg = format!("WaitForSingleObject returned unexpected code {}", wait);
        log::error!("[driver] {}", err_msg);
        return Err(err_msg);
    }

    // 进程已退出，取退出码确认安装器是否真的成功。
    // GetExitCodeProcess 必须在 CloseHandle 之前调用——句柄关了就读不到了。
    let mut exit_code: u32 = 0;
    let exit_ok = unsafe { GetExitCodeProcess(sei.hProcess, &mut exit_code) };
    unsafe { CloseHandle(sei.hProcess) };

    if exit_ok == 0 {
        let err_code = unsafe { GetLastError() };
        let err_msg = format!("GetExitCodeProcess failed (GetLastError={})", err_code);
        log::error!("[driver] {}", err_msg);
        return Err(err_msg);
    }

    if exit_code != 0 {
        // 安装器以非零退出码结束 = 实际安装/卸载失败。
        // 不能让前端拿到 Ok 走「已安装待重启」分支，否则用户被引导去做无效重启。
        let err_msg = format!("installer exited with code {}", exit_code);
        log::error!("[driver] {} (param={})", err_msg, action_param);
        return Err(err_msg);
    }

    log::info!(
        "[driver] installer process exited successfully (param={})",
        action_param
    );
    Ok(())
}

/// 触发系统重启 — Windows 实现，调用 `shutdown /r /t 0`
#[cfg(windows)]
fn reboot_system_windows() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // CREATE_NO_WINDOW (0x08000000) 避免弹出黑色控制台窗口
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    Command::new("shutdown")
        .args(["/r", "/t", "0"])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| {
            let msg = format!("Failed to invoke shutdown: {}", e);
            log::error!("[driver] {}", msg);
            msg
        })?;

    log::info!("[driver] reboot command dispatched");
    Ok(())
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
