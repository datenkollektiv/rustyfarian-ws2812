# Release Plan

Release process for the `rustyfarian-ws2812` workspace.

## Versioning

- **Scheme:** SemVer (0.x.y — pre-stable)
- **Model:** Single workspace version — all crates move to the same version together
- **Snapshot convention:** None; crates live at the released version between releases
- **Who decides version:** Maintainer; minor bump for new public API, patch for fixes only

## Branch and Tag Convention

- **Release branch:** `main`
- **Tag format:** `v0.X.Y` (e.g., `v0.2.0`)
- **Tagging:** Manual annotated tag on the release commit

## Pre-flight Checklist

Before any release:

- [ ] Build passes: `just check`
- [ ] Tests pass: `just test`
- [ ] `cargo deny` clean: `just deny`
- [ ] Changelog has an `[Unreleased]` section with content for this release
- [ ] Version consistent across all crates: `crates/*/Cargo.toml`
- [ ] No uncommitted changes: `git status --short` is empty
- [ ] Working tree is on `main`

## Version Bump

Files to update when bumping to `X.Y.Z`:

- `crates/bunting/Cargo.toml` — `version = "X.Y.Z"`
- `crates/ferriswheel/Cargo.toml` — `version = "X.Y.Z"`
- `crates/pennant/Cargo.toml` — `version = "X.Y.Z"`
- `crates/rustyfarian-esp-idf-ws2812/Cargo.toml` — `version = "X.Y.Z"`
- `crates/rustyfarian-esp-hal-ws2812/Cargo.toml` — `version = "X.Y.Z"`

Post-release version bump: none (no snapshot pattern).

## Publish

**Target:** Git tag only — no crates.io publishing at this stage.
**Credentials:** None required.
**Downstream consumption:** via `git` dependencies in `Cargo.toml`.

## Changelog

**Location:** `CHANGELOG.md`
**Format:** [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
**Process:**

1. Replace `## [Unreleased]` with `## [X.Y.Z] - YYYY-MM-DD`
2. Add a fresh `## [Unreleased]` section above it (empty, for the next cycle)

## Tagging

```sh
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main --follow-tags
```

## GitHub Release

After pushing the tag, create a GitHub release:

- Navigate to the repository releases page
- Select tag `vX.Y.Z`
- Title: `v X.Y.Z`
- Body: paste the `## [X.Y.Z]` changelog section verbatim

## Rollback Procedure

If a mistake is caught after tagging but before any downstream picks it up:

1. Delete remote tag: `git push --delete origin vX.Y.Z`
2. Delete local tag: `git tag -d vX.Y.Z`
3. Revert the version bump commit: `git revert HEAD`
4. Push the revert: `git push origin main`

Since crates are not published to crates.io, a yanked tag is sufficient — no registry rollback required.

## Release Record Location

Each release produces files in `release/`:

1. `YYYY-MM-DD-<version>-preflight.md` — pre-flight assessment
2. `YYYY-MM-DD-<version>-plan.md` — ordered execution plan
3. `YYYY-MM-DD-<version>-record.md` — what was done and what remains
