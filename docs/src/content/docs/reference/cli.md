---
title: CLI reference
description: Commands, arguments, and global options.
---

All commands accept `--config PATH`, `--socket PATH`, and `--log-level LEVEL`. `--config` selects the file at daemon startup; passing it to a client does not reconfigure a daemon that is already running. `--socket` selects the private control socket.

Run `kact COMMAND --help` for Clap’s argument help, for example `kact click --help`. Action commands require a running daemon; run `kact start` first.

## Service and configuration

| Command | Purpose |
| --- | --- |
| `start` | Start the background daemon; safe to repeat. |
| `daemon` | Run the daemon in the foreground. |
| `service install` / `uninstall` | Configure macOS login startup. |
| `status` | Return daemon and navigation state. |
| `quit` | Stop the daemon and release held buttons. |
| `doctor` | Check config, Accessibility, and service availability without pointer movement. |
| `config init` | Create the default configuration without overwriting one. |
| `config check` | Validate the configuration and bindings. |
| `config path` | Print the resolved config path. |
| `reload` | Reload the active daemon configuration. |

### Lifecycle example

```sh
kact start
kact status
kact stop       # stops held movement; daemon remains running
kact deactivate # also hides an active overlay
kact quit       # stops the daemon
```

`status` returns JSON including daemon state, pointer position when available, active navigation mode, whether held movement or a glide is active, configuration path, and enabled shortcut counts.

## Navigation and motion

| Command | Syntax | Purpose |
| --- | --- | --- |
| Activate | `activate [grid\|elements\|freestyle]` | Start a mode; default is `freestyle`. |
| Toggle | `toggle [grid\|elements\|freestyle]` | Activate the mode or deactivate an active mode. |
| Deactivate | `deactivate` | Hide navigation and release held input. |
| Cancel | `cancel` | Clear a label prefix or deactivate if empty. |
| Move | `move --dx N --dy N [--glide]` | Move by relative points, optionally animating to the resolved endpoint. |
| Move absolute | `move-to --x N --y N [--glide]` | Move to global coordinates, optionally animating to the endpoint. |
| Start motion | `move-start <up\|down\|left\|right> [--speed normal\|precise\|fast]` | Begin continuous movement, with an optional held speed override. |
| Stop motion direction | `move-stop <direction>` | Stop one continuous direction. |
| Speed | `speed <normal\|precise\|fast>` | Select motion multiplier. |
| Stop | `stop` | Stop movement and release every held button. |
| Jump | `jump <top\|bottom\|left\|right\|center\|cycle>` | Move to display edge, center, or cycle corners. |

Coordinates must be finite and within ±1,000,000. A glide captures its start and target when received, reaches that target exactly, and never queues. A new pointer action, `move-start`, `stop`, deactivation, reload, or quit cancels it. Continuous commands require a running daemon and should be paired in order.

`activate grid` shows labeled cells on every display. `activate elements` labels accessible elements in the focused macOS window and falls back to a grid when no targets are available. `activate freestyle` enables local movement controls without showing an overlay. `toggle MODE` deactivates any active mode, regardless of the requested mode.

## Pointer and labels

| Command | Syntax | Purpose |
| --- | --- | --- |
| Click | `click [--button left\|right\|middle] [--count 1..3] [--modifiers cmd,shift]` | Click at the cursor. |
| Hold | `button-down [--button …] [--modifiers …]` | Start a drag. |
| Release | `button-up [--button …] [--modifiers …]` | End a drag. |
| Scroll | `scroll --dx N --dy N` | Positive `dy` scrolls up; positive `dx` scrolls left. |
| Select | `select LABEL` | Enter an active target label. |
| Backspace | `backspace` | Remove one label character. |
| Refine | `refine` | Subdivide the most recently selected grid cell. |
| Presentation | `show <grid-lines\|labels\|larger\|smaller\|more-contrast\|less-contrast>` | Change active overlay presentation. |

Label values must be 1–64 ASCII characters. Scroll inputs are limited to ±100,000 by the CLI; X11 additionally limits them to 1,000 wheel steps. Modified clicks are macOS-only.

### Pointer examples

```sh
# Right-click, double-click, and Cmd-click.
kact click --button right
kact click --count 2
kact click --modifiers cmd

# Drag, then release it. `stop` can recover a held button.
kact button-down --button left
kact move --dx 160 --dy 40
kact button-up --button left

# Select a displayed target through the command interface.
kact activate grid
kact select aa
```

`show` changes the current overlay session only: it does not write configuration. `show larger` and `show smaller` change grid density; contrast, label visibility, and grid lines reset at the next activation.
