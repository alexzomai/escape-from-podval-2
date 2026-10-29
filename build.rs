//! Вшивает `resources/` и `maps/` в бинарник: игра — один файл без папок рядом.

use std::fmt::Write;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn main() {
    let mut files = Vec::new();
    for dir in ["resources", "maps"] {
        println!("cargo:rerun-if-changed={dir}");
        walk(Path::new(dir), &mut files);
    }
    files.sort();
    let mut code = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for file in &files {
        let rel = file.to_str().unwrap().replace('\\', "/");
        writeln!(
            code,
            "    ({rel:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{rel}\"))),"
        )
        .unwrap();
    }
    code.push_str("];\n");
    // SDL2 2.32, слинкованный статически на Windows, использует ещё и cfgmgr32
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if windows && std::env::var_os("CARGO_FEATURE_STATIC_SDL_VCPKG").is_some() {
        println!("cargo:rustc-link-lib=cfgmgr32");
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded.rs");
    std::fs::write(out, code).unwrap();
}
