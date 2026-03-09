use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo"),
    );
    let nitroarc_dir = manifest_dir.join("nitroarc");
    let nitroarc_ffi_dir = nitroarc_dir.join("ffi");
    let nitroarc_bin_dir = nitroarc_dir.join("bin");

    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_dir.join("Makefile").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_dir.join("Makefile.devel").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_dir.join("lib").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_dir.join("ffi").display()
    );

    let status = Command::new("make")
        .arg("ffi")
        .current_dir(&nitroarc_dir)
        .status()
        .expect("failed to run `make ffi` in nitroarc");

    assert!(
        status.success(),
        "`make ffi` failed in {}",
        nitroarc_dir.display()
    );

    println!(
        "cargo:rustc-link-search=native={}",
        nitroarc_bin_dir.display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        nitroarc_ffi_dir.display()
    );
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,{}",
        nitroarc_bin_dir.display()
    );
    println!("cargo:rustc-link-lib=dylib=nitroarc_ffi");
}
