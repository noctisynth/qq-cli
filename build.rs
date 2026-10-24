fn main() {
    println!("cargo:rerun-if-changed=src/native/macos/census.c");
    println!("cargo:rerun-if-changed=src/native/macos/socket.c");
    println!("cargo:rerun-if-changed=src/native/macos/terminate.c");
    println!("cargo:rerun-if-changed=src/native/macos/disk.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64")
    {
        cc::Build::new()
            .file("src/native/macos/census.c")
            .file("src/native/macos/socket.c")
            .file("src/native/macos/terminate.c")
            .file("src/native/macos/disk.c")
            .warnings_into_errors(true)
            .compile("qq_cli_socket");
    }
}
