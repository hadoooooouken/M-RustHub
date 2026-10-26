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
        | Commands::SetSleep { .. } => {
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
            let target_stage = stage.map(|s| s as usize).unwrap_or(cfg.active_dpi_stage);
            let stage_idx = target_stage.clamp(1, 6) - 1;
            cfg.dpi_stages[stage_idx] = dpi;
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
