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
kact click --button right
kact button-down
kact move --dx 250
kact button-up
```

`move-start right` and `move-stop right` are for ordered key press/release integrations. They do not install keyboard capture. Use `kact stop` to halt motion and release every held mouse button.

## Optional bindings

Built-in navigation keys are controlled by `keybindings.navigation_enabled`, which defaults to `true`. Custom `global` and `local` maps are not installed until `keybindings.enabled = true`. Global shortcuts must include a modifier. Custom local bindings override the selected preset.
