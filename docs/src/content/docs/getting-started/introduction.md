---
title: Introduction
description: What Kact does and how its command-first model works.
---

Kact controls the pointer from the keyboard or a shell command. It is for people building keyboard-led desktop workflows who still need precise pointer movement, target selection, clicks, dragging, and scrolling.

Kact is not a window manager or a general-purpose macro engine. It owns pointer actions. Your preferred shortcut tool can own activation.

## What you can do

| Need | Kact action |
| --- | --- |
| Move a fixed amount | `move --dx N --dy N` |
| Move to a screen coordinate | `move-to --x N --y N` |
| Animate a one-shot movement | Add `--glide` to `move` or `move-to` |
| Move while a key is held | Pair `move-start` with `move-stop` |
| Choose a visible target | `activate grid` or `activate elements` on macOS |
| Click, drag, scroll, or jump | Send the matching CLI command |

## Mental model

Run `kact setup` once to create configuration, guide macOS permission, and start the local daemon. Commands then travel over a private Unix socket to that daemon. On macOS, activating grid or element navigation draws labels and captures only the keys it handles. On X11, use shell commands directly; native overlays and keyboard capture are unavailable.

Global shortcuts are disabled by default. This preserves normal typing and lets Hammerspoon, Karabiner-Elements, Kanata, or a window manager invoke Kact commands.

## Platform capability

| Capability | macOS | Linux X11 |
| --- | --- | --- |
| Shell pointer actions | Yes | Yes |
| Grid and element overlays | Yes | No |
| Accessibility element targets | Yes | No |
| Built-in local and global bindings | Yes | No |
| Modified clicks | Yes | No |

Wayland and Windows are unsupported. See [platform setup](/kact/getting-started/platform-setup/) before choosing a workflow.
