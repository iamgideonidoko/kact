---
title: Quick start
description: Set up Kact and make your first pointer action.
---

1. Run setup. It creates the default configuration, asks for Accessibility on macOS, and starts the daemon. If macOS opens System Settings, grant permission and run the same command again.

   ```sh
   kact setup
   ```

2. Activate the grid.

   ```sh
   kact activate grid
   ```

3. Type the labels shown on screen to place the cursor. Press <kbd>Enter</kbd> to click, or <kbd>Escape</kbd> to clear a partial label and press it again to cancel.

4. Inspect the daemon when needed:

   ```sh
   kact status
   ```

5. Stop the daemon when finished.

   ```sh
   kact quit
   ```

For a no-overlay shell workflow, use direct actions instead:

```sh
kact move --dx 20 --dy -10
kact click
```

For one-shot animated movement, add `--glide`:

```sh
kact move --dx 240 --dy 0 --glide
```

`kact stop` only stops held movement and releases held mouse buttons. Use `kact deactivate` to hide an active overlay, and `kact quit` to stop the daemon.

On X11, set `keybindings.navigation_enabled = false` before activating `freestyle`; this keeps Kact from attempting the macOS-only keyboard listener.
