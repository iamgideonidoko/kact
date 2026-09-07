---
title: External integrations
description: Use Kact with Hammerspoon, Karabiner-Elements, and Kanata.
---

Kact commands are the integration boundary. Start Kact once, configure the external tool to call an absolute path to `kact`, and leave Kact global bindings disabled when that tool owns activation.

## Hammerspoon

[`examples/hammerspoon.lua`](https://github.com/iamgideonidoko/kact/blob/main/examples/hammerspoon.lua) binds activation keys and serializes commands through `hs.task`. Serialization matters for `move-start` and `move-stop`, which must arrive in order.

## Karabiner-Elements

[`examples/karabiner.json`](https://github.com/iamgideonidoko/kact/blob/main/examples/karabiner.json) is a complex-modification rule that calls Kact through a shell command. Replace the binary path with your installed location.

## Kanata

[`examples/kanata.kbd`](https://github.com/iamgideonidoko/kact/blob/main/examples/kanata.kbd) uses Kanata’s `cmd` action. It requires a Kanata build with command support and `danger-enable-cmd yes`. Run it as the desktop user and use an absolute Kact path.

These example files are templates; they have not been exercised inside each third-party application.
