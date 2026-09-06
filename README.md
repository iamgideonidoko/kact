# Kact

A keyboard-driven cursor controller written in Rust. The goal is fast cursor navigation through configurable shortcuts, labeled targets, and shell commands.

## Status

Early prototype. Directional movement, three speed modes, TOML loading, and partial configuration reload are implemented. macOS Core Graphics and Linux X11 backends exist; neither is production-ready. Native Wayland and Windows are not implemented.

Grid navigation, UI element targeting, clicking, dragging, scrolling, and command control of a running instance are planned.

## Run

Install a Rust toolchain supporting edition 2024. On Linux, install X11 development libraries first:

```sh
sudo apt-get install libx11-dev libxtst-dev
```

Build and run from the repository:

```sh
cargo build --release
./target/release/kact --config kact.toml
```

On macOS, enable Accessibility permission for the app or terminal running Kact in System Settings → Privacy & Security → Accessibility, then restart. If input capture fails, inspect the logs. On Linux, use an X11 session with a valid `DISPLAY`; system-wide control under Wayland is not supported.

Available options:

```sh
kact --config /path/to/kact.toml
kact --log-level debug
kact --generate-config --config /path/to/new-config.toml
```

Use the built binary path unless Kact is installed on `PATH`. Config generation overwrites the destination. Optional local installation: `cargo install --path .`.

### Current controls

Kact starts inactive.

| Key | Action |
| --- | --- |
| Space | Toggle cursor control |
| W/A/S/D | Move up/left/down/right |
| 1 / 2 / 3 | Normal / precise / fast speed |
| Escape | Trigger emergency stop |

These keys are currently hardcoded and monitored globally. On macOS, they also reach the focused app; holding Space can toggle repeatedly. Escape stops movement, but clean process shutdown is incomplete. Ctrl+C terminates the process without coordinated cleanup.

## Configuration

The default path is `kact.toml` in the working directory. See the [complete sample](kact.toml) for all required fields; partial configs are not supported yet. Missing or malformed files fall back to defaults.

```toml
[motion]
curve_type = "sigmoid" # sigmoid | exponential | linear
max_speed = 2000.0    # pixels per second before the mode multiplier
acceleration = 0.8    # response factor, intended range 0–1
friction = 0.95       # velocity retained per tick without input
target_fps = 144
```

Edit the existing `[motion]` section rather than replacing the whole file with this excerpt. Higher acceleration responds faster; higher friction retains more movement after release. Lower `max_speed` for slower movement or `target_fps` for fewer updates.

Current limitations:

- `[keybindings]` and `[modes]` are parsed but ignored; speed multipliers remain 1.0, 0.3, and 2.5.
- Logging uses `--log-level`, not `[system].log_level`.
- `[system].hot_reload` enables watching at startup. Motion settings reload, except `target_fps`, which requires a restart. Changing the watcher setting also requires a restart.
- Values are not validated. Keep `target_fps` positive, speed finite and positive, and acceleration/friction within 0–1.
- Invalid reloads retain the previous config. File replacement by editors may disrupt watching.

## Implementation

The core separates input state and motion calculations from OS calls:

```text
OS keyboard events -> input state -> motion tick -> OS cursor movement
```

- `src/core/`: direction vectors, activation/speed state, and pure velocity/position calculations.
- `src/platform/`: input and cursor traits, macOS event taps, and Linux XRecord/XTest integration.
- `src/runtime/`: input forwarding, motion loop, configuration watching, and coordination.
- `src/config.rs`: TOML schema and loading; `src/main.rs`: CLI and startup.

Bounded channels carry input and control messages; shared state uses a mutex. Platform input capture and configuration watching add worker threads beyond the input and motion threads.

The motion engine normalizes directional input and interpolates toward a target velocity. Curves currently operate on normalized input magnitude, not elapsed hold time; friction depends on tick rate. Worker failures and shutdown need coordinated handling. Latency, CPU, and memory targets have not been established by benchmarks.

## Roadmap

The finished product should let a user activate Kact from a shortcut or command, select a screen location or UI element, move/click/drag/scroll, and return to normal typing. Configuration should cover bindings, movement, navigation, and overlay appearance without rebuilding.

### 1. Reliable macOS foundation

- [ ] Use a deliberate activation shortcut; suppress handled keys only while active and ignore repeat events for toggles.
- [ ] Make cancellation immediate; clear held input on exit and recover safely from dropped events or disabled event taps.
- [ ] Propagate permission and worker failures; stop and join workers on Escape, Ctrl+C, and SIGTERM.
- [ ] Make movement timing consistent across frame rates and verify cursor coordinates across displays.

Done when activation, movement, cancellation, and shutdown work without interfering with ordinary typing.

### 2. Configuration and command control

- [ ] Apply configured bindings and speed multipliers; support modifier combinations and reject conflicting bindings.
- [ ] Add defaults for omitted fields, numeric validation, unknown-field errors, and useful error messages.
- [ ] Use a predictable user config location with `--config` override; reload valid changes atomically and retain the last valid config on failure.
- [ ] Run one background instance with a local control socket; route shortcuts and CLI requests through the same actions.
- [ ] Add commands for activation, deactivation, toggling, status, and reload; report when no instance is running.

Done when users can configure Kact and control it from a shell or external shortcut manager without launching duplicate instances.

### 3. Complete cursor workflow

- [ ] Add a labeled grid overlay with prefix selection, cancellation, and refinement for precise targets.
- [ ] Support left/right/middle click, double-click, drag/drop, and horizontal/vertical scrolling; release held buttons on cancellation or shutdown.
- [ ] Support multiple displays, display scaling, Spaces, and fullscreen apps.
- [ ] Show active mode and configurable, readable labels without stealing the destination app's focus unnecessarily.

Done when a user can select a target, perform a mouse action, and resume typing entirely from the keyboard.

### 4. UI element navigation

- [ ] Discover actionable elements in the focused window through macOS Accessibility APIs.
- [ ] Label targets, filter by typed prefix, and activate or move to the selection.
- [ ] Handle slow or inaccessible apps with cancellation and a grid fallback.

Done when element navigation and grid navigation share consistent controls and work together reliably.

### 5. Release readiness

- [ ] Cover config/reload, input transitions, motion timing, command control, and shutdown with regression tests.
- [ ] Verify real macOS workflows, including permission denial, multiple displays, and fullscreen apps.
- [ ] Add CI, measure idle/active resource use and end-to-end latency, and publish results.
- [ ] Package signed/notarized macOS releases with installation, startup, upgrade, and uninstall instructions.
- [ ] Validate and harden Linux X11 support before claiming support; scope native Wayland and Windows separately.

Per-app profiles, macros, and a configuration GUI come after the core workflow is reliable.

## Development

```sh
cargo test
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

Existing tests cover default config values, basic motion, and vector operations. They do not verify OS input capture or cursor interaction.

## License

[MIT](LICENSE).
