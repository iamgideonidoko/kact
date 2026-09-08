---
title: Quick start
description: Start the Kact daemon and make your first pointer action.
---

1. Check where Kact will store its configuration, then start the daemon. `start` creates a default config when none exists.

   ```sh
   kact config path
   kact start
   ```

2. On macOS, grant Accessibility when prompted in **System Settings → Privacy & Security → Accessibility**. Input Monitoring may also be needed for keyboard capture.

3. Activate the grid.

   ```sh
   kact activate grid
   ```

4. Type the labels shown on screen to place the cursor. Press <kbd>Enter</kbd> to click, or <kbd>Escape</kbd> to clear a partial label and press it again to cancel.

5. Inspect the daemon when needed:

   ```sh
   kact status
   ```

6. Stop the daemon when finished.

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
