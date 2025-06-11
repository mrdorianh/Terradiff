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

live:
    ./live_demo.sh 

# Tag & push a signed release (usage: `just release v0.1.0-rc1`)
release TAG:
    git tag -s {{TAG}} -m "Terradrift {{TAG}}"
    git push origin {{TAG}} 