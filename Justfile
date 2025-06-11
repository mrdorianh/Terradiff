default:
    just --list

lint:
    cargo fmt --all
    cargo clippy --all-targets -- -D warnings

test:
    cargo test --all-features

build-static:
    # Ensure `cross` is available; install once if missing
    bash -c 'command -v cross >/dev/null 2>&1 || { echo "⏳ Installing cross..."; cargo install cross --git https://github.com/cross-rs/cross --locked; }'
    cross build --release --target x86_64-unknown-linux-musl

man:
    cargo run -p terradrift --features man --bin gen-man -- terradrift.1 

demo:
    ./demo.sh 

# Run live demo: `just live` (defaults to dev) or `just live prod`
live PROFILE='dev':
    bash -c 'if [ "{{PROFILE}}" = "dev" ]; then export TD_CONFIG=terradrift.dev.toml; fi; TD_PROFILE="{{PROFILE}}" ./live_demo.sh'

# Tag & push a signed release (usage: `just release v0.1.0-rc1`)
release TAG:
    bash -c '[[ -n "$SIGKEY" ]] && git tag -s {{TAG}} -m "Terradrift {{TAG}}" --local-user "$SIGKEY" || git tag {{TAG}} -m "Terradrift {{TAG}}"'
    echo "Created git tag {{TAG}} (unsigned if SIGKEY not provided)"
    git push origin {{TAG}} || echo "No remote configured – push skipped"

sbom:
    cargo install --locked cargo-cyclonedx --version ^0.5.0
    cargo cyclonedx -f xml > sbom.xml
    shasum -a 256 sbom.xml | awk '{print $1}' > sbom.xml.sha256
    echo "SBOM generated at sbom.xml (sha256 in sbom.xml.sha256)" 