use clap::{Parser, Subcommand};
use crate::driver::{DPI_STEP, MAX_DPI, MIN_DPI};

pub fn parse_dpi(s: &str) -> Result<u16, String> {
    let val: i64 = s
        .parse()
        .map_err(|_| format!("'{s}' is not a valid integer"))?;
    if !(MIN_DPI as i64..=MAX_DPI as i64).contains(&val) {
        return Err(format!(
            "DPI must be between {} and {}",
            MIN_DPI,
            MAX_DPI
        ));
    }
    if (val as u16) % DPI_STEP != 0 {
        return Err(format!(
            "DPI must be a multiple of {} (sensor step)",
            DPI_STEP
        ));
    }
    Ok(val as u16)
}

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

    /// Set DPI value for current or specified stage (100 to 12000, step 100)
    SetDpi {
        /// DPI value (100 to 12000, step 100)
        #[arg(value_parser = parse_dpi)]
        dpi: u16,

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dpi_valid_multiples() {
        assert_eq!(parse_dpi("100").unwrap(), 100);
        assert_eq!(parse_dpi("800").unwrap(), 800);
        assert_eq!(parse_dpi("1600").unwrap(), 1600);
        assert_eq!(parse_dpi("12000").unwrap(), 12000);
    }

    #[test]
    fn test_parse_dpi_not_multiple_of_step() {
        assert!(parse_dpi("850").is_err());
        assert!(parse_dpi("1234").is_err());
        assert!(parse_dpi("105").is_err());
        assert!(parse_dpi("11950").is_err());
    }

    #[test]
    fn test_parse_dpi_out_of_range() {
        assert!(parse_dpi("0").is_err());
        assert!(parse_dpi("50").is_err());
        assert!(parse_dpi("-100").is_err());
        assert!(parse_dpi("12100").is_err());
        assert!(parse_dpi("20000").is_err());
    }

    #[test]
    fn test_parse_dpi_invalid_string() {
        assert!(parse_dpi("abc").is_err());
        assert!(parse_dpi("").is_err());
    }
}
