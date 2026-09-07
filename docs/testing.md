# Testing

```sh
make lint
make test
make release
```

`make smoke` checks the macOS daemon, overlays, configuration reload, and shutdown without moving the pointer. `make smoke-mouse` uses a temporary fixture window to check real pointer actions and keyboard capture; it restores the pointer afterwards.

Run both macOS smoke checks after changing native input, overlays, accessibility, or the runtime path that connects them. Linux automated checks compile and test X11 support; desktop behavior still needs manual X11 verification.

