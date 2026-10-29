use crate::driver::{BatteryStatus, ButtonAction, DriverError, MchoseMouse, MouseConfig, DEFAULT_BUTTON_MAPPINGS};
use std::fmt;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

// ============================================================================
// BACKGROUND WORKER THREAD (Non-blocking HID operations)
// ============================================================================
pub(super) enum WorkerRequest {
    ConnectAndRead,
    ApplyConfig(MouseConfig, Option<[ButtonAction; 5]>),
}

pub(super) enum WorkerResponse {
    Connected {
        config: MouseConfig,
        battery: Option<BatteryStatus>,
        is_wireless: bool,
        button_mappings: [ButtonAction; 5],
    },
    Reloaded {
        config: MouseConfig,
        battery: Option<BatteryStatus>,
        is_wireless: bool,
        button_mappings: [ButtonAction; 5],
    },
    Telemetry {
        battery: Option<BatteryStatus>,
        is_wireless: bool,
    },
    DpiStageChanged(usize),
    ApplySuccess {
        config: MouseConfig,
        button_mappings: Option<[ButtonAction; 5]>,
    },
    ApplyError(String),
    Disconnected(String),
}

pub struct AppInitState {
    pub mouse: Option<MchoseMouse>,
    pub config: MouseConfig,
    pub battery: Option<BatteryStatus>,
    pub is_wireless: bool,
    pub is_connected: bool,
    pub button_mappings: [ButtonAction; 5],
}

impl AppInitState {
    pub fn detect() -> Self {
        match try_connect() {
            Ok(conn) => Self {
                is_wireless: conn.mouse.info.is_wireless,
                mouse: Some(conn.mouse),
                config: conn.config,
                battery: conn.battery,
                is_connected: true,
                button_mappings: conn.button_mappings,
            },
            Err(_) => Self {
                mouse: None,
                config: MouseConfig::default(),
                battery: None,
                is_wireless: true,
                is_connected: false,
                button_mappings: DEFAULT_BUTTON_MAPPINGS,
            },
        }
    }
}

// ============================================================================
// CONNECTING
// ============================================================================
/// An online mouse together with the state read from it
struct Connection {
    mouse: MchoseMouse,
    config: MouseConfig,
    battery: Option<BatteryStatus>,
    button_mappings: [ButtonAction; 5],
}

enum ConnectError {
    Offline,
    Driver(DriverError),
}

impl From<DriverError> for ConnectError {
    fn from(e: DriverError) -> Self {
        ConnectError::Driver(e)
    }
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectError::Offline => f.write_str("Mouse offline"),
            ConnectError::Driver(e) => e.fmt(f),
        }
    }
}

fn ensure_online(mouse: MchoseMouse) -> Result<MchoseMouse, ConnectError> {
    if mouse.get_online_status().unwrap_or(false) {
        Ok(mouse)
    } else {
        Err(ConnectError::Offline)
    }
}

/// Checks that the mouse is online and reads its config, buttons and battery
fn read_connection(mouse: MchoseMouse) -> Result<Connection, ConnectError> {
    let mouse = ensure_online(mouse)?;
    let config = mouse.get_config()?;
    let button_mappings = mouse.get_button_mappings()?;
    let battery = mouse.get_battery().ok();
    Ok(Connection {
        mouse,
        config,
        battery,
        button_mappings,
    })
}

/// Opens a fresh handle and reads the full mouse state
fn try_connect() -> Result<Connection, ConnectError> {
    read_connection(MchoseMouse::open()?)
}

// ============================================================================
// WORKER LOOP
// ============================================================================
struct Worker {
    tx: Sender<WorkerResponse>,
    mouse: Option<MchoseMouse>,
    last_known_stage: Option<usize>,
    consecutive_poll_errors: u32,
    last_battery_poll: Instant,
    last_stage_poll: Instant,
    last_reconnect_poll: Instant,
}

