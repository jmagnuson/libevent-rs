use std::env;
use std::path::{Path, PathBuf};

#[cfg(feature = "bundled")]
fn build_libevent(libevent_path: impl AsRef<std::path::Path>) -> PathBuf {
    let mut config = cmake::Config::new(libevent_path);

    if !cfg!(feature = "threading") {
        config.define("EVENT__DISABLE_THREAD_SUPPORT", "ON");
    } else {
        config.define("EVENT__DISABLE_THREAD_SUPPORT", "OFF");
    }

    if cfg!(feature = "openssl") || cfg!(feature = "openssl_bundled") {
        config.define("EVENT__DISABLE_OPENSSL", "OFF");

        if cfg!(feature = "openssl_bundled") {
            if !cfg!(feature = "threading") {
                unimplemented!(
                    "feature `openssl_bundled` without feature `threading` \
                    not currently supported as `CMAKE_USE_PTHREADS_INIT` gets injected \
                    into libevent cmake build, causing build failures"
                );
            }

            println!("cargo:rustc-link-lib=static=ssl");
            println!("cargo:rustc-link-lib=static=crypto");
        } else {
            println!("cargo:rustc-link-lib=ssl");
            println!("cargo:rustc-link-lib=crypto");
        }

        config.register_dep("openssl");

        let openssl_root = if let Ok(root) = env::var("DEP_OPENSSL_ROOT") {
            root
        } else {
            let include_str = env::var("DEP_OPENSSL_INCLUDE").unwrap();
            let include_dir = std::path::Path::new(&include_str);
            let root_dir = format!("{}", include_dir.parent().unwrap().display());
            env::set_var("DEP_OPENSSL_ROOT", &root_dir);
            root_dir
        };

        config.define("OPENSSL_ROOT_DIR", openssl_root);
    } else {
        config.define("EVENT__DISABLE_OPENSSL", "ON");
    }

    // TODO: Or just both and decide elsewhere?
    if cfg!(feature = "static") {
        config.define("EVENT__LIBRARY_TYPE", "STATIC");
    } else {
        config.define("EVENT__LIBRARY_TYPE", "SHARED");
    }

    // Disable unnecessary dev-only features
    {
        config
            .define("EVENT__DISABLE_BENCHMARK", "ON")
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

    if cfg!(feature = "openssl") || cfg!(feature = "openssl_bundled") {
        println!("cargo:rustc-link-lib=static=event_openssl");
    }

    if cfg!(feature = "threading") {
        println!("cargo:rustc-link-lib=static=event_pthreads");
    }

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

    if let Ok(mut lib) = pkg
        .probe("libevent_core")
        .or_else(|_| pkg.probe("libevent"))
    {
        include_paths.extend(lib.include_paths.drain(..));
    } else {
        return None;
    }

    {
        match pkg.probe("libevent_extra") {
            Ok(mut lib) => include_paths.extend(lib.include_paths.drain(..)),
            Err(e) => println!("Failed to find libevent_extra: {:?}", e),
        }
    }

    if cfg!(feature = "openssl") {
        let mut lib = pkg.cargo_metadata(true).probe("libevent_openssl").unwrap();
        include_paths.extend(lib.include_paths.drain(..));
    }

    if cfg!(feature = "threading") {
        let mut lib = pkg.cargo_metadata(true).probe("libevent_pthreads").unwrap();
        include_paths.extend(lib.include_paths.drain(..));
    }

    let include_paths = include_paths
        .drain()
        .map(|path| {
            let path_s = path.into_os_string().into_string().unwrap();
            println!("cargo:include={}", &path_s);
            path_s
        })
        .collect();

    Some(include_paths)
}

#[cfg(feature = "bundled")]
fn find_libevent() -> Option<Vec<String>> {
    use std::process::Command;

    if !Path::new("libevent/.git").exists() {
        let _ = Command::new("git")
            .args(&["submodule", "update", "--init"])
            .status();
    } else {
        let _ = Command::new("git")
            .args(&["submodule", "update", "--recursive"])
            .status();
    }

    Some(vec![format!(
        "{}/include",
        build_libevent("libevent").display()
    )])
}
#[cfg(not(feature = "bundled"))]
fn find_libevent() -> Option<Vec<String>> {
    run_pkg_config()
}

#[cfg(feature = "buildtime_bindgen")]
fn generate_bindings(include_paths: Vec<String>, out_path: impl AsRef<Path>) {
    println!("cargo:rerun-if-changed=wrapper.h");

    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();

    let mut builder = bindgen::Builder::default();

    if cfg!(feature = "verbose_build") {
        builder = builder.clang_arg("-v");
    }

    if target != host {
        // TODO: Is it necessary to specify target in clang_arg?
        // Ref: https://github.com/rust-lang/rust-bindgen/issues/1780
    }

    // Let bindgen know about all include paths that were found.
    for path in include_paths {
        builder = builder.clang_arg(format!("-I{}", path));
    }

    // Some of the libevent internals need to be exposed to inject a tokio backend.
    if cfg!(feature = "tokio_backend") {
        builder = builder
            .clang_arg("-Ilibevent")
            .clang_arg(format!("-I{}/build/include", out_path.as_ref().display()))
            .header("libevent/event-internal.h")
            .header("libevent/evmap-internal.h")
            .blocklist_item(".*voucher.*")
            .blocklist_item("strto.*");
    }

    let bindings = builder
        .header("wrapper.h")
        // Enable for more readable bindings
        // .rustfmt_bindings(true)
        // Fixes a bug with a duplicated const
        .blocklist_item("IPPORT_RESERVED")
        .generate()
        .expect("Failed to generate bindings");

    bindings
        .write_to_file(out_path.as_ref().join("bindings.rs"))
        .expect("Failed to write bindings");
}

#[cfg(not(feature = "buildtime_bindgen"))]
fn generate_bindings(_include_paths: Vec<String>, out_path: impl AsRef<Path>) {
    use std::fs;
    let in_path = env::var("LIBEVENT_SYS_BINDINGS_FILE")
        .expect("LIBEVENT_SYS_BINDINGS_FILE should be populated if buildtime_bindgen feature is not enabled");
    fs::copy(in_path, out_path.as_ref().join("bindings.rs"))
        .expect("Failed to copy bindings to output destination");
}

fn main() {
    if cfg!(feature = "verbose_build") {
        for (key, val) in env::vars() {
            println!("{}: {}", key, val);
        }
        let args: Vec<String> = env::args().collect();
        println!("args: {:?}", args);
    }

    let include_paths = find_libevent().expect("No include paths for libevent found");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    generate_bindings(include_paths, out_path);
}
