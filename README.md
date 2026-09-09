# Kact

Keyboard-driven cursor ACTuator. Written in Rust, with native macOS overlays and a local command service.

- Grid labels and accessibility element labels jump directly to targets.
- Freestyle mode provides movement without an overlay.
- Click, double/triple-click, drag, scroll, and jump to screen edges or corners.
- Multiple displays, configurable appearance, and vi/Emacs controls.
- TOML configuration with validation and automatic reload.
- Global shortcuts are **disabled by default**. External shortcut tools can invoke every action through the CLI.

macOS is the primary platform. Linux X11 supports shell-driven mouse actions; native overlays, element discovery, keyboard capture, and modified clicks are not available there. Native Wayland and Windows are unsupported.

## Install

Published binaries are installed from GitHub Releases:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/iamgideonidoko/kact/main/scripts/install.sh | bash # -- --version vX.Y.Z
kact setup
```

The command installs the latest release. Remove `#` and replace `vX.Y.Z` with a tag to select a specific release. The installer selects the macOS Apple Silicon, macOS Intel, or Linux x86_64 archive, verifies its SHA-256 checksum before installing, and writes only to `~/.local/bin` by default. Current macOS releases are unsigned and unnotarized, so macOS may ask for confirmation before the first run. See the [installation guide](https://iamgideonidoko.github.io/kact/getting-started/installation/) for source builds, updates, and supported platforms.

## Build from source

Install the pinned Rust 1.88.0 toolchain:

```sh
mise install
cargo install --locked --path .
kact setup
kact activate grid
```

`kact setup` creates the default configuration, opens macOS Accessibility settings when needed, and starts the daemon. After granting permission, rerun `kact setup`. If keyboard capture is denied, check Input Monitoring too. `kact doctor` checks configuration, permission, and service availability without moving the cursor.

`start` creates a default user config if needed and launches one background instance. It is safe to repeat. Logs are saved beside the config as `kact.log`. Use `kact daemon` for foreground logs or `kact quit` to stop.

```sh
kact service install      # enable startup at the next login
kact service uninstall    # remove login startup and unload the managed service
kact update               # install the latest verified release and restart Kact
kact update --check       # report whether a release update is available
kact uninstall            # remove a release-script installation; keep config and logs
kact uninstall --purge    # also remove config, logs, and socket
```

Linux X11 builds need `libx11-dev` and `libxtst-dev`. Set `keybindings.navigation_enabled = false` when using freestyle commands there.

## Commands

```sh
kact activate grid         # labeled cells on every screen
kact activate elements     # targets in the focused window; grid fallback
kact activate freestyle    # movement controls without an overlay
kact toggle grid
kact deactivate
kact cancel                # clear a label prefix, otherwise deactivate

kact move --dx 20 --dy -10
kact move-to --x 800 --y 400
kact move --dx 200 --dy 0 --glide
kact move-to --x 800 --y 400 --glide
kact move-start right --speed fast # temporary normal | precise | fast override
kact move-stop right
kact speed precise        # normal | precise | fast
kact stop                  # stop motion and release held mouse buttons
kact jump center          # top | bottom | left | right | center | cycle

kact click
kact click --button right
kact click --count 2 --modifiers cmd
kact button-down           # begin dragging
kact move --dx 100
kact button-up             # release the drag
kact scroll --dy 80        # positive: up; negative: down
kact scroll --dx -80       # positive: left; negative: right

kact select aa             # enter a displayed label through the CLI
kact backspace
kact refine                # subdivide the last selected grid cell
kact show grid-lines       # toggle grid lines
kact show labels           # toggle labels
kact show larger           # larger cells; also: smaller
kact show more-contrast    # also: less-contrast
kact reload
kact doctor
kact status
kact quit
```

Clicking exits navigation. Clicking a held button drops it without an extra click. Deactivation, shutdown, and fatal input failures release held buttons. `stop` leaves an existing navigation overlay active; `deactivate` also hides it.

Actions sent to the running daemon return JSON and a nonzero exit status on failure. Setup and configuration commands print human-readable results. Continuous movement commands do not activate keyboard capture. Send press/release commands in order; the Hammerspoon example serializes them. `--glide` animates a one-shot move to a target captured at command receipt; a new pointer action, `move-start`, `stop`, deactivation, reload, or quit cancels it. `scroll` uses pixels on macOS and wheel steps on X11. Coordinates use screen points on macOS, with `(0, 0)` at the primary display's top-left; other displays may have negative coordinates.

