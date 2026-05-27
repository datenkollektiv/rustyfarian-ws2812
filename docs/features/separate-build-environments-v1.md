# Feature: Separate Build Environments v1

Keep `esp-hal` (no\_std / bare-metal) and `esp-idf` (std) build artifacts in
separate directories on a RAM disk.
The justfile auto-detects whether the RAM disk is attached and routes each recipe
to the correct target dir — no `.envrc` or `direnv` required.

## Decisions

| Decision                                                            | Reason                                                                                                                                              | Rejected Alternative                                                                           |
|:--------------------------------------------------------------------|:----------------------------------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------|
| RAM disk for `target/`                                              | Fast I/O eliminates SSD wear from Rust's heavy write load; `target/` is ephemeral and safe to lose on reboot                                        | SSD-backed `target/` — slow on large workspaces, accelerates SSD degradation                   |
| Separate target dirs per runtime (`targets/hal/` vs `targets/idf/`) | IDF (std) and HAL (no\_std) produce incompatible artifacts; sharing a single `target/` causes full rebuilds on every switch                         | Single shared `target/` — switching runtimes triggers complete recompilation                   |
| Host/pure crates stay in `./target`                                 | Pure crates (`bunting`, `ferriswheel`, `pennant`) compile fast on the host target and don't share artifacts with hal or idf; no separate dir needed | Third RAM-disk slot for host — extra complexity with no measurable benefit                     |
| AVR builds isolated by subdirectory                                 | `examples/avr-nano-rainbow/` runs `cargo` from its own directory, giving it a separate `target/` automatically                                      | Explicit `avr_dir` variable — unnecessary; the `cd` already provides isolation                 |
| justfile decides target dirs — no `.envrc` needed                   | Single source of truth; works identically locally and in CI without any shell setup; `path_exists` auto-detects the RAM disk                        | `.envrc` + direnv — adds a dependency, requires `direnv allow`, breaks CI without extra config |
| `sccache` shared between both runtimes (optional)                   | Compiler output is reusable across IDF and HAL; one cache serves both without conflict                                                              | Per-runtime caches — miss cross-runtime hits; no caching — cold starts after every reboot      |
| RAM disk managed via `just ramdisk attach / detach`                 | Self-documenting, idempotent, discoverable via `just --list`                                                                                        | Shell script or launch agent — opaque, easy to forget                                          |

## Constraints

- macOS only — uses `hdiutil attach` and `diskutil erasevolume` for RAM disk creation
- RAM disk is lost on reboot; a cold start rebuilds from scratch (sccache warms subsequent builds)
- These paths must stay **persistent** (never on the RAM disk):
  - `~/.cargo` — registry and git sources
  - `~/.rustup` — toolchains
  - `~/.cache/sccache` — sccache store
  - `~/.espressif` — Espressif toolchain and ESP-IDF (`ESP_IDF_TOOLS_INSTALL_DIR = "global"`, shared across projects)
- `sccache` is optional; set `RUSTC_WRAPPER=sccache` in your shell profile to enable it
- No `direnv` or `.envrc` required
- The `clean-idf` recipe must use `{{ idf_dir }}` instead of hardcoded `target/` once this feature lands

## How It Works

Two justfile variables resolve at parse time using `path_exists`:

```
ramdisk := "/Volumes/RustBuilds"
hal_dir  := if path_exists(ramdisk + "/targets/hal") == "true" { ramdisk + "/targets/hal/" + file_name(justfile_directory()) } else { "target" }
idf_dir  := if path_exists(ramdisk + "/targets/idf") == "true" { ramdisk + "/targets/idf/" + file_name(justfile_directory()) } else { "target" }
```

Every `cargo` invocation in a HAL recipe gets `--target-dir {{ hal_dir }}`.
Every `cargo` invocation in an IDF recipe gets `--target-dir {{ idf_dir }}`.
Pure/host recipes (`verify`, `test`, `clippy`, etc.) use the default `./target` — no `--target-dir` needed.
When the RAM disk is not attached, `hal_dir` and `idf_dir` both resolve to `./target`.

### Environment Map

| Recipes                                                                            | Target dir variable                            | Toolchain    |
|:-----------------------------------------------------------------------------------|:-----------------------------------------------|:-------------|
| `check-hal`, `clippy-hal`, `build-example` (hal), `run-example` (hal)              | `hal_dir`                                      | stable       |
| `check-idf`, `clippy-idf`, `build-all`, `build-example` (idf), `run-example` (idf) | `idf_dir`                                      | `+esp`       |
| `verify`, `test`, `check`, `clippy`, `doc` (pure crates)                           | `./target` (default)                           | stable       |
| AVR recipes (`check-avr-target`, `build-avr-example`, etc.)                        | `./target` inside `examples/avr-nano-rainbow/` | `+nightly-*` |

## Just Recipes

```sh
just doctor           # show RAM disk status, resolved target dirs, sccache
just ramdisk attach   # create and mount the RAM disk (idempotent, 6 GB default)
just ramdisk detach   # eject the RAM disk and free memory
```

`just doctor` output with RAM disk attached:

```
  ramdisk    ok      /Volumes/RustBuilds
  hal target ok      /Volumes/RustBuilds/targets/hal/rustyfarian-ws2812
  idf target ok      /Volumes/RustBuilds/targets/idf/rustyfarian-ws2812
  sccache    ok      sccache 0.8.1
```

`just doctor` output without RAM disk:

```
  ramdisk    MISSING  run: just ramdisk attach
  hal target fallback target/
  idf target fallback target/
  sccache    MISSING  run: brew install sccache  (optional, speeds up cold builds)
```

`just doctor` is informational and always exits 0 — it is a status display, not a gate.

## `clean-idf` Adaptation

The current `clean-idf` recipe has hardcoded `target/` paths:

```sh
rm -rf target/riscv32imac-esp-espidf/debug/build/esp-idf-sys-*/
rm -rf target/riscv32imc-esp-espidf/debug/build/esp-idf-sys-*/
rm -rf target/xtensa-esp32-espidf/debug/build/esp-idf-sys-*/
```

Once `idf_dir` is wired up, these must become:

```sh
rm -rf {{ idf_dir }}/riscv32imac-esp-espidf/debug/build/esp-idf-sys-*/
rm -rf {{ idf_dir }}/riscv32imc-esp-espidf/debug/build/esp-idf-sys-*/
rm -rf {{ idf_dir }}/xtensa-esp32-espidf/debug/build/esp-idf-sys-*/
```

## Open Questions

None.

## State

- [x] Design approved
- [ ] `just doctor`, `just ramdisk attach`, `just ramdisk detach` added to justfile
- [ ] `hal_dir` and `idf_dir` variables added to justfile
- [ ] HAL recipes (`check-hal`, `clippy-hal`, `build-example`/`run-example` for hal) route to `hal_dir`
- [ ] IDF recipes (`check-idf`, `clippy-idf`, `build-all`, `build-example`/`run-example` for idf) route to `idf_dir`
- [ ] `clean-idf` recipe updated to use `idf_dir`
- [ ] Tested end-to-end with RAM disk actually attached
- [ ] Documentation updated (README / AGENTS.md prerequisites)

## Session Log

- 2026-05-27 — Feature doc created; the basic recipe provided by user
