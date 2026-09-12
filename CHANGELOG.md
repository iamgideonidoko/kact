# Changelog

All notable user-facing changes are recorded here.

## Unreleased

### Added

- `kact refresh` and privacy-safe `kact inspect elements` diagnostics (`--show-text` opts into accessible text).
- `kact inspect elements --pid PID` inspects a target macOS app without changing focus.
- `make smoke-elements` validates anonymous Accessibility structure against a deterministic native control fixture.
- Optional `navigation.elements.enhanced_user_interface` enables legacy browser accessibility compatibility; it remains disabled by default.
- Optional `navigation.elements.visual_fallback` uses on-device OCR only when a completed Accessibility scan lacks content controls, avoiding capture overhead on usable native trees.
- Accessibility enablement retries now depend on successful requests and share a hydration deadline. Distinct nested controls remain labelled, and partial scans cannot reuse targets from a previous window.
- Active element overlays now coalesce macOS structural changes, keep labels stable across unchanged scans, and use only two bounded settling probes after a refresh.
- Visual fallback requests Screen Recording access when needed and reports unavailable capture access instead of silently showing grid navigation.
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
