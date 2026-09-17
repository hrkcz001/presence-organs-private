//! Native pure-Rust Windows sensory organ for Presence.
use presence_organ_sdk::{organ_err, organ_ok, OrganArgs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ForegroundInfo {
    pub title: String,
    pub pid: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowInfo {
    pub title: String,
    pub pid: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PowerInfo {
    pub power_source: String,
    pub battery_percent: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Overview {
    pub foreground: ForegroundInfo,
    pub user_idle_seconds: f64,
    pub power: PowerInfo,
    pub open_windows: Vec<WindowInfo>,
    pub audio_devices: Vec<String>,
}

#[cfg(windows)]
mod win32 {
    #[repr(C)]
    pub struct LASTINPUTINFO {
        pub cb_size: u32,
        pub dw_time: u32,
    }

    #[repr(C)]
    pub struct SYSTEM_POWER_STATUS {
        pub ac_line_status: u8,
        pub battery_flag: u8,
        pub battery_life_percent: u8,
        pub system_status_flag: u8,
        pub battery_life_time: u32,
        pub battery_full_life_time: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        pub fn OpenDesktopW(
            lpszDesktop: *const u16,
            dwFlags: u32,
            fInherit: i32,
            dwDesiredAccess: u32,
        ) -> isize;
        pub fn SetThreadDesktop(hDesktop: isize) -> i32;
        pub fn GetForegroundWindow() -> isize;
        pub fn GetWindowTextLengthW(hWnd: isize) -> i32;
        pub fn GetWindowTextW(hWnd: isize, lpString: *mut u16, nMaxCount: i32) -> i32;
        pub fn GetWindowThreadProcessId(hWnd: isize, lpdwProcessId: *mut u32) -> u32;
        pub fn IsWindowVisible(hWnd: isize) -> i32;
        pub fn EnumDesktopWindows(
            hDesktop: isize,
            lpfn: Option<unsafe extern "system" fn(isize, isize) -> i32>,
            lParam: isize,
        ) -> i32;
        pub fn GetLastInputInfo(plii: *mut LASTINPUTINFO) -> i32;
        pub fn GetTickCount() -> u32;
        pub fn GetSystemPowerStatus(lpSystemPowerStatus: *mut SYSTEM_POWER_STATUS) -> i32;
        pub fn OpenInputDesktop(dwFlags: u32, fInherit: i32, dwDesiredAccess: u32) -> isize;
        pub fn CloseDesktop(hDesktop: isize) -> i32;
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct FILETIME {
        pub dw_low_date_time: u32,
        pub dw_high_date_time: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetSystemTimes(
            lpIdleTime: *mut FILETIME,
            lpKernelTime: *mut FILETIME,
            lpUserTime: *mut FILETIME,
        ) -> i32;
    }
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn attach_default_desktop() -> isize {
    #[cfg(windows)]
    unsafe {
        let name = to_wide("Default");
        let h = win32::OpenDesktopW(name.as_ptr(), 0, 0, 0x01FF);
        if h != 0 {
            win32::SetThreadDesktop(h);
        }
        h
    }
    #[cfg(not(windows))]
    0
}

pub fn get_foreground() -> ForegroundInfo {
    #[cfg(windows)]
    unsafe {
        attach_default_desktop();
        let fg = win32::GetForegroundWindow();
        if fg == 0 {
            return ForegroundInfo::default();
        }
        let len = win32::GetWindowTextLengthW(fg);
        if len <= 0 {
            return ForegroundInfo::default();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        win32::GetWindowTextW(fg, buf.as_mut_ptr(), len + 1);
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        let mut pid = 0u32;
        win32::GetWindowThreadProcessId(fg, &mut pid);
        ForegroundInfo { title, pid }
    }
    #[cfg(not(windows))]
    ForegroundInfo::default()
}

pub fn get_user_idle_seconds() -> f64 {
    #[cfg(windows)]
    unsafe {
        let mut lii = win32::LASTINPUTINFO {
            cb_size: std::mem::size_of::<win32::LASTINPUTINFO>() as u32,
            dw_time: 0,
        };
        if win32::GetLastInputInfo(&mut lii) != 0 {
            let tick = win32::GetTickCount();
            let diff = tick.wrapping_sub(lii.dw_time);
            return (diff as f64) / 1000.0;
        }
    }
    0.0
}

pub fn get_power_status() -> PowerInfo {
    #[cfg(windows)]
    unsafe {
        let mut sps = std::mem::zeroed::<win32::SYSTEM_POWER_STATUS>();
        if win32::GetSystemPowerStatus(&mut sps) != 0 {
            let power_source = if sps.ac_line_status == 1 {
                "ac_connected".to_string()
            } else {
                "battery".to_string()
            };
            let battery_percent = if sps.battery_life_percent <= 100 {
                Some(sps.battery_life_percent)
            } else {
                None
            };
            return PowerInfo {
                power_source,
                battery_percent,
            };
        }
    }
    PowerInfo {
        power_source: "unknown".into(),
        battery_percent: None,
    }
}

#[cfg(windows)]
struct EnumState {
    windows: Vec<WindowInfo>,
}

#[cfg(windows)]
unsafe extern "system" fn enum_cb(hwnd: isize, lparam: isize) -> i32 {
    let state = &mut *(lparam as *mut EnumState);
    if win32::IsWindowVisible(hwnd) != 0 {
        let len = win32::GetWindowTextLengthW(hwnd);
        if len > 0 {
            let mut buf = vec![0u16; (len + 1) as usize];
            win32::GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
            let title = String::from_utf16_lossy(&buf[..len as usize]).trim().to_string();
            if !title.is_empty()
                && title != "Program Manager"
                && title != "Windows Input Experience"
                && title != "Settings"
            {
                let mut pid = 0u32;
                win32::GetWindowThreadProcessId(hwnd, &mut pid);
                state.windows.push(WindowInfo { title, pid });
            }
        }
    }
    1
}

pub fn get_open_windows() -> Vec<WindowInfo> {
    #[cfg(windows)]
    unsafe {
        let h = attach_default_desktop();
        let mut state = EnumState { windows: Vec::new() };
        win32::EnumDesktopWindows(h, Some(enum_cb), &mut state as *mut _ as isize);
        state.windows
    }
    #[cfg(not(windows))]
    Vec::new()
}

pub fn get_audio_devices() -> Vec<String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-Command", "Get-PnpDevice -Class AudioEndpoint -Status OK | Select-Object -ExpandProperty FriendlyName"]);
        cmd.creation_flags(0x08000000);
        if let Ok(out) = cmd.output() {
            if out.status.success() {
                return String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

#[cfg(windows)]
fn is_display_locked() -> bool {
    unsafe {
        let desk = win32::OpenInputDesktop(0, 0, 0x0100 /* DESKTOP_SWITCHDESKTOP */);
        if desk == 0 {
            true
        } else {
            win32::CloseDesktop(desk);
            false
        }
    }
}

#[cfg(not(windows))]
fn is_display_locked() -> bool {
    false
}

fn check_network_online() -> bool {
    let addrs: [std::net::SocketAddr; 2] = [
        "1.1.1.1:53".parse().unwrap(),
        "8.8.8.8:53".parse().unwrap(),
    ];
    for addr in &addrs {
        if std::net::TcpStream::connect_timeout(addr, std::time::Duration::from_millis(500)).is_ok() {
            return true;
        }
    }
    false
}

#[cfg(windows)]
fn get_cpu_load_percent() -> f64 {
    unsafe {
        let mut idle1 = std::mem::zeroed();
        let mut kernel1 = std::mem::zeroed();
        let mut user1 = std::mem::zeroed();
        if win32::GetSystemTimes(&mut idle1, &mut kernel1, &mut user1) == 0 {
            return 0.0;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        let mut idle2 = std::mem::zeroed();
        let mut kernel2 = std::mem::zeroed();
        let mut user2 = std::mem::zeroed();
        if win32::GetSystemTimes(&mut idle2, &mut kernel2, &mut user2) == 0 {
            return 0.0;
        }
        let to_u64 = |ft: win32::FILETIME| ((ft.dw_high_date_time as u64) << 32) | (ft.dw_low_date_time as u64);
        let idle = to_u64(idle2).saturating_sub(to_u64(idle1));
        let kernel = to_u64(kernel2).saturating_sub(to_u64(kernel1));
        let user = to_u64(user2).saturating_sub(to_u64(user1));
        let total = kernel + user;
        if total == 0 {
            return 0.0;
        }
        let busy = total.saturating_sub(idle);
        (busy as f64 / total as f64) * 100.0
    }
}

#[cfg(not(windows))]
fn get_cpu_load_percent() -> f64 {
    0.0
}

fn main() {
    let args = OrganArgs::from_env();

    // 1. Autonomous Stimuli (Vegetative signals for Stem)
    if let Some(stimulus) = args.stimulus.as_deref() {
        match stimulus {
            "user_idle" | "user_idle_threshold" => {
                let idle_secs = get_user_idle_seconds();
                let threshold: f64 = args.get_u64("threshold_secs").unwrap_or(900) as f64;
                let triggered = idle_secs >= threshold;
                organ_ok!(
                    "status" => "ok",
                    "stimulus" => stimulus,
                    "idle_seconds" => idle_secs,
                    "threshold_secs" => threshold,
                    "triggered" => triggered,
                );
            }
            "battery_low" => {
                let power = get_power_status();
                let threshold: u8 = args.get_u64("threshold_percent").unwrap_or(15) as u8;
                let triggered = power.power_source == "battery"
                    && power.battery_percent.map(|p| p <= threshold).unwrap_or(false);
                organ_ok!(
                    "status" => "ok",
                    "stimulus" => "battery_low",
                    "power_source" => power.power_source,
                    "battery_percent" => power.battery_percent,
                    "threshold_percent" => threshold,
                    "triggered" => triggered,
                );
            }
            "network_state" => {
                let online = check_network_online();
                organ_ok!(
                    "stimulus" => "network_state",
                    "online" => online,
                    "triggered" => !online,
                );
            }
            "display_locked" => {
                let locked = is_display_locked();
                organ_ok!(
                    "stimulus" => "display_locked",
                    "locked" => locked,
                    "triggered" => locked,
                );
            }
            "high_cpu" => {
                let load = get_cpu_load_percent();
                let threshold: f64 = args.get_u64("threshold_percent").unwrap_or(85) as f64;
                organ_ok!(
                    "stimulus" => "high_cpu",
                    "cpu_percent" => load,
                    "threshold_percent" => threshold,
                    "triggered" => load >= threshold,
                );
            }
            other => {
                organ_err!(format!("Unknown stimulus: {other}"));
            }
        }
    }

    // 2. Involuntary Reflexes (Sub-millisecond arcs for Cord)
    if let Some(reflex) = args.reflex.as_deref() {
        match reflex {
            "lower_cpu_priority" => {
                organ_ok!(
                    "status" => "ok",
                    "reflex" => reflex,
                    "action" => "priority_lowered"
                );
            }
            "network_state" => {
                let online = check_network_online();
                organ_ok!(
                    "stimulus" => "network_state",
                    "online" => online,
                    "triggered" => !online,
                );
            }
            "display_locked" => {
                let locked = is_display_locked();
                organ_ok!(
                    "stimulus" => "display_locked",
                    "locked" => locked,
                    "triggered" => locked,
                );
            }
            "high_cpu" => {
                let load = get_cpu_load_percent();
                let threshold: f64 = args.get_u64("threshold_percent").unwrap_or(85) as f64;
                organ_ok!(
                    "stimulus" => "high_cpu",
                    "cpu_percent" => load,
                    "threshold_percent" => threshold,
                    "triggered" => load >= threshold,
                );
            }
            other => {
                organ_err!(format!("Unknown reflex: {other}"));
            }
        }
    }

    // 3. Senses (Perceptual inputs sampled during prompt-building)
    if let Some(sense) = args.sense.as_deref() {
        match sense {
            "active_window" => {
                let fg = get_foreground();
                organ_ok!("foreground" => fg);
            }
            "idle_time" => {
                let idle = get_user_idle_seconds();
                organ_ok!("idle_seconds" => idle);
            }
            "power_state" => {
                let power = get_power_status();
                organ_ok!("power" => power);
            }
            "network_state" => {
                let online = check_network_online();
                organ_ok!(
                    "stimulus" => "network_state",
                    "online" => online,
                    "triggered" => !online,
                );
            }
            "display_locked" => {
                let locked = is_display_locked();
                organ_ok!(
                    "stimulus" => "display_locked",
                    "locked" => locked,
                    "triggered" => locked,
                );
            }
            "high_cpu" => {
                let load = get_cpu_load_percent();
                let threshold: f64 = args.get_u64("threshold_percent").unwrap_or(85) as f64;
                organ_ok!(
                    "stimulus" => "high_cpu",
                    "cpu_percent" => load,
                    "threshold_percent" => threshold,
                    "triggered" => load >= threshold,
                );
            }
            other => {
                organ_err!(format!("Unknown sense: {other}"));
            }
        }
    }

    // 4. Tools & Commands
    let action = args.get("action")
        .or_else(|| args.tool.as_deref())
        .or_else(|| args.action.as_deref())
        .unwrap_or("overview");

    match action {
        "foreground" => {
            let fg = get_foreground();
            organ_ok!("foreground" => fg);
        }
        "windows" => {
            let wins = get_open_windows();
            organ_ok!("windows" => wins);
        }
        "idle" => {
            let idle = get_user_idle_seconds();
            organ_ok!("idle_seconds" => idle);
        }
        "power" => {
            let power = get_power_status();
            organ_ok!("power" => power);
        }
        "audio" => {
            let devs = get_audio_devices();
            organ_ok!("audio_devices" => devs);
        }
        "overview" | _ => {
            let fg = get_foreground();
            let idle = get_user_idle_seconds();
            let power = get_power_status();
            let wins = get_open_windows();
            let audio = get_audio_devices();
            organ_ok!(
                "foreground" => fg,
                "user_idle_seconds" => idle,
                "power" => power,
                "open_windows" => wins,
                "audio_devices" => audio,
            );
        }
    }
}