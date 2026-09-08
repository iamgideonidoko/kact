---
title: Changelog
description: Record user-facing changes and prepare a GitHub Release.
---

The canonical release history is maintained in [`CHANGELOG.md`](https://github.com/iamgideonidoko/kact/blob/main/CHANGELOG.md).

Current unreleased work includes command-driven navigation, configurable bindings, native macOS overlays, X11 shell-driven pointer controls, and the Vi default binding preset.

Before tagging a release, move its user-facing entries into a `## X.Y.Z` section. The release workflow uses that exact section as the GitHub Release notes and rejects a tag that does not match the package version in `Cargo.toml`.
