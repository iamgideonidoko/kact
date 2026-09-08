---
title: Development setup
description: Build, run, format, lint, and test Kact.
---

```sh
git clone https://github.com/iamgideonidoko/kact.git
cd kact
mise install
make build
make test
```

Use `make help` for available Rust and local daemon commands. `make daemon` runs a foreground daemon with debug logs. Use `--config` and `--socket` to isolate development instances.

Before opening a pull request, run:

```sh
make lint
make test
make release
```

Run `make coverage-core` after changing `src/core/`. For documentation changes, run `pnpm --dir docs run check` and `make docs-build`. Linux builds require `libx11-dev` and `libxtst-dev`. See the repository’s canonical [contribution guide](https://github.com/iamgideonidoko/kact/blob/main/CONTRIBUTING.md) for contribution expectations.
