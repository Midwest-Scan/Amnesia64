//! Regenerates `include/hpl2_rs.h` from the `extern "C"` items in `src/`.
//! The file is only rewritten when its contents change, so C++ files that
//! include it are not rebuilt needlessly.

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let header = crate_dir.join("include").join("hpl2_rs.h");

    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")).expect("cbindgen.toml");
    match cbindgen::Builder::new().with_crate(&crate_dir).with_config(config).generate() {
        Ok(bindings) => {
            bindings.write_to_file(&header);
        }
        Err(e) => panic!("cbindgen failed to generate {}: {e}", header.display()),
    }
}
