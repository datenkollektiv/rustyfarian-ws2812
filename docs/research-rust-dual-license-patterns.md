# Rust Dual-Licence File Patterns for Workspace Publishing

## Context

Before the first crates.io publish wave for `rustyfarian-ws2812`, we must decide how to
distribute the canonical `LICENSE-MIT` and `LICENSE-APACHE` texts — which already exist at
the workspace root — so that each published `.crate` archive actually carries them.

The workspace declares `license = "MIT OR Apache-2.0"` in `[workspace.package]`.
The three crates targeted for the v1 publish wave are `bunting`, `pennant`, and `ferriswheel`.
Three further crates (`rustyfarian-esp-idf-ws2812`, `rustyfarian-esp-hal-ws2812`,
`rustyfarian-avr-ws2812`) will follow in a later wave.

### The packaging problem

`cargo publish` packages files from the **per-crate directory** only, not from the workspace
root.
A `LICENSE-MIT` sitting at the workspace root is therefore silently absent from every
`.crate` archive unless something bridges the gap.

### Three known approaches

**Approach A — Symlinks.**
Place `crates/ferriswheel/LICENSE-MIT -> ../../LICENSE-MIT` (and the Apache equivalent) in
each crate directory.
`cargo package` follows symlinks on Unix and copies the target file content into the archive.
On Windows, Git does not create symlinks by default unless `core.symlinks = true` is enabled;
if a contributor checks out the repository on Windows without that setting, the symlink becomes
a plain text file containing the path string, which breaks the publish.
This is a known open issue in Cargo (rust-lang/cargo#5664, still open as of May 2026).

**Approach B — Hard copies.**
Place a verbatim copy of `LICENSE-MIT` and `LICENSE-APACHE` in every crate directory.
Robust across all operating systems and CI environments.
Results in duplication (two extra files per crate, twelve extra files across a six-crate
workspace).
The copies never drift from the originals in practice because licence texts are immutable.

**Approach C — Workspace-root only, rely on SPDX field.**
Ship no licence text files inside the `.crate` archive.
The SPDX `license` field in `Cargo.toml` is machine-readable and crates.io displays it.
Compliance scanners and downstream consumers that vendor crates may flag the absence of the
actual licence text.
Both the MIT and Apache-2.0 licences require the text to accompany redistributed software;
relying only on the SPDX identifier does not strictly satisfy that requirement.

---

## Comparison Table

| Project             | Licence files at root                                  | Per-crate licence files                                                  | Type                             | Pointer `LICENSE` | Notes                                                                                                 |
|:--------------------|:-------------------------------------------------------|:-------------------------------------------------------------------------|:---------------------------------|:------------------|:------------------------------------------------------------------------------------------------------|
| **tokio**           | `LICENSE` (MIT only)                                   | `LICENSE` in each subcrate                                               | Copy                             | No                | MIT-only; single file per crate; no dual licence; sub-crates mirror root exactly                      |
| **serde**           | `LICENSE-MIT`, `LICENSE-APACHE`                        | `LICENSE-MIT`, `LICENSE-APACHE` in `serde/` and `serde_derive/`          | Copy (verified via GitHub API)   | No                | Both files present in every published crate (confirmed on docs.rs)                                    |
| **rust-lang/cargo** | `LICENSE-MIT`, `LICENSE-APACHE`, `LICENSE-THIRD-PARTY` | `LICENSE-MIT`, `LICENSE-APACHE` in sub-crates (e.g. `crates/cargo-util`) | Symlinks (PR #12953) then copies | No                | Cargo itself retroactively added symlinks to fix missing licence files in published credential crates |
| **rayon**           | `LICENSE-MIT`, `LICENSE-APACHE`                        | `LICENSE-MIT`, `LICENSE-APACHE` in `rayon-core/`                         | Copy (verified via GitHub API)   | No                | Workspace inheritance for `license` field; no `include` field; files at crate root suffice            |
| **hyper**           | `LICENSE` (MIT only)                                   | N/A — single-crate repo                                                  | N/A                              | No                | Single crate; MIT only                                                                                |
| **anyhow**          | `LICENSE-MIT`, `LICENSE-APACHE`                        | N/A — single-crate repo                                                  | N/A                              | No                | Both files present in published crate; no `include` field needed                                      |
| **clap**            | `LICENSE-MIT`, `LICENSE-APACHE`                        | `LICENSE-MIT`, `LICENSE-APACHE` in `clap_builder/`, `clap_derive/` etc.  | Copy (verified via GitHub API)   | No                | Uses `include = ["...", "LICENSE*", ...]` glob in each subcrate `Cargo.toml` to guarantee inclusion   |
| **ripgrep**         | `LICENSE-MIT`, `UNLICENSE`, `COPYING`                  | `LICENSE-MIT`, `UNLICENSE` in sub-crates (e.g. `crates/grep/`)           | Copy                             | No                | MIT + Unlicense dual; not relevant as a dual MIT/Apache model                                         |
| **embassy**         | `LICENSE-MIT`, `LICENSE-APACHE`, `LICENSE-CC-BY-SA`    | None (verified via GitHub API and docs.rs)                               | Absent                           | No                | Relies solely on `license = "MIT OR Apache-2.0"` SPDX field; no licence text in published crates      |
| **esp-hal**         | `LICENSE-MIT`, `LICENSE-APACHE`                        | None (verified on docs.rs 1.1.0)                                         | Absent                           | No                | Uses `exclude` field; no `include`; no licence files reach the `.crate` archive                       |
| **esp-idf-hal**     | `LICENSE-MIT`, `LICENSE-APACHE`                        | N/A — single-crate repo                                                  | N/A                              | No                | Both files at root, published as-is                                                                   |
| **smart-leds**      | `LICENSE-MIT`, `LICENSE-APACHE`                        | N/A — single-crate repo                                                  | N/A                              | No                | Single crate; both files at root                                                                      |

---

## Pattern Summary

### Dominant pattern: hard copies in every published crate

Among the dual-licensed workspaces examined, **serde, rayon, cargo (credential sub-crates),
and clap all place physical copies of `LICENSE-MIT` and `LICENSE-APACHE` inside each
published crate directory**.
The copies end up in the `.crate` archive because `cargo package` includes all non-hidden,
non-ignored files from the crate root when no `include` field is set —
or because the crate's `include` field lists `"LICENSE*"` as a glob pattern.

Clap is the most explicit: each sub-crate `Cargo.toml` contains
`include = ["...", "LICENSE*", "README.md"]`, which guarantees the files are included even if
someone later adds a restrictive `include` list.

The rust-lang/cargo repository initially used symlinks (added retroactively in PR #12953) to
fix the same gap.
The PR author cited "this is a requirement for both the Apache-2.0 and the MIT license" as
the motivation.

### Minority pattern: SPDX field only

**Embassy and esp-hal both ship zero licence text files in their `.crate` archives.**
Both declare `license = "MIT OR Apache-2.0"` in `Cargo.toml` and rely on crates.io displaying
that metadata.
This is pragmatic for deeply embedded-systems projects where compliance scanning of vendored
crates is less common, but it does not strictly satisfy the redistribution requirement of
either licence.

### No pointer `LICENSE` file found

None of the twelve projects examined keeps a short pointer `LICENSE` file alongside the named
`LICENSE-MIT` and `LICENSE-APACHE` files.
The API Guidelines boilerplate places the dual-licence notice in the `README.md` instead, which
is the de facto standard.

---

## Trade-offs

<details>
<summary><strong>Approach A — Symlinks</strong></summary>

**Pros**
- Single source of truth; the workspace-root files are never duplicated.
- `cargo package` on Unix flattens the symlink and embeds the full licence text.

**Cons**
- Broken on Windows unless `core.symlinks = true` is set (rust-lang/cargo#5664, open since
  2018).
- CI on GitHub Actions Windows runners may silently include a text file containing the path
  string `../../LICENSE-MIT` rather than the licence text.
- The Cargo team closed a request to handle this automatically (rust-lang/cargo#13328) as
  "not planned".
- The rust-lang/cargo project itself used symlinks briefly but must document the Windows risk
  for contributors.

</details>

<details>
<summary><strong>Approach B — Hard copies</strong></summary>

**Pros**
- Works on all operating systems and all CI environments without configuration.
- Compliance scanners (`cargo-deny`, `cargo-about`, FOSSA, and similar) find the texts in the
  expected location every time.
- Licence texts are immutable; copies never become stale.
- `cargo package --list` will always confirm the files are present.
- No dependency on git configuration (`core.symlinks`).

**Cons**
- Twelve extra files across a six-crate workspace (two per crate).
- A contributor updating a copyright year or adding a notice must update all copies; however,
  neither MIT nor Apache-2.0 texts change, so this concern is theoretical.

</details>

<details>
<summary><strong>Approach C — SPDX field only</strong></summary>

**Pros**
- Zero extra files; trivially maintained.
- crates.io displays the SPDX identifier prominently.

**Cons**
- Does not satisfy the redistribution requirement of the MIT or Apache-2.0 licence.
- Both licences require the licence notice to accompany the software when distributed.
- Compliance scanners that vendor the `.crate` archive (FOSSA, Black Duck, etc.) will flag the
  absence of the text and require a manual clarification.
- Embassy and esp-hal use this approach, but as embedded-systems projects with specialist
  audiences; a library published for general use faces stricter scrutiny.

</details>

---

## Mechanism: how `cargo package` picks up licence files

When a crate's `Cargo.toml` has **no `include` field**, `cargo package` includes all tracked,
non-hidden files in the crate root by default.
A `LICENSE-MIT` placed directly in the crate directory will therefore be included automatically.

When an `include` field is present, only matching files are packaged.
The safe pattern (as used by clap) is `"LICENSE*"` as a glob entry in `include`,
which captures `LICENSE-MIT` and `LICENSE-APACHE` regardless of exact naming.

The `license-file` manifest field is mutually exclusive with `license`; using `license-file`
for dual-licensed projects would require setting it to a single file and is therefore
unsuitable here.

Cargo does not currently inherit `include` from `workspace.package` in a way that affects all
member crates automatically without each crate opting in with `include.workspace = true`.

---

## Recommendation

**Use hard copies (Approach B), combined with an explicit `"LICENSE*"` glob in each crate's
`include` field.**

Concretely, for each of the six crates:

1. Copy `LICENSE-MIT` and `LICENSE-APACHE` from the workspace root into the crate directory
   (e.g. `crates/ferriswheel/LICENSE-MIT`).
2. In each crate's `Cargo.toml`, add or extend the `include` field:

   ```toml
   [package]
   include = ["src/**/*", "LICENSE*", "README.md"]
   ```

   If a crate already has an `include` field, append `"LICENSE*"` to the list.
   If a crate has no `include` field, adding one is still advisable so that future
   additions of large files (test fixtures, generated code) do not bloat the published archive.

3. Run `cargo package --list -p ferriswheel` (and equivalently for each publish-target crate)
   to confirm `LICENSE-MIT` and `LICENSE-APACHE` appear in the output before pushing to
   crates.io.

**Why not symlinks?** The Windows symlink bug (rust-lang/cargo#5664) is open and unresolved.
We cannot control whether contributors or automated tooling checks out the repository with
`core.symlinks = true`.
Symlinks also introduce a class of failure — licence text silently replaced by a path string —
that is hard to detect without explicit `cargo package --list` inspection.

**Why not SPDX-only?** `rustyfarian-ws2812` targets general-purpose publishing on crates.io.
Downstream users who vendor crates for embedded projects routinely run compliance tools.
Including the licence text costs twelve small static files and eliminates any downstream
compliance friction.

**On the pointer `LICENSE` file.** Do not add one.
No examined project uses a pointer `LICENSE` file alongside the named pair.
The dual-licence notice already appears in the workspace `README.md`.
A separate pointer file adds clutter without providing information that is not already conveyed
by the `license` field and the two named files.

---

## Sources

- [tokio-rs/tokio — repository root](https://github.com/tokio-rs/tokio)
- [tokio-rs/tokio — tokio/ subcrate](https://github.com/tokio-rs/tokio/tree/master/tokio)
- [tokio-rs/tokio — tokio-util/ subcrate](https://github.com/tokio-rs/tokio/tree/master/tokio-util)
- [serde-rs/serde — repository root](https://github.com/serde-rs/serde)
- [serde-rs/serde — serde/ subcrate (GitHub API)](https://api.github.com/repos/serde-rs/serde/contents/serde)
- [serde-rs/serde — serde_derive/ subcrate](https://github.com/serde-rs/serde/tree/master/serde_derive)
- [serde 1.0.219 — docs.rs source view](https://docs.rs/crate/serde/1.0.219/source/)
- [serde_derive 1.0.219 — docs.rs source view](https://docs.rs/crate/serde_derive/1.0.219/source/)
- [rust-lang/cargo — repository root](https://github.com/rust-lang/cargo)
- [rust-lang/cargo — crates/cargo-util/ (GitHub API)](https://api.github.com/repos/rust-lang/cargo/contents/crates/cargo-util)
- [cargo-util latest — docs.rs source view](https://docs.rs/crate/cargo-util/latest/source/)
- [rust-lang/cargo PR #12953 — credential: include license files](https://github.com/rust-lang/cargo/pull/12953)
- [rust-lang/cargo issue #5664 — symlinks on Windows](https://github.com/rust-lang/cargo/issues/5664)
- [rust-lang/cargo issue #3537 — published crates should include LICENSE](https://github.com/rust-lang/cargo/issues/3537)
- [rust-lang/cargo issue #13328 — auto-symlink LICENSE in new workspace member (closed not planned)](https://github.com/rust-lang/cargo/issues/13328)
- [rust-lang/cargo PR #7905 — better support for license-file](https://github.com/rust-lang/cargo/pull/7905)
- [rayon-rs/rayon — repository root](https://github.com/rayon-rs/rayon)
- [rayon-rs/rayon — rayon-core/ (GitHub API)](https://api.github.com/repos/rayon-rs/rayon/contents/rayon-core)
- [rayon latest — docs.rs source view](https://docs.rs/crate/rayon/latest/source/)
- [rayon-core latest — docs.rs source view](https://docs.rs/crate/rayon-core/latest/source/)
- [hyperium/hyper — repository root](https://github.com/hyperium/hyper/tree/master)
- [dtolnay/anyhow — repository root](https://github.com/dtolnay/anyhow)
- [anyhow latest — docs.rs source view](https://docs.rs/crate/anyhow/latest/source/)
- [clap-rs/clap — repository root](https://github.com/clap-rs/clap/tree/master)
- [clap-rs/clap — clap_builder/ (GitHub API)](https://api.github.com/repos/clap-rs/clap/contents/clap_builder)
- [clap 4.5.40 Cargo.toml — docs.rs source view](https://docs.rs/crate/clap/4.5.40/source/Cargo.toml)
- [clap 4.6.1 — docs.rs source view](https://docs.rs/crate/clap/latest/source/)
- [clap_derive 4.6.1 — docs.rs source view](https://docs.rs/crate/clap_derive/latest/source/)
- [BurntSushi/ripgrep — repository root](https://github.com/BurntSushi/ripgrep)
- [ripgrep — crates/grep/](https://github.com/BurntSushi/ripgrep/tree/master/crates/grep)
- [embassy-rs/embassy — repository root](https://github.com/embassy-rs/embassy/tree/main)
- [embassy-rs/embassy — embassy-executor/ (GitHub API)](https://api.github.com/repos/embassy-rs/embassy/contents/embassy-executor)
- [embassy-executor 0.10.0 — docs.rs source view](https://docs.rs/crate/embassy-executor/0.10.0/source/)
- [embassy-executor 0.7.0 — docs.rs source view](https://docs.rs/crate/embassy-executor/0.7.0/source/)
- [esp-rs/esp-hal — repository root](https://github.com/esp-rs/esp-hal/tree/main)
- [esp-rs/esp-hal — esp-hal/ subcrate](https://github.com/esp-rs/esp-hal/tree/main/esp-hal)
- [esp-hal 1.1.0 — docs.rs source view](https://docs.rs/crate/esp-hal/1.1.0/source/)
- [esp-rs/esp-idf-hal — repository root](https://github.com/esp-rs/esp-idf-hal/tree/master)
- [smart-leds-rs/smart-leds — repository root](https://github.com/smart-leds-rs/smart-leds/tree/master)
- [Cargo manifest reference — license and license-file fields](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields)
- [Cargo manifest reference — include and exclude fields](https://doc.rust-lang.org/cargo/reference/manifest.html#the-include-and-exclude-fields)
- [Rust API Guidelines — Necessities (C-PERMISSIVE)](https://rust-lang.github.io/api-guidelines/necessities.html)
- [crate-ci/cargo-release Cargo.toml — "LICENSE*" include pattern](https://github.com/crate-ci/cargo-release/blob/master/Cargo.toml)

---

*Research date: 2026-05-05*
