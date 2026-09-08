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

The LaunchAgent uses the configuration and socket paths supplied when you run `service install`. Check the active daemon after signing in:

```sh
kact status
```

If you change the binary location, configuration path, or socket path used by the login service, run `kact service uninstall` and install it again with the intended options.
