fn main() {
    cxx_build::bridge("src/lib.rs")
        .include("include")
        .compile("test");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=include/dl_allocator.hpp");
    println!("cargo:rerun-if-changed=include/dl_string_bridge.hpp");
}
