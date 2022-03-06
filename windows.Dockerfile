FROM buildpack-deps:bullseye

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# Cross-compilation support
RUN apt clean && apt update && apt install gcc-mingw-w64-x86-64 -y

# Install Rust without any toolchains
RUN set -eux; \
    dpkgArch="$(dpkg --print-architecture)"; \
    case "${dpkgArch##*-}" in \
        amd64) rustArch='x86_64-unknown-linux-gnu' ;; \
        arm64) rustArch='aarch64-unknown-linux-gnu' ;; \
        *) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;; \
    esac; \
    mkdir $RUSTUP_HOME $CARGO_HOME $CARGO_HOME/bin; \
    wget "https://static.rust-lang.org/rustup/dist/${rustArch}/rustup-init"; \
    mv rustup-init $CARGO_HOME/bin/rustup; \
    cp $CARGO_HOME/bin/rustup $CARGO_HOME/bin/cargo; \
    cp $CARGO_HOME/bin/rustup $CARGO_HOME/bin/rustc; \
    chmod -R a+rwx $CARGO_HOME/bin/; \
    rustup --version;

# Install toolchain from current directory
COPY rust-toolchain.toml .
RUN set -eux; \
    rustup target add x86_64-pc-windows-gnu; \
    chmod -R a+rwx $RUSTUP_HOME $CARGO_HOME;

# Setup a scaffold project
RUN cargo init --bin --name trustworthy-dolphin

# Dependencies
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN set -eux; \
    cargo build --target x86_64-pc-windows-gnu --release --no-default-features; \
    rm ./target/x86_64-pc-windows-gnu/release/trustworthy-dolphin*; \
    rm ./target/x86_64-pc-windows-gnu/release/deps/trustworthy_dolphin*; \
    rm src/*.rs;

# Build for release
COPY ./src ./src
COPY ./assets ./assets
RUN cargo build --target x86_64-pc-windows-gnu --release --no-default-features --features "embed_assets"