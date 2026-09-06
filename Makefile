APP := kact
CARGO := cargo

.PHONY: help build release run run-release fmt clippy clippy-warn test check doc clean install lint \
	config-init doctor daemon start status grid elements freestyle deactivate stop quit reload smoke smoke-mouse bundle

help:
	@printf "Available targets:\n"
	@printf "  make build          - Build in debug mode\n"
	@printf "  make release        - Build in release mode\n"
	@printf "  make run            - Build and run (debug). Pass ARGS='-- --flag' to forward args to the program\n"
	@printf "  make run-release    - Build and run in release mode\n"
	@printf "  make fmt            - Run rustfmt\n"
	@printf "  make clippy         - Run clippy and treat warnings as errors\n"
	@printf "  make clippy-warn    - Run clippy without denying warnings\n"
	@printf "  make test           - Run tests\n"
	@printf "  make check          - Run cargo check\n"
	@printf "  make doc            - Build and open docs\n"
	@printf "  make config-init    - Create default user config if absent\n"
	@printf "  make doctor         - Check config, permission, and service\n"
	@printf "  make daemon         - Run foreground daemon with debug logs\n"
	@printf "  make start          - Start background service\n"
	@printf "  make status         - Show service status\n"
	@printf "  make grid           - Activate grid navigation\n"
	@printf "  make elements       - Activate element navigation\n"
	@printf "  make freestyle      - Activate cursor movement mode\n"
	@printf "  make deactivate     - Hide navigation and release input\n"
	@printf "  make stop           - Stop movement and release buttons\n"
	@printf "  make quit           - Stop background service\n"
	@printf "  make reload         - Reload configuration\n"
	@printf "  make smoke          - Run native overlay/service smoke check\n"
	@printf "  make smoke-mouse    - Run native mouse/keyboard smoke check\n"
	@printf "  make bundle         - Build signed local macOS app bundle\n"
	@printf "  make clean          - Clean build artifacts\n"
	@printf "  make install        - Install the binary locally (cargo install --path .)\n"

build:
	$(CARGO) build

release:
	$(CARGO) build --release

run: build
	@# Forward ARGS to the binary; example: make run ARGS="-- --config custom.toml"
	$(CARGO) run -- $(ARGS)

run-release: release
	$(CARGO) run --release -- $(ARGS)

fmt:
	$(CARGO) fmt

clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

clippy-warn:
	$(CARGO) clippy --all-targets --all-features

test:
	$(CARGO) test

check:
	$(CARGO) check

doc:
	$(CARGO) doc --open

clean:
	$(CARGO) clean

install:
	$(CARGO) install --path .

lint: fmt clippy
	@printf "Ran fmt and clippy\n"

config-init:
	$(CARGO) run -- config init

doctor:
	$(CARGO) run -- doctor

daemon:
	$(CARGO) run -- --log-level debug daemon

start:
	$(CARGO) run -- start

status:
	$(CARGO) run -- status

grid:
	$(CARGO) run -- activate grid

elements:
	$(CARGO) run -- activate elements

freestyle:
	$(CARGO) run -- activate freestyle

deactivate:
	$(CARGO) run -- deactivate

stop:
	$(CARGO) run -- stop

quit:
	$(CARGO) run -- quit

reload:
	$(CARGO) run -- reload

smoke: build
	python3 scripts/smoke_macos.py target/debug/$(APP)

smoke-mouse: build
	python3 scripts/smoke_mouse_macos.py target/debug/$(APP)

bundle:
	python3 scripts/bundle_macos.py
