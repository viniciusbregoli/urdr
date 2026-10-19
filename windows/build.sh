#!/bin/sh
# Builds inside Docker so the host needs no Rust: ./build.sh test | ./build.sh exe | ./build.sh linux
# The registry and target dir live on named volumes, so rebuilds are incremental.
set -e
cd "$(dirname "$0")"
docker build -q -t urdr-win-build . >/dev/null
run() {
    docker run --rm -v "$PWD":/src -v urdr-cargo:/usr/local/cargo/registry -v urdr-target:/target \
        -e CARGO_TARGET_DIR=/target urdr-win-build sh -c "$1; status=\$?; chown -R $(id -u):$(id -g) /src; exit \$status"
}
case "$1" in
    test) run "cargo test" ;;
    # Static libstdc++ so the exe does not need MinGW DLLs next to it.
    exe) run "CXXSTDLIB_x86_64_pc_windows_gnu=static=stdc++ \
        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS=\"-L native=\$(dirname \$(x86_64-w64-mingw32-g++ -print-file-name=libstdc++.a))\" \
        cargo build --release --target x86_64-pc-windows-gnu && mkdir -p dist && cp /target/x86_64-pc-windows-gnu/release/urdr.exe dist/" ;;
    linux) run "cargo build --release && mkdir -p dist && cp /target/release/urdr dist/urdr-linux" ;;
    *) echo "usage: $0 test|exe|linux" >&2; exit 2 ;;
esac
