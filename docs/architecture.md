# Architecture

Kact runs one local daemon and exposes actions through a private Unix socket. `kact start` launches the daemon; subsequent action commands forward requests to it.

`src/core/` holds platform-independent motion, navigation, and state. `src/runtime/` turns commands and active key bindings into state changes. `src/ipc.rs` owns daemon locking and bounded local requests. `src/platform/` injects mouse input and captures keys. `src/desktop/` owns macOS AppKit overlays and accessibility discovery on the main thread. `src/service.rs` manages macOS login startup.

Platform implementations must not change shared command semantics. Commands are the stable integration boundary for shells and remappers.
