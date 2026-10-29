use hidapi::{HidApi, HidDevice};
use std::fmt;

pub const VENDOR_ID_MCHOSE: u16 = 0x3837;
pub const USAGE_PAGE_CONFIG: u16 = 0xFF01;
pub const USAGE_CONFIG: u16 = 0x0010;

// Supported PIDs (MCHOSE G3 V2)
pub const PID_G3_V2_WIRED: u16 = 0x418C;
pub const PID_G3_V2_WIRELESS: u16 = 0x4245;

// DPI limits supported by PAW3311 sensor and official software (100..=12000, step 100)
pub const MIN_DPI: u16 = 100;
pub const MAX_DPI: u16 = 12000;
pub const DPI_STEP: u16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollingRate {
    Hz125 = 0,
    Hz250 = 1,
    Hz500 = 2,
    Hz1000 = 3,
    Hz2000 = 4,
    Hz4000 = 5,
    Hz8000 = 6,
}

impl PollingRate {
    pub fn from_index(idx: u8) -> Self {
        match idx {
            0 => PollingRate::Hz125,
            1 => PollingRate::Hz250,
            2 => PollingRate::Hz500,
            3 => PollingRate::Hz1000,
            4 => PollingRate::Hz2000,
            5 => PollingRate::Hz4000,
            6 => PollingRate::Hz8000,
            _ => PollingRate::Hz1000,
        }
    }

    pub fn to_hz(&self) -> u32 {
        match self {
            PollingRate::Hz125 => 125,
            PollingRate::Hz250 => 250,
            PollingRate::Hz500 => 500,
            PollingRate::Hz1000 => 1000,
            PollingRate::Hz2000 => 2000,
            PollingRate::Hz4000 => 4000,
            PollingRate::Hz8000 => 8000,
        }
    }

    pub fn from_hz(hz: u32) -> Option<Self> {
        match hz {
            125 => Some(PollingRate::Hz125),
            250 => Some(PollingRate::Hz250),
            500 => Some(PollingRate::Hz500),
            1000 => Some(PollingRate::Hz1000),
            _ => None,
        }
    }
}

impl fmt::Display for PollingRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Hz", self.to_hz())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lod {
    Low1mm = 0,
    High2mm = 1,
}

impl Lod {
    pub fn from_u8(val: u8) -> Self {
        if val == 0 { Lod::Low1mm } else { Lod::High2mm }
    }
    pub fn to_u8(&self) -> u8 {
        match self {
            Lod::Low1mm => 0,
            Lod::High2mm => 1,
        }
    }
}

impl fmt::Display for Lod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lod::Low1mm => write!(f, "1 mm"),
            Lod::High2mm => write!(f, "2 mm"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    LeftClick,
    RightClick,
    MiddleClick,
    Forward,
    Backward,
    Disabled,
    Custom([u8; 4]),
}

