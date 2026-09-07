---
title: Keybindings
description: Default local controls, Vi and Emacs presets, and optional custom maps.
---

Kact captures navigation keys only while a mode is active. Built-in local controls are enabled by `navigation_enabled = true`. Custom maps require `enabled = true`. Global shortcuts need a modifier and are empty by default.

| Action | Default local key | Notes |
| --- | --- | --- |
| Cancel / deactivate | <kbd>Escape</kbd>, <kbd>Cmd</kbd>+<kbd>.</kbd>, <kbd>Ctrl</kbd>+<kbd>G</kbd> | First cancel clears a label prefix. |
| Hide navigation | <kbd>Cmd</kbd>+<kbd>H</kbd> | Deactivates. |
| Select label | Displayed label | Grid and elements modes. |
| Backspace | <kbd>Backspace</kbd> | Removes one label character. |
| Move | Arrow keys | 10 points. |
| Move farther | <kbd>Alt</kbd> + arrows | 100 points. |
| Jump to edge | <kbd>Cmd</kbd> + arrows | |
| Scroll | <kbd>Shift</kbd> + arrows | |
| Click / drop drag | <kbd>Enter</kbd> | |
| Begin drag | <kbd>=</kbd> | |
| Double click | <kbd>\</kbd> | |
| Middle / right click | <kbd>[</kbd> / <kbd>]</kbd> | |
| Modified click | modifiers + <kbd>Enter</kbd> | Any non-empty Ctrl/Alt/Shift/Cmd combination. |

## Presets

`vi` is the default. It adds <kbd>H</kbd>/<kbd>J</kbd>/<kbd>K</kbd>/<kbd>L</kbd> movement, Ctrl+H/J/K/L larger movement, Shift+H/J/K/L edge jumps, Shift+M center/corner cycling, and Ctrl+B/F/I/A scrolling.

`emacs` adds Ctrl+P/N/B/F movement, Alt+A/E/B/F larger movement, Ctrl+A/E edge jumps, Ctrl+L center/corner cycling, and Shift+P/N/B/F scrolling.

Keys used by local bindings are removed from the label alphabet. Kact rejects a configuration that leaves fewer than two label characters.

## Custom maps

```toml
[keybindings]
enabled = true

[keybindings.global]
"ctrl+alt+g" = "activate grid"

[keybindings.local]
"tab" = "refine"
```

Binding values are Kact actions, never shell commands. They use the same syntax as the [CLI reference](/reference/cli/). Custom local keys override preset keys.
