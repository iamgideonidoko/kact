---
title: FAQ
description: Common questions about Kact platforms, input, and configuration.
---

## Does Kact support macOS?

Yes. macOS is the primary platform and provides overlays, accessibility targeting, keyboard capture, and modified clicks.

## Does Kact support Linux?

It supports shell-driven pointer actions in X11. It does not provide native X11 overlays, element discovery, keyboard capture, or modified clicks.

## Does it work on Wayland or Windows?

No. Wayland and Windows have no native Kact backend.

## Does Kact run as a daemon?

Yes. `kact start` runs a background local daemon. `kact daemon` keeps it in the foreground.

## Can I use my own keys?

Yes. Enable custom bindings, or have an external shortcut tool call the CLI. Global Kact shortcuts are off by default.

## Where is the config?

Run `kact config path`. By default it is `~/.config/kact/kact.toml` unless `XDG_CONFIG_HOME` is set to an absolute path.

## How do I stop Kact?

Use `kact stop` to stop motion and release buttons, `kact deactivate` to also hide navigation, and `kact quit` to stop the daemon.
