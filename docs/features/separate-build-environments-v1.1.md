# Feature: Split IDF Target Outputs from Cargo Intermediates

## Status

**Document type:** evaluation record for a deferred build-config option — *not* an approved implementation plan.

Evaluated 2026-07-09 — **not recommended for adoption as written**.
The core mechanism is sound, but the change largely re-solves a capacity problem the shipped 12 GB RAM disk resize already fixed, and it breaks bootloader discovery and `clean-idf` in the process.
See [Evaluation](#evaluation-2026-07-09) for the verdict and the must-fix gates that precede any adoption.

## Context

`separate-build-environments-v1.md` routes HAL and IDF builds into separate target directories.
When `/Volumes/RustBuilds` is attached, both directories live on the shared RAM disk.
That keeps HAL and IDF artifacts from invalidating each other and avoids heavy SSD writes.

The current pressure point is capacity.
An 8 GB RAM disk is too small when four rustyfarian projects keep IDF build trees warm at the same time.
The ESP-IDF side is already partly shared through `ESP_IDF_TOOLS_INSTALL_DIR = "global"`.
That setting keeps the downloaded ESP-IDF framework, tools, and Python environment in `~/.espressif` instead of under each workspace.

The remaining large IDF footprint is not the downloaded framework.
It is the per-project Cargo target tree, especially the `esp-idf-sys` build-script output.
In `esp-idf-sys 0.37.2`, the native builder creates its generated CMake project under Cargo `OUT_DIR` and hard-codes the CMake build directory as `<OUT_DIR>/build`.
There is no supported `esp-idf-sys` environment variable that moves only that CMake build directory into a shared location.

## Goal

Reduce RAM disk pressure without merging incompatible HAL and IDF target directories.
Keep the fast-path outputs that developers touch on the RAM disk when available.
Move bulky IDF build intermediates to persistent disk when the Cargo toolchain supports it.

## Non-Goals

- Do not share one complete IDF `target-dir` across multiple projects.
- Do not move `~/.cargo`, `~/.rustup`, `~/.cache/sccache`, or `~/.espressif` onto the RAM disk.
- Do not symlink internal `esp-idf-sys` subdirectories unless the supported Cargo option fails in practice.
- Do not relax exact-pinned ESP dependency versions as part of this change.

## Recommended Option

Use Cargo's `build.build-dir` setting for IDF recipes.
Keep `--target-dir {{ idf_dir }}` as the final artifact directory.
Set `build.build-dir` to a persistent cache path outside the RAM disk.

`build.build-dir` is stable as of Cargo 1.91.0.
The pinned `+esp` toolchain (`cargo 1.95.0-nightly`, 2026-03-21) accepts it with no `-Z` flag and no `[unstable]` entry.
Cargo's build cache reference confirms build-script `OUT_DIR` is an intermediate artifact that follows `build.build-dir`, while final artifacts stay under `--target-dir` — so the `esp-idf-sys` CMake tree does relocate as intended.
The template placeholders are `{workspace-root}`, `{cargo-cache-home}`, and `{workspace-path-hash}`; `{workspace-path-hash}` is the correct literal spelling.

The intended shape is:

```sh
cargo +esp check -p rustyfarian-esp-idf-ws2812 \
    --target riscv32imac-esp-espidf \
    --target-dir /Volumes/RustBuilds/targets/idf/rustyfarian-ws2812 \
    --config 'build.build-dir = "/Users/fluffi/Library/Caches/rustyfarian-cargo-build/{workspace-path-hash}"'
```

The `{workspace-path-hash}` template keeps projects isolated while allowing one shared parent cache directory.
It avoids collisions between projects with different manifests, `sdkconfig.defaults`, profiles, features, or chip settings.
It also keeps `cargo clean --target-dir {{ idf_dir }}` from accidentally deleting the persistent intermediate cache.

## Evaluation (2026-07-09)

This section records the review verdict, the facts that were verified, and the work that must precede adoption.

### Verdict

The core mechanism works, but the change is not recommended as written.
It largely re-solves a capacity problem the shipped 12 GB resize already fixed, and to do so it breaks bootloader discovery and `clean-idf` and reverses the RAM disk's wear-reduction rationale for the single heaviest writer.
Treat it as a warm-cache-reuse optimisation, not a capacity fix.

### Verified as sound

- `build.build-dir` is the correct knob and is stable as of Cargo 1.91.0; the `+esp` toolchain needs no `-Z` flag.
- Build-script `OUT_DIR` (the `esp-idf-sys` CMake tree) does follow `build.build-dir` onto persistent disk while final artifacts stay under `--target-dir`.
- `{workspace-path-hash}` is a real, correctly spelled template placeholder, and `--config 'build.build-dir=…'` is a valid way to pass it.

The first two points are confirmed from primary Cargo documentation; they are not yet reproduced on this repo's toolchain (see below).

### Not yet validated (inferred, needs empirical proof)

These are plausible from the docs and source but were **not** reproduced end-to-end during this evaluation:

- That `esp-idf-sys`'s CMake tree writes exclusively under Cargo's `OUT_DIR` (and therefore follows `build.build-dir`), rather than into a fixed or manifest-relative path.
- That `{workspace-path-hash}` substitution behaves identically when passed via `--config` as it does from a config file.
- Where `bootloader.bin` and `partition-table.bin` actually land once `build.build-dir` is set — under `idf_dir` (final artifacts) or under the build-dir cache. This is the pivot for the bootloader-discovery blocker below.
- That the internal build-dir layout (`<build-dir>/debug/build/<pkg>-<hash>/out`) is stable enough to hard-code in scripts; the Cargo docs explicitly disclaim this layout as internal.

### Blocking problems

1. **Bootloader discovery breaks (critical).**
   `scripts/lib.sh:find_idf_bootloader` globs for `bootloader.bin` under `{{ idf_dir }}/…/build/esp-idf-sys-*/out/build/bootloader/`.
   That file lives inside `esp-idf-sys`'s `OUT_DIR`, which this change moves to the persistent cache.
   The glob then finds nothing, `run-example.sh` falls back to espflash's bundled bootloader, and the build re-hits the IDF v5.3.3 32 KB-page MMU mismatch that the `--bootloader` override exists to avoid.
2. **`clean-idf` becomes a silent no-op.**
   It hard-codes `rm -rf {{ idf_dir }}/…/build/esp-idf-sys-*/`; once that tree lives in the cache, the recipe deletes nothing and leaves stale CMake state behind.
3. **embuild is already known to be path-fragile.**
   `.cargo/config.toml` documents that embuild derives the project root from the target-dir's parent and already needed the explicit `ESP_IDF_SDKCONFIG_DEFAULTS` pin to cope with an off-workspace target-dir.
   This change introduces a third location (the cache) far from both workspace and target-dir, which the existing workaround does not cover.

### Framing corrections

- **Capacity is already solved.**
  The 8 GB-full incident was resolved by growing the shared disk to 12 GB (verified: the esp32 example uses ~1.1 GB; four warm projects land near 5.5–6.5 GB of 12 GB).
  The real benefit of this proposal is cache survival across RAM disk remounts — a marginal convenience, not a capacity crisis.
- **It reverses the wear-reduction rationale.**
  v1 chose the RAM disk to spare the SSD from Rust's write load; the `esp-idf-sys` CMake build is the heaviest single writer, and this change routes precisely that back onto SSD.
  The one-line "trades some speed" risk understates this.

### Recommendation

Prefer, in order:

1. Stay on the shipped 12 GB resize — it meets every stated acceptance criterion at zero risk.
2. If pressure genuinely returns, route only the heavy `xtensa-esp32-espidf` build's build-dir to SSD (a chip-conditional rule), rather than a wholesale path-model change.
3. Only then pursue full adoption, gated on the must-fix items below.

### Must-fix gates before adoption

- [ ] Update `find_idf_bootloader` to also search the build-dir, or empirically confirm where `bootloader.bin` / `partition-table.bin` actually land.
- [ ] Rewrite `clean-idf` to target the cache, and decide between extending `clean-idf` and adding a separate `clean-idf-cache`.
- [ ] Prove the change with a real **flash**, not `just check-idf` — the risk lives in embuild's runtime path resolution.
- [ ] Replace the nested-quote `idf_build_config` with a small wrapper script; verify with `just --dry-run`.

## Expected Layout

With the RAM disk attached:

| Data                                                | Location                                                    | Persistence      |
|:----------------------------------------------------|:------------------------------------------------------------|:-----------------|
| HAL final artifacts                                 | `/Volumes/RustBuilds/targets/hal/<project>`                 | Ephemeral        |
| IDF final artifacts                                 | `/Volumes/RustBuilds/targets/idf/<project>`                 | Ephemeral        |
| IDF Cargo intermediates and `esp-idf-sys` `OUT_DIR` | `~/Library/Caches/rustyfarian-cargo-build/<workspace hash>` | Persistent cache |
| ESP-IDF tools and framework                         | `~/.espressif`                                              | Persistent       |
| Rust registry, git sources, and toolchains          | `~/.cargo`, `~/.rustup`                                     | Persistent       |
| Optional compiler cache                             | `~/.cache/sccache`                                          | Persistent       |

Without the RAM disk attached, `idf_dir` still falls back to `target/idf`.
The persistent `build.build-dir` cache may still be used if the recipe passes the same config.

## Why Not Share the Whole IDF Target Directory?

Sharing one complete IDF `target-dir` across projects is risky.
Cargo target directories contain profile, target, dependency, build-script, and root-package state that is not designed as a cross-project public cache.
`esp-idf-sys` also copies generated files such as `bootloader.bin` and `partition-table.bin` into the target tree.
Those files can vary by target, chip, profile, SDK configuration, and root crate.

The v1 design already exists because HAL and IDF artifacts are incompatible.
This v1.1 proposal keeps that invariant and only moves intermediates through a supported Cargo knob.

## Secondary Option: Trim ESP-IDF Components

Use `ESP_IDF_COMPONENTS` to reduce what `esp-idf-sys` asks ESP-IDF to build.
The native builder accepts a component list and lets ESP-IDF include transitive dependencies.

The review process should first discover the current component set:

```sh
cargo +esp check -vv -p rustyfarian-esp-idf-ws2812 \
    --target riscv32imac-esp-espidf \
    --target-dir /Volumes/RustBuilds/targets/idf/rustyfarian-ws2812
```

Then inspect the `Built components:` line emitted by `esp-idf-sys`.
A minimal list can be tested from there.

This option reduces both disk use and build time, but it has more behavioral risk than `build.build-dir`.
Future IDF examples may need Wi-Fi, networking, logging, NVS, or other ESP-IDF components that are not required by the current RMT-only driver.

## Last-Resort Option: Symlink `esp-idf-sys` Build Subtrees

A script could replace the `esp-idf-sys` `OUT_DIR/build` directory with a symlink into a persistent cache.
The cache key would need to include at least project path, target triple, profile, MCU, ESP-IDF version, `sdkconfig.defaults`, selected features, and relevant environment variables.

This is intentionally not recommended for the first implementation.
`esp-idf-sys` expects its generated CMake project, SDK configuration, CMake build tree, binding inputs, and metadata to live under one Cargo `OUT_DIR`.
Moving only part of that tree is more fragile than moving Cargo's intermediate directory as a whole.

## Implementation Sketch

> **Exploratory — not approved.**
> This sketch shows the intended shape *if* the must-fix gates in the [Evaluation](#evaluation-2026-07-09) are cleared first.
> It is not a green-lit implementation plan.

Add an IDF build-dir variable to the justfile.
Use a persistent default under `~/Library/Caches` on macOS.
Keep the value project-specific by relying on Cargo's `{workspace-path-hash}` template.

Possible justfile shape:

```sh
idf_build_dir := env_var_or_default("RUSTYFARIAN_IDF_BUILD_DIR", env_var("HOME") + "/Library/Caches/rustyfarian-cargo-build/{workspace-path-hash}")
idf_build_config := "--config 'build.build-dir = \"" + idf_build_dir + "\"'"
```

Every IDF recipe would continue to pass `--target-dir {{ idf_dir }}`.
Every IDF recipe would also pass the build-dir config.

Example:

```sh
cargo +esp check -p rustyfarian-esp-idf-ws2812 \
    --target {{ idf_target }} \
    --target-dir {{ idf_dir }} \
    --config 'build.build-dir = "{{ idf_build_dir }}"'
```

The exact quoting should be verified in `just --dry-run` before landing code.
If Just quoting gets awkward, a small wrapper script may be clearer than a deeply escaped justfile variable.

## Validation Plan

1. Record RAM disk usage before the change with a warm IDF build.
2. Run `cargo +esp --version` and confirm the active `+esp` Cargo accepts `build.build-dir`.
3. Run `just check-idf` with the new build-dir config.
4. Confirm `esp-idf-sys` `OUT_DIR` lands under the persistent build directory, not under `/Volumes/RustBuilds`.
5. Confirm final IDF artifacts still land under `{{ idf_dir }}`.
6. **Locate the flash-critical artifacts before any flash attempt.** Find `bootloader.bin` and `partition-table.bin` under the new layout and confirm whether they sit under `{{ idf_dir }}` or under the build-dir cache. This determines whether `scripts/lib.sh:find_idf_bootloader` still resolves them.
7. **Run a real flash** (`just run-example …`), not just `just check-idf` — the bootloader/embuild path breakage only surfaces at flash time. Confirm the correct `--bootloader` is passed and the board boots (no IDF v5.3.3 MMU page-offset error).
8. Run `just clean-idf` and verify it removes the relocated `esp-idf-sys` build tree (not a silent no-op) and behaves correctly on a second run.
9. Detach the RAM disk and verify the recipe still falls back to `target/idf`.
10. Reattach the RAM disk and verify a warm persistent build-dir reduces rebuild cost without filling the RAM disk.

## Risks

- `build.build-dir` is stable as of Cargo 1.91.0 and needs no `-Z` flag on the pinned `+esp` toolchain (verified 2026-07-09).
- The unstable `-Z build-dir-new-layout` is a separate "v2" reorganisation with a rocky stabilisation history; it is **not** required for this change and should not be used.
- Moving the `esp-idf-sys` CMake build (the heaviest single writer) to SSD reverses v1's SSD-wear-reduction rationale, not merely "some speed" — see [Evaluation](#evaluation-2026-07-09).
- `just clean-idf` semantics become less complete if intermediates are intentionally outside `idf_dir`.
- RustRover or rust-analyzer invocations that do not use the justfile will not automatically pick up this split.

## Open Questions

- Should `RUSTYFARIAN_IDF_BUILD_DIR` be supported as a shared override across all rustyfarian repos?
- Should `just doctor` report the resolved IDF build-dir and its disk usage?
- Should `just clean-idf` leave the persistent build-dir alone by default, or should a separate `just clean-idf-cache` remove it?
- Should this be macOS-only initially, or should Linux use `${XDG_CACHE_HOME:-$HOME/.cache}` from the first implementation?
- Is `ESP_IDF_COMPONENTS` worth adopting after `build.build-dir`, or should it remain a manual optimization for IDF-heavy examples?

## Acceptance Criteria

- [ ] IDF recipes keep `--target-dir {{ idf_dir }}` and preserve v1 HAL/IDF isolation.
- [ ] Warm IDF builds no longer require the full `esp-idf-sys` CMake build tree to live on `/Volumes/RustBuilds`.
- [ ] Four projects can keep a warm IDF state without requiring a large RAM disk.
- [ ] `ESP_IDF_TOOLS_INSTALL_DIR = "global"` remains unchanged.
- [ ] `just check-idf` passes with the new configuration.
- [ ] `just clean-idf` behavior is documented after the persistent build-dir decision is made.

## Session Log

- 2026-07-09 - Investigated sharing ESP-IDF build outputs after an 8 GB RAM disk became too small for four projects.
- 2026-07-09 - Confirmed the repo already shares ESP-IDF tools through `ESP_IDF_TOOLS_INSTALL_DIR = "global"`.
- 2026-07-09 - Confirmed `esp-idf-sys 0.37.2` hard-codes its native CMake build under Cargo `OUT_DIR`.
- 2026-07-09 - Proposed `build.build-dir` as the least invasive split for review.
- 2026-07-09 - Evaluated. Confirmed `build.build-dir` is stable (Cargo 1.91.0), needs no `-Z` flag on the `+esp` toolchain, and does relocate `esp-idf-sys`'s `OUT_DIR` CMake tree. Found blocking issues: bootloader discovery and `clean-idf` break, embuild path-fragility is uncovered, and the capacity problem was already solved by the 12 GB resize. Verdict: not recommended as written; treat as a warm-cache optimisation gated on must-fix items.
- 2026-07-09 - Incorporated PR review feedback: added an explicit document-type label, a "Not yet validated" split between confirmed facts and inferences, flagged the Implementation Sketch as exploratory, and added flash-artifact-location and real-flash steps to the Validation Plan. Cross-linked v1 (adopted) ↔ v1.1 (deferred). Declined the file rename to preserve the v1/v1.1 convention and inbound links. Also updated `scripts/ramdisk.sh` to name `RUSTBUILDS_RAMDISK_SIZE_GB` in its size-validation error.
