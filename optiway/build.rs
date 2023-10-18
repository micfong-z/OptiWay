fn main() {
    let env = std::env::var("OPTIWAY_COMPILE_ENV").unwrap_or("default".to_string());
    println!("cargo:rerun-if-env-changed=OPTIWAY_COMPILE_ENV");
    println!("cargo:rustc-cfg=compile_env=\"{}\"", env);
}
