//! Copies `assets/` next to the built executable (`target/<profile>/assets`), so that folder is a
//! complete build: zip the exe and `assets/` together to release the game. Only changed files are
//! copied, and files deleted from `assets/` are removed from the copy.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=build.rs");

    let src = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets");
    // OUT_DIR is target/<profile>/build/hexfold-<hash>/out; the exe lives in target/<profile>.
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let Some(exe_dir) = out.ancestors().nth(3) else { return };
    let dst = exe_dir.join("assets");

    if let Err(e) = sync(&src, &dst) {
        println!("cargo:warning=could not copy assets to {}: {e}", dst.display());
    }
}

/// Makes `dst` a copy of `src`: copies new and changed files, removes ones no longer in `src`.
fn sync(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            sync(&from, &to)?;
        } else if changed(&from, &to)? {
            fs::copy(&from, &to)?;
        }
    }
    for entry in fs::read_dir(dst)? {
        let entry = entry?;
        if !src.join(entry.file_name()).exists() {
            let path = entry.path();
            if entry.file_type()?.is_dir() { fs::remove_dir_all(path)? } else { fs::remove_file(path)? }
        }
    }
    Ok(())
}

/// Whether `to` is missing or differs from `from` in size or is older.
fn changed(from: &Path, to: &Path) -> io::Result<bool> {
    let Ok(old) = fs::metadata(to) else { return Ok(true) };
    let new = fs::metadata(from)?;
    Ok(new.len() != old.len() || new.modified()? > old.modified()?)
}
