---
title: Installation
description: Install the pinned Rust toolchain and build Kact from source.
---

Kact currently ships as a Rust source build. No package-manager formula, release binary, or install script is configured in this repository.

## Requirements

- macOS or a Linux X11 session.
- Rust 1.88.0 with `rustfmt` and `clippy` for development. The repository pins this in `rust-toolchain.toml` and `.mise.toml`.
- On Linux: `libx11-dev` and `libxtst-dev` to compile the X11 backend.

## Install with Mise

```sh
git clone https://github.com/iamgideonidoko/kact.git
cd kact
mise install
cargo install --path .
kact --version
```

## Install with Rustup

```sh
rustup toolchain install 1.88.0 --profile minimal --component rustfmt --component clippy
git clone https://github.com/iamgideonidoko/kact.git
cd kact
cargo install --path .
kact --version
```

`cargo install --path .` installs the executable into Cargo’s binary directory, commonly `~/.cargo/bin`. Ensure that directory is on `PATH` before using Kact from a GUI shortcut tool.
