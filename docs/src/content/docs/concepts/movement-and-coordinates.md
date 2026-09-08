---
title: Movement and coordinates
description: Relative movement, absolute positions, acceleration, and displays.
---

`kact move` is relative to the current pointer position. `kact move-to` uses global screen coordinates. On macOS, values are display points, with `(0, 0)` at the primary display’s top-left; displays positioned to the left or above it can have negative coordinates.

Grid navigation creates a configured number of cells on every display. Element navigation labels accessible rectangles in the focused macOS window; if no elements are available, Kact falls back to grid navigation.

Continuous motion begins with `move-start` and stops with the matching `move-stop`. The current `speed` mode applies `normal_multiplier`, `precise_multiplier`, or `fast_multiplier` to `max_speed`. `move-start left --speed fast` temporarily uses that configured multiplier until the matching `move-stop left`; it then restores the selected speed mode.

`curve_type` governs how velocity approaches its target while an input direction is active:

| Value | Behavior |
| --- | --- |
| `linear` | Constant acceleration until target speed. |
| `sigmoid` | Smooth logistic ramp; the default. |
| `exponential` | Exponential approach to target speed. |

On release, `friction` always applies exponential velocity decay. The calculation is time-based, so behavior is independent of the configured frame rate.
