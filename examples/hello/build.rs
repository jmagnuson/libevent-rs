use cmake;

fn main() {
    let libevent_sys_include = dbg!(std::env::var("DEP_LIBEVENT_INCLUDE_PATH").unwrap());

    let dst = cmake::Config::new(".")
        .cflag(format!("-I{}", libevent_sys_include))
        .very_verbose(true)
        .build();

    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=helloc");
}
