// Version: v1.0
//! # Máquina de Estados Maestra (MSM) — Nodo B / Arduino Mega

const WATCHDOG_MAX: u16 = 100;
const AVOID_SPEED: i16 = 60;
const RETREAT_SPEED: i16 = -50;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AvoidDir { Left, Right }

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Command {
    Ping,
    Standby,
    Explore { left: i16, right: i16 },
    Avoid(AvoidDir),
    Retreat,
    Fault,
    Reset,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RoverState {
    Standby,
    Explore,
    Avoid,
    Retreat,
    Fault,
    Safe,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum SafetyState {
    Normal,
    Warn,
    Limit,
    FaultStall,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct DriveOutput {
    pub left: i16,
    pub right: i16,
}

impl DriveOutput {
    pub const STOP: Self = Self { left: 0, right: 0 };
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SensorFrame {
    pub tick_ms: u32,
    pub batt_mv: u16,
    pub batt_ma: i32,
    pub currents: [i32; 6],
    pub temp_c: i32,
    pub batt_temps: [i32; 6],
    pub dist_mm: u16,
    pub enc_left: i32,
    pub enc_right: i32,
    pub x_mm: i32,
    pub y_mm: i32,
    pub theta_mrad: i16,
}

impl SensorFrame {
    pub const ZERO: Self = Self {
        tick_ms: 0, batt_mv: 0, batt_ma: 0,
        currents: [0; 6], temp_c: 0, batt_temps: [0; 6],
        dist_mm: 0, enc_left: 0, enc_right: 0,
        x_mm: 0, y_mm: 0, theta_mrad: 0,
    };
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Response {
    Pong,
    Ack(RoverState),
    Telemetry { safety: SafetyState, stall_mask: u8, sensors: SensorFrame },
    ErrEstop,
    ErrUnknown,
    ErrWatchdog,
}

pub struct MasterStateMachine {
    pub state: RoverState,
    pub safety: SafetyState,
    pub drive: DriveOutput,
    watchdog: u16,
}

impl MasterStateMachine {
    pub fn new() -> Self {
        Self {
            state: RoverState::Standby,
            safety: SafetyState::Normal,
            drive: DriveOutput::STOP,
            watchdog: 0,
        }
    }

    pub fn tick(&mut self) -> Option<Response> {
        match self.state {
            RoverState::Explore | RoverState::Avoid | RoverState::Retreat => {}
            _ => return None,
        }
        self.watchdog = self.watchdog.saturating_add(1);
        if self.watchdog >= WATCHDOG_MAX {
            self.state = RoverState::Fault;
            self.drive = DriveOutput::STOP;
            return Some(Response::ErrWatchdog);
        }
        None
    }

    pub fn process(&mut self, cmd: Command) -> Response {
        self.watchdog = 0;
        match cmd {
            Command::Ping => Response::Pong,
            Command::Reset => {
                self.state = RoverState::Standby;
                self.drive = DriveOutput::STOP;
                self.safety = SafetyState::Normal;
                Response::Ack(RoverState::Standby)
            }
            _ if self.state == RoverState::Fault || self.state == RoverState::Safe => Response::ErrEstop,
            Command::Standby => {
                self.state = RoverState::Standby;
                self.drive = DriveOutput::STOP;
                Response::Ack(RoverState::Standby)
            }
            Command::Explore { left, right } => {
                self.state = RoverState::Explore;
                self.drive = DriveOutput { left: left.clamp(-99, 99), right: right.clamp(-99, 99) };
                Response::Ack(RoverState::Explore)
            }
            Command::Avoid(dir) => {
                self.state = RoverState::Avoid;
                self.drive = match dir {
                    AvoidDir::Left  => DriveOutput { left: -AVOID_SPEED, right:  AVOID_SPEED },
                    AvoidDir::Right => DriveOutput { left:  AVOID_SPEED, right: -AVOID_SPEED },
                };
                Response::Ack(RoverState::Avoid)
            }
            Command::Retreat => {
                self.state = RoverState::Retreat;
                self.drive = DriveOutput { left: RETREAT_SPEED, right: RETREAT_SPEED };
                Response::Ack(RoverState::Retreat)
            }
            Command::Fault => {
                self.state = RoverState::Fault;
                self.drive = DriveOutput::STOP;
                Response::Ack(RoverState::Fault)
            }
        }
    }

    pub fn update_safety(&mut self, stall_mask: u8) {
        if stall_mask != 0 {
            self.safety = SafetyState::FaultStall;
            self.state = RoverState::Fault;
            self.drive = DriveOutput::STOP;
        }
    }

    pub fn update_overcurrent(&mut self, level: SafetyState) -> bool {
        match level {
            SafetyState::FaultStall => {
                self.safety = SafetyState::FaultStall;
                self.state  = RoverState::Fault;
                self.drive  = DriveOutput::STOP;
                true
            }
            SafetyState::Warn | SafetyState::Limit => {
                if level > self.safety { self.safety = level; }
                false
            }
            SafetyState::Normal => {
                if self.safety == SafetyState::Warn || self.safety == SafetyState::Limit {
                    self.safety = SafetyState::Normal;
                }
                false
            }
        }
    }

    pub fn telemetry(&self, stall_mask: u8, sensors: SensorFrame) -> Response {
        Response::Telemetry { safety: self.safety, stall_mask, sensors }
    }
}

pub fn parse_command(bytes: &[u8]) -> Option<Command> {
    match bytes {
        b"PING" => Some(Command::Ping),
        b"STB"  => Some(Command::Standby),
        b"RET"  => Some(Command::Retreat),
        b"FLT"  => Some(Command::Fault),
        b"RST"  => Some(Command::Reset),
        _ if bytes.starts_with(b"EXP:") => parse_explore(&bytes[4..]),
        _ if bytes.starts_with(b"AVD:") => parse_avoid(&bytes[4..]),
        _ => None,
    }
}

fn parse_explore(bytes: &[u8]) -> Option<Command> {
    let colon = bytes.iter().position(|&b| b == b':')?;
    let left  = parse_i16(&bytes[..colon])?;
    let right = parse_i16(&bytes[colon + 1..])?;
    Some(Command::Explore { left, right })
}

fn parse_avoid(bytes: &[u8]) -> Option<Command> {
    match bytes {
        b"L" => Some(Command::Avoid(AvoidDir::Left)),
        b"R" => Some(Command::Avoid(AvoidDir::Right)),
        _    => None,
    }
}

fn parse_i16(bytes: &[u8]) -> Option<i16> {
    if bytes.is_empty() { return None; }
    let (neg, digits) = if bytes[0] == b'-' { (true, &bytes[1..]) } else { (false, bytes) };
    if digits.is_empty() || digits.len() > 3 { return None; }
    let mut val: i16 = 0;
    for &b in digits {
        if b < b'0' || b > b'9' { return None; }
        val = val * 10 + (b - b'0') as i16;
    }
    Some(if neg { -val } else { val })
}

pub fn format_response<'a>(resp: Response, buf: &'a mut [u8]) -> &'a [u8] {
    match resp {
        Response::Pong           => copy_literal(buf, b"PONG\n"),
        Response::ErrEstop       => copy_literal(buf, b"ERR:ESTOP\n"),
        Response::ErrUnknown     => copy_literal(buf, b"ERR:UNKNOWN\n"),
        Response::ErrWatchdog    => copy_literal(buf, b"ERR:WDOG\n"),
        Response::Ack(state)     => format_ack(buf, state_label(state)),
        Response::Telemetry { safety, stall_mask, sensors } => format_tlm(buf, safety, stall_mask, sensors),
    }
}

fn state_label(state: RoverState) -> &'static [u8] {
    match state {
        RoverState::Standby => b"STB",
        RoverState::Explore => b"EXP",
        RoverState::Avoid   => b"AVD",
        RoverState::Retreat => b"RET",
        RoverState::Fault   => b"FLT",
        RoverState::Safe    => b"SFE",
    }
}

fn copy_literal<'a>(buf: &'a mut [u8], src: &[u8]) -> &'a [u8] {
    let len = src.len().min(buf.len());
    buf[..len].copy_from_slice(&src[..len]);
    &buf[..len]
}

