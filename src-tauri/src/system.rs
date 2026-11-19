/// Windows native system state detection:
/// 1. Fullscreen / D3D game / Presentation / Locked screen detection via SHQueryUserNotificationState
/// 2. User keyboard/mouse idle seconds via GetLastInputInfo + GetTickCount

#[cfg(windows)]
pub fn is_fullscreen_or_locked() -> bool {
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_ACCEPTS_NOTIFICATIONS, QUNS_APP,
    };

    unsafe {
        match SHQueryUserNotificationState() {
            Ok(state) => state != QUNS_ACCEPTS_NOTIFICATIONS && state != QUNS_APP,
            Err(_) => false,
        }
    }
}

#[cfg(not(windows))]
pub fn is_fullscreen_or_locked() -> bool {
    false
}

#[cfg(windows)]
pub fn get_idle_seconds() -> u64 {
    use std::mem::size_of;
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii).as_bool() {
            let now_tick = GetTickCount();
            let elapsed_ms = now_tick.wrapping_sub(lii.dwTime);
            (elapsed_ms / 1000) as u64
        } else {
            0
        }
    }
}

#[cfg(not(windows))]
pub fn get_idle_seconds() -> u64 {
    0
}
