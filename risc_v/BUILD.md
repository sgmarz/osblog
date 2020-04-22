# PREREQS
You will need to install the riscv64gc target using rustup as well as cargo-binutils using cargo.

[] rustup target add riscv64gc-unknown-none-elf
[] cargo install cargo-binutils

# BUILDING
Edit .cargo/config to match your host's configuration. The runner will execute when you type `cargo run`.

Type `cargo build` to start the build process.
Type `cargo run` to run using the runner provided in .cargo/config