pub(super) fn start_worker(
    tx_resp: Sender<WorkerResponse>,
    rx_req: Receiver<WorkerRequest>,
    initial_mouse: Option<MchoseMouse>,
    initial_stage: Option<usize>,
) {
    std::thread::spawn(move || {
        let now = Instant::now();
        // Without a mouse at startup the first reconnect attempt runs right away
        let last_reconnect_poll = if initial_mouse.is_none() {
            now.checked_sub(Duration::from_secs(10)).unwrap_or(now)
        } else {
            now
        };
        let mut worker = Worker {
            tx: tx_resp,
            mouse: initial_mouse,
            last_known_stage: initial_stage,
            consecutive_poll_errors: 0,
            last_battery_poll: now,
            last_stage_poll: now,
            last_reconnect_poll,
        };

        loop {
            match rx_req.recv_timeout(Duration::from_millis(200)) {
                Ok(WorkerRequest::ConnectAndRead) => worker.connect_and_read(),
                Ok(WorkerRequest::ApplyConfig(cfg, btn_mappings)) => worker.apply_config(cfg, btn_mappings),
                Err(RecvTimeoutError::Timeout) => worker.poll(),
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

impl Worker {
    fn send(&self, resp: WorkerResponse) {
        let _ = self.tx.send(resp);
    }

    /// Makes `mouse` the active handle with a known DPI stage
    fn attach(&mut self, mouse: MchoseMouse, stage: usize) {
        self.last_known_stage = Some(stage);
        self.consecutive_poll_errors = 0;
        self.mouse = Some(mouse);
    }

    /// Drops the active handle, reports it to the UI and restarts the reconnect timer
    fn detach(&mut self, reason: String) {
        self.mouse = None;
        self.last_reconnect_poll = Instant::now();
        self.send(WorkerResponse::Disconnected(reason));
    }

    /// Counts a failed poll; returns false once the mouse should be treated as gone
    fn poll_error(&mut self) -> bool {
        self.consecutive_poll_errors += 1;
        self.consecutive_poll_errors <= 4
    }

    fn connect_and_read(&mut self) {
        let Some(m) = self.mouse.take().or_else(|| MchoseMouse::open().ok()) else {
            self.send(WorkerResponse::Disconnected("Mouse not detected".into()));
            return;
        };
        let result = match read_connection(m) {
            // A stale handle fails on reads: retry once with a fresh one
            Err(ConnectError::Driver(_)) => try_connect(),
            other => other,
        };
        match result {
            Ok(conn) => {
                let is_wireless = conn.mouse.info.is_wireless;
                self.attach(conn.mouse, conn.config.active_dpi_stage);
                self.send(WorkerResponse::Reloaded {
                    config: conn.config,
                    battery: conn.battery,
                    is_wireless,
                    button_mappings: conn.button_mappings,
                });
            }
            Err(e) => self.detach(e.to_string()),
        }
    }

    fn apply_config(&mut self, cfg: MouseConfig, btn_mappings: Option<[ButtonAction; 5]>) {
        let Some(m) = self.mouse.take().or_else(|| MchoseMouse::open().ok()) else {
            self.send(WorkerResponse::ApplyError("Mouse not connected".into()));
            self.send(WorkerResponse::Disconnected("Mouse not connected".into()));
            return;
        };
        let Ok(m) = ensure_online(m) else {
            self.send(WorkerResponse::ApplyError("Mouse is offline or sleeping".into()));
            self.detach("Mouse offline".into());
            return;
        };

        let result = m.set_config(&cfg).and_then(|()| match btn_mappings {
            Some(btns) => m.set_button_mappings(&btns),
            None => Ok(()),
        });

        match result {
            Ok(()) => {
                let norm_count = cfg.dpi_stages_count.clamp(1, 6);
                self.attach(m, cfg.active_dpi_stage.clamp(1, norm_count));
                self.send(WorkerResponse::ApplySuccess {
                    config: cfg,
                    button_mappings: btn_mappings,
                });
            }
            Err(e) => {
                drop(m);
                self.send(WorkerResponse::ApplyError(e.to_string()));
                // Stay connected if a fresh handle can still read the mouse
                match MchoseMouse::open().map_err(ConnectError::from).and_then(ensure_online) {
                    Ok(m2) => match m2.get_config() {
                        Ok(cfg2) => self.attach(m2, cfg2.active_dpi_stage),
                        Err(_) => self.detach("Mouse offline".into()),
                    },
                    Err(e2) => self.detach(e2.to_string()),
                }
            }
        }
    }

    fn poll(&mut self) {
        if self.mouse.is_none() && self.last_reconnect_poll.elapsed() >= Duration::from_millis(500) {
            self.last_reconnect_poll = Instant::now();
            if let Ok(conn) = try_connect() {
                let is_wireless = conn.mouse.info.is_wireless;
                self.attach(conn.mouse, conn.config.active_dpi_stage);
                self.send(WorkerResponse::Connected {
                    config: conn.config,
                    battery: conn.battery,
                    is_wireless,
                    button_mappings: conn.button_mappings,
                });
            }
        }

        let Some(m) = self.mouse.take() else {
            return;
        };
        let mut mouse_ok = true;

        // Fast check online status (1-2ms) before attempting any heavy queries
        if m.info.is_wireless {
            match m.get_online_status() {
                Ok(true) => {}
                Ok(false) => mouse_ok = false,
                Err(_) => mouse_ok = self.poll_error(),
            }
        }

        // 1. Fast poll for active DPI stage (~200ms) to detect physical mouse button presses
        if mouse_ok && self.last_stage_poll.elapsed() >= Duration::from_millis(200) {
            self.last_stage_poll = Instant::now();
            match m.get_config() {
                Ok(cfg) => {
                    self.consecutive_poll_errors = 0;
                    if Some(cfg.active_dpi_stage) != self.last_known_stage {
                        self.last_known_stage = Some(cfg.active_dpi_stage);
                        self.send(WorkerResponse::DpiStageChanged(cfg.active_dpi_stage));
                    }
                }
                Err(_) => mouse_ok = self.poll_error(),
            }
        }

        // 2. Slow poll for battery & wireless telemetry (every 5s)
        if mouse_ok && self.last_battery_poll.elapsed() >= Duration::from_secs(5) {
            self.last_battery_poll = Instant::now();
            match m.get_battery() {
                Ok(bat) => {
                    self.consecutive_poll_errors = 0;
                    self.send(WorkerResponse::Telemetry {
                        battery: Some(bat),
                        is_wireless: m.info.is_wireless,
                    });
                }
                Err(_) => mouse_ok = self.poll_error(),
            }
        }

        if mouse_ok {
            self.mouse = Some(m);
        } else {
            drop(m);
            self.detach("Mouse offline".into());
        }
    }
}
