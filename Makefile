APP := kact
CARGO := cargo

.PHONY: help build release package run run-release fmt clippy clippy-warn test check coverage coverage-core doc clean install lint docs docs-build docs-preview \
	config-init doctor setup uninstall daemon start status grid elements freestyle deactivate stop quit reload smoke smoke-mouse

help:
	@printf "Available targets:\n"
	@printf "  make build          - Build in debug mode\n"
	@printf "  make release        - Build in release mode\n"
	@printf "  make package        - Build a host release archive in dist/\n"
	@printf "  make run            - Build and run (debug). Pass ARGS='-- --flag' to forward args to the program\n"
	@printf "  make run-release    - Build and run in release mode\n"
	@printf "  make fmt            - Run rustfmt\n"
	@printf "  make clippy         - Run clippy and treat warnings as errors\n"
	@printf "  make clippy-warn    - Run clippy without denying warnings\n"
	@printf "  make test           - Run tests\n"
	@printf "  make check          - Run cargo check\n"
	@printf "  make coverage       - Create and open the HTML coverage report\n"
	@printf "  make coverage-core  - Check pure core coverage\n"
	@printf "  make doc            - Build and open docs\n"
	@printf "  make config-init    - Create default user config if absent\n"
	@printf "  make setup          - Create config, guide permissions, and start Kact\n"
	@printf "  make uninstall      - Remove a release-script installation\n"
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
	@printf "  make clean          - Clean build artifacts\n"
	@printf "  make install        - Install the binary locally (cargo install --path .)\n"
	@printf "  make docs           - Run the documentation site locally\n"
	@printf "  make docs-build     - Build the documentation site\n"
	@printf "  make docs-preview   - Preview the built documentation site\n"

build:
	$(CARGO) build

release:
	$(CARGO) build --release

package:
	scripts/package-release.sh

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

coverage:
	$(CARGO) llvm-cov --locked --all-targets --html --open

coverage-core:
	$(CARGO) llvm-cov --locked --all-targets --json --summary-only --output-path target/coverage.json
	python3 scripts/check_core_coverage.py target/coverage.json

doc:
	$(CARGO) doc --open

clean:
	$(CARGO) clean

install:
	$(CARGO) install --path .

docs:
	pnpm --dir docs run dev

docs-build:
	pnpm --dir docs run build

docs-preview:
	pnpm --dir docs run preview

lint: fmt clippy
	@printf "Ran fmt and clippy\n"

config-init:
	$(CARGO) run -- config init

setup:
	$(CARGO) run -- setup

uninstall:
	$(CARGO) run -- uninstall

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
