# Kact agent guide

- Keep the CLI command-first; global shortcuts remain opt-in.
- Preserve native platform boundaries: shared behavior belongs outside `platform/` and `desktop/`.
- Keep configuration backward compatible unless a documented breaking change is intended.
- Run `make lint` and `make test` after Rust changes. Run macOS smoke checks for native input, overlay, or accessibility changes.
- Do not add dependencies, platform support, or user-facing features without a concrete need.
- Update the README and relevant `docs/` page when behavior, configuration, or supported platforms change.

