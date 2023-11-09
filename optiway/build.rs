use cxx_build::CFG;

fn main() {
    let env = std::env::var("OPTIWAY_COMPILE_ENV").unwrap_or("default".to_string());
    println!("cargo:rerun-if-env-changed=OPTIWAY_COMPILE_ENV");
    println!("cargo:rustc-cfg=compile_env=\"{}\"", env);
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/demo.cc");
    println!("cargo:rerun-if-changed=include/demo.h");

    CFG.include_prefix = "optiway/include";

    cxx_build::bridge("src/ffi.rs")
        .file("src/floyd.cpp")
        .file("src/multi-objective-agent.cpp")
        .flag_if_supported("-std=c++20");
}
