#![windows_subsystem = "windows"]

mod cli;
mod driver;
mod gui;

use clap::Parser;
use cli::{Cli, Commands};
use driver::{MchoseMouse, PollingRate};

#[cfg(windows)]
fn attach_parent_console() {
    unsafe {
        unsafe extern "system" {
            fn AttachConsole(dw_process_id: u32) -> i32;
            fn GetStdHandle(n_std_handle: u32) -> isize;
            fn SetStdHandle(n_std_handle: u32, h_handle: isize) -> i32;
            fn CreateFileW(
                lp_file_name: *const u16,
                dw_desired_access: u32,
                dw_share_mode: u32,
                lp_security_attributes: isize,
                dw_creation_disposition: u32,
                dw_flags_and_attributes: u32,
                h_template_file: isize,
            ) -> isize;
        }
        const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
        const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5; // ((DWORD)-11)
        const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4;  // ((DWORD)-12)

        let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let stderr_handle = GetStdHandle(STD_ERROR_HANDLE);
        let need_stdout = stdout_handle == 0 || stdout_handle == -1;
        let need_stderr = stderr_handle == 0 || stderr_handle == -1;

        if (need_stdout || need_stderr) && AttachConsole(ATTACH_PARENT_PROCESS) != 0 {
            const GENERIC_READ: u32 = 0x8000_0000;
            const GENERIC_WRITE: u32 = 0x4000_0000;
            const FILE_SHARE_READ: u32 = 1;
            const FILE_SHARE_WRITE: u32 = 2;
            const OPEN_EXISTING: u32 = 3;

            let conout_name: [u16; 8] = [
                'C' as u16, 'O' as u16, 'N' as u16, 'O' as u16, 'U' as u16, 'T' as u16, '$' as u16, 0,
            ];
            let handle = CreateFileW(
                conout_name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                0,
                OPEN_EXISTING,
                0,
                0,
            );
            if handle != -1 {
                if need_stdout {
                    SetStdHandle(STD_OUTPUT_HANDLE, handle);
                }
                if need_stderr {
                    SetStdHandle(STD_ERROR_HANDLE, handle);
                }
            }
        }
    }
}

