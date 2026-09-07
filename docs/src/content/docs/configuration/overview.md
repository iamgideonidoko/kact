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

## Complete configuration

```toml title="~/.config/kact/kact.toml"
[motion]
curve_type = "sigmoid" # linear | sigmoid | exponential
max_speed = 2000.0      # 1.0..=100000.0
acceleration = 0.8      # 0.001..=1.0
friction = 0.95         # 0.0..=0.9999, retained per 1/60 s after release
target_fps = 144        # 1..=1000

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

`auto_click` clicks after a completed target selection. `rows`, `columns`, and `alphabet` control grid labels. `label_position` places labels inside their target; corner positions fall back to center for small targets. `[navigation.grid]`, `[navigation.elements]`, `[appearance.grid]`, and `[appearance.elements]` are optional sparse overrides: each omitted field inherits its global value. Appearance settings apply to macOS overlays. `target_fps` controls runtime polling while pointer motion is active.

See the [configuration implementation](https://github.com/iamgideonidoko/kact/blob/main/src/config.rs) for validation rules.
