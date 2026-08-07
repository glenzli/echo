//! Generates the CXX ABI for the desktop bridge. No native sources are
//! compiled here: engine implementation lives in `echo-bridge`'s build.

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    let mut build = cxx_build::bridge("src/lib.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        build.flag("/W4").flag("/permissive-");
    } else {
        build.flag("-Wall").flag("-Wextra").flag("-Wpedantic");
    }
    build.compile("echo-desktop-bridge-cxx");
}
