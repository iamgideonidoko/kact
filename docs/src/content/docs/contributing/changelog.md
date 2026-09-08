---
title: Changelog
description: Record user-facing changes and prepare a GitHub Release.
---

The canonical release history is maintained in [`CHANGELOG.md`](https://github.com/iamgideonidoko/kact/blob/main/CHANGELOG.md).

Current unreleased work includes command-driven navigation, configurable bindings, native macOS overlays, X11 shell-driven pointer controls, and the Vi default binding preset.

Before publishing a release, move its user-facing entries into a `## X.Y.Z` or `## X.Y.Z-rc.N` section and set the same version in `Cargo.toml`. Then run:

```sh
make publish VERSION=X.Y.Z[-rc.N]
```

The command accepts changes only to `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md`; runs the release checks; commits the preparation; tags the commit; and pushes it. GitHub Actions uses the matching changelog section as the GitHub Release notes. A version with a prerelease suffix such as `-rc.1` is published as a GitHub prerelease, so it does not become the default installer target.
