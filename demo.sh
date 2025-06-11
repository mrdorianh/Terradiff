#!/usr/bin/env bash
# demo.sh – quick showcase using mock provider
set -euo pipefail

# Create temporary directory with mock tfstate files
WORKDIR=$(mktemp -d)
cat <<EOF > "$WORKDIR/prod1.tfstate"
{}
EOF
cat <<EOF > "$WORKDIR/prod2.tfstate"
{}
EOF

# Generate one tfstate that will trigger drift
cat <<EOF > "$WORKDIR/drift.tfstate"
{}
EOF

echo "Running Terradrift demo with mock provider…"

cat <<TOML > terradrift.demo.toml
[profiles.demo.storage]
provider = "mock"
path = "$WORKDIR"
TOML

# Run scan – expect exit code 2 due to drift.tfstate
cargo run -q -p terradrift --bin terradrift -- diff -p demo --config terradrift.demo.toml || true

echo "\nCleanup: $WORKDIR (left for inspection)" 