All commands accept `--config PATH` and `--socket PATH`; these identify the daemon's configuration at startup and its private socket respectively. Use the same `--socket` for clients of a custom instance. `--config` on a client does not switch a running daemon's config. Socket parent directories must be owned by you with mode `0700`.

## Keyboard behavior

Normal typing is untouched while navigation is inactive. Explicit activation enables local navigation controls by default; only handled keys are consumed.

| Key                       | Action                                    |
| ------------------------- | ----------------------------------------- |
| Displayed label           | Jump to the target                        |
| Escape / Cmd+. / Ctrl+G   | Clear prefix, then exit on the next press |
| Backspace                 | Remove one prefix character               |
| Cmd+H                     | Hide navigation                           |
| Arrows / Alt+arrows       | Move 10 / 100 points                      |
| Cmd+arrows                | Jump to screen edges                      |
| Enter                     | Click or drop a drag                      |
| `=` / `\`                 | Begin drag / double-click                 |
| `[` / `]`                 | Middle / right click                      |
| Shift+arrows              | Scroll                                    |
| Modifier+Enter            | Click with those modifiers                |
| Ctrl+= / Ctrl+Shift+=     | Toggle grid lines / labels                |
| Cmd+Shift+= / Cmd+Shift+- | Increase / decrease cell size             |
| Cmd+= / Cmd+-             | Increase / decrease contrast              |

The default `keybindings.preset = "vi"` uses H/J/K/L movement, Ctrl+H/J/K/L larger steps, Shift+H/J/K/L edges, Shift+M center/corners, and Ctrl+B/F/I/A scrolling. Set `keybindings.preset = "emacs"` for Ctrl+P/N/B/F movement, Alt+A/E/B/F larger steps, Ctrl+A/E edges, Ctrl+L center/corners, and Shift+P/N/B/F scrolling. Keys reserved by local bindings are removed from the label alphabet so targets remain reachable.

## Configuration

```sh
kact config path
kact config init           # creates defaults; never overwrites an existing file
kact config check
```

Default: `$XDG_CONFIG_HOME/kact/kact.toml`, otherwise `~/.config/kact/kact.toml`. Omitted fields use defaults. Unknown fields, invalid values, conflicting aliases, and invalid binding actions are rejected. See [kact.toml](kact.toml) for the full schema.

Optional global shortcuts:

```toml
[keybindings]
enabled = true
navigation_enabled = true
preset = "vi"

[keybindings.global]
"ctrl+alt+g" = "activate grid"
"ctrl+alt+e" = "activate elements"
"ctrl+alt+f" = "toggle freestyle"

[keybindings.local]
"tab" = "refine"
```

Global shortcuts require modifiers. Binding values are Kact actions, never shell programs. Custom local bindings override the selected preset. For completely command-only use:

```toml
[keybindings]
enabled = false
navigation_enabled = false
```

Movement settings control speed, acceleration, friction, frame rate, and normal/precise/fast multipliers. `motion.curve_type` controls acceleration toward target speed: `linear`, `sigmoid` (the default), or `exponential`. Releasing movement always uses exponential friction. `[glide]` controls opt-in one-shot animation separately, with a duration and easing curve. Navigation settings control rows, columns, label alphabet, and optional automatic clicking after selection. Optional `[navigation.grid]`, `[navigation.elements]`, `[appearance.grid]`, and `[appearance.elements]` tables override only their specified fields; all other values inherit from the global navigation or appearance settings. Appearance settings control font size, foreground/background/highlight colors, opacity, grid lines, and label position.

Valid reloads apply atomically and exit navigation to clear held input. Invalid reloads retain the previous configuration. Atomic editor saves are supported. Logging changes reload too, unless overridden by `--log-level`. When automatic reload is disabled, use `kact reload` to apply changes.

## External integrations

Run `kact setup` once, then let your shortcut tool invoke the CLI:

- [Hammerspoon](examples/hammerspoon.lua): activation shortcuts and ordered continuous-movement press/release commands using [hs.task](https://www.hammerspoon.org/docs/hs.task.html).
- [Karabiner-Elements](examples/karabiner.json): import a complex modification using [shell commands](https://karabiner-elements.pqrs.org/docs/json/complex-modifications-manipulator-definition/to/shell-command/).
- [Kanata](examples/kanata.kbd): adapt the binary path and merge the bindings. Requires a build with [command support](https://jtroo.github.io/config.html#_cmd) and `danger-enable-cmd yes`.

Use an absolute executable path; GUI tools may have a different `PATH`. Run commands as the desktop user. Keep Kact's global shortcuts disabled when another tool owns them. Templates have not been exercised inside those third-party apps.

## Development and release checks

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --locked --release
python3 scripts/smoke_macos.py target/debug/kact
python3 scripts/smoke_mouse_macos.py target/debug/kact
```

