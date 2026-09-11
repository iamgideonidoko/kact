# Changelog

All notable user-facing changes are recorded here.

## Unreleased

### Added

- `kact refresh` and privacy-safe `kact inspect elements` diagnostics (`--show-text` opts into accessible text).
- `kact inspect elements --pid PID` inspects a target macOS app without changing focus.
- `kact update` and `kact update --check` for verified updates of release-script installations.
- Dependabot and CodeQL automation, pinned GitHub Actions, and repository security and contributor policies.

## 0.1.0-rc.1

### Added

- Command-driven cursor navigation, configurable bindings, glides, and native macOS overlays.
- X11 shell-driven pointer controls.
- Guided `kact setup`, release installation, and safe release uninstallation.

### Changed

- macOS element navigation now uses visible, action-aware accessibility discovery with role-only control fallback, ranking/deduplication, a bounded Chromium/Electron probe, and semantic `AXPress` automatic activation.
- Vi is the default navigation preset; Emacs is also available.