fn format_ack<'a>(buf: &'a mut [u8], label: &[u8]) -> &'a [u8] {
    let mut i = 0;
    for &b in b"ACK:" { buf[i] = b; i += 1; }
    for &b in label   { buf[i] = b; i += 1; }
    buf[i] = b'\n'; i += 1;
    &buf[..i]
}

fn format_tlm<'a>(buf: &'a mut [u8], safety: SafetyState, stall_mask: u8, sensors: SensorFrame) -> &'a [u8] {
    let safety_label: &[u8] = match safety {
        SafetyState::Normal     => b"NORMAL",
        SafetyState::Warn       => b"WARN",
        SafetyState::Limit      => b"LIMIT",
        SafetyState::FaultStall => b"FAULT",
    };
    let mut i = 0;
    for &b in b"TLM:"      { buf[i] = b; i += 1; }
    for &b in safety_label  { buf[i] = b; i += 1; }
    buf[i] = b':'; i += 1;
    for bit in (0..6u8).rev() {
        buf[i] = if (stall_mask >> bit) & 1 == 1 { b'1' } else { b'0' };
        i += 1;
    }
    buf[i] = b':'; i += 1;
    write_u32(sensors.tick_ms, buf, &mut i);
    buf[i] = b'm'; i += 1;
    buf[i] = b's'; i += 1;
    buf[i] = b':'; i += 1;
    write_u32(sensors.batt_mv as u32, buf, &mut i);
    buf[i] = b'm'; i += 1;
    buf[i] = b'V'; i += 1;
    buf[i] = b':'; i += 1;
    write_i32(sensors.batt_ma, buf, &mut i);
    buf[i] = b'm'; i += 1;
    buf[i] = b'A'; i += 1;
    for current in &sensors.currents {
        buf[i] = b':'; i += 1;
        write_i32(*current, buf, &mut i);
    }
    buf[i] = b':'; i += 1;
    write_i32(sensors.temp_c, buf, &mut i);
    buf[i] = b'C'; i += 1;
    for batt_t in &sensors.batt_temps {
        buf[i] = b':'; i += 1;
        write_i32(*batt_t, buf, &mut i);
    }
    buf[i] = b'C'; i += 1;
    buf[i] = b':'; i += 1;
    write_i32(sensors.dist_mm as i32, buf, &mut i);
    buf[i] = b'm'; i += 1;
    buf[i] = b'm'; i += 1;
    buf[i] = b':'; i += 1;
    write_i32(sensors.enc_left, buf, &mut i);
    buf[i] = b':'; i += 1;
    write_i32(sensors.enc_right, buf, &mut i);
    buf[i] = b':'; i += 1;
    write_i32(sensors.x_mm, buf, &mut i);
    buf[i] = b':'; i += 1;
    write_i32(sensors.y_mm, buf, &mut i);
    buf[i] = b':'; i += 1;
    write_i32(sensors.theta_mrad as i32, buf, &mut i);
    buf[i] = b'\n'; i += 1;
    &buf[..i]
}

fn write_u32(val: u32, buf: &mut [u8], pos: &mut usize) {
    if val == 0 { buf[*pos] = b'0'; *pos += 1; return; }
    let mut v = val;
    let start = *pos;
    let mut tmp = [0u8; 10];
    let mut len = 0usize;
    while v > 0 { tmp[len] = b'0' + (v % 10) as u8; v /= 10; len += 1; }
    for k in 0..len { buf[start + k] = tmp[len - 1 - k]; }
    *pos += len;
}

fn write_i32(val: i32, buf: &mut [u8], pos: &mut usize) {
    if val == 0 { buf[*pos] = b'0'; *pos += 1; return; }
    let mut v = val;
    if v < 0 { buf[*pos] = b'-'; *pos += 1; v = -v; }
    write_u32(v as u32, buf, pos);
}
