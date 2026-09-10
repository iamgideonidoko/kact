---
title: Changelog
description: Record user-facing changes and prepare a GitHub Release.
---

The canonical release history is maintained in [`CHANGELOG.md`](https://github.com/iamgideonidoko/kact/blob/main/CHANGELOG.md).

Add user-visible changes under `## Unreleased`. When preparing a version, move them into a matching `## X.Y.Z` or `## X.Y.Z-rc.N` section and leave a new empty `## Unreleased` heading at the top.

For stable releases, make the section cumulative: include user-visible changes from any release candidates as well as work completed after them. Keep release-candidate sections as history. GitHub uses the matching version section as the release notes.

See the [release guide](./releasing/) for the complete preparation, publishing, verification, and failure-recovery process.
