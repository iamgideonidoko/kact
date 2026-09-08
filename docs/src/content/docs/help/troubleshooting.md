---
title: Troubleshooting
description: Diagnose Kact configuration, permission, service, and platform failures.
---

## Kact will not start on macOS

**Cause:** Accessibility is not granted.

**Check:** Run `kact doctor`.

**Fix:** Run `kact setup`. It opens **System Settings → Privacy & Security → Accessibility** when required. Enable Kact or the launching terminal, then rerun `kact setup`.

## Navigation keys do not work

**Cause:** Input Monitoring is missing, navigation is inactive, or keyboard navigation is disabled.

**Check:** Activate a mode and inspect `[keybindings]` in the active config.

**Fix:** Grant Input Monitoring where macOS requests it. Set `navigation_enabled = true` for native local controls. On X11, bind shell commands in your window manager or shortcut tool instead.

## A command says the service is unavailable

**Cause:** The action command does not start a daemon by itself.

**Fix:** Run `kact setup`, then retry. Use the same `--socket` argument for a custom daemon and its clients.

## `stop` did not stop Kact

**Cause:** `stop` is an input safety command, not a daemon shutdown command.

**Fix:** Use `kact stop` to halt continuous movement and release held mouse buttons. Use `kact deactivate` to hide navigation. Use `kact quit` to stop the background service.

## Linux cannot move the pointer

**Cause:** There is no X11 display, XTest is missing, or the session is Wayland.

**Check:** Confirm `DISPLAY` points to an X11 session.

**Fix:** Use an X11 session and install the X11 dependencies. Wayland is unsupported by Kact’s native Linux backend.

## Configuration does not apply

**Cause:** The file is invalid, the daemon uses another `--config` path, or automatic reload is disabled.

**Check:** Run `kact --config PATH config check`.

**Fix:** Correct the reported setting. A valid active config reloads automatically when `system.hot_reload = true`; otherwise run `kact reload`.

## A glide stops before its target

**Cause:** A pointer action, `move-start`, `stop`, deactivation, reload, or quit cancels glides by design.

**Fix:** Avoid sending another pointer command until the glide ends. For automation that must act immediately, omit `--glide`.

## A displayed label cannot be typed

**Cause:** Local bindings reserve characters from the label alphabet.

**Fix:** Change the local binding or `navigation.alphabet`. At least two unreserved lowercase characters must remain.
