use std::env;
use std::path::PathBuf;

#[cfg(feature = "bundled")]
fn build_libevent(libevent_path: &str) -> PathBuf {
    let dst = cmake::Config::new(libevent_path)
        .define("CMAKE_POLICY_DEFAULT_CMP0056", "NEW")
        .define("CMAKE_POLICY_DEFAULT_CMP0066", "NEW")
        .define("CMAKE_SUPPRESS_DEVELOPER_WARNINGS", "1")
        .define("EVENT__DISABLE_THREAD_SUPPORT", "ON")
        .define("EVENT__DISABLE_OPENSSL", "ON")
        .define("EVENT__DISABLE_THREAD_SUPPORT", "ON")
        .define("EVENT__DISABLE_BENCHMARK", "ON")
        .define("EVENT__DISABLE_TESTS", "ON")
        .define("EVENT__DISABLE_REGRESS", "ON")
        .define("EVENT__DISABLE_SAMPLES", "ON")
        .define("EVENT_LIBRARY_STATIC", "ON")
        //.very_verbose(true)
        .build();

    //println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=event");
    // TODO: static feature ^^

    // TODO: Do I need/want?
    println!("cargo:include={}/include", dst.display());

    //println!("cargo:rustc-link-lib=static=libevent_core");
    //println!("cargo:rustc-link-lib=static=libevent_extra");

    dst
}

#[cfg(not(feature = "bundled"))]
fn run_pkg_config() -> Option<Vec<String>> {
    let mut pkg = pkg_config::Config::new();
    pkg.cargo_metadata(false)
        .atleast_version("2")
        .statik(cfg!(feature = "static"));

    let mut lib = match pkg.probe("libevent") {
        Ok(lib) => lib,
        Err(_e) => { return None; }
    };

    pkg.cargo_metadata(true).probe("libevent").unwrap();

    let include_paths = lib.include_paths.drain(..).map(|path| {
        let path_s = path.to_str().unwrap().to_string();
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
        dbg!(Command::new("git").args(&["submodule", "update", "--init"])
            .status().expect("Running `git submodule init` failed."));
    } else {
        dbg!(Command::new("git").args(&["submodule", "update", "--recursive"])
            .status().expect("Running `git submodule update` failed."));
    }
    Some(vec![format!("{}/include", build_libevent("libevent").display())])
}
#[cfg(not(feature = "bundled"))]
fn find_libevent() -> Option<Vec<String>> {
    run_pkg_config()
}

fn main() {

    let include_paths = find_libevent()
        .expect("No include paths for libevent found");

    let mut builder = bindgen::Builder::default();

    /*for path in include_paths {
        builder = builder.clang_arg(format!("-I{}", path));
    }*/

    let bindings = builder.header("wrapper.h")
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
