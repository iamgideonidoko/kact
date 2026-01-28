APP := kact
CARGO := cargo

.PHONY: help build release run run-release fmt clippy clippy-warn test check doc clean install lint

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
	@printf "  make clean          - Clean build artifacts
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