#[cfg(windows)]
fn focus_existing_gui_window() {
    unsafe {
        unsafe extern "system" {
            fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> isize;
            fn IsIconic(h_wnd: isize) -> i32;
            fn ShowWindow(h_wnd: isize, n_cmd_show: i32) -> i32;
            fn SetForegroundWindow(h_wnd: isize) -> i32;
        }

        const SW_RESTORE: i32 = 9;
        let title: Vec<u16> = "M-RustHub - G3 V2 Manager\0".encode_utf16().collect();

        // Retry up to 10 times (50ms each) in case the first GUI instance is still creating its window
        for attempt in 0..10 {
            let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            if hwnd != 0 {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }
                SetForegroundWindow(hwnd);
                return;
            }
            if attempt < 9 {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

#[cfg(windows)]
fn acquire_instance_lock_internal(
    is_gui: bool,
    gui_mutex_name: &str,
    dev_mutex_name: &str,
    dev_timeout_ms: u32,
    focus_window: bool,
) -> Result<(bool, Option<isize>, Option<isize>), String> {
    unsafe {
        unsafe extern "system" {
            fn CreateMutexW(
                lp_mutex_attributes: isize,
                b_initial_owner: i32,
                lp_name: *const u16,
            ) -> isize;
            fn WaitForSingleObject(h_handle: isize, dw_milliseconds: u32) -> u32;
            fn ReleaseMutex(h_mutex: isize) -> i32;
            fn CloseHandle(h_object: isize) -> i32;
        }

        const WAIT_OBJECT_0: u32 = 0x0000_0000;
        const WAIT_ABANDONED: u32 = 0x0000_0080;

        let is_acquired = |status: u32| status == WAIT_OBJECT_0 || status == WAIT_ABANDONED;

        let mut gui_name_u16: Vec<u16> = gui_mutex_name.encode_utf16().collect();
        gui_name_u16.push(0);
        let mut dev_name_u16: Vec<u16> = dev_mutex_name.encode_utf16().collect();
        dev_name_u16.push(0);

        let gui_mutex = CreateMutexW(0, 0, gui_name_u16.as_ptr());
        if gui_mutex == 0 {
            return Ok((true, None, None));
        }

        // 1. Check immediately (0ms timeout) if a GUI instance is already running
        if !is_acquired(WaitForSingleObject(gui_mutex, 0)) {
            CloseHandle(gui_mutex);
            if is_gui {
                if focus_window {
                    focus_existing_gui_window();
                }
                return Ok((false, None, None));
            } else {
                return Err(
                    "M-RustHub GUI is currently running. Close the GUI window before using CLI commands."
                        .into(),
                );
            }
        }

        // Release gui_mutex immediately if running in CLI mode; keep it locked if launching GUI
        let retained_gui = if !is_gui {
            ReleaseMutex(gui_mutex);
            CloseHandle(gui_mutex);
            None
        } else {
            Some(gui_mutex)
        };

        // 2. Acquire the device access mutex (wait up to dev_timeout_ms for any active CLI command to finish)
        let dev_mutex = CreateMutexW(0, 0, dev_name_u16.as_ptr());
        if dev_mutex == 0 {
            return Ok((true, retained_gui, None));
        }

        if is_acquired(WaitForSingleObject(dev_mutex, dev_timeout_ms)) {
            // Keep mutex handles open for the lifetime of the process; Windows releases them on exit
            return Ok((true, retained_gui, Some(dev_mutex)));
        }

        CloseHandle(dev_mutex);
        if let Some(h) = retained_gui {
            ReleaseMutex(h);
            CloseHandle(h);
        }
        Err("Another M-RustHub CLI command is currently busy with the mouse (timeout).".into())
    }
}

#[cfg(windows)]
fn acquire_instance_lock(is_gui: bool) -> Result<bool, String> {
    acquire_instance_lock_internal(
        is_gui,
        "Local\\M_RustHub_Gui_Mutex",
        "Local\\M_RustHub_Device_Mutex",
        2000,
        true,
    )
    .map(|(success, _, _)| success)
}

fn handle_cli(cmd: Commands) -> Result<(), Box<dyn std::error::Error>> {
    if let Commands::Gui = cmd {
        gui::run_gui()?;
        return Ok(());
    }

    let mouse = MchoseMouse::open()?;

    // Check if mouse is awake and ready before attempting to write settings
    match &cmd {
        Commands::SetStage { .. }
        | Commands::SetLevels { .. }
        | Commands::SetDpi { .. }
        | Commands::SetRate { .. }
        | Commands::SetDebounce { .. }
        | Commands::SetSleep { .. }
        | Commands::SetButton { .. } => {
            if !mouse.get_online_status()? {
                return Err("Mouse is offline or sleeping. Move mouse or turn it on to wake it up before changing settings.".into());
            }
        }
        Commands::Status | Commands::Gui => {}
    }

    match cmd {
        Commands::Status => {
            let online = mouse.get_online_status().unwrap_or(false);
            let (fw, build_date) = if online {
                mouse
                    .query_version()
                    .unwrap_or_else(|_| ("Unknown".into(), "Unknown".into()))
            } else {
                ("Unavailable (sleeping)".into(), "Unavailable (sleeping)".into())
            };

            println!("==================================================");
            println!("  M-RUSTHUB - MCHOSE MOUSE TELEMETRY");
            println!("==================================================");
            println!("Device:        {}", mouse.info.product_name);
            println!("Connection:    {}", if mouse.info.is_wireless { "2.4G Wireless (Dongle)" } else { "USB Wired" });
            println!("Firmware:      {}", fw);
            println!("Build Date:    {}", build_date);
            println!("Hardware ID:   0x{:04X}:0x{:04X}", mouse.info.vid, mouse.info.pid);
            println!("Link Status:   {}", if online { "Online (Ready)" } else { "Sleeping / Standby" });

            if !online {
                println!("Battery:       Unavailable (mouse is sleeping / in standby)");
                println!("Config:        Unavailable (move mouse to wake up)");
            } else {
                match mouse.get_battery() {
                    Ok(bat) => {
                        let charge_str = if bat.charging { " [Charging]" } else { "" };
                        println!("Battery:       {}%{}", bat.percent, charge_str);
                    }
                    Err(e) => println!("Battery:       Error: {e}"),
                }

                match mouse.get_config() {
                    Ok(cfg) => {
                        println!("\nConfiguration:");
                        println!("  Polling Rate:     {}", cfg.report_rate);
                        println!("  Active DPI Stage: #{} of {}", cfg.active_dpi_stage, cfg.dpi_stages_count);
                        for (i, dpi) in cfg.dpi_stages.iter().enumerate() {
                            let mark = if i + 1 == cfg.active_dpi_stage { " <-- ACTIVE" } else { "" };
                            println!("    Stage {}:        {} DPI{}", i + 1, dpi, mark);
                        }
                        println!("  Button Debounce:  {} ms", cfg.debounce_ms);
                        if cfg.sleep_minutes == 0 {
                            println!("  Sleep Timeout:    0 min (Never sleep)");
                        } else {
                            println!("  Sleep Timeout:    {} min", cfg.sleep_minutes);
                        }
                    }
                    Err(e) => println!("Config:        Error: {e}"),
                }
            }
            println!("==================================================");
        }

        Commands::SetStage { stage } => {
            println!("Switching active DPI stage to #{}...", stage);
            mouse.set_active_dpi_stage(stage as usize)?;
            println!("✓ Active DPI stage set to #{}", stage);
        }

        Commands::SetLevels { count } => {
            println!("Setting number of active DPI stages to {}...", count);
            mouse.set_dpi_stages_count(count as usize)?;
            println!("✓ Number of active DPI stages set to {}", count);
        }

        Commands::SetDpi { dpi, stage } => {
            if dpi < crate::driver::MIN_DPI || dpi > crate::driver::MAX_DPI || dpi % crate::driver::DPI_STEP != 0 {
                return Err(format!(
                    "DPI must be between {} and {} and a multiple of {} DPI",
                    crate::driver::MIN_DPI,
                    crate::driver::MAX_DPI,
                    crate::driver::DPI_STEP
                )
                .into());
            }
            let mut cfg = mouse.get_config()?;
            let target_stage = stage.map(|s| s as usize).unwrap_or(cfg.active_dpi_stage).clamp(1, 6);
            if target_stage > cfg.dpi_stages_count {
                cfg.dpi_stages_count = target_stage;
            }
            cfg.dpi_stages[target_stage - 1] = dpi;
            mouse.set_config(&cfg)?;
            println!("✓ Stage #{} DPI set to {} DPI", target_stage, dpi);
        }

        Commands::SetRate { rate } => {
            let pr = PollingRate::from_hz(rate).ok_or("Invalid polling rate. MCHOSE G3 V2 supports: 125, 250, 500, 1000 Hz")?;
            println!("Setting polling rate to {}...", pr);
            mouse.set_polling_rate(pr)?;
            println!("✓ Polling rate successfully set to {}", pr);
        }

        Commands::SetDebounce { ms } => {
            println!("Setting debounce time to {} ms...", ms);
            mouse.set_debounce(ms)?;
            println!("✓ Debounce time successfully set to {} ms", ms);
        }

        Commands::SetSleep { minutes } => {
            if minutes == 0 {
                println!("Setting wireless sleep timeout to Never sleep (disabled)...");
            } else {
                println!("Setting wireless sleep timeout to {} min...", minutes);
            }
            mouse.set_sleep_timeout(minutes)?;
            if minutes == 0 {
                println!("✓ Wireless sleep disabled (Never sleep)");
            } else {
                println!("✓ Wireless sleep timeout successfully set to {} min", minutes);
            }
        }

        Commands::SetButton { button, action } => {
            let act = driver::ButtonAction::from_str_name(&action).ok_or_else(|| {
                format!(
                    "Invalid button action '{action}'. Supported actions: left, right, middle, forward, backward"
                )
            })?;
            let key_index = button - 1;
            println!(
                "Mapping button #{} ({}) to {}...",
                button,
                driver::BUTTON_NAMES[key_index as usize],
                act.label()
            );
            mouse.set_button_mapping(key_index, act)?;
            println!(
                "✓ Button #{} ({}) mapped to {}",
                button,
                driver::BUTTON_NAMES[key_index as usize],
                act.label()
            );
        }

        Commands::Gui => unreachable!(),
    }

    Ok(())
}

fn main() {
    #[cfg(windows)]
    if std::env::args_os().len() > 1 {
        attach_parent_console();
    }

    let args = Cli::parse();
    let is_gui = matches!(args.command, Some(Commands::Gui) | None);

    #[cfg(windows)]
    match acquire_instance_lock(is_gui) {
        Ok(true) => {}
        Ok(false) => return,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    match args.command {
        Some(Commands::Gui) | None => {
            // Default action when run without CLI arguments or with `gui`: launch GUI
            if let Err(e) = gui::run_gui() {
                eprintln!("GUI Error: {e}");
                std::process::exit(1);
            }
        }
        Some(cmd) => {
            if let Err(e) = handle_cli(cmd) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    struct TestLockGuard {
        gui: Option<isize>,
        dev: Option<isize>,
    }

    #[cfg(windows)]
    impl Drop for TestLockGuard {
        fn drop(&mut self) {
            unsafe {
                unsafe extern "system" {
                    fn ReleaseMutex(h: isize) -> i32;
                    fn CloseHandle(h: isize) -> i32;
                }
                if let Some(h) = self.gui.take() {
                    ReleaseMutex(h);
                    CloseHandle(h);
                }
                if let Some(h) = self.dev.take() {
                    ReleaseMutex(h);
                    CloseHandle(h);
                }
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_gui_first_instance_acquires_both_mutexes() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let gui_name = format!("Local\\M_RustHub_Test_Gui_{id}_1");
        let dev_name = format!("Local\\M_RustHub_Test_Dev_{id}_1");

        let res = acquire_instance_lock_internal(true, &gui_name, &dev_name, 1000, false);
        assert!(res.is_ok());
        let (success, gui_h, dev_h) = res.unwrap();
        assert!(success);
        assert!(gui_h.is_some());
        assert!(dev_h.is_some());

        let _guard = TestLockGuard {
            gui: gui_h,
            dev: dev_h,
        };
    }

    #[cfg(windows)]
    #[test]
    fn test_gui_second_instance_returns_false() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let gui_name = format!("Local\\M_RustHub_Test_Gui_{id}_2");
        let dev_name = format!("Local\\M_RustHub_Test_Dev_{id}_2");

        let res1 = acquire_instance_lock_internal(true, &gui_name, &dev_name, 1000, false);
        assert!(res1.is_ok());
        let (success1, gui_h1, dev_h1) = res1.unwrap();
        assert!(success1);
        let _guard1 = TestLockGuard {
            gui: gui_h1,
            dev: dev_h1,
        };

        let gui_name_clone = gui_name.clone();
        let dev_name_clone = dev_name.clone();
        let handle = std::thread::spawn(move || {
            acquire_instance_lock_internal(true, &gui_name_clone, &dev_name_clone, 100, false)
        });
        let res2 = handle.join().unwrap();
        assert!(res2.is_ok());
        let (success2, gui_h2, dev_h2) = res2.unwrap();
        assert!(!success2);
        assert!(gui_h2.is_none());
        assert!(dev_h2.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn test_cli_blocked_when_gui_running() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let gui_name = format!("Local\\M_RustHub_Test_Gui_{id}_3");
        let dev_name = format!("Local\\M_RustHub_Test_Dev_{id}_3");

        let res1 = acquire_instance_lock_internal(true, &gui_name, &dev_name, 1000, false);
        assert!(res1.is_ok());
        let (success1, gui_h1, dev_h1) = res1.unwrap();
        assert!(success1);
        let _guard1 = TestLockGuard {
            gui: gui_h1,
            dev: dev_h1,
        };

        let gui_name_clone = gui_name.clone();
        let dev_name_clone = dev_name.clone();
        let handle = std::thread::spawn(move || {
            acquire_instance_lock_internal(false, &gui_name_clone, &dev_name_clone, 100, false)
        });
        let res2 = handle.join().unwrap();
        assert!(res2.is_err());
        let err_msg = res2.unwrap_err();
        assert!(
            err_msg.contains("M-RustHub GUI is currently running"),
            "unexpected error: {err_msg}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_cli_succeeds_when_gui_closed_and_releases_gui_mutex() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let gui_name = format!("Local\\M_RustHub_Test_Gui_{id}_4");
        let dev_name = format!("Local\\M_RustHub_Test_Dev_{id}_4");

        let res = acquire_instance_lock_internal(false, &gui_name, &dev_name, 1000, false);
        assert!(res.is_ok());
        let (success, gui_h, dev_h) = res.unwrap();
        assert!(success);
        assert!(gui_h.is_none(), "CLI must not retain GUI mutex");
        assert!(dev_h.is_some(), "CLI must retain device mutex");

        let _guard = TestLockGuard {
            gui: None,
            dev: dev_h,
        };
    }

    #[cfg(windows)]
    #[test]
    fn test_cli_waits_for_another_cli_and_succeeds() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let gui_name = format!("Local\\M_RustHub_Test_Gui_{id}_5");
        let dev_name = format!("Local\\M_RustHub_Test_Dev_{id}_5");

        let dev_name_t1 = dev_name.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        let t1 = std::thread::spawn(move || {
            let res = acquire_instance_lock_internal(false, "Local\\DummyGui", &dev_name_t1, 1000, false);
            let (_ok, _gui, dev) = res.unwrap();
            tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _guard = TestLockGuard { gui: None, dev };
        });

        rx.recv().unwrap();

        let gui_name_t2 = gui_name.clone();
        let dev_name_t2 = dev_name.clone();
        let t2 = std::thread::spawn(move || {
            acquire_instance_lock_internal(false, &gui_name_t2, &dev_name_t2, 1000, false)
        });

        let res2 = t2.join().unwrap();
        t1.join().unwrap();

        assert!(res2.is_ok());
        let (success2, _, dev_h2) = res2.unwrap();
        assert!(success2);
        let _guard2 = TestLockGuard {
            gui: None,
            dev: dev_h2,
        };
    }

    #[cfg(windows)]
    #[test]
    fn test_gui_waits_for_active_cli_and_succeeds() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let gui_name = format!("Local\\M_RustHub_Test_Gui_{id}_7");
        let dev_name = format!("Local\\M_RustHub_Test_Dev_{id}_7");

        let dev_name_t1 = dev_name.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        let t1 = std::thread::spawn(move || {
            let res = acquire_instance_lock_internal(false, "Local\\DummyGui3", &dev_name_t1, 1000, false);
            let (_ok, _gui, dev) = res.unwrap();
            tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _guard = TestLockGuard { gui: None, dev };
        });

        rx.recv().unwrap();

        let gui_name_t2 = gui_name.clone();
        let dev_name_t2 = dev_name.clone();
        let t2 = std::thread::spawn(move || {
            acquire_instance_lock_internal(true, &gui_name_t2, &dev_name_t2, 1000, false)
        });

        let res2 = t2.join().unwrap();
        t1.join().unwrap();

        assert!(res2.is_ok());
        let (success2, gui_h2, dev_h2) = res2.unwrap();
        assert!(success2);
        assert!(gui_h2.is_some());
        assert!(dev_h2.is_some());
        let _guard2 = TestLockGuard {
            gui: gui_h2,
            dev: dev_h2,
        };
    }

    #[cfg(windows)]
    #[test]
    fn test_cli_times_out_when_device_mutex_held_too_long() {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let gui_name = format!("Local\\M_RustHub_Test_Gui_{id}_6");
        let dev_name = format!("Local\\M_RustHub_Test_Dev_{id}_6");

        let dev_name_t1 = dev_name.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        let t1 = std::thread::spawn(move || {
            let res = acquire_instance_lock_internal(false, "Local\\DummyGui2", &dev_name_t1, 1000, false);
            let (_ok, _gui, dev) = res.unwrap();
            tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(250));
            let _guard = TestLockGuard { gui: None, dev };
        });

        rx.recv().unwrap();

        let gui_name_t2 = gui_name.clone();
        let dev_name_t2 = dev_name.clone();
        let t2 = std::thread::spawn(move || {
            acquire_instance_lock_internal(false, &gui_name_t2, &dev_name_t2, 50, false)
        });

        let res2 = t2.join().unwrap();
        t1.join().unwrap();

        assert!(res2.is_err());
        let err_msg = res2.unwrap_err();
        assert!(
            err_msg.contains("timeout"),
            "unexpected error message: {err_msg}"
        );
    }
}
