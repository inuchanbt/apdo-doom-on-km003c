use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=APDO_SAFE_NOR");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    for file in ["memory.x", "application.x", "device.x"] {
        fs::copy(file, out.join(file)).unwrap();
        println!("cargo:rerun-if-changed={file}");
    }
    println!("cargo:rustc-link-search={}", out.display());
    let engine =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../artifacts/engine");
    println!("cargo:rustc-link-search=native={}", engine.display());
    println!("cargo:rustc-link-lib=static=kmdoom");
    println!(
        "cargo:rerun-if-changed={}",
        engine.join("libkmdoom.a").display()
    );
}
