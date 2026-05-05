# Feature: crates.io Publication v1

First-wave publication of the three pure-logic library crates to crates.io.
The maintainer's downstream projects will switch from git dependencies to
versioned dependencies, and the broader Rust ecosystem will be able to discover
the crates through the standard index.

## Decisions

| Decision                                                                                                         | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                      | Rejected Alternative                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
|:-----------------------------------------------------------------------------------------------------------------|:--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| First wave covers only the three `no_std` library crates: `bunting`, `pennant`, `ferriswheel`                    | Most stable APIs; lowest publish risk; covers all pure-logic crates that have no hardware dependencies                                                                                                                                                                                                                                                                                                                                      | Publish drivers at the same time — drivers carry HAL-version risk and platform-specific concerns better deferred to v2                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Manual `just release` recipe drives the publish                                                                  | Consistent with existing workspace tooling (`just verify`, `just pre-commit`, `just ci`); first run benefits from human-in-the-loop discovery of rough edges                                                                                                                                                                                                                                                                                | CI-driven automation (`release-plz`, `cargo-release`) — overhead before we know the workflow shape                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Continue the existing `v0.5.x` version series                                                                    | Continuity with git history and existing CHANGELOG; downstream consumers see a coherent version trajectory                                                                                                                                                                                                                                                                                                                                  | Restart at `v0.1.0` — misaligns published versions from in-repo workspace versions and adds one-time bookkeeping cost                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Publish order: `bunting` → `pennant` → `ferriswheel`                                                             | `bunting` (renamed from `ws2812-pure`) has no internal workspace deps; the canonical order matches the documented dependency narrative even though the three v1 crates are independent in practice                                                                                                                                                                                                                                          | Reverse order — would fail dependency resolution at `cargo publish` time if cross-deps are added later                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Internal workspace deps use the dual `{ path = "...", version = "0.5" }` form                                    | Required for crates that are both consumed locally during workspace development and published to crates.io: `cargo publish` strips `path` and uses `version` for the published manifest                                                                                                                                                                                                                                                     | Path-only — breaks `cargo publish` resolution; version-only — breaks local workspace builds before crates.io has the dep available                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Rename `led-effects` → `lantern` before first publish                                                            | Avoid semantic collision with the existing `smart_led_effects` crate; "lantern" is unambiguously visual (no audio interpretation), pairs with `ferriswheel` as a sibling metaphor (single LED vs ring of LEDs), and was confirmed free on crates.io                                                                                                                                                                                         | `ferriswheel-effects` (misleading — implies ring scope but content is single-LED); `firefly` (taken; collides with the Firefly Zero gaming ecosystem on crates.io); `glowworm` (taken by an unrelated hashing library); `rustyfarian-firefly` (asymmetric with the unnamespaced `ferriswheel`)                                                                                                                                                                                                                                                        |
| Rename `ws2812-pure` → `bunting` before first publish                                                            | Pulls the third pure-logic crate out of the crowded `ws2812-*` namespace and into the fairground naming family alongside `ferriswheel` and `lantern`; `bunting` semantically maps onto a strung sequence of coloured units, which is exactly what a WS2812 strip is. Free on crates.io, no meaningful external collisions.                                                                                                                  | `tinsel` (free, but the Christmas-tree connotation overshadows the fairground theme); `chameleon` (taken on crates.io + collides with the Pyramid framework's Python template engine); `harlequin` (free on crates.io but collides with the well-known Python SQL IDE at harlequin.sh); painter's-studio candidates `pigment`/`swatch`/`primer`/`canvas`/`easel` (all blocked or carry severe external collisions); keep `ws2812-pure` (rejected — too generic against the crowded `ws2812-*` prefix and inconsistent with the other two crate names) |
| Rename `lantern` → `pennant` before first publish                                                                | The earlier "lantern is free" claim was based on a `cargo search latern` typo; on re-verification `lantern` returned HTTP 200 from the crates.io API (taken). `pennant` is verified free, semantically perfect (a single triangular flag — a fairground signalling object pairs naturally with the crate's status-LED purpose, and `bunting` is literally a string of pennants), and continues the fairground theme alongside `ferriswheel` | Keep `lantern` — name is already registered on crates.io; `pennon`/`ensign`/`barker`/`bandstand`/`placard` (all free but weaker semantic fit); `rustyfarian-lantern` (asymmetric prefix vs the unprefixed siblings)                                                                                                                                                                                                                                                                                                                                   |
| Sole crates.io ownership: `fwaibel@datenkollektiv.de`                                                            | Maintainer is the sole publisher today; matches current contribution reality                                                                                                                                                                                                                                                                                                                                                                | GitHub team owner up front — premature without contributors; can be added retroactively                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Apply standard pre-publish hygiene during the `just release` run rather than pre-locking it as separate v1 gates | The `cargo publish --dry-run` step is the natural enforcement point for metadata completeness, docs.rs build correctness, and API surface review                                                                                                                                                                                                                                                                                            | Pre-flight every check as a separate v1 gate — adds friction without changing the outcome                                                                                                                                                                                                                                                                                                                                                                                                                                                             |

## Constraints

- v1 covers only the three pure-logic library crates (`bunting`, `pennant`, `ferriswheel`).
  Driver crates (`rustyfarian-esp-idf-ws2812`, `rustyfarian-esp-hal-ws2812`, `rustyfarian-avr-ws2812`) publish in a later iteration once the workflow is proven.
- No CI automation in v1 — manual `just release` flow only.
- Standard pre-publish hygiene applied during the run:
  complete `Cargo.toml` metadata (`description`, `license`, `repository`, `keywords`, `categories`, `documentation`, `readme`),
  `cargo publish --dry-run` clean for each crate,
  docs.rs build green,
  public API audit before the first publish.
- The renames `led-effects` → `pennant` (via `lantern` as an intermediate that was found taken on crates.io) and `ws2812-pure` → `bunting` are breaking changes for any current git-dep consumer.
  CHANGELOG entries required under `## [Unreleased]` calling out both renames and the migration paths
  (`use led_effects::...` → `use pennant::...`; `use ws2812_pure::...` → `use bunting::...`).

## Open Questions

- [x] Verify `ferriswheel`, `pennant`, and `bunting` availability on crates.io before first publish. **Confirmed 2026-05-06** — `pennant` (HTTP 200 → free), `bunting` (free), `ferriswheel` (free). Replaces the earlier `lantern` claim, which was based on a `cargo search latern` typo and turned out to be wrong (`lantern` is taken).
- [x] Do `bunting`, `pennant`, and `ferriswheel` each have their own per-crate `README.md` for the docs.rs landing page?
      **Confirmed 2026-05-05** — all three crates have a `README.md` (title, one-liner, workspace-context paragraph, minimal example, docs.rs link, license, pointer to the workspace CHANGELOG) and the `readme = "README.md"` field is set in each `Cargo.toml`.
- [x] **`blinksy` evaluation** — resolved 2026-05-05 via `research-analyst` pass; full analysis in [`docs/blinksy-ecosystem-evaluation.md`](../blinksy-ecosystem-evaluation.md).
      Outcome: `blinksy` is complementary, not competing — different niche (spatial installations vs embedded rings), different abstraction model (stateless coordinate-driven vs stateful effect loop), and EUPL-1.2 licensing makes it un-adoptable as a Cargo dependency in any case.
      No name collisions in the `blinksy` namespace.
      Does not affect v1 publish scope, crate names, or upstream-contribution strategy (the long-term roadmap target remains `smart-leds-rs`).
      Positioning paragraphs added to `README.md` and `docs/why-yet-another-ws2812-crate.md`.
- [ ] CHANGELOG cutover style — keep workspace-level `CHANGELOG.md` as the single source of truth for all published crates and link to it from each crate's `repository` URL, or move to per-crate changelogs?
      Default for v1: keep workspace-level.
      Revisit if downstream users complain about coarse-grained release notes.
- [ ] Future ownership migration trigger — when do we transition from sole owner (`fwaibel@datenkollektiv.de`) to a GitHub team owner (e.g. `github:datenkollektiv:wheel`)?
      Candidate triggers: first external contributor's PR merged, or at the `v1.0.0` cut.

## State

- [x] Design approved
- [x] Core implementation (renames: `led-effects` → `pennant` (via `lantern` as a typo-driven intermediate) and `ws2812-pure` → `bunting`; add per-crate metadata; add `just release` recipe; per-crate READMEs)
- [x] Tests passing (`cargo publish --dry-run` clean for each of the three crates; full test suite green on host target)
- [ ] Documentation updated — README install snippets switched to versioned crates.io deps; CHANGELOG entries under `## [Unreleased]` for both renames recorded; **remaining**: cut `## [Unreleased]` to a versioned release section as part of the publish step

## Session Log

- 2026-05-05 — Feature doc created via /feature dialog.
  Captured: publish scope (three pure-logic crates), manual `just release` flow,
  version policy (continue `v0.5.x`), publish order, dual-path-version dep form,
  `led-effects` → `lantern` rename, sole ownership, pre-publish hygiene posture.
  Discovered `blinksy` during cargo-search work — queued for follow-up `research-analyst` pass.
- 2026-05-05 — Resolved the `blinksy` open question via `research-analyst` pass
  (full analysis in `docs/blinksy-ecosystem-evaluation.md`).
  Added complementary-positioning paragraph to `README.md` and a "Distinction
  from Spatial LED Frameworks" section to `docs/why-yet-another-ws2812-crate.md`.
  v1 publish scope unchanged.
- 2026-05-05 — Reconsidered the `ws2812-pure` keep-as-is decision after the
  maintainer raised crowded-namespace concerns. Brainstormed carnival-themed
  and painter-studio alternatives via `research-analyst`; ruled out `chameleon`
  (taken on crates.io + Pyramid template-engine collision), `harlequin`
  (collides with the Python SQL IDE), `kaleidoscope` (Keyboardio firmware),
  and the painter-studio set (`pigment`, `swatch`, `primer`, `canvas`, `easel`
  — all blocked or with severe external collisions). Maintainer chose
  `bunting`: free on crates.io, semantically a strung sequence of coloured
  units (matching a WS2812 strip), and consistent with the fairground theme
  alongside `ferriswheel` and `lantern`.
- 2026-05-05 — Implementation landed: both renames (`ws2812-pure` → `bunting`,
  `led-effects` → `lantern` with feature-flag rename), per-crate `README.md`
  files, `readme` / `documentation` Cargo metadata, and three `just release-*`
  recipes (`release-dry-run`, `release-dry-run-crate`, `release-publish`).
  `just release-dry-run` runs clean for all three crates: `bunting` packages
  7 files / 30.7 KiB, `lantern` 9 files / 23.0 KiB, `ferriswheel` 24 files /
  277.3 KiB. Remaining work: real `cargo publish` of the three crates,
  followed by cutting `## [Unreleased]` to a versioned release section in
  `CHANGELOG.md`.
- 2026-05-05 — Reviewer feedback addressed: README install snippets now
  default to versioned crates.io deps (with a "track main" git snippet kept
  for contributors), feature-doc state checkboxes ticked where work is done,
  the CHANGELOG-cutover open question resolved (workspace-level for v1).
- 2026-05-06 — Third rename: `lantern` → `pennant`. Maintainer manually
  re-verified `lantern` on crates.io and found it taken (HTTP 200), exposing
  that the original "free" claim came from a `cargo search latern` typo. New
  name `pennant` (a single triangular fairground flag, semantically the unit
  that `bunting` is strung from) was confirmed free via the crates.io API
  and selected over `pennon`/`ensign`/`barker` and a prefixed
  `rustyfarian-lantern` option. Implementation re-ran the established
  five-commit shape (manifest + workspace dep, consumer manifests + driver
  cfg/impl + examples, own-crate doc tests, tooling + CI + scripts, docs +
  feature doc + CHANGELOG). The CHANGELOG entry for `led-effects` → `lantern`
  was collapsed into a single `led-effects` → `pennant` entry under
  `## [Unreleased]` per Option X (the `lantern` intermediate was never
  published, so downstream consumers only need to migrate from the last
  released name to the next one). Two new feedback memories were captured:
  always re-verify name availability with the canonical spelling, and run a
  comprehensive workspace-wide grep covering examples / scripts / CI /
  tooling — both `examples/*_async.rs` (caught manually) and
  `scripts/*.sh` (caught by external PR review) had been missed by agent
  enumerations.