impl ButtonAction {
    pub const ALL: [ButtonAction; 5] = [
        ButtonAction::LeftClick,
        ButtonAction::RightClick,
        ButtonAction::MiddleClick,
        ButtonAction::Forward,
        ButtonAction::Backward,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ButtonAction::LeftClick => "Left button",
            ButtonAction::RightClick => "Right button",
            ButtonAction::MiddleClick => "Middle button",
            ButtonAction::Forward => "Forward button",
            ButtonAction::Backward => "Backward button",
            ButtonAction::Disabled => "Disabled",
            ButtonAction::Custom(_) => "Custom",
        }
    }

    /// Internal 24-bit action code as defined in protocol specification (Section 4 & 7)
    #[allow(dead_code)]
    pub fn action_code(&self) -> u32 {
        match self {
            ButtonAction::LeftClick => 0x000001,
            ButtonAction::RightClick => 0x000002,
            ButtonAction::MiddleClick => 0x000004,
            ButtonAction::Forward => 0x000010,
            ButtonAction::Backward => 0x000008,
            ButtonAction::Disabled => 0x000000,
            ButtonAction::Custom(raw) => {
                ((raw[0] as u32) << 24)
                    | ((raw[1] as u32) << 16)
                    | ((raw[2] as u32) << 8)
                    | (raw[3] as u32)
            }
        }
    }

    /// Encoded 4-byte action payload bytes as defined by G3 V2 controller specification
    pub fn to_payload_bytes(&self) -> [u8; 4] {
        match self {
            ButtonAction::LeftClick => [0x20, 0x01, 0x00, 0x00],
            ButtonAction::RightClick => [0x20, 0x02, 0x00, 0x00],
            ButtonAction::MiddleClick => [0x20, 0x04, 0x00, 0x00],
            ButtonAction::Backward => [0x20, 0x08, 0x00, 0x00],
            ButtonAction::Forward => [0x20, 0x10, 0x00, 0x00],
            ButtonAction::Disabled => [0x20, 0x00, 0x00, 0x00],
            ButtonAction::Custom(raw) => *raw,
        }
    }

    pub fn from_payload_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() >= 4 {
            match (bytes[0], bytes[1], bytes[2], bytes[3]) {
                (0x20, 0x01, 0x00, 0x00) => Some(ButtonAction::LeftClick),
                (0x20, 0x02, 0x00, 0x00) => Some(ButtonAction::RightClick),
                (0x20, 0x04, 0x00, 0x00) => Some(ButtonAction::MiddleClick),
                (0x20, 0x08, 0x00, 0x00) => Some(ButtonAction::Backward),
                (0x20, 0x10, 0x00, 0x00) => Some(ButtonAction::Forward),
                (0x20, 0x00, 0x00, 0x00) => Some(ButtonAction::Disabled),
                (b0, b1, b2, b3) => Some(ButtonAction::Custom([b0, b1, b2, b3])),
            }
        } else {
            None
        }
    }

    pub fn from_str_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().trim() {
            "left" | "leftclick" | "lmb" | "left button" => Some(ButtonAction::LeftClick),
            "right" | "rightclick" | "rmb" | "right button" => Some(ButtonAction::RightClick),
            "middle" | "middleclick" | "mmb" | "middle button" => Some(ButtonAction::MiddleClick),
            "forward" | "mouse5" | "forward button" => Some(ButtonAction::Forward),
            "backward" | "back" | "mouse4" | "backward button" => Some(ButtonAction::Backward),
            "disabled" | "disable" | "none" | "off" => Some(ButtonAction::Disabled),
            _ => None,
        }
    }
}

pub const DEFAULT_BUTTON_MAPPINGS: [ButtonAction; 5] = [
    ButtonAction::LeftClick,
    ButtonAction::RightClick,
    ButtonAction::MiddleClick,
    ButtonAction::Forward,
    ButtonAction::Backward,
];

pub const LEFT_BUTTON_LOCKED: &str = "Left button cannot be remapped (locked by mouse firmware)";

pub const BUTTON_NAMES: [&str; 5] = [
    "Left button",
    "Right button",
    "Middle button",
    "Forward button",
    "Backward button",
];

#[derive(Debug, Clone)]
pub struct BatteryStatus {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub product_name: String,
    #[allow(dead_code)]
    pub firmware: String,
    #[allow(dead_code)]
    pub build_date: String,
    pub is_wireless: bool,
    pub vid: u16,
    pub pid: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseConfig {
    pub report_rate: PollingRate,
    pub active_dpi_stage: usize, // 1..=6
    pub dpi_stages_count: usize, // e.g. 6
    pub dpi_stages: [u16; 6],
    pub lod: Lod,
    pub debounce_ms: u8,
    pub sleep_minutes: u8,
    pub sensor_flags: u8,
    pub scroll_flag: u8,
    pub highspeed_mode: u8,
    pub sensor_angle: i8,
}

impl Default for MouseConfig {
    /// Authentic factory defaults verified against official MCHOSE firmware
    fn default() -> Self {
        Self {
            report_rate: PollingRate::Hz250,
            active_dpi_stage: 3,
            dpi_stages_count: 6,
            dpi_stages: [400, 800, 1600, 3200, 6400, 12000],
            lod: Lod::High2mm,
            debounce_ms: 8,
            sleep_minutes: 3,
            sensor_flags: 0,
            scroll_flag: 0,
            highspeed_mode: 0,
            sensor_angle: 0,
        }
    }
}

#[derive(Debug)]
pub enum DriverError {
    DeviceNotFound,
    Hid(hidapi::HidError),
    Protocol(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::DeviceNotFound => write!(f, "MCHOSE G3 mouse not found. Check USB or wireless connection."),
            DriverError::Hid(e) => write!(f, "HID error: {e}"),
            DriverError::Protocol(e) => write!(f, "Protocol error: {e}"),
        }
    }
}