The first native smoke check briefly shows overlays and starts keyboard capture without moving or clicking. The second opens a temporary test window, checks real mouse actions and keyboard suppression, then restores the cursor. Both require a macOS desktop and Accessibility permission; the second also requires the Xcode command-line tools.

`src/core/` contains pure motion and label logic. `src/platform/` handles input and mouse injection. `src/desktop/` owns main-thread AppKit overlays and Accessibility discovery. `src/runtime/` applies shared actions from the CLI and optional bindings. `src/ipc.rs` provides a private, bounded Unix socket transport with single-instance locking.

Automated tests cover config validation, movement timing, labels, state transitions, drag release, shortcut suppression, IPC framing/locking, and atomic reload. Native smoke checks cover grid/element presentation, keyboard suppression/passthrough, real clicks/drag/scroll, reload, and clean shutdown. CI is configured for macOS and Linux; Linux desktop behavior still needs verification.

## Roadmap

- Validate real pointer workflows, permissions, sleep/wake, keyboard layouts, displays, remappers, and long-running macOS sessions.
- Measure and publish latency and idle/active resource use.
- Add Developer ID signing and notarization for macOS releases.
- Replace legacy Cocoa bindings before their transitive `block` dependency becomes incompatible with Rust.
- Add `kact config init` presets and generate starter integrations for supported remappers and shells.
- Add window-relative moves and glides for focused-window centers, edges, corners, and fractional positions.
- Add held scrolling with the same normal, precise, and fast speed modes as held pointer movement.
- Improve element targeting with role filters, minimum target sizes, deduplication, app-specific exclusions, and optional accessible names.
- Extend `kact status` and `kact doctor` with permissions, enabled bindings, display details, and supported capabilities.
- Improve Linux support through explicit, testable capabilities rather than a blanket platform claim.
  - Harden X11 behavior across desktop environments and window managers; report the active session, display, XTest, and available capabilities through `kact doctor`.
  - Add AT-SPI2 accessibility discovery for element targeting where applications expose accessible UI information.
  - Add compositor-specific Wayland profiles, beginning with wlroots-based desktops such as Hyprland, for supported pointer actions.
  - Support opt-in desktop global shortcuts through the XDG portal while keeping command and remapper integrations first-class.
  - Add overlays and advanced input actions only where a compositor supplies a supported native path.
- Add a dedicated native Windows implementation through explicit, testable capabilities.
  - Add pointer movement, clicks, dragging, scrolling, and modified clicks; account for virtual-desktop coordinates, display scaling, and Windows integrity levels.
  - Report the Windows session and available capabilities through `kact doctor`.
  - Support opt-in modifier-based global shortcuts, then active-mode keyboard capture where it is reliable.
  - Add per-display, click-through overlays for grid and element labels.
  - Add UI Automation element targeting where applications expose accessible UI information.
  - Ship Windows x86_64 release archives, a PowerShell installer, per-user startup support, and code signing.
- Add per-app profiles, macros, and a graphical configuration editor.

## Project documentation

- [Contributing](CONTRIBUTING.md), [security policy](SECURITY.md), and [changelog](CHANGELOG.md)
- [Documentation site](https://iamgideonidoko.github.io/kact/) for installation, configuration, CLI reference, troubleshooting, and contributor guides
- [Documentation source](docs/) for local preview and documentation changes

## License

[MIT](LICENSE).
