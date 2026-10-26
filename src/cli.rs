use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "m_rusthub",
    about = "Ultra-fast native configuration utility for MCHOSE G3 V2 mouse",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show current mouse telemetry (battery, DPI, rate, firmware, etc.)
    Status,

    /// Switch active DPI stage (1 to 6)
    SetStage {
        #[arg(value_parser = clap::value_parser!(u8).range(1..=6))]
        stage: u8,
    },

    /// Set number of active DPI levels/stages (1 to 6)
    SetLevels {
        #[arg(value_parser = clap::value_parser!(u8).range(1..=6))]
        count: u8,
    },

    /// Set DPI value for current or specified stage (100 to 12000)
    SetDpi {
        /// DPI value (100 to 12000)
        #[arg(value_parser = clap::value_parser!(u32).range((crate::driver::MIN_DPI as i64)..=(crate::driver::MAX_DPI as i64)))]
        dpi: u32,

        /// Optional stage number (1 to 6). Defaults to current active stage.
        #[arg(short, long, value_parser = clap::value_parser!(u8).range(1..=6))]
        stage: Option<u8>,
    },

    /// Set polling rate in Hz (125, 250, 500, 1000)
    SetRate {
        rate: u32,
    },

    /// Set button debounce time in milliseconds (0 to 20 ms)
    SetDebounce {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=20))]
        ms: u8,
    },

    /// Set wireless sleep timeout in minutes (0 to 100, 0 = Never sleep)
    SetSleep {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
        minutes: u8,
    },

    /// Launch graphical user interface (GUI)
    Gui,
}
