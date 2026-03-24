// Version: v1.0
// Pruebas unitarias de la Máquina de Estados Maestra (MSM).
// Se ejecutan en el PC, NO en el Arduino.
// Comando: cargo test --target x86_64-unknown-linux-gnu

use rover_low_level_controller::state_machine::{
    parse_command, format_response,
    AvoidDir, Command, DriveOutput, MasterStateMachine,
    Response, RoverState, SafetyState,
};

// ─── Parser ───────────────────────────────────────────────────────────────────

#[test]
fn test_parse_ping() {
    assert_eq!(parse_command(b"PING"), Some(Command::Ping));
}

#[test]
fn test_parse_standby() {
    assert_eq!(parse_command(b"STB"), Some(Command::Standby));
}

#[test]
fn test_parse_retreat() {
    assert_eq!(parse_command(b"RET"), Some(Command::Retreat));
}

#[test]
fn test_parse_fault() {
    assert_eq!(parse_command(b"FLT"), Some(Command::Fault));
}

#[test]
fn test_parse_reset() {
    assert_eq!(parse_command(b"RST"), Some(Command::Reset));
}

#[test]
fn test_parse_explore_positive() {
    assert_eq!(
        parse_command(b"EXP:80:80"),
        Some(Command::Explore { left: 80, right: 80 })
    );
}

#[test]
fn test_parse_explore_negative_left() {
    assert_eq!(
        parse_command(b"EXP:-50:60"),
        Some(Command::Explore { left: -50, right: 60 })
    );
}

#[test]
fn test_parse_explore_both_negative() {
    assert_eq!(
        parse_command(b"EXP:-99:-99"),
        Some(Command::Explore { left: -99, right: -99 })
    );
}

#[test]
fn test_parse_explore_zero() {
    assert_eq!(
        parse_command(b"EXP:0:0"),
        Some(Command::Explore { left: 0, right: 0 })
    );
}

#[test]
fn test_parse_avoid_left() {
    assert_eq!(parse_command(b"AVD:L"), Some(Command::Avoid(AvoidDir::Left)));
}

#[test]
fn test_parse_avoid_right() {
    assert_eq!(parse_command(b"AVD:R"), Some(Command::Avoid(AvoidDir::Right)));
}

#[test]
fn test_parse_unknown_returns_none() {
    assert_eq!(parse_command(b"XYZ"), None);
}

#[test]
fn test_parse_empty_returns_none() {
    assert_eq!(parse_command(b""), None);
}

#[test]
fn test_parse_explore_missing_colon_returns_none() {
    assert_eq!(parse_command(b"EXP:8080"), None);
}

#[test]
fn test_parse_explore_letters_in_value_returns_none() {
    assert_eq!(parse_command(b"EXP:AB:80"), None);
}

// ─── Máquina de estados ───────────────────────────────────────────────────────

#[test]
fn test_initial_state_is_standby() {
    let msm = MasterStateMachine::new();
    assert_eq!(msm.state, RoverState::Standby);
    assert_eq!(msm.drive, DriveOutput::STOP);
    assert_eq!(msm.safety, SafetyState::Normal);
}

#[test]
fn test_explore_updates_drive() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Explore { left: 80, right: 80 });
    assert_eq!(msm.state, RoverState::Explore);
    assert_eq!(msm.drive.left, 80);
    assert_eq!(msm.drive.right, 80);
}

#[test]
fn test_explore_clamps_over_99() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Explore { left: 150, right: -150 });
    assert_eq!(msm.drive.left, 99);
    assert_eq!(msm.drive.right, -99);
}

#[test]
fn test_avoid_left_turns_in_place() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Avoid(AvoidDir::Left));
    assert_eq!(msm.state, RoverState::Avoid);
    assert!(msm.drive.left < 0, "giro izq: ruedas izq deben ir atrás");
    assert!(msm.drive.right > 0, "giro izq: ruedas der deben ir adelante");
}

#[test]
fn test_avoid_right_turns_in_place() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Avoid(AvoidDir::Right));
    assert_eq!(msm.state, RoverState::Avoid);
    assert!(msm.drive.left > 0);
    assert!(msm.drive.right < 0);
}

#[test]
fn test_retreat_moves_backwards() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Retreat);
    assert_eq!(msm.state, RoverState::Retreat);
    assert!(msm.drive.left < 0);
    assert!(msm.drive.right < 0);
}

#[test]
fn test_fault_stops_motors() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Explore { left: 80, right: 80 });
    msm.process(Command::Fault);
    assert_eq!(msm.state, RoverState::Fault);
    assert_eq!(msm.drive, DriveOutput::STOP);
}

#[test]
fn test_fault_blocks_explore() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Fault);
    let resp = msm.process(Command::Explore { left: 80, right: 80 });
    assert_eq!(resp, Response::ErrEstop);
    assert_eq!(msm.drive, DriveOutput::STOP);
}

#[test]
fn test_fault_blocks_avoid() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Fault);
    let resp = msm.process(Command::Avoid(AvoidDir::Left));
    assert_eq!(resp, Response::ErrEstop);
}

#[test]
fn test_reset_clears_fault() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Fault);
    msm.process(Command::Reset);
    assert_eq!(msm.state, RoverState::Standby);
    assert_eq!(msm.drive, DriveOutput::STOP);
    assert_eq!(msm.safety, SafetyState::Normal);
}

