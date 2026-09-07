---
title: Introduction
description: What Kact does and how its command-first model works.
---

Kact controls the pointer from the keyboard or a shell command. It is for people building keyboard-led desktop workflows who still need precise pointer movement, target selection, clicks, dragging, and scrolling.

Kact is not a window manager or a general-purpose macro engine. It owns pointer actions. Your preferred shortcut tool can own activation.

## Mental model

Run one local daemon with `kact start`. Commands then travel over a private Unix socket to that daemon. On macOS, activating grid or element navigation draws labels and captures only the keys it handles. On X11, use shell commands directly; native overlays and keyboard capture are unavailable.

Global shortcuts are disabled by default. This preserves normal typing and lets Hammerspoon, Karabiner-Elements, Kanata, or a window manager invoke Kact commands.
