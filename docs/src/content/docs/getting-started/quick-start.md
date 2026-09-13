---
title: Quick start
description: Set up Kact and make your first pointer action.
---

1. Run setup. It creates the default configuration, asks for Accessibility on macOS, and starts the daemon. If macOS opens System Settings, grant permission and run the same command again.

   ```sh
   kact setup
   ```

2. On macOS, activate the grid.

   ```sh
   kact activate grid
   ```

3. On macOS, type the labels shown on screen to place the cursor. Press <kbd>Enter</kbd> to click, or <kbd>Escape</kbd> to clear a partial label and press it again to cancel.

4. Inspect the daemon when needed:

   ```sh
   kact status
   ```

5. Stop the daemon when finished.

   ```sh
   kact quit
   ```

On Linux X11, and for a no-overlay workflow on macOS, use direct actions:

```sh
kact move --dx 20 --dy -10
kact click
```

For one-shot animated movement, add `--glide`:

```sh
kact move --dx 240 --dy 0 --glide
```

`kact stop` only stops held movement and releases held mouse buttons. Use `kact deactivate` to hide an active overlay, and `kact quit` to stop the daemon.

On X11, do not activate navigation modes; bind shell actions from your window manager or remapper instead.
