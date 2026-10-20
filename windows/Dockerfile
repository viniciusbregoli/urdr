# Build image: Linux host, native target for the tests, x86_64-pc-windows-gnu for the exe.
FROM rust:1-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
        mingw-w64 clang libclang-dev libasound2-dev pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-pc-windows-gnu
WORKDIR /src
