---
title: Platform setup
description: macOS permissions and Linux X11 requirements.
---

## macOS

Kact uses Accessibility to inject pointer events and inspect accessible UI elements. Run `kact setup`: it requests permission and opens **System Settings → Privacy & Security → Accessibility** when required. Grant the application that actually runs Kact: your terminal for development, or the installed binary for a login service. Then run `kact setup` again.

## Accessibility and visual surfaces

Element navigation reads only macOS Accessibility metadata: roles, actions, bounds, and optional accessible text. It does not capture windows, read pixels, run OCR, or request **Screen Recording** permission. This keeps normal navigation usable without screen-content access and ensures labels represent semantic Accessibility targets.

Some interfaces intentionally expose no useful Accessibility controls: canvas and game views, video surfaces, and some custom browser or Electron UIs. Kact falls back to grid navigation for those surfaces. A future visual fallback must remain explicit, require Screen Recording permission before capture, and label visual targets separately from Accessibility targets; it must never silently change normal element navigation.

Native local and optional global keyboard bindings use the macOS event tap. If key capture is denied, grant Input Monitoring to Kact or the terminal that launched it.

Check configuration, Accessibility, and daemon availability without moving the cursor:

```sh
kact doctor
```

## Linux X11

Install the X11 development headers to build Kact:

```sh
sudo apt-get install libx11-dev libxtst-dev

# Arch Linux and Omarchy
sudo pacman -S libx11 libxtst
```

The X11 backend needs a reachable `DISPLAY` and the XTest extension. It supports shell-driven movement, clicks, dragging, scrolling, and absolute movement. It does not provide overlays, accessibility-element discovery, built-in keyboard capture, or modified clicks.

Wayland is rejected when `WAYLAND_DISPLAY` is set. Use an X11 session for the native Linux backend.

For X11, set `keybindings.navigation_enabled = false` and let your window manager, compositor, or remapper invoke shell commands. Do not try to activate grid or element navigation: those need the macOS desktop backend.
