# Development

Rust 1.88.0 is pinned in `rust-toolchain.toml` and `.mise.toml`.

```sh
mise install
make build
make run ARGS='status'
make daemon
```

Use `make help` for local commands. Point development instances at isolated paths with `--config` and `--socket` when needed. `kact config init` never overwrites an existing configuration.

Code follows standard Rust formatting. Keep platform-specific code in its platform or desktop module and test shared logic without native permissions where possible.

