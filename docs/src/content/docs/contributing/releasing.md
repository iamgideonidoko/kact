---
title: Releasing
description: Prepare, publish, and verify a Kact release.
---

Kact releases are prepared locally from `main`. `make publish` runs the release checks, creates the release commit and tag, and pushes both. GitHub Actions builds the archives and creates the GitHub Release.

## Prepare a stable release

1. Start with an up-to-date, clean `main` branch.

2. Consolidate the user-visible changes since the last stable release under a new `## X.Y.Z` heading in `CHANGELOG.md`.

   Include changes that previously appeared in release candidates so the stable GitHub Release stands on its own. Keep the earlier `-rc.N` sections as historical records. Leave a fresh, empty `## Unreleased` heading above the new version.

3. Set `[package].version` in `Cargo.toml` to the same `X.Y.Z` value and update `Cargo.lock`.

4. Review the release preparation:

   ```sh
   git diff --check
   git status --short
   ```

   Before publishing, only `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` may have changes. Commit or remove any unrelated work first.

5. Publish:

   ```sh
   make publish
   ```

The command reads the version from `Cargo.toml`; do not pass a version argument. It runs linting, tests, documentation build, and host archive packaging, then creates `chore: release vX.Y.Z`, tags it, and pushes `main` with the tag.

## Release candidates

Use the same process with a prerelease version such as `0.1.0-rc.1` and a matching `## 0.1.0-rc.1` changelog section. GitHub publishes tags with a prerelease suffix as GitHub prereleases, so they do not replace the stable installer target.

When promoting an RC to `X.Y.Z`, make the stable changelog section cumulative: include every user-visible feature and fix delivered by the RC, plus any changes since it.

## Verify the published release

After the GitHub Actions release workflow finishes:

1. Confirm its checks, three platform archives, `SHA256SUMS`, and generated release notes on the GitHub Release page.

2. Install into a temporary directory and check the version:

   ```sh
   test_dir=$(mktemp -d)
   curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/iamgideonidoko/kact/main/scripts/install.sh | KACT_INSTALL_DIR="$test_dir" bash
   "$test_dir/kact" --version
   rm -rf "$test_dir"
   ```

   For a stable release, the command must install the new version without `--version`.

3. On macOS, run the installed binary through `kact setup`, activate a grid, and verify pointer actions. On X11, verify a harmless shell action such as `kact move --dx 1`.

## If publishing fails

If local checks fail, fix the issue before rerunning `make publish`; it has not created a tag yet. If GitHub Actions fails after the tag is pushed, rerun the workflow only for an infrastructure failure. For a release defect, fix it and publish a new version. Do not move or reuse a published tag.