impl std::error::Error for DriverError {}

impl From<hidapi::HidError> for DriverError {
    fn from(e: hidapi::HidError) -> Self {
        DriverError::Hid(e)
    }
}

pub struct MchoseMouse {
    device: HidDevice,
    pub info: DeviceInfo,
}

impl MchoseMouse {
    /// Discovers and opens the active MCHOSE mouse (wired or 2.4G wireless)
    pub fn open() -> Result<Self, DriverError> {
        let api = HidApi::new()?;
        
        let is_target = |d: &hidapi::DeviceInfo, expected_pid: u16| {
            d.vendor_id() == VENDOR_ID_MCHOSE
                && d.product_id() == expected_pid
                && d.usage_page() == USAGE_PAGE_CONFIG
                && d.usage() == USAGE_CONFIG
        };

        // Prefer wired connection first (e.g. charging while 2.4G dongle is plugged in),
        // fallback to wireless dongle if wired is not connected.
        let target = api
            .device_list()
            .find(|d| is_target(d, PID_G3_V2_WIRED))
            .or_else(|| api.device_list().find(|d| is_target(d, PID_G3_V2_WIRELESS)));

        let d_info = target.ok_or(DriverError::DeviceNotFound)?;
        let vid = d_info.vendor_id();
        let pid = d_info.product_id();
        let is_wireless = pid == PID_G3_V2_WIRELESS;
        let product_name = d_info.product_string().unwrap_or("MCHOSE G3 V2").to_string();

        let device = d_info.open_device(&api)?;

        Ok(Self {
            device,
            info: DeviceInfo {
                product_name,
                firmware: String::new(),
                build_date: String::new(),
                is_wireless,
                vid,
                pid,
            },
        })
    }

    /// Discards any stale unread packets from the OS HID input buffer
    fn flush_input(&self) {
        let mut buf = [0u8; 65];
        while let Ok(n) = self.device.read_timeout(&mut buf, 0) {
            if n == 0 {
                break;
            }
        }
    }

    /// Sends a 64-byte payload with report ID 0 (total 65 bytes)
    fn write_packet(&self, payload: &[u8]) -> Result<(), DriverError> {
        self.flush_input();
        let mut pkt = [0u8; 65];
        pkt[0] = 0x00; // Report ID 0
        let len = payload.len().min(64);
        pkt[1..1 + len].copy_from_slice(&payload[..len]);
        self.device.write(&pkt)?;
        Ok(())
    }

    /// Reads response packet with timeout (default 300ms)
    fn read_packet(&self, timeout_ms: i32) -> Result<[u8; 64], DriverError> {
        let mut buf = [0u8; 65];
        let n = self.device.read_timeout(&mut buf, timeout_ms)?;
        if n == 0 {
            return Err(DriverError::Protocol("Read timeout from mouse".to_string()));
        }
        let mut out = [0u8; 64];
        if buf[0] == 170 { // 0xAA directly at byte 0
            out.copy_from_slice(&buf[..64]);
        } else if buf[1] == 170 { // 0xAA shifted by 1 byte (report ID prefix)
            out.copy_from_slice(&buf[1..65]);
        } else {
            out.copy_from_slice(&buf[..64]);
        }
        Ok(out)
    }

