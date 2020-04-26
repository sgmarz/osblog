# PREREQS
You will need to install the riscv64gc target using rustup as well as cargo-binutils using cargo.

* rustup target add riscv64gc-unknown-none-elf
* cargo install cargo-binutils

# BUILDING
Edit .cargo/config to match your host's configuration. The runner will execute when you type `cargo run`.

Type `cargo build` to start the build process.
Type `cargo run` to run using the runner provided in .cargo/config

# RELEASE BUILDS

Release builds turn on the optimizer and make it run much quicker. To run release builds, add `--release` as a parameter to cargo:

* cargo build --release
* cargo run --release

# HARD DRIVE FILE

To run this as I have it configured, you'll need a hard drive file called hdd.dsk in this directory. You can create an empty
one by typing the following.

* fallocate -l 32M hdd.dsk

