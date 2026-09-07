---
title: Platform setup
description: macOS permissions and Linux X11 requirements.
---

## macOS

Kact uses Accessibility to inject pointer events and inspect accessible UI elements. Grant it in **System Settings → Privacy & Security → Accessibility**. Kact asks for this permission when starting or activating a mode.

Native local and optional global keyboard bindings use the macOS event tap. If key capture is denied, grant Input Monitoring to Kact or the terminal that launched it.

Check configuration, Accessibility, and daemon availability without moving the cursor:

```sh
kact doctor
```

## Linux X11

Install the X11 development headers to build Kact:

```sh
sudo apt-get install libx11-dev libxtst-dev
```

The X11 backend needs a reachable `DISPLAY` and the XTest extension. It supports shell-driven movement, clicks, dragging, scrolling, and absolute movement. It does not provide overlays, accessibility-element discovery, built-in keyboard capture, or modified clicks.

Wayland is rejected when `WAYLAND_DISPLAY` is set. Use an X11 session for the native Linux backend.
