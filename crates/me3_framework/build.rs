fn main() {
    println!("cargo:rerun-if-changed=src/framework/tracing/runtime/profiler.asm");

    cc::Build::new()
        .file("src/framework/tracing/runtime/profiler.asm")
        .compile("profiler_rt");
}
