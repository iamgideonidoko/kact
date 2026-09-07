---
title: Start at login
description: Configure the macOS LaunchAgent managed by Kact.
---

On macOS, Kact can write and manage its own LaunchAgent:

```sh
kact service install
```

The service starts Kact at the next Aqua login. Run `kact start` to run it in the current session. Remove the managed service with:

```sh
kact service uninstall
```

Kact refuses to overwrite or remove an unmanaged LaunchAgent. On non-macOS platforms, use the session manager to run `kact daemon`.
