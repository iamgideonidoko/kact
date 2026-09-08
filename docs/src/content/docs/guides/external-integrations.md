---
title: External integrations
description: Use Kact with Hammerspoon, Karabiner-Elements, and Kanata.
---

Kact commands are the integration boundary. Start Kact once, configure the external tool to call an absolute path to `kact`, and leave Kact global bindings disabled when that tool owns activation.

```sh
kact start
kact status
```

Find the binary path with `command -v kact`. GUI applications often start with a minimal `PATH`, so use that absolute path in their configuration. Each command contacts the already-running daemon; it does not start one automatically.

Use immediate actions when the integration emits a single event:

```sh
/absolute/path/to/kact move --dx 40 --dy 0
/absolute/path/to/kact activate grid
```

Use ordered `move-start` and `move-stop` only when the integration has reliable key press and release hooks. Use `--glide` for a one-shot animation; it does not require a release event.

## Hammerspoon

[`examples/hammerspoon.lua`](https://github.com/iamgideonidoko/kact/blob/main/examples/hammerspoon.lua) binds activation keys and serializes commands through `hs.task`. Serialization matters for `move-start` and `move-stop`, which must arrive in order. It is the right pattern for held motion because `hs.hotkey.bind` has both pressed and released callbacks.

Adapt the binary path at the top of the example, reload your Hammerspoon configuration, then test an activation shortcut before adding held movement. Use `kact stop` as an escape binding that releases held pointer input.

## Karabiner-Elements

[`examples/karabiner.json`](https://github.com/iamgideonidoko/kact/blob/main/examples/karabiner.json) is a complex-modification rule that calls Kact through a shell command. Replace the binary path with your installed location, import it in Karabiner-Elements, then enable the rule.

Karabiner is well suited to one-shot commands such as `activate grid`, `move`, `move-to`, `jump`, and `move --glide`. Its native `mouse_key` action is a held velocity control; in Kact, the equivalent is an ordered `move-start` on press and `move-stop` on release. Keep those shell requests ordered, and include a separate `stop` shortcut for recovery.

## Kanata

[`examples/kanata.kbd`](https://github.com/iamgideonidoko/kact/blob/main/examples/kanata.kbd) uses Kanata’s `cmd` action. It requires a Kanata build with command support and `danger-enable-cmd yes`. Run it as the desktop user and use an absolute Kact path.

Start with activation and immediate actions in Kanata. Add press/release continuous movement only when your Kanata configuration can guarantee both commands reach Kact in order. On Linux, Kanata plus Kact’s shell commands is the intended alternative to Kact’s unavailable native keyboard capture.

## Recommended Kact configuration

When an external tool owns keybindings, keep Kact command-only:

```toml
[keybindings]
enabled = false
navigation_enabled = false
```

These example files are templates. Test them in the third-party application you use, beginning with `kact status` and a harmless `move --dx 1`, before assigning broader shortcuts.
