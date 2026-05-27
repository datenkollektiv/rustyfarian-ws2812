fn main() {
    // Only run IDF-specific setup when building for an ESP-IDF target.
    // For bare-metal targets (e.g. riscv32imac-unknown-none-elf used by IDE
    // tooling), return early — the library body is also cfg-gated, so no IDF
    // link arguments are needed.
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "espidf" {
        return;
    }
    embuild::espidf::sysenv::output();
}
