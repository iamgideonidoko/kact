# Kact agent guide

- Keep the CLI command-first; global shortcuts remain opt-in.
- Preserve native platform boundaries: shared behavior belongs outside `platform/` and `desktop/`.
- Keep configuration backward compatible unless a documented breaking change is intended.
- Use the pinned Rust toolchain through Mise for local development.
- Run `make lint` and `make test` after Rust changes. Run `make coverage-core` after changing `src/core/`. Run macOS smoke checks for native input, overlay, or accessibility changes.
- Do not add dependencies, platform support, or user-facing features without a concrete need.
- Update `CHANGELOG.md`, the README, and relevant `docs/src/content/docs/` page when user-facing behavior, configuration, or supported platforms change. Run `pnpm --dir docs run check` and `make docs-build` after documentation-site changes.
- Preserve release distribution guarantees: verify archive checksums, never use `sudo`, and require an installation receipt before deleting a release-installed binary.
- Never run `make publish`, create a tag, push a release, or publish a GitHub Release without explicit user instruction.
