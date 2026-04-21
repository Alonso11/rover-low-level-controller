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
        flash flash-20a \
        test-sensors test-protocol test-motors calibrate i2c-scan \
        capture-tlm \
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
	@echo "    make flash-no-oc     OC desactivado (pruebas HW sin ACS712)"
	@echo ""
	@printf "  $(BOLD)Tests con hardware (PORT=$(PORT))$(RESET)\n"
	@echo "    make i2c-scan        Verifica 0x29/0x40/0x68 en bus I2C"
	@echo "    make test-sensors    INT-04b: 8 sensores en rango TLM"
	@echo "    make test-protocol   INT-05: 13 tests protocolo UART"
	@echo "    make test-motors     INT-07: motores interactivo"
	@echo "    make calibrate       INT-08: odometría + sensores"
	@echo "    make int-all         INT-04b + INT-05 secuencial"
	@echo ""
	@printf "  $(BOLD)Captura sigrok$(RESET)\n"
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

flash-no-oc:
	@printf "$(BOLD)>> Flash LLC → $(PORT) [OC DESACTIVADO — solo pruebas HW]$(RESET)\n"
	RAVEDUDE_PORT=$(PORT) \
	RUSTFLAGS="-C target-cpu=atmega2560" \
	cargo +nightly run --release \
	  -Zjson-target-spec \
	  -Zbuild-std=core \
	  --features no-oc

# ── Tests con hardware ────────────────────────────────────
i2c-scan:
	@printf "$(BOLD)>> I2C scan → $(PORT)$(RESET)\n"
	uv run python tests/hardware/i2c_scan.py $(PORT)

test-sensors:
	@printf "$(BOLD)>> INT-04b: sensores individuales → $(PORT)$(RESET)\n"
	uv run python tests/hardware/test_sensors_individual.py $(PORT)

test-protocol:
	@printf "$(BOLD)>> INT-05: protocolo UART (13 tests) → $(PORT)$(RESET)\n"
	uv run python tests/hardware/test_msm_protocol.py $(PORT)

test-motors:
	@printf "$(BOLD)>> INT-07: motores interactivo → $(PORT)$(RESET)\n"
	uv run python tests/hardware/test_motors_debug.py $(PORT)

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
