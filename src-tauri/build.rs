use std::process::Command;

fn main() {
    // whisper.cpp's Metal backend uses `@available(macOS 15, *)`. Below that
    // deployment target clang emits a call to `___isPlatformVersionAtLeast`,
    // which lives in clang's runtime; rustc does not link it by itself.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && let Ok(out) = Command::new("xcrun")
            .args(["clang", "--print-resource-dir"])
            .output()
        && let Ok(dir) = String::from_utf8(out.stdout)
    {
        println!("cargo:rustc-link-search=native={}/lib/darwin", dir.trim());
        println!("cargo:rustc-link-lib=static=clang_rt.osx");
    }
    tauri_build::build()
}
