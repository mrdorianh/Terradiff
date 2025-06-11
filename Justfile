default:
    just --list

lint:
    cargo fmt --all
    cargo clippy --all-targets -- -D warnings

test:
    cargo test --all-features

build-static:
    cross build --release --target x86_64-unknown-linux-musl 

man:
    cargo run -p terradrift --features man --bin gen-man -- terradrift.1 

demo:
    ./demo.sh 