    /// Reads packets until a response matching `expected_cmd` is received or timeout expires
    fn read_response(&self, expected_cmd: u8, timeout_ms: u64) -> Result<[u8; 64], DriverError> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);
        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Err(DriverError::Protocol(format!(
                    "Read timeout waiting for response to cmd {expected_cmd}"
                )));
            }
            let remaining_ms = (timeout - elapsed).as_millis().max(1) as i32;
            let pkt = self.read_packet(remaining_ms)?;
            if pkt[0] == 170 && pkt[1] == expected_cmd {
                return Ok(pkt);
            }
        }
    }

    /// Query firmware version and build date
    pub fn query_version(&self) -> Result<(String, String), DriverError> {
        let cmd = [85, 3];
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            if self.write_packet(&cmd).is_err() {
                continue;
            }
            if let Ok(resp) = self.read_response(3, 150) {
                let fw = String::from_utf8_lossy(&resp[8..23]).trim_matches('\0').trim().to_string();
                let date = String::from_utf8_lossy(&resp[26..32]).trim_matches('\0').trim().to_string();
                if !fw.is_empty() {
                    return Ok((fw, date));
                }
            }
        }
        Err(DriverError::Protocol("Failed to read firmware version".to_string()))
    }

    /// Get live battery level and charging status
    pub fn get_battery(&self) -> Result<BatteryStatus, DriverError> {
        let cmd = [85, 48, 165, 11, 46, 1, 1, 1, 0, 0];
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            if self.write_packet(&cmd).is_err() {
                continue;
            }
            if let Ok(resp) = self.read_response(48, 150) {
                let percent = resp[8];
                if (1..=100).contains(&percent) {
                    return Ok(BatteryStatus {
                        percent,
                        charging: resp[9] != 0,
                    });
                }
            }
        }
        Err(DriverError::Protocol("Failed to read valid battery status".to_string()))
    }

    /// Get online connection status of the mouse (1 = online, 0 = sleep/offline)
    pub fn get_online_status(&self) -> Result<bool, DriverError> {
        // Wired connection is directly attached via USB cable
        if !self.info.is_wireless {
            return Ok(true);
        }

        let cmd = [85, 237];
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            if self.write_packet(&cmd).is_err() {
                continue;
            }
            if let Ok(resp) = self.read_response(237, 150) {
                return Ok(resp[8] != 0);
            }
        }
        Ok(false)
    }

    /// Read the complete mouse configuration
    pub fn get_config(&self) -> Result<MouseConfig, DriverError> {
        let cmd = [85, 14, 165, 11, 48, 1, 1, 1, 0, 0];
        for attempt in 0..4 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(60));
            }
            if self.write_packet(&cmd).is_err() {
                continue;
            }
            if let Ok(resp) = self.read_response(14, 150) {
                // Check for uninitialized / offline state (dongle has not yet linked with mouse)
                let dpi_stage1 = (resp[13] as u16) | ((resp[14] as u16) << 8);
                let is_uninitialized = (resp[13] == 0 && resp[14] == 0 && resp[15] == 0)
                    || (resp[13] == 255 && resp[14] == 255 && resp[15] == 255)
                    || resp[11] == 0
                    || dpi_stage1 < 50;

                if is_uninitialized {
                    continue;
                }

                let report_rate = PollingRate::from_index(resp[10].saturating_sub(1));
                let dpi_stages_count = (resp[11] as usize).clamp(1, 6);
                let active_dpi_stage = (resp[12] as usize).clamp(1, dpi_stages_count);

                let mut dpi_stages = [400, 800, 1600, 3200, 6400, 12000];
                for i in 0..6 {
                    let offset = 13 + i * 2;
                    let val = (resp[offset] as u16) | ((resp[offset + 1] as u16) << 8);
                    if val >= 50 && val <= 50000 {
                        dpi_stages[i] = val;
                    }
                }

                let scroll_flag = resp[48];
                let lod = Lod::from_u8(resp[49]);
                let sensor_flags = resp[50];
                let debounce_ms = resp[51];
                let sleep_minutes = resp[52];
                let highspeed_mode = resp[53];
                let sensor_angle = resp[55] as i8;

                return Ok(MouseConfig {
                    report_rate,
                    active_dpi_stage,
                    dpi_stages_count,
                    dpi_stages,
                    lod,
                    debounce_ms,
                    sleep_minutes,
                    sensor_flags,
                    scroll_flag,
                    highspeed_mode,
                    sensor_angle,
                });
            }
        }
        Err(DriverError::Protocol("Failed to read mouse configuration (device offline or uninitialized)".to_string()))
    }

    /// Build configuration packet ensuring all parameters including stage counts are normalized
    pub(crate) fn build_config_packet(cfg: &MouseConfig) -> [u8; 64] {
        let mut pkt = [0u8; 64];
        pkt[0] = 85;  // 0x55
        pkt[1] = 15;  // 0x0F
        pkt[2] = 174; // 0xAE
        pkt[3] = 10;  // 0x0A
        pkt[4] = 48;  // 0x30
        pkt[5] = 1;
        pkt[6] = 1;
        pkt[7] = 1;
        pkt[8] = 0;
        pkt[9] = 0;

        let dpi_stages_count = cfg.dpi_stages_count.clamp(1, 6);
        let active_dpi_stage = cfg.active_dpi_stage.clamp(1, dpi_stages_count);
        pkt[10] = (cfg.report_rate as u8) + 1;
        pkt[11] = dpi_stages_count as u8;
        pkt[12] = active_dpi_stage as u8;

        for i in 0..6 {
            let offset = 13 + i * 2;
            let val = cfg.dpi_stages[i];
            pkt[offset] = (val & 0xFF) as u8;
            pkt[offset + 1] = ((val >> 8) & 0xFF) as u8;
        }

        pkt[48] = cfg.scroll_flag;
        pkt[49] = cfg.lod.to_u8();
        pkt[50] = cfg.sensor_flags;
        pkt[51] = cfg.debounce_ms;
        pkt[52] = cfg.sleep_minutes;
        pkt[53] = cfg.highspeed_mode;
        pkt[55] = cfg.sensor_angle as u8;

        pkt
    }

    /// Write updated configuration to the mouse
    pub fn set_config(&self, cfg: &MouseConfig) -> Result<(), DriverError> {
        if !self.get_online_status()? {
            return Err(DriverError::Protocol("Mouse is offline or sleeping".to_string()));
        }
        let pkt = Self::build_config_packet(cfg);

        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            if self.write_packet(&pkt).is_err() {
                continue;
            }
            if self.read_response(15, 150).is_ok() {
                std::thread::sleep(std::time::Duration::from_millis(30));
                return Ok(());
            }
        }

        Err(DriverError::Protocol(
            "Failed to receive ACK after writing configuration".to_string(),
        ))
    }

    /// Quick helper: set active DPI stage (1..=6)
    pub fn set_active_dpi_stage(&self, stage: usize) -> Result<(), DriverError> {
        let mut cfg = self.get_config()?;
        if stage > cfg.dpi_stages_count {
            cfg.dpi_stages_count = stage.clamp(1, 6);
        }
        cfg.active_dpi_stage = stage.clamp(1, cfg.dpi_stages_count);
        self.set_config(&cfg)
    }

    /// Quick helper: set total number of active DPI stages (1..=6)
    pub fn set_dpi_stages_count(&self, count: usize) -> Result<(), DriverError> {
        let mut cfg = self.get_config()?;
        cfg.dpi_stages_count = count.clamp(1, 6);
        if cfg.active_dpi_stage > cfg.dpi_stages_count {
            cfg.active_dpi_stage = cfg.dpi_stages_count;
        }
        self.set_config(&cfg)
    }

    /// Quick helper: set polling rate
    pub fn set_polling_rate(&self, rate: PollingRate) -> Result<(), DriverError> {
        let mut cfg = self.get_config()?;
        cfg.report_rate = rate;
        self.set_config(&cfg)
    }

    /// Quick helper: set debounce time in ms (e.g. 0..=20)
    pub fn set_debounce(&self, ms: u8) -> Result<(), DriverError> {
        let mut cfg = self.get_config()?;
        cfg.debounce_ms = ms.clamp(0, 20);
        self.set_config(&cfg)
    }

    /// Quick helper: set LOD
    #[allow(dead_code)]
    pub fn set_lod(&self, lod: Lod) -> Result<(), DriverError> {
        let mut cfg = self.get_config()?;
        cfg.lod = lod;
        self.set_config(&cfg)
    }

    /// Quick helper: set wireless sleep timeout in minutes (0..=100, 0 = Never sleep)
    pub fn set_sleep_timeout(&self, minutes: u8) -> Result<(), DriverError> {
        let mut cfg = self.get_config()?;
        cfg.sleep_minutes = minutes.clamp(0, 100);
        self.set_config(&cfg)
    }

    /// Read all 5 button mappings from the mouse hardware microcontroller memory
    pub fn get_button_mappings(&self) -> Result<[ButtonAction; 5], DriverError> {
        let cmd = [85, 8, 165, 11, 32, 0, 0, 0, 0, 0];
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            if self.write_packet(&cmd).is_err() {
                continue;
            }
            if let Ok(resp) = self.read_response(8, 200) {
                let parse_slot = |offset: usize, default_action: ButtonAction| -> ButtonAction {
                    let slice = &resp[offset..offset + 4];
                    if slice == [0, 0, 0, 0] {
                        default_action
                    } else {
                        ButtonAction::from_payload_bytes(slice).unwrap_or(default_action)
                    }
                };

                // Firmware ignores slot 0, so the left button always acts as left click
                let lmb = ButtonAction::LeftClick;
                let rmb = parse_slot(12, ButtonAction::RightClick);
                let mmb = parse_slot(16, ButtonAction::MiddleClick);
                let back = parse_slot(20, ButtonAction::Backward);
                let forward = parse_slot(24, ButtonAction::Forward);

                // Firmware slot 3 is Backward (0x08), slot 4 is Forward (0x10)
                // GUI array order: [Left, Right, Middle, Forward, Backward]
                return Ok([lmb, rmb, mmb, forward, back]);
            }
        }
        Err(DriverError::Protocol("Failed to read button mappings from mouse".to_string()))
    }

    /// Builds a 64-byte USB HID payload for Command 9 (Set Mouse Keys) following G3 V2 controller spec
    pub fn build_set_buttons_packet(mappings: &[ButtonAction; 5]) -> [u8; 64] {
        let mut pkt = [0u8; 64];
        pkt[0] = 85;  // 0x55 Magic
        pkt[1] = 9;   // 0x09 Command: Set Keymap
        pkt[2] = 165; // 0xA5
        pkt[3] = 34;  // 0x22 Length parameter
        pkt[4] = 32;  // 0x20 Payload length (8 slots * 4 bytes = 32 bytes)
        pkt[5] = 0;
        pkt[6] = 0;
        pkt[7] = 0;

        // Slot 0 (Left button): locked to left click, as in the official driver;
        // the firmware ignores remaps of this slot anyway
        pkt[8..12].copy_from_slice(&ButtonAction::LeftClick.to_payload_bytes());

        // Slot 1 (Right button): mappings[1]
        pkt[12..16].copy_from_slice(&mappings[1].to_payload_bytes());

        // Slot 2 (Middle button): mappings[2]
        pkt[16..20].copy_from_slice(&mappings[2].to_payload_bytes());

        // Slot 3 (Backward / Mouse 4 in firmware): mappings[4]
        pkt[20..24].copy_from_slice(&mappings[4].to_payload_bytes());

        // Slot 4 (Forward / Mouse 5 in firmware): mappings[3]
        pkt[24..28].copy_from_slice(&mappings[3].to_payload_bytes());

        // Slot 5 (DPI button): 0x21, 0x55, 0, 0
        pkt[28] = 33;
        pkt[29] = 85;
        pkt[30] = 0;
        pkt[31] = 0;

        // Slot 6 (Scroll Up): 0x21, 0x38, 1, 0
        pkt[32] = 33;
        pkt[33] = 56;
        pkt[34] = 1;
        pkt[35] = 0;

        // Slot 7 (Scroll Down): 0x21, 0x38, 255, 0
        pkt[36] = 33;
        pkt[37] = 56;
        pkt[38] = 255;
        pkt[39] = 0;

        pkt
    }

    /// Writes all 5 button mappings to the mouse microcontroller flash memory (Cmd 9)
    pub fn set_button_mappings(&self, mappings: &[ButtonAction; 5]) -> Result<(), DriverError> {
        if !self.get_online_status()? {
            return Err(DriverError::Protocol("Mouse is offline or sleeping".to_string()));
        }
        let pkt = Self::build_set_buttons_packet(mappings);
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            if self.write_packet(&pkt).is_err() {
                continue;
            }
            if self.read_response(9, 200).is_ok() {
                std::thread::sleep(std::time::Duration::from_millis(250));
                return Ok(());
            }
        }
        Err(DriverError::Protocol(
            "Failed to receive ACK after writing button mappings".to_string(),
        ))
    }

    /// Sets the hardware mapping for a single button (1..=4; the left button is locked)
    pub fn set_button_mapping(&self, key_index: u8, action: ButtonAction) -> Result<(), DriverError> {
        if key_index == 0 {
            return Err(DriverError::Protocol(LEFT_BUTTON_LOCKED.to_string()));
        }
        if key_index > 4 {
            return Err(DriverError::Protocol(format!("Invalid keyIndex {key_index}, expected 1..=4")));
        }
        let mut mappings = self.get_button_mappings()?;
        mappings[key_index as usize] = action;
        self.set_button_mappings(&mappings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_set_buttons_packet_layout() {
        let mappings = DEFAULT_BUTTON_MAPPINGS;
        let pkt = MchoseMouse::build_set_buttons_packet(&mappings);

        // Header
        assert_eq!(pkt[0], 85);  // 0x55
        assert_eq!(pkt[1], 9);   // 0x09
        assert_eq!(pkt[2], 165); // 0xA5
        assert_eq!(pkt[3], 34);  // 0x22
        assert_eq!(pkt[4], 32);  // 0x20

        // Slot 0: LMB (32, 1, 0, 0)
        assert_eq!(&pkt[8..12], &[32, 1, 0, 0]);

        // Slot 1: RMB (32, 2, 0, 0)
        assert_eq!(&pkt[12..16], &[32, 2, 0, 0]);

        // Slot 2: MMB (32, 4, 0, 0)
        assert_eq!(&pkt[16..20], &[32, 4, 0, 0]);

        // Slot 3: Backward (32, 8, 0, 0)
        assert_eq!(&pkt[20..24], &[32, 8, 0, 0]);

        // Slot 4: Forward (32, 16, 0, 0)
        assert_eq!(&pkt[24..28], &[32, 16, 0, 0]);

        // Slot 5: DPI (33, 85, 0, 0)
        assert_eq!(&pkt[28..32], &[33, 85, 0, 0]);

        // Slot 6: Scroll Up (33, 56, 1, 0)
        assert_eq!(&pkt[32..36], &[33, 56, 1, 0]);

        // Slot 7: Scroll Down (33, 56, 255, 0)
        assert_eq!(&pkt[36..40], &[33, 56, 255, 0]);
    }

    #[test]
    fn test_build_set_buttons_packet_keeps_left_button_locked() {
        let mut mappings = DEFAULT_BUTTON_MAPPINGS;
        mappings[0] = ButtonAction::RightClick;
        mappings[1] = ButtonAction::LeftClick;
        let pkt = MchoseMouse::build_set_buttons_packet(&mappings);

        assert_eq!(&pkt[8..12], &[32, 1, 0, 0], "Slot 0 must stay left click");
        assert_eq!(&pkt[12..16], &[32, 1, 0, 0], "Slot 1 remap must still be written");
    }

    #[test]
    fn test_build_config_packet_normalizes_dpi_stages() {
        let mut cfg = MouseConfig::default();
        cfg.dpi_stages_count = 2;
        cfg.active_dpi_stage = 4; // active > count

        let pkt = MchoseMouse::build_config_packet(&cfg);
        assert_eq!(pkt[11], 2, "dpi_stages_count should be 2");
        assert_eq!(pkt[12], 2, "active_dpi_stage should be clamped to count (2)");

        // Out-of-bounds stage counts
        cfg.dpi_stages_count = 0;
        cfg.active_dpi_stage = 0;
        let pkt = MchoseMouse::build_config_packet(&cfg);
        assert_eq!(pkt[11], 1, "dpi_stages_count clamped to min 1");
        assert_eq!(pkt[12], 1, "active_dpi_stage clamped to min 1");

        cfg.dpi_stages_count = 10;
        cfg.active_dpi_stage = 5;
        let pkt = MchoseMouse::build_config_packet(&cfg);
        assert_eq!(pkt[11], 6, "dpi_stages_count clamped to max 6");
        assert_eq!(pkt[12], 5, "active_dpi_stage valid within 1..=6");
    }

    #[test]
    fn test_default_config_matches_factory_spec() {
        let def = MouseConfig::default();
        assert_eq!(def.report_rate, PollingRate::Hz250);
        assert_eq!(def.active_dpi_stage, 3);
        assert_eq!(def.dpi_stages_count, 6);
        assert_eq!(def.dpi_stages, [400, 800, 1600, 3200, 6400, 12000]);
        assert_eq!(def.lod, Lod::High2mm);
        assert_eq!(def.debounce_ms, 8);
        assert_eq!(def.sleep_minutes, 3);
        assert_eq!(def.sensor_flags, 0);
        assert_eq!(def.scroll_flag, 0);
        assert_eq!(def.highspeed_mode, 0);
        assert_eq!(def.sensor_angle, 0);
    }

    #[test]
    fn test_button_action_conversions() {
        assert_eq!(ButtonAction::from_str_name("left"), Some(ButtonAction::LeftClick));
        assert_eq!(ButtonAction::from_str_name("LMB"), Some(ButtonAction::LeftClick));
        assert_eq!(ButtonAction::from_str_name("Right"), Some(ButtonAction::RightClick));
        assert_eq!(ButtonAction::from_str_name("middle"), Some(ButtonAction::MiddleClick));
        assert_eq!(ButtonAction::from_str_name("forward"), Some(ButtonAction::Forward));
        assert_eq!(ButtonAction::from_str_name("backward"), Some(ButtonAction::Backward));
        assert_eq!(ButtonAction::from_str_name("back"), Some(ButtonAction::Backward));
        assert_eq!(ButtonAction::from_str_name("disabled"), Some(ButtonAction::Disabled));
        assert_eq!(ButtonAction::from_str_name("invalid"), None);

        for action in ButtonAction::ALL {
            let bytes = action.to_payload_bytes();
            assert_eq!(ButtonAction::from_payload_bytes(&bytes), Some(action));
            assert!(action.action_code() > 0);
        }

        let disabled = ButtonAction::Disabled;
        assert_eq!(ButtonAction::from_payload_bytes(&disabled.to_payload_bytes()), Some(disabled));
        assert_eq!(disabled.label(), "Disabled");

        let custom = ButtonAction::Custom([0x21, 0x04, 0x00, 0x00]);
        assert_eq!(ButtonAction::from_payload_bytes(&custom.to_payload_bytes()), Some(custom));
        assert_eq!(custom.label(), "Custom");
    }
}
