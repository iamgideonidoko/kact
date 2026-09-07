---
title: Quick start
description: Start the Kact daemon and make your first pointer action.
---

1. Start the daemon. It creates a default config when no config exists.

   ```sh
   kact start
   ```

2. On macOS, grant Accessibility when prompted in **System Settings → Privacy & Security → Accessibility**. Input Monitoring may also be needed for keyboard capture.

3. Activate the grid.

   ```sh
   kact activate grid
   ```

4. Type the labels shown on screen to place the cursor. Press <kbd>Enter</kbd> to click, or <kbd>Escape</kbd> to cancel.

5. Stop the daemon when finished.

   ```sh
   kact quit
   ```

For a no-overlay shell workflow, use direct actions instead:

```sh
kact move --dx 20 --dy -10
kact click
```

On X11, set `keybindings.navigation_enabled = false` before activating `freestyle`; this keeps Kact from attempting the macOS-only keyboard listener.
