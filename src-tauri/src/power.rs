//! Shut down, restart or sleep the PC. The frontend asks twice before it
//! calls in here; this side only accepts the three known actions.

use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerAction {
    Shutdown,
    Restart,
    Sleep,
}

/// `shutdown.exe` arguments for the actions it handles. `/t 0` on purpose:
/// any longer timeout implies `/f`, which closes apps without letting them
/// save, while at 0 Windows still stops for apps with unsaved work.
fn shutdown_args(action: PowerAction) -> Option<&'static [&'static str]> {
    match action {
        PowerAction::Shutdown => Some(&["/s", "/t", "0"]),
        PowerAction::Restart => Some(&["/r", "/t", "0"]),
        PowerAction::Sleep => None,
    }
}

pub fn run(action: PowerAction) -> Result<(), String> {
    crate::log_line(&format!("power: {action:?}"));
    match shutdown_args(action) {
        Some(args) => shutdown_exe(args),
        None => sleep(),
    }
}

#[cfg(windows)]
fn shutdown_exe(args: &[&str]) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("shutdown.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Sleep, never hibernate. This PC class (Modern Standby, S0) has no S3, and
/// the common `rundll32 powrprof.dll,SetSuspendState` trick passes arguments
/// that hibernate instead; calling the API with `hibernate = false` is what
/// Start > Sleep amounts to.
#[cfg(windows)]
fn sleep() -> Result<(), String> {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
    use windows::Win32::Security::{
        AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES,
        SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows::Win32::System::Power::SetSuspendState;
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: plain Win32 calls on our own process token. Every out-pointer
    // is a live local, and the token handle is closed before returning.
    unsafe {
        // Suspending requires the shutdown privilege, which signed-in users
        // hold but have switched off by default.
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token)
            .map_err(|e| format!("open token: {e}"))?;
        let mut luid = LUID::default();
        let enabled = LookupPrivilegeValueW(None, w!("SeShutdownPrivilege"), &mut luid)
            .and_then(|()| {
                let privileges = TOKEN_PRIVILEGES {
                    PrivilegeCount: 1,
                    Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }],
                };
                AdjustTokenPrivileges(token, false, Some(&privileges), 0, None, None)
            });
        let _ = CloseHandle(token);
        enabled.map_err(|e| format!("enable shutdown privilege: {e}"))?;

        if SetSuspendState(false, false, false) {
            Ok(())
        } else {
            Err(format!("sleep refused: {}", windows::core::Error::from_win32()))
        }
    }
}

#[cfg(not(windows))]
fn shutdown_exe(_args: &[&str]) -> Result<(), String> {
    Err("not supported on this platform".into())
}

#[cfg(not(windows))]
fn sleep() -> Result<(), String> {
    Err("not supported on this platform".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_and_restart_never_force_apps_closed() {
        for action in [PowerAction::Shutdown, PowerAction::Restart] {
            let args = shutdown_args(action).unwrap();
            assert!(!args.contains(&"/f"), "{action:?} must not force-close apps");
            assert_eq!(args[args.len() - 2..], ["/t", "0"], "{action:?}: any delay implies /f");
        }
        assert_eq!(shutdown_args(PowerAction::Shutdown).unwrap()[0], "/s");
        assert_eq!(shutdown_args(PowerAction::Restart).unwrap()[0], "/r");
    }

    #[test]
    fn sleep_does_not_go_through_shutdown_exe() {
        assert_eq!(shutdown_args(PowerAction::Sleep), None);
    }

    #[test]
    fn only_the_three_known_actions_are_accepted() {
        let parse = |s: &str| serde_json::from_str::<PowerAction>(&format!("\"{s}\""));
        assert_eq!(parse("shutdown").unwrap(), PowerAction::Shutdown);
        assert_eq!(parse("restart").unwrap(), PowerAction::Restart);
        assert_eq!(parse("sleep").unwrap(), PowerAction::Sleep);
        for bad in ["hibernate", "logoff", "Shutdown", "shutdown /f", ""] {
            assert!(parse(bad).is_err(), "{bad:?} must be rejected");
        }
    }
}