#[test]
fn test_ping_returns_pong_in_any_state() {
    let mut msm = MasterStateMachine::new();
    assert_eq!(msm.process(Command::Ping), Response::Pong);
    msm.process(Command::Fault);
    assert_eq!(msm.process(Command::Ping), Response::Pong);
}

#[test]
fn test_standby_ack_response() {
    let mut msm = MasterStateMachine::new();
    let resp = msm.process(Command::Standby);
    assert_eq!(resp, Response::Ack(RoverState::Standby));
}

// ─── Watchdog ─────────────────────────────────────────────────────────────────

#[test]
fn test_watchdog_triggers_fault_after_100_ticks() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Explore { left: 80, right: 80 });
    let mut wdog_fired = false;
    for _ in 0..100 {
        if let Some(Response::ErrWatchdog) = msm.tick() {
            wdog_fired = true;
        }
    }
    assert!(wdog_fired, "watchdog debe dispararse");
    assert_eq!(msm.state, RoverState::Fault);
    assert_eq!(msm.drive, DriveOutput::STOP);
}

#[test]
fn test_ping_resets_watchdog() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Explore { left: 80, right: 80 });
    // 50 ticks sin PING
    for _ in 0..50 { msm.tick(); }
    // PING resetea el contador
    msm.process(Command::Ping);
    // 99 ticks más → no debe fallar (99 < 100)
    let mut wdog_fired = false;
    for _ in 0..99 {
        if msm.tick().is_some() { wdog_fired = true; }
    }
    assert!(!wdog_fired, "watchdog NO debe dispararse después de PING");
    assert_eq!(msm.state, RoverState::Explore);
}

#[test]
fn test_watchdog_no_corre_en_fault() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Fault);
    // 200 ticks en FAULT → no debe cambiar estado
    for _ in 0..200 {
        assert_eq!(msm.tick(), None);
    }
    assert_eq!(msm.state, RoverState::Fault);
}

#[test]
fn test_watchdog_no_corre_en_standby() {
    let mut msm = MasterStateMachine::new();
    // 200 ticks en STANDBY → nunca debe ir a FAULT
    for _ in 0..200 {
        assert_eq!(msm.tick(), None);
    }
    assert_eq!(msm.state, RoverState::Standby);
}

// ─── Safety / Stall ───────────────────────────────────────────────────────────

#[test]
fn test_stall_triggers_fault() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Explore { left: 80, right: 80 });
    msm.update_safety(0b000011); // motores 0 y 1 stallados
    assert_eq!(msm.state, RoverState::Fault);
    assert_eq!(msm.safety, SafetyState::FaultStall);
    assert_eq!(msm.drive, DriveOutput::STOP);
}

#[test]
fn test_no_stall_no_fault() {
    let mut msm = MasterStateMachine::new();
    msm.process(Command::Explore { left: 80, right: 80 });
    msm.update_safety(0b000000);
    assert_eq!(msm.state, RoverState::Explore);
}

// ─── Formateador de respuestas ────────────────────────────────────────────────

#[test]
fn test_format_pong() {
    let mut buf = [0u8; 24];
    assert_eq!(format_response(Response::Pong, &mut buf), b"PONG\n");
}

#[test]
fn test_format_ack_standby() {
    let mut buf = [0u8; 24];
    assert_eq!(format_response(Response::Ack(RoverState::Standby), &mut buf), b"ACK:STB\n");
}

#[test]
fn test_format_ack_explore() {
    let mut buf = [0u8; 24];
    assert_eq!(format_response(Response::Ack(RoverState::Explore), &mut buf), b"ACK:EXP\n");
}

#[test]
fn test_format_ack_fault() {
    let mut buf = [0u8; 24];
    assert_eq!(format_response(Response::Ack(RoverState::Fault), &mut buf), b"ACK:FLT\n");
}

#[test]
fn test_format_err_estop() {
    let mut buf = [0u8; 24];
    assert_eq!(format_response(Response::ErrEstop, &mut buf), b"ERR:ESTOP\n");
}

#[test]
fn test_format_err_watchdog() {
    let mut buf = [0u8; 24];
    assert_eq!(format_response(Response::ErrWatchdog, &mut buf), b"ERR:WDOG\n");
}

#[test]
fn test_format_tlm_normal_no_stall() {
    let mut buf = [0u8; 24];
    let resp = Response::Telemetry { safety: SafetyState::Normal, stall_mask: 0 };
    assert_eq!(format_response(resp, &mut buf), b"TLM:NORMAL:000000\n");
}

#[test]
fn test_format_tlm_fault_with_stall() {
    let mut buf = [0u8; 24];
    // motores 1 y 2 stallados: mask = 0b000110
    let resp = Response::Telemetry { safety: SafetyState::FaultStall, stall_mask: 0b000110 };
    assert_eq!(format_response(resp, &mut buf), b"TLM:FAULT:000110\n");
}

#[test]
fn test_format_tlm_warn() {
    let mut buf = [0u8; 24];
    let resp = Response::Telemetry { safety: SafetyState::Warn, stall_mask: 0 };
    assert_eq!(format_response(resp, &mut buf), b"TLM:WARN:000000\n");
}
