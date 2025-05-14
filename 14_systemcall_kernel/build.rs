fn main() {
    // Get the object file path
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let obj_path = format!(
        "{}/{}",
        manifest_dir,
        "../14_systemcall_userland/target/riscv32i-unknown-none-elf/debug/shell_bin.o"
    );

    // Pass the object file path directly to the linker
    // https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-arg
    println!("cargo:rustc-link-arg={}", obj_path);
}
