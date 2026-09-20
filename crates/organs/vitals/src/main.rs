//! Native pure-Rust system vitals sensory organ for Presence.
use presence_organ_sdk::{organ_err, organ_ok, OrganArgs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VitalsData {
    pub cpu_percent: Option<f64>,
    pub mem_percent: Option<f64>,
    pub battery: Option<String>,
    pub is_critical_battery: bool,
    pub is_cpu_throttle: bool,
}

#[cfg(windows)]
mod sys {
    use super::*;
    use std::mem;

    #[repr(C)]
    struct MEMORYSTATUSEX {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
        ull_total_page_file: u64,
        ull_avail_page_file: u64,
        ull_total_virtual: u64,
        ull_avail_virtual: u64,
        ull_avail_extended_virtual: u64,
    }

    #[repr(C)]
    struct SYSTEM_POWER_STATUS {
        ac_line_status: u8,
        battery_flag: u8,
        battery_life_percent: u8,
        system_status_flag: u8,
        battery_life_time: u32,
        battery_full_life_time: u32,
    }

    #[repr(C)]
    struct FILETIME {
        dw_low_date_time: u32,
        dw_high_date_time: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(lp_buffer: *mut MEMORYSTATUSEX) -> i32;
        fn GetSystemTimes(lp_idle_time: *mut FILETIME, lp_kernel_time: *mut FILETIME, lp_user_time: *mut FILETIME) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemPowerStatus(lp_system_power_status: *mut SYSTEM_POWER_STATUS) -> i32;
    }

    fn filetime_to_u64(ft: &FILETIME) -> u64 {
        ((ft.dw_high_date_time as u64) << 32) | (ft.dw_low_date_time as u64)
    }

    pub fn sample_vitals() -> VitalsData {
        let mut data = VitalsData::default();

        // 1. Memory
        unsafe {
            let mut mem_status = MEMORYSTATUSEX {
                dw_length: mem::size_of::<MEMORYSTATUSEX>() as u32,
                dw_memory_load: 0,
                ull_total_phys: 0,
                ull_avail_phys: 0,
                ull_total_page_file: 0,
                ull_avail_page_file: 0,
                ull_total_virtual: 0,
                ull_avail_virtual: 0,
                ull_avail_extended_virtual: 0,
            };
            if GlobalMemoryStatusEx(&mut mem_status) != 0 {
                data.mem_percent = Some(mem_status.dw_memory_load as f64);
            }
        }

        // 2. Battery
        unsafe {
            let mut sps = SYSTEM_POWER_STATUS {
                ac_line_status: 255,
                battery_flag: 255,
                battery_life_percent: 255,
                system_status_flag: 0,
                battery_life_time: 0,
                battery_full_life_time: 0,
            };
            if GetSystemPowerStatus(&mut sps) != 0 {
                if sps.battery_life_percent <= 100 {
                    data.battery = Some(format!("{}%", sps.battery_life_percent));
                    if sps.ac_line_status == 0 && sps.battery_life_percent <= 15 {
                        data.is_critical_battery = true;
                    }
                } else {
                    data.battery = Some("none".to_string());
                }
            }
        }

        // 3. CPU (instant delta sample over 10ms)
        unsafe {
            let mut idle1 = FILETIME { dw_low_date_time: 0, dw_high_date_time: 0 };
            let mut kernel1 = FILETIME { dw_low_date_time: 0, dw_high_date_time: 0 };
            let mut user1 = FILETIME { dw_low_date_time: 0, dw_high_date_time: 0 };

            if GetSystemTimes(&mut idle1, &mut kernel1, &mut user1) != 0 {
                std::thread::sleep(std::time::Duration::from_millis(15));
                let mut idle2 = FILETIME { dw_low_date_time: 0, dw_high_date_time: 0 };
                let mut kernel2 = FILETIME { dw_low_date_time: 0, dw_high_date_time: 0 };
                let mut user2 = FILETIME { dw_low_date_time: 0, dw_high_date_time: 0 };

                if GetSystemTimes(&mut idle2, &mut kernel2, &mut user2) != 0 {
                    let idle = filetime_to_u64(&idle2).saturating_sub(filetime_to_u64(&idle1));
                    let kernel = filetime_to_u64(&kernel2).saturating_sub(filetime_to_u64(&kernel1));
                    let user = filetime_to_u64(&user2).saturating_sub(filetime_to_u64(&user1));
                    let total = kernel + user;
                    if total > 0 {
                        let load = 100.0 * (total.saturating_sub(idle) as f64) / (total as f64);
                        let rounded = (load * 10.0).round() / 10.0;
                        data.cpu_percent = Some(rounded);
                        if rounded > 90.0 {
                            data.is_cpu_throttle = true;
                        }
                    }
                }
            }
        }

        data
    }
}

#[cfg(not(windows))]
mod sys {
    use super::*;
    pub fn sample_vitals() -> VitalsData {
        let mut data = VitalsData::default();
        // Fallback for unix procfs
        if let Ok(mem) = std::fs::read_to_string("/proc/meminfo") {
            let mut total = 0u64;
            let mut avail = 0u64;
            for line in mem.lines() {
                if line.starts_with("MemTotal:") {
                    total = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    avail = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
                }
            }
            if total > 0 {
                let used = total.saturating_sub(avail);
                data.mem_percent = Some(((used as f64 / total as f64) * 1000.0).round() / 10.0);
            }
        }
        data.battery = Some("ac".to_string());
        data
    }
}

fn handle_tool(tool: &str, _args: &OrganArgs) -> Result<String, String> {
    match tool {
        "vitals_summary" => {
            let vitals = sys::sample_vitals();
            Ok(serde_json::to_string_pretty(&vitals).unwrap_or_default())
        }
        _ => Err(format!("Unknown tool: {tool}")),
    }
}

fn handle_sense(sense: &str, _args: &OrganArgs) -> Result<String, String> {
    match sense {
        "system_vitals" => {
            let v = sys::sample_vitals();
            let cpu_str = v.cpu_percent.map(|c| format!("{c}%")).unwrap_or_else(|| "N/A".into());
            let mem_str = v.mem_percent.map(|m| format!("{m}%")).unwrap_or_else(|| "N/A".into());
            let bat_str = v.battery.unwrap_or_else(|| "none".into());
            Ok(format!("CPU: {cpu_str} | RAM: {mem_str} | Battery: {bat_str}"))
        }
        _ => Err(format!("Unknown sense: {sense}")),
    }
}

fn handle_stimulus(stimulus: &str, _args: &OrganArgs) -> Result<String, String> {
    let v = sys::sample_vitals();
    match stimulus {
        "battery_critical" => {
            let triggered = v.is_critical_battery;
            let payload = serde_json::json!({
                "triggered": triggered,
                "battery": v.battery,
                "reason": if triggered { "Host battery below 15% on DC" } else { "Battery nominal" }
            });
            Ok(payload.to_string())
        }
        "cpu_throttle" => {
            let triggered = v.is_cpu_throttle;
            let payload = serde_json::json!({
                "triggered": triggered,
                "cpu_percent": v.cpu_percent,
                "reason": if triggered { "Host CPU load exceeds 90%" } else { "CPU nominal" }
            });
            Ok(payload.to_string())
        }
        _ => Err(format!("Unknown stimulus: {stimulus}")),
    }
}

fn main() {
    let args = OrganArgs::from_env();

    if let Some(tool) = args.get("tool") {
        match handle_tool(tool, &args) {
            Ok(val) => organ_ok!("result" => val),
            Err(e) => organ_err!(e),
        }
    }

    if let Some(sense) = args.get("sense") {
        match handle_sense(sense, &args) {
            Ok(val) => organ_ok!("system_vitals" => val),
            Err(e) => organ_err!(e),
        }
    }

    if let Some(stim) = args.get("stimulus") {
        match handle_stimulus(stim, &args) {
            Ok(val) => println!("{val}"),
            Err(e) => organ_err!(e),
        }
        return;
    }

    organ_err!("Missing action: expected --tool, --sense, or --stimulus");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_vitals() {
        let v = sys::sample_vitals();
        assert!(v.mem_percent.is_some());
    }

    #[test]
    fn test_handle_sense() {
        let args = OrganArgs::default();
        let res = handle_sense("system_vitals", &args).unwrap();
        assert!(res.contains("CPU:"));
        assert!(res.contains("RAM:"));
    }
}
