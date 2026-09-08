---
title: Configuration
description: Config location, validation, reloads, and every supported setting.
---

Kact reads TOML from `$XDG_CONFIG_HOME/kact/kact.toml` when `XDG_CONFIG_HOME` is absolute; otherwise it uses `~/.config/kact/kact.toml`. Pass `--config PATH` to use a different file. `kact config init` writes defaults but never overwrites a file.

```sh
kact config path
kact config init
kact config check
```

Unknown fields, invalid values, duplicate shortcuts, shortcut conflicts, and invalid binding actions are rejected. A running daemon watches valid config changes by default. Reloads deactivate navigation before applying the new configuration; invalid reloads retain the previous configuration. Set `system.hot_reload = false` and use `kact reload` for manual reloads.

Start with the generated file and change only the sections you need. Omitted fields always use their defaults.

## Complete configuration

```toml title="~/.config/kact/kact.toml"
[motion]
curve_type = "sigmoid" # linear | sigmoid | exponential
max_speed = 2000.0      # 1.0..=100000.0
acceleration = 0.8      # 0.001..=1.0
friction = 0.95         # 0.0..=0.9999, retained per 1/60 s after release
target_fps = 144        # 1..=1000

[glide]
duration_ms = 140       # 16..=2000
easing = "ease-out"     # linear | ease-in-out | ease-out

[modes]
normal_multiplier = 1.0 # 0.01..=10.0
precise_multiplier = 0.3
fast_multiplier = 2.5

[keybindings]
enabled = false
navigation_enabled = true
preset = "vi"          # vi | emacs

[keybindings.global]
# "ctrl+alt+g" = "activate grid"

[keybindings.local]
# "tab" = "refine"

[navigation]
rows = 12               # 1..=100
columns = 20            # 1..=100
alphabet = "asdfghjklqwertyuiopzxcvbnm" # unique lowercase ASCII; at least 2 chars
auto_click = false

# Optional overrides inherit any omitted field from [navigation].
[navigation.grid]
rows = 10
columns = 16

[navigation.elements]
auto_click = true

[appearance]
font_size = 14.0        # 8.0..=96.0
foreground = "#FFFFFF" # #RRGGBB
background = "#17212B"
highlight = "#FFD166"
opacity = 0.85          # 0.1..=1.0
grid_lines = true
label_position = "center" # center | top | right | bottom | left | top-left | top-right | bottom-left | bottom-right

# Optional overrides inherit any omitted field from [appearance].
[appearance.grid]
grid_lines = true

[appearance.elements]
grid_lines = false
label_position = "top-left"

[system]
hot_reload = true
log_level = "info"     # error | warn | info | debug | trace | off
```

`auto_click` clicks after a completed target selection. `rows`, `columns`, and `alphabet` control grid labels. `label_position` places labels inside their target; corner positions fall back to center for small targets. `[navigation.grid]`, `[navigation.elements]`, `[appearance.grid]`, and `[appearance.elements]` are optional sparse overrides: each omitted field inherits its global value. Appearance settings apply to macOS overlays. `target_fps` controls runtime polling while pointer motion is active. `[glide]` controls the duration and profile for opt-in one-shot cursor animation.

See the [configuration implementation](https://github.com/iamgideonidoko/kact/blob/main/src/config.rs) for validation rules.

## Motion and speed modes

`[motion]` applies only to continuous `move-start` / `move-stop` motion. `max_speed` is the normal target speed in points per second. `acceleration` controls how quickly motion approaches that speed, `friction` controls deceleration after release, and `target_fps` is the active motion polling rate.

`[modes]` multiplies `max_speed`. The selected mode starts as `normal`; use `kact speed precise` to change it, or apply a temporary value to one held direction:

```sh
kact move-start right --speed fast
kact move-stop right
```

The default values mean normal movement targets 2000 points/s, precise targets 600 points/s, and fast targets 5000 points/s. These settings do not affect immediate `move`, `move-to`, or glides.

## Glide

`--glide` is an opt-in one-shot animation. `duration_ms` controls how long it runs. `ease-out` starts quickly and slows as it reaches the target; `ease-in-out` is slow at both ends; `linear` moves at a constant rate.

```toml
[glide]
duration_ms = 180
easing = "ease-in-out"
```

Glides capture their endpoint when the command arrives. A new pointer action, held movement, `stop`, deactivation, reload, or quit cancels the active glide.

## Navigation and appearance

`[navigation]` controls the grid and label alphabet. `rows` × `columns` determines grid density. Choose an alphabet with unique lowercase letters; Kact removes any letters reserved by active local keybindings and requires at least two letters to remain. Set `auto_click = true` when selecting a completed label should click immediately.

`[appearance]` is used by macOS overlays. Colors use `#RRGGBB`; `opacity` applies to label backgrounds; `label_position` places text within its target. Use an edge or corner position for element labels when center text would cover useful UI.

Mode-specific tables are sparse overrides. For example, this keeps the global defaults for grid navigation while making element labels smaller and placed at the top-left:

```toml
[navigation.elements]
auto_click = true

[appearance.elements]
font_size = 12
label_position = "top-left"
grid_lines = false
```

## Keybindings and system

`keybindings.navigation_enabled` enables built-in local controls only while navigation is active. `keybindings.enabled` separately enables your custom `global` and `local` maps; custom global shortcuts must have a modifier. Keep both custom global maps and Kact’s own global shortcuts disabled when another tool owns your keys. See [keybindings](/kact/configuration/keybindings/) for the full behavior.

`system.hot_reload` watches the configuration file and applies valid changes automatically. `system.log_level` controls daemon logging unless `kact daemon --log-level LEVEL` overrides it.
