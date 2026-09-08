---
title: Keyboard and shell workflows
description: Choose native navigation, optional bindings, or direct commands.
---

## Native macOS navigation

Activate a mode, then use its local keys:

```sh
kact activate grid
```

Grid and element modes show labels. Freestyle provides keyboard movement without an overlay. Local navigation keys are active only after explicit activation.

## Direct shell actions

Any pointer action can be sent from a shell or remapper:

```sh
kact move --dx 100
kact move-to --x 800 --y 400 --glide
kact click --button right
kact button-down
kact move --dx 250
kact button-up
```

`move-start right` and `move-stop right` are for ordered key press/release integrations. They do not install keyboard capture. Add `--speed normal`, `--speed precise`, or `--speed fast` to apply a configured multiplier while that direction is held. Use `kact stop` to halt motion and release every held mouse button.

`--glide` is for a one-shot animated endpoint, not held movement. It is opt-in; plain `move` and `move-to` remain immediate for scripts and automation.

### Common shell recipes

```sh
# Click with a modifier on macOS.
kact click --modifiers cmd

# Drag 300 points to the right.
kact button-down
kact move --dx 300
kact button-up

# Scroll down. Positive dy scrolls up; negative dy scrolls down.
kact scroll --dy -120

# Move to the center or a display edge.
kact jump center
kact jump right
```

`button-down` remains held by the daemon until `button-up`, `stop`, `deactivate`, `quit`, or a fatal input error. Always arrange a release path in an external binding.

## Optional bindings

Built-in navigation keys are controlled by `keybindings.navigation_enabled`, which defaults to `true`. Custom `global` and `local` maps are not installed until `keybindings.enabled = true`. Global shortcuts must include a modifier. Custom local bindings override the selected preset.

For the common “my remapper owns every key” setup, disable both binding layers and invoke commands from that remapper:

```toml
[keybindings]
enabled = false
navigation_enabled = false
```

On macOS, this does not disable overlays or CLI control. It only prevents Kact from capturing keyboard events.
