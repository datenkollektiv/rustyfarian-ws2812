fn main() {
    // Set by `sysenv::output()` when the firmware enables `CONFIG_RMT_ISR_IRAM_SAFE` or,
    // from ESP-IDF 5.5, `CONFIG_RMT_TX_ISR_CACHE_SAFE`.
    println!("cargo::rustc-check-cfg=cfg(esp_idf_rmt_isr_iram_safe)");
    println!("cargo::rustc-check-cfg=cfg(esp_idf_rmt_tx_isr_cache_safe)");
    // Only run IDF-specific setup when building for an ESP-IDF target.
    // For bare-metal targets (e.g. riscv32imac-unknown-none-elf used by IDE
    // tooling), return early — the library body is also cfg-gated, so no IDF
    // link arguments are needed.
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "espidf" {
        return;
    }
    embuild::espidf::sysenv::output();
}
