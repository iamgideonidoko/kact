---
title: CLI reference
description: Commands, arguments, and global options.
---

All commands accept `--config PATH`, `--socket PATH`, and `--log-level LEVEL`. `--config` selects the file at daemon startup; passing it to a client does not reconfigure a daemon that is already running. `--socket` selects the private control socket.

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

## Navigation and motion

| Command | Syntax | Purpose |
| --- | --- | --- |
| Activate | `activate [grid\|elements\|freestyle]` | Start a mode; default is `freestyle`. |
| Toggle | `toggle [grid\|elements\|freestyle]` | Activate the mode or deactivate an active mode. |
| Deactivate | `deactivate` | Hide navigation and release held input. |
| Cancel | `cancel` | Clear a label prefix or deactivate if empty. |
| Move | `move --dx N --dy N` | Move by relative points. |
| Move absolute | `move-to --x N --y N` | Move to global coordinates. |
| Start motion | `move-start <up\|down\|left\|right> [--speed normal\|precise\|fast]` | Begin continuous movement, with an optional held speed override. |
| Stop motion direction | `move-stop <direction>` | Stop one continuous direction. |
| Speed | `speed <normal\|precise\|fast>` | Select motion multiplier. |
| Stop | `stop` | Stop movement and release every held button. |
| Jump | `jump <top\|bottom\|left\|right\|center\|cycle>` | Move to display edge, center, or cycle corners. |

Coordinates must be finite and within ±1,000,000. Continuous commands require a running daemon and should be paired in order.

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
