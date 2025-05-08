use cxx_build::CFG;

fn main() {
    let env = std::env::var("OPTIWAY_COMPILE_ENV").unwrap_or("default".to_string());
    println!("cargo:rerun-if-env-changed=OPTIWAY_COMPILE_ENV");
    println!("cargo:rustc-cfg=compile_env=\"{}\"", env);
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/demo.cc");
    println!("cargo:rerun-if-changed=include/demo.h");

    cxx_build::bridge("src/ffi.rs")
        .std("c++20")
        .include("include")
        .file("src/floyd.cpp")
        .file("src/utilities.cpp")
        .file("src/multi-objective-agent.cpp")
        .compile("libfloyd");
}
