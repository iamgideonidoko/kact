---
title: Movement and coordinates
description: Relative movement, absolute positions, acceleration, and displays.
---

`kact move` is relative to the current pointer position. `kact move-to` uses global screen coordinates. On macOS, values are display points, with `(0, 0)` at the primary display’s top-left; displays positioned to the left or above it can have negative coordinates.

## Choose the movement model

| Command | Use it when | Result |
| --- | --- | --- |
| `move --dx N --dy N` | A script needs an immediate relative adjustment | Moves now from the current pointer position. |
| `move-to --x N --y N` | You know the absolute screen coordinate | Moves now to that coordinate. |
| Add `--glide` | You want a one-shot visible transition | Animates to a captured endpoint, then stops. |
| `move-start` / `move-stop` | An external keybinding has press and release events | Moves continuously while a direction is held. |

Examples:

```sh
# Nudge left and up immediately.
kact move --dx -20 --dy -10

# Move to the center of a known 1440×900 primary display.
kact move-to --x 720 --y 450

# Hold-style movement supplied by a remapper.
kact move-start right --speed fast
kact move-stop right
```

Add `--glide` to either command for a bounded animation to its captured endpoint:

```sh
kact move --dx 200 --dy 0 --glide
kact move-to --x 800 --y 400 --glide
```

Glides finish exactly at their target and never queue. Another pointer action, continuous movement, `stop`, deactivation, reload, or quit cancels the active glide. They use `[glide]` duration and easing settings, independently of held-motion acceleration and friction. Use glides for a deliberate, visible one-shot movement; leave them off for automation that needs the pointer to move immediately.

Grid navigation creates a configured number of cells on every display. Element navigation labels accessible rectangles in the focused macOS window; if no elements are available, Kact falls back to grid navigation.

Continuous motion begins with `move-start` and stops with the matching `move-stop`. The current `speed` mode applies `normal_multiplier`, `precise_multiplier`, or `fast_multiplier` to `max_speed`. `move-start left --speed fast` temporarily uses that configured multiplier until the matching `move-stop left`; it then restores the selected speed mode.

Directions can overlap. For example, start `up` and `right` to move diagonally; stop each direction separately. If multiple held directions specify `--speed`, the most recently started direction chooses the temporary mode. Releasing it restores the most recently held remaining mode, or the selected base mode.

`curve_type` governs how velocity approaches its target while an input direction is active:

| Value | Behavior |
| --- | --- |
| `linear` | Constant acceleration until target speed. |
| `sigmoid` | Smooth logistic ramp; the default. |
| `exponential` | Exponential approach to target speed. |

On release, `friction` always applies exponential velocity decay. The calculation is time-based, so behavior is independent of the configured frame rate.

## Displays and coordinates

`move-to` and `jump` use the virtual desktop, not an individual display’s local coordinate system. A display placed left of or above the primary display has negative coordinates. `jump` chooses the display containing the pointer; if the pointer is outside every known display, it uses the first display. `jump cycle` alternates the current display’s center and four corners.
