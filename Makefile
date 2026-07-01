# Olympus LLC — Makefile unificado Rust + Python
# Uso: make <target> [PORT=/dev/ttyUSB0] [SECONDS=5]

PORT    ?= /dev/ttyUSB0
SECONDS ?= 5

# ── Colores ──────────────────────────────────────────────
BOLD  := \033[1m
RESET := \033[0m
OK    := \033[32m✓\033[0m
FAIL  := \033[31m✗\033[0m

.PHONY: help setup test-unit test-rust test-hlc \
        flash flash-20a flash-mixed flash-mixed-no-stall flash-allbts flash-allbts-no-stall \
        flash-allbts-bringup flash-allbts-rampfix flash-allbts-rampfix-prox flash-allbts-nowatchdog flash-allbts-rl-l298n flash-allbts-rl-l298n-bringup \
        flash-test-6motors flash-test-motors-enc-acs flash-test-motors-enc-acs-rl-l298n flash-test-only-rl flash-test-integration-full flash-test-integration-no-ina \
        test-sensors test-protocol test-motors test-motors-main calibrate i2c-scan \
        capture-tlm monitor \
        int-all clean

# ── Ayuda ────────────────────────────────────────────────
help:
	@echo ""
	@printf "$(BOLD)Olympus LLC — comandos disponibles$(RESET)\n"
	@echo ""
	@printf "  $(BOLD)Setup$(RESET)\n"
	@echo "    make setup           Crea entorno uv + instala deps Python"
	@echo ""
	@printf "  $(BOLD)Tests sin hardware$(RESET)\n"
	@echo "    make test-unit       Rust x86 (3 suites) + pytest unit"
	@echo "    make test-rust       Solo suites Rust x86"
	@echo "    make test-hlc        pytest HLC en olympus-hlc-rpi5"
	@echo ""
	@printf "  $(BOLD)Flash$(RESET)\n"
	@echo "    make flash           ACS712-30A (default)"
	@echo "    make flash-20a       ACS712-20A (feature all-20a)"
	@echo "    make flash-no-oc          OC desactivado (pruebas HW sin ACS712)"
	@echo "    make flash-mixed-no-stall mixed-drivers sin stall (prueba 1 motor BTS7960)"
	@echo "    make flash-allbts         6x BTS7960 (feature all-bts7960)"
	@echo "    make flash-allbts-no-stall all-bts7960 sin stall"
	@echo "    make flash-allbts-bringup  6x BTS7960 + no-stall,no-oc (verificar MSM/marcha sin falso OC del ACS RR)"
	@echo "    make flash-allbts-rampfix-prox  rampfix + HC-SR04 en telemetría (proximidad real, sin FAULT por prox)"
	@echo "    make flash-allbts-rl-l298n        5x BTS7960 + RL=L298N (firmware de operación)"
	@echo "    make flash-allbts-rl-l298n-bringup  igual + no-stall,no-oc (verificar comms/sensores)"
	@echo "    make flash-motors-only    Sin OC ni stall (solo motores, sin encoders)"
	@echo "    make flash-test-6motors   Ejemplo HW: 6 motores BTS7960 fwd 40% 3s (sin sensores)"
	@echo "    make flash-test-motors-enc-acs  Ejemplo HW: motores+encoders+ACS712 (CSV serial)"
	@echo ""
	@printf "  $(BOLD)Tests con hardware (PORT=$(PORT))$(RESET)\n"
	@echo "    make i2c-scan        Verifica 0x29/0x40/0x68 en bus I2C"
	@echo "    make test-sensors    INT-04b: 8 sensores en rango TLM"
	@echo "    make test-protocol   INT-05: 13 tests protocolo UART"
	@echo "    make test-motors     INT-07: motores (ejemplo debug_motors_l298n)"
	@echo "    make test-motors-main INT-07: motores firmware principal (EXP mode)"
	@echo "    make calibrate       INT-08: odometría + sensores"
	@echo "    make int-all         INT-04b + INT-05 secuencial"
	@echo ""
	@printf "  $(BOLD)Captura sigrok$(RESET)\n"
	@echo "    make monitor         Monitor serial (screen 115200) — Ctrl+A luego k para salir"
	@echo "    make capture-tlm     FT232H ADBUS0→Mega TX, $(SECONDS)s"
	@echo ""
	@echo "  Ejemplo: make flash PORT=/dev/ttyACM0"
	@echo ""

# ── Setup ────────────────────────────────────────────────
setup:
	uv sync
	@printf "$(OK) Entorno listo. Usa 'uv run <comando>' o 'make <target>'\n"

# ── Tests sin hardware ────────────────────────────────────
test-rust:
	@printf "$(BOLD)>> Rust x86 unit tests$(RESET)\n"
	./test_native.sh

test-unit: test-rust
	@printf "\n$(BOLD)>> pytest (markers: unit)$(RESET)\n"
	uv run pytest tests/ -m unit -v

test-hlc:
	@printf "$(BOLD)>> pytest HLC$(RESET)\n"
	cd ../olympus-hlc-rpi5/layers/meta-olympus/recipes-apps/python3-rover-bridge/files/ && \
	uv run pytest tests/ -v

# ── Flash ─────────────────────────────────────────────────
flash:
	@printf "$(BOLD)>> Flash LLC → $(PORT) (ACS712-30A)$(RESET)\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core

flash-20a:
	@printf "$(BOLD)>> Flash LLC → $(PORT) (ACS712-20A)$(RESET)\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features all-20a

flash-mixed:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [FR/FL=L298N  CR/CL/RR/RL=BTS7960]$(RESET)\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features mixed-drivers

flash-mixed-no-stall:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [mixed-drivers, stall desactivado — prueba 1 motor]$(RESET)\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features mixed-drivers,no-stall

flash-allbts:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [OPERATIVO: 6x BTS7960 + fix FR por DEFAULT + protecciones ON + ntc4, SIN VL53L0X/MPU/LM335]$(RESET)\n"
	@printf "  Sensores ACTIVOS: encoders + ACS712 (stall+OC) + HC-SR04 + TF02 LiDAR + INA3221 + 4xNTC; telemetría TLM siempre.\n"
	@printf "  DESACTIVADOS (no se usan): VL53L0X (no-tof), MPU-6050/EKF (no-mpu), LM335 (no-lm335). Distancia = TF02; sin odometría inercial.\n"
	@printf "  El fix del FR ya es default (de-energiza drivers en reposo). Si un sensor flaky mete FALSE FAULT: usa flash-allbts-rampfix.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features all-bts7960,ntc4,no-tof,no-mpu,no-lm335

flash-allbts-no-stall:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [all-bts7960, stall desactivado — prueba 1 motor]$(RESET)\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features all-bts7960,no-stall

flash-allbts-bringup:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [6x BTS7960 — BRING-UP: stall + OC + HC-SR04 DESACTIVADOS]$(RESET)\n"
	@printf "  Para validar comunicación MSM + marcha sin que el ACS de RR dañado ni el HC-SR04 ruidoso disparen FAULT.\n"
	@printf "  NO usar en operación: sin protección de stall, sobrecorriente ni proximidad ultrasónica.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features all-bts7960,no-stall,no-oc,no-lm335,no-hcsr04,ntc4

flash-allbts-rampfix:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [6x BTS7960 + FIX 'FR gira en FAULT': PWM true-off (disable canal) + ramp-converge]$(RESET)\n"
	@printf "  CAUSA RAIZ (2026-06-02): set_duty(0) en Fast PWM NO es 0V — deja un spike de ~0.4%% que hacia girar al FR.\n"
	@printf "  FIX en bts7960.rs: stop()/brake() DESCONECTAN el comparador (disable -> pin a PORT=LOW=0V real); set_speed reconecta.\n"
	@printf "  Maneja con EXP, deja de mandar PING (~2s) -> FAULT; los 6 motores deben quedar QUIETOS (FR incluido). Sin OC/stall para aislar.\n"
	@printf "  idle-disable: en Fault/Safe/Standby de-energiza los drivers (R_EN/L_EN=LOW, Hi-Z) = el MISMO reposo que el debug_all_motors_bts que NO gira.\n"
	@printf "  no-ntc: ignora termistores de batería (un NTC desconectado lee -20/100C y daba FAULT de sobre-temp falso).\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features all-bts7960,ramp-converge,idle-disable,no-stall,no-oc,no-lm335,no-hcsr04,no-ntc,ntc4

flash-allbts-rampfix-prox:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [rampfix + HC-SR04 EN TELEMETRÍA (proximidad real, f[19]=dist_mm)]$(RESET)\n"
	@printf "  = flash-allbts-rampfix pero con el HC-SR04 ACTIVO leyendo distancia para el TLM.\n"
	@printf "  hcsr04-no-fault: el HC-SR04 NO faultea por proximidad (es ruidoso → FAULT falso); solo telemetría.\n"
	@printf "  La protección de proximidad la decide el HLC (retreat <300mm), gobernada por el selector de seguridad de la GUI.\n"
	@printf "  Mantiene los fixes del FR (ramp-converge, idle-disable) y sin stall/OC/NTC (no calibrados → no FAULTs falsos).\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features all-bts7960,ramp-converge,idle-disable,no-stall,no-oc,no-lm335,no-ntc,ntc4,hcsr04-no-fault

flash-allbts-nowatchdog:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [6x BTS7960 + watchdog comms DESACTIVADO (no-watchdog)]$(RESET)\n"
	@printf "  BISECCION: con esto el rover NO entra en FAULT al perder PING. Maneja con EXP, corta comms.\n"
	@printf "  Si el FR YA NO gira solo => confirmado que la causa es el path de FAULT (rampa congelada), no un subsistema.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features all-bts7960,no-watchdog,no-stall,no-oc,no-lm335,no-hcsr04,ntc4

flash-allbts-rl-l298n:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [5x BTS7960 + RL=L298N (OC_FAULT_L298N=1500 mA)]$(RESET)\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features all-bts7960,rl-l298n

flash-allbts-rl-l298n-bringup:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [5x BTS7960 + RL=L298N — BRING-UP: stall + OC DESACTIVADOS]$(RESET)\n"
	@printf "  Para verificar comunicación MSM y sensores sin que el HW pendiente (RR/CR/CL) dispare FAULT.\n"
	@printf "  NO usar en operación: sin protección de stall ni sobrecorriente.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features all-bts7960,rl-l298n,no-stall,no-oc,no-lm335,ntc4

flash-no-oc:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [OC DESACTIVADO — solo pruebas HW]$(RESET)\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features no-oc

flash-motors-only:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [sin OC, stall ni LM335 — solo motores]$(RESET)\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features no-oc,no-stall,no-lm335,no-hcsr04

flash-test-6motors:
	@printf "$(BOLD)>> Flash test_6_motors_all_bts → $(PORT) [6 BTS7960 fwd 40%% 3s, sin sensores]$(RESET)\n"
	@printf "  ROVER DEBE ESTAR ELEVADO. Los 6 motores giran 3 s y se detienen.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example test_6_motors_all_bts

flash-test-motors-enc-acs:
	@printf "$(BOLD)>> Flash test_motors_encoders_acs → $(PORT) [motores + 6 encoders + 6 ACS712-20A, CSV serial]$(RESET)\n"
	@printf "  ROVER DEBE ESTAR ELEVADO. Captura: 'make monitor PORT=$(PORT) | tee logs/test.csv'\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example test_motors_encoders_acs

flash-test-motors-enc-acs-rl-l298n:
	@printf "$(BOLD)>> Flash test_motors_encoders_acs_rl_l298n → $(PORT) [5xBTS + RL=L298N, OC RL=1500mA]$(RESET)\n"
	@printf "  ROVER DEBE ESTAR ELEVADO. Captura: 'make monitor PORT=$(PORT) | tee logs/test.csv'\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example test_motors_encoders_acs_rl_l298n

flash-test-only-rl:
	@printf "$(BOLD)>> Flash test_only_rl_l298n → $(PORT) [SOLO RL via L298N, sin ruido PWM]$(RESET)\n"
	@printf "  Solo RL se mueve (25%% max, 5s hold). Otros 5 motores no se inicializan.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example test_only_rl_l298n

flash-debug-motors-seq:
	@printf "$(BOLD)>> Flash debug_motors_sequential → $(PORT) [1 motor a la vez, pinout MSM, otros 5 Hi-Z]$(RESET)\n"
	@printf "  ROVER ELEVADO. Prueba FR,FL,CR,CL,RR,RL en secuencia (30%%, HOLD 5s c/u). Mide los pines en cada HOLD.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_motors_sequential

flash-debug-only-fr:
	@printf "$(BOLD)>> Flash debug_only_fr → $(PORT) [SOLO FR, adelante 3s y para, máximo aislamiento]$(RESET)\n"
	@printf "  ROVER ELEVADO. Solo la FR se inicializa. Gira adelante 3s y para. Mide RPWM=D9 LPWM=D44.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_fr

flash-debug-only-fr-fl:
	@printf "$(BOLD)>> Flash debug_only_fr_fl → $(PORT) [SOLO FR+FL, adelante 3s y paran]$(RESET)\n"
	@printf "  ROVER ELEVADO. Solo FR (D9/D44) y FL (D10/D45) se inicializan. Ambas adelante 3s y paran.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_fr_fl

flash-debug-only-fl:
	@printf "$(BOLD)>> Flash debug_only_fl → $(PORT) [SOLO FL, HOLD 15s para medir pines]$(RESET)\n"
	@printf "  ROVER ELEVADO. Solo la FL se inicializa. Adelante 30%% con HOLD 15s. Mide B+, D22, D24, D10, D45, M+/M-.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_fl

flash-debug-only-cr:
	@printf "$(BOLD)>> Flash debug_only_cr → $(PORT) [SOLO CR, adelante 3s y para]$(RESET)\n"
	@printf "  ROVER ELEVADO. Solo la CR se inicializa. Gira adelante 3s y para. RPWM=D5 LPWM=D11.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_cr

flash-debug-only-cl:
	@printf "$(BOLD)>> Flash debug_only_cl → $(PORT) [SOLO CL, adelante 3s y para]$(RESET)\n"
	@printf "  ROVER ELEVADO. Solo la CL se inicializa. Gira adelante 3s y para. RPWM=D6 LPWM=D11 (pinout real).\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_cl

flash-debug-only-cr-cl:
	@printf "$(BOLD)>> Flash debug_only_cr_cl → $(PORT) [SOLO CR+CL, adelante 3s y paran, pinout real]$(RESET)\n"
	@printf "  ROVER ELEVADO. Solo CR (D5/D12/D30/D31) y CL (D6/D11/D28/D29). Ambas adelante 3s y paran.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_cr_cl

flash-debug-only-rr:
	@printf "$(BOLD)>> Flash debug_only_rr → $(PORT) [SOLO RR, HOLD 15s para medir pines]$(RESET)\n"
	@printf "  ROVER ELEVADO. Solo la RR se inicializa. Adelante 30%% con HOLD 15s. Mide B+, D34, D35, D7, D13, M+/M-.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_rr

flash-debug-only-rl:
	@printf "$(BOLD)>> Flash debug_only_rl → $(PORT) [SOLO RL via L298N, HOLD 15s para medir]$(RESET)\n"
	@printf "  ROVER ELEVADO. Solo la RL (L298N) se inicializa. Adelante 30%% con HOLD 15s. Mide B+, D8, D36, D37, M+/M-.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_rl

flash-debug-only-rl-bts:
	@printf "$(BOLD)>> Flash debug_only_rl_bts → $(PORT) [SOLO RL via BTS7960, HOLD 15s]$(RESET)\n"
	@printf "  ROVER ELEVADO. RL con módulo BTS7960 (RPWM=D8 LPWM=D4 R_EN=D36 L_EN=D37). Verifica si el BTS RL está sano.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_rl_bts

flash-debug-only-rr-rl-bts:
	@printf "$(BOLD)>> Flash debug_only_rr_rl_bts → $(PORT) [SOLO RR+RL ambos BTS7960, adelante 3s]$(RESET)\n"
	@printf "  ROVER ELEVADO. RR (D7/D13/D34/D35) + RL BTS (D8/D4/D36/D37). Ambas adelante 3s y paran.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_rr_rl_bts

flash-debug-only-rr-rl:
	@printf "$(BOLD)>> Flash debug_only_rr_rl → $(PORT) [SOLO RR(BTS)+RL(L298N), adelante 3s y paran]$(RESET)\n"
	@printf "  ROVER ELEVADO. Solo RR (D7/D13/D34/D35) y RL (D8/D36/D37). Ambas adelante 3s y paran.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_rr_rl

flash-debug-all:
	@printf "$(BOLD)>> Flash debug_all_motors → $(PORT) [LOS 6 motores, adelante 3s y paran, pinout+inverts reales]$(RESET)\n"
	@printf "  ROVER ELEVADO. Las 6 ruedas adelante 3s, mismo sentido, y paran. Sin MSM/watchdog.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_all_motors

flash-debug-fl-allint:
	@printf "$(BOLD)>> Flash debug_fl_allint → $(PORT) [mueve SOLO FL, 6 INT habilitados, CSV fl+fr]$(RESET)\n"
	@printf "  ROVER ELEVADO. Decide si FL=0 es motor o routing de interrupts (D20/D21).\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_fl_allint

flash-ina-logger:
	@printf "$(BOLD)>> Flash ina3221_battery_logger → $(PORT) [logger dedicado curva de descarga V, 3 bancos]$(RESET)\n"
	@printf "  Mega standalone. Captura en PC: cat $(PORT) | tee logs/descarga_baterias.csv\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example ina3221_battery_logger

flash-debug-hcsr04-d38:
	@printf "$(BOLD)>> Flash debug_hcsr04_d38 → $(PORT) [HC-SR04 D38/D39 rango completo, loop continuo]$(RESET)\n"
	@printf "  Mueve la mano frente al sensor: dist_mm debe seguirla. No mueve motores.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_hcsr04_d38

flash-medir-bateria:
	@printf "$(BOLD)>> Flash medir_bateria → $(PORT) [BANCO DE MEDICIÓN: 2º Mega, voltaje de batería]$(RESET)\n"
	@printf "  SOLO Mega: lee divisor en A0 (factor 3.7). Emite CSV 9600 'tiempo_s,voltaje_V' → capturar_bateria.py.\n"
	@printf "  Cableado: pot 10k como divisor (extremos a V_bat y GND, wiper a A0). Calibrar con 12V hasta leer 12.0.\n"
	@printf "  USAR EN UN MEGA DISTINTO AL LLC (no toca motores). A0 nunca >5V (batería siempre por el divisor).\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example medir_bateria

flash-medir-bateria-raw:
	@printf "$(BOLD)>> Flash medir_bateria_raw → $(PORT) [2º Mega, ADC crudo, factor en Python]$(RESET)\n"
	@printf "  Emite ADC crudo (0-1023), el factor se pasa a capturar_bateria.py --factor.\n"
	@printf "  Cableado: pot 10k como divisor (extremos a V_bat y GND, wiper a A0/A1).\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example medir_bateria_raw

flash-debug-fr-enc:
	@printf "$(BOLD)>> Flash debug_only_fr_enc → $(PORT) [FR vs FL mismo duty, ratio de conteo encoder]$(RESET)\n"
	@printf "  ROVER ELEVADO. ratio_x100~100=ok, ~200=FR sobre-cuenta (flancos espurios fase A).\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_fr_enc

flash-debug-fl-bisect:
	@printf "$(BOLD)>> Flash debug_fl_bisect → $(PORT) [FL continuo, desenmascara INT uno a uno, CSV fl_delta]$(RESET)\n"
	@printf "  ROVER ELEVADO. 1 solo flasheo: la 1a fase con fl_delta~0 senala el INT que rompe el conteo.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_fl_bisect

flash-debug-only-fl-enc:
	@printf "$(BOLD)>> Flash debug_only_fl_enc → $(PORT) [SOLO FL motor + su encoder, CSV ticks]$(RESET)\n"
	@printf "  ROVER ELEVADO. Mueve solo FL 30%% 8s e imprime ticks_fl. Aísla motor vs encoder.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_only_fl_enc

flash-debug-encoders-drive:
	@printf "$(BOLD)>> Flash debug_encoders_drive → $(PORT) [6 motores 30%% + 6 encoders, CSV ticks]$(RESET)\n"
	@printf "  ROVER ELEVADO. Mueve los 6 a 30%% 8s e imprime ticks. Para verificar el encoder FL.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_encoders_drive

flash-debug-rr-acs:
	@printf "$(BOLD)>> Flash debug_rr_acs → $(PORT) [SOLO RR motor + ACS712 A4, diagnóstico sensor]$(RESET)\n"
	@printf "  ROVER ELEVADO. RR a 50%% 10s. Imprime ADC crudo + mA para ver si el ACS RR es basura u offset.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_rr_acs

flash-debug-encoders-acs:
	@printf "$(BOLD)>> Flash debug_encoders_acs → $(PORT) [6 BTS + 6 encoders + 6 ACS712, config real, CSV]$(RESET)\n"
	@printf "  ROVER ELEVADO. Captura: 'make monitor PORT=$(PORT) | tee logs/2026-05-31_enc_acs.csv'\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_encoders_acs

flash-debug-all-bts:
	@printf "$(BOLD)>> Flash debug_all_motors_bts → $(PORT) [LOS 6 TODOS BTS7960, adelante 3s y paran]$(RESET)\n"
	@printf "  ROVER ELEVADO. Las 6 ruedas (RL como BTS) adelante 3s, mismo sentido, y paran. Sin MSM/watchdog.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_all_motors_bts

flash-debug-front-center:
	@printf "$(BOLD)>> Flash debug_front_center → $(PORT) [FR+FL+CR+CL, adelante 3s y paran, pinout real]$(RESET)\n"
	@printf "  ROVER ELEVADO. 4 ruedas (frente+centro) adelante 3s, mismo sentido, y paran. RR/RL no se tocan.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example debug_front_center

flash-test-integration-full:
	@printf "$(BOLD)>> Flash test_integration_full → $(PORT) [motores+encoders+ACS+HC-SR04+TF02+INA3221+4NTC]$(RESET)\n"
	@printf "  Validación integral previa al MSM. Cableado idéntico al PDF cableado_rover_olympus_all_bts.\n"
	@printf "  Pinout: HC-SR04 D38/D39 · TF02 USART2 D17 · INA3221 0x40 SDA=D42 SCL=D43 · NTC A7..A10 · resto = test anterior.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example test_integration_full

flash-test-integration-no-ina:
	@printf "$(BOLD)>> Flash test_integration_no_ina → $(PORT) [igual que -full SIN INA3221]$(RESET)\n"
	@printf "  Para usar mientras el INA3221 no está cableado o se diagnostica el soft I2C.\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --example test_integration_no_ina

monitor:
	@printf "$(BOLD)>> Monitor serial → $(PORT) @ 115200$(RESET)\n"
	@printf "  Salir: Ctrl+A luego k\n"
	screen $(PORT) 115200

# ── Tests con hardware ────────────────────────────────────
i2c-scan:
	@printf "$(BOLD)>> I2C scan → $(PORT)$(RESET)\n"
	uv run python tests/hardware/i2c_scan.py $(PORT)

test-tf02:
	@printf "$(BOLD)>> TF02 LiDAR UART → $(PORT)$(RESET)\n"
	uv run python tests/hardware/test_tf02.py $(PORT)

test-sensors:
	@printf "$(BOLD)>> INT-04b: sensores individuales → $(PORT)$(RESET)\n"
	uv run python tests/hardware/test_sensors_individual.py $(PORT)

test-protocol:
	@printf "$(BOLD)>> INT-05: protocolo UART (13 tests) → $(PORT)$(RESET)\n"
	uv run python tests/hardware/test_msm_protocol.py $(PORT)

test-motors:
	@printf "$(BOLD)>> INT-07: motores interactivo → $(PORT)$(RESET)\n"
	uv run python tests/hardware/test_motors_debug.py $(PORT)

test-motors-main:
	@printf "$(BOLD)>> INT-07: motores firmware principal → $(PORT) [no-oc]$(RESET)\n"
	uv run python tests/hardware/test_motors_main.py $(PORT)

calibrate:
	@printf "$(BOLD)>> INT-08: calibración → $(PORT)$(RESET)\n"
	uv run python tests/hardware/calibrate_odometry.py $(PORT)

int-all: test-sensors test-protocol
	@printf "\n$(OK) $(BOLD)INT-04b + INT-05 completados$(RESET)\n"

# ── Captura sigrok ────────────────────────────────────────
capture-tlm:
	@printf "$(BOLD)>> Captura TLM FT232H ADBUS0, $(SECONDS)s @ 1 MHz$(RESET)\n"
	@SAMPLES=$$((1000000 * $(SECONDS))); \
	sigrok-cli \
	  -d ftdi-la \
	  --config samplerate=1000000 \
	  --samples $$SAMPLES \
	  -P uart:rx=ADBUS0:baudrate=115200:parity=none:stopbits=1 \
	  --pd-annotations uart=rx 2>/dev/null \
	| grep -oP 'TLM:\S+' \
	| while IFS=: read -r _ mode stall tick bmv bma i0 i1 i2 i3 i4 i5 temp bt0 bt1 bt2 bt3 bt4 bt5 dist el er x y th; do \
	    echo "---"; \
	    echo "mode=$$mode  stall=$$stall  tick=$$tick"; \
	    echo "bat=$${bmv}  $${bma}"; \
	    echo "currents=[$$i0,$$i1,$$i2,$$i3,$$i4,$$i5]"; \
	    echo "temp=$${temp}  dist=$${dist}"; \
	    echo "enc=L:$$el R:$$er  pose=($$x,$$y,$${th})"; \
	  done

# ── Limpieza ──────────────────────────────────────────────
clean:
	cargo clean
	rm -rf .venv __pycache__ tests/__pycache__ tests/hardware/__pycache__
	@printf "$(OK) Limpio\n"
