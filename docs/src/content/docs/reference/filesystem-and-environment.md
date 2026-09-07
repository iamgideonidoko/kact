---
title: Filesystem and environment
description: Configuration, logs, sockets, and environment variables read by Kact.
---

## Files

| Item | Location | Purpose |
| --- | --- | --- |
| Config | `$XDG_CONFIG_HOME/kact/kact.toml` or `~/.config/kact/kact.toml` | User settings. |
| Log | Config path with `.log` extension | Background `start` output. |
| Socket | `$XDG_RUNTIME_DIR/kact.sock`, otherwise the system temporary directory | Local daemon control. |
| macOS LaunchAgent | `~/Library/LaunchAgents/io.kact.agent.plist` | Optional login startup. |

Use `kact config path` to see the chosen config path. The socket directory must be owned by the current user and have mode `0700`.

## Environment

| Variable | Use |
| --- | --- |
| `XDG_CONFIG_HOME` | Chooses the config base when it is an absolute path. |
| `XDG_RUNTIME_DIR` | Chooses the socket directory. |
| `WAYLAND_DISPLAY` | Causes the Linux backend to reject the session because native Wayland injection is unsupported. |
| `HOME` | Fallback config location and macOS LaunchAgent location. |
| `DISPLAY` | Used by X11 through the native Xlib connection. |
