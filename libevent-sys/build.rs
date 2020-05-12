use std::env;
use std::path::PathBuf;

#[cfg(feature = "bundled")]
fn build_libevent(libevent_path: impl AsRef<std::path::Path>) -> PathBuf {
    let mut config = cmake::Config::new(libevent_path);

    // TODO: Or just both and decide elsewhere?
    if cfg!(feature = "static") {
        config.define("EVENT__LIBRARY_TYPE", "STATIC");
    } else {
        config.define("EVENT__LIBRARY_TYPE", "SHARED");
    }

    // Disable unnecessary dev-only features
    {
        config.define("EVENT__DISABLE_BENCHMARK", "ON")
            .define("EVENT__DISABLE_TESTS", "ON")
            .define("EVENT__DISABLE_REGRESS", "ON")
            .define("EVENT__DISABLE_SAMPLES", "ON");

        // TODO: Address policy warnings for cross-compilation?
        /*config.define("CMAKE_POLICY_DEFAULT_CMP0056", "NEW")
            .define("CMAKE_POLICY_DEFAULT_CMP0066", "NEW")*/
    }

    // TODO: Can we tap into "-vv" cargo argument?
    if cfg!(feature = "verbose_build") {
        config.very_verbose(true);
    }

    let dst = config.build();

    println!("cargo:rustc-link-search={}/lib", dst.display());

    // Library 'event' is considered deprecated, so link each sub-component
    // individually.
    println!("cargo:rustc-link-lib=static=event_core");
    println!("cargo:rustc-link-lib=static=event_extra");

    println!("cargo:include={}/include", dst.display());

    dst
}

#[cfg(not(feature = "bundled"))]
fn run_pkg_config() -> Option<Vec<String>> {
    use std::collections::HashSet;

    let mut pkg = pkg_config::Config::new();
    pkg.cargo_metadata(true)
        .atleast_version("2")
        .statik(cfg!(feature = "static"));

    let mut include_paths = HashSet::new();

    if let Ok(mut lib) = pkg.probe("libevent_core") {
        include_paths.extend(lib.include_paths.drain(..));
    } else {
        return None;
    }

    {
        let mut lib = pkg.probe("libevent_extra").unwrap();
        include_paths.extend(lib.include_paths.drain(..));
    }

    let include_paths = include_paths.drain().map(|path| {
        let path_s = path.into_os_string().into_string().unwrap();
        println!("cargo:include={}", &path_s);
        path_s
    }).collect();

    Some(include_paths)
}

#[cfg(feature = "bundled")]
fn find_libevent() -> Option<Vec<String>> {
    use std::path::Path;
    use std::process::Command;

    if !Path::new("libevent/.git").exists() {
        Command::new("git").args(&["submodule", "update", "--init"])
            .status().expect("Running `git submodule init` failed.");
    } else {
        Command::new("git").args(&["submodule", "update", "--recursive"])
            .status().expect("Running `git submodule update` failed.");
    }

    Some(vec![format!("{}/include", build_libevent("libevent").display())])
}
#[cfg(not(feature = "bundled"))]
fn find_libevent() -> Option<Vec<String>> {
    run_pkg_config()
}

fn main() {
    println!("cargo:rerun-if-changed=libevent");
    println!("cargo:rerun-if-changed=wrapper.h");

    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();

    if cfg!(feature = "verbose_build") {
        for (key, val) in env::vars() {
            println!("{}: {}", key, val);
        }
        let args: Vec<String> = env::args().collect();
        println!("args: {:?}", args);
    }

    let include_paths = find_libevent()
        .expect("No include paths for libevent found");

    let mut builder = bindgen::Builder::default();

    if target != host {
        builder = builder.clang_arg("-target");
        builder = builder.clang_arg(target);
    }

    // Let bindgen know about all include paths that were found.
    for path in include_paths {
        builder = builder.clang_arg(format!("-I{}", path));
    }

    let bindings = builder
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
