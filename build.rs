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

    let status = Command::new("make")
        .arg("ffi")
        .current_dir(&nitroarc_work_dir)
        .status()
        .expect("failed to run `make ffi` in vendored nitroarc");

    assert!(
        status.success(),
        "`make ffi` failed in {}",
        nitroarc_work_dir.display()
    );

    let nitroarc_bin_dir = nitroarc_work_dir.join("bin");
    let nitroarc_ffi_dir = nitroarc_work_dir.join("ffi");

    println!(
        "cargo:rustc-link-search=native={}",
        nitroarc_bin_dir.display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        nitroarc_ffi_dir.display()
    );

    #[cfg(target_os = "linux")]
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,{}",
        nitroarc_bin_dir.display()
    );

    #[cfg(target_os = "macos")]
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,{}",
        nitroarc_bin_dir.display()
    );

    println!("cargo:rustc-link-lib=dylib=nitroarc_ffi");
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
            src.parent()
                .unwrap_or_else(|| Path::new("."))
                .join(target)
                .into_os_string()
                .into()
        };

        let metadata = fs::metadata(&resolved_target)?;
        if metadata.is_dir() {
            std::os::windows::fs::symlink_dir(pathbuf_from_os_string(resolved_target), dst)
        } else {
            std::os::windows::fs::symlink_file(pathbuf_from_os_string(resolved_target), dst)
        }
    }
}

#[cfg(windows)]
fn pathbuf_from_os_string(value: OsString) -> PathBuf {
    PathBuf::from(value)
}
