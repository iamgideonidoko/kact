---
title: FAQ
description: Common questions about Kact platforms, input, and configuration.
---

## Does Kact support macOS?

Yes. macOS currently provides overlays, accessibility targeting, keyboard capture, modified clicks, and shell actions.

## Does Kact support Linux?

It supports shell-driven pointer actions in X11. It does not provide native X11 overlays, element discovery, keyboard capture, or modified clicks.

## Does it work on Wayland or Windows?

No. Wayland and Windows have no native Kact backend.

## Does Kact run as a daemon?

Yes. `kact start` runs a background local daemon. `kact daemon` keeps it in the foreground.

## Can I use my own keys?

Yes. Enable custom bindings, or have an external shortcut tool call the CLI. Global Kact shortcuts are off by default.

## What is the difference between `move`, `move-to`, and held movement?

`move` is an immediate relative adjustment. `move-to` is an immediate absolute placement. `move-start` and `move-stop` create continuous directional movement for integrations that have separate press and release events. Add `--glide` to `move` or `move-to` for an opt-in one-shot animation to a fixed target.

## Does `--glide` change normal movement?

No. Plain `move` and `move-to` remain immediate. Glides are cancellable and never queue, so they are useful when a visible transition is wanted without changing automation behavior.

## Where is the config?

Run `kact config path`. By default it is `~/.config/kact/kact.toml` unless `XDG_CONFIG_HOME` is set to an absolute path.

## How do I stop Kact?

Use `kact stop` to stop motion and release buttons, `kact deactivate` to also hide navigation, and `kact quit` to stop the daemon.

## Can Kact use a configuration without restarting?

Yes. Valid changes reload automatically by default. Set `system.hot_reload = false` to turn that off, then use `kact reload` after editing. Reloading ends active navigation and releases held input so changed bindings cannot leave a key or button held.
