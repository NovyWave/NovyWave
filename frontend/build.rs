fn main() {
    let platform = std::env::var("NOVYWAVE_PLATFORM").unwrap_or_else(|_| "WEB".to_string());

    // Add check-cfg configuration for the custom cfg
    println!("cargo:rustc-check-cfg=cfg(NOVYWAVE_PLATFORM, values(\"WEB\", \"TAURI\"))");

    println!("cargo:rustc-cfg=NOVYWAVE_PLATFORM=\"{}\"", platform);

    // For TAURI platform, we need to enable the tauri feature at build time
    // This is done by setting CARGO_CFG_FEATURE_TAURI=1 environment variable
    if platform == "TAURI" {
        println!("cargo:rustc-env=CARGO_CFG_FEATURE_TAURI=1");
        println!("cargo:rustc-cfg=feature=\"tauri\"");
    }

    println!("cargo:rerun-if-env-changed=NOVYWAVE_PLATFORM");
}
