use std::env;
use std::path::PathBuf;

use bindgen;
use pkg_config;

fn main() {
    // Use pkg-config to find and link libevent
    let pkg = pkg_config::Config::new()
        .atleast_version("2")
        .statik(cfg!(feature = "static"))
        .probe("libevent")
        .unwrap();

    let bindings = bindgen::Builder::default()
        .clang_arg(format!("-I{}", pkg.include_paths.get(0).unwrap().display()))
        .header("wrapper.h")
        // Enable for more readable bindings
        // .rustfmt_bindings(true)
        // Fixes a bug with a duplicated const
        .blacklist_item("IPPORT_RESERVED")
        .generate()
        .expect("Failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}
