use std::env;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo"),
    );
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo for build scripts"));

    let nitroarc_src_dir = manifest_dir.join("nitroarc");
    assert!(
        nitroarc_src_dir.exists(),
        "vendored nitroarc source directory is missing: {}",
        nitroarc_src_dir.display()
    );

    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_src_dir.join("Makefile").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_src_dir.join("Makefile.devel").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_src_dir.join("ffi").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_src_dir.join("lib").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_src_dir.join("src").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_src_dir.join("doc").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_src_dir.join("COPYING").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        nitroarc_src_dir.join("COPYING.LESSER").display()
    );
    println!("cargo:rerun-if-env-changed=CC");

    let nitroarc_build_root = out_dir.join("nitroarc-build");
    let nitroarc_work_dir = nitroarc_build_root.join("src");

    if nitroarc_work_dir.exists() {
        fs::remove_dir_all(&nitroarc_work_dir).unwrap_or_else(|err| {
            panic!(
                "failed to remove previous nitroarc build directory {}: {err}",
                nitroarc_work_dir.display()
            )
        });
    }

    copy_dir_recursive(&nitroarc_src_dir, &nitroarc_work_dir).unwrap_or_else(|err| {
        panic!(
            "failed to copy nitroarc sources from {} to {}: {err}",
            nitroarc_src_dir.display(),
            nitroarc_work_dir.display()
        )
    });

    let mut make = Command::new("make");
    make.arg("ffi").current_dir(&nitroarc_work_dir);

    // Prefer `gcc` unless the user explicitly chose a compiler via `CC`.
    if cfg!(windows) && env::var_os("CC").is_none() {
        make.env("CC", "gcc");
    }

    let status = make
        .status()
        .expect("failed to run `make ffi` in vendored nitroarc");

    assert!(
        status.success(),
        "`make ffi` failed in {}",
        nitroarc_work_dir.display()
    );

    let nitroarc_bin_dir = nitroarc_work_dir.join("bin");
    let nitroarc_ffi_dir = nitroarc_work_dir.join("ffi");
    let nitroarc_shared_lib = nitroarc_bin_dir.join(shared_library_name());

    println!(
        "cargo:rustc-link-search=native={}",
        nitroarc_bin_dir.display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        nitroarc_ffi_dir.display()
    );

    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", runtime_library_search_path());

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", runtime_library_search_path());

    println!("cargo:rustc-link-lib=dylib=nitroarc_ffi");
    stage_runtime_artifacts(&out_dir, &nitroarc_shared_lib);
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        } else if file_type.is_symlink() {
            copy_symlink(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn copy_symlink(src: &Path, dst: &Path) -> io::Result<()> {
    let target = fs::read_link(src)?;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, dst)
    }

    #[cfg(windows)]
    {
        let resolved_target = if target.is_absolute() {
            target
        } else {
            src.parent().unwrap_or_else(|| Path::new(".")).join(target)
        };

        let metadata = fs::metadata(&resolved_target)?;
        if metadata.is_dir() {
            std::os::windows::fs::symlink_dir(&resolved_target, dst)
        } else {
            std::os::windows::fs::symlink_file(&resolved_target, dst)
        }
    }
}

fn shared_library_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "nitroarc_ffi.dll"
    }

    #[cfg(target_os = "macos")]
    {
        "libnitroarc_ffi.dylib"
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        "libnitroarc_ffi.so"
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn runtime_library_search_path() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "$ORIGIN"
    }

    #[cfg(target_os = "macos")]
    {
        "@loader_path"
    }
}

fn stage_runtime_artifacts(out_dir: &Path, shared_lib: &Path) {
    let profile_dir = cargo_profile_dir(out_dir);
    let filename = shared_lib.file_name().unwrap_or_else(|| {
        panic!(
            "shared library path does not have a filename component: {}",
            shared_lib.display()
        )
    });

    copy_runtime_artifact(shared_lib, &profile_dir.join(filename)).unwrap_or_else(|err| {
        panic!(
            "failed to stage {} next to built binaries in {}: {err}",
            shared_lib.display(),
            profile_dir.display()
        )
    });

    let deps_dir = profile_dir.join("deps");
    copy_runtime_artifact(shared_lib, &deps_dir.join(filename)).unwrap_or_else(|err| {
        panic!(
            "failed to stage {} in {}: {err}",
            shared_lib.display(),
            deps_dir.display()
        )
    });
}

fn cargo_profile_dir(out_dir: &Path) -> &Path {
    out_dir.ancestors().nth(3).unwrap_or_else(|| {
        panic!(
            "failed to derive Cargo profile directory from OUT_DIR {}",
            out_dir.display()
        )
    })
}

fn copy_runtime_artifact(src: &Path, dst: &Path) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dst)?;
    Ok(())
}
