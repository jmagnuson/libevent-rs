use std::path::PathBuf;
use std::env;

use pkg_config;
use bindgen;

fn main() {
    // Use pkg-config to find and link libevent
    pkg_config::probe_library("libevent").unwrap();

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .rustfmt_bindings(true)

        // Fixes a bug with a duplicated const
        .blacklist_item("IPPORT_RESERVED")

        .generate()
        .expect("Failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}