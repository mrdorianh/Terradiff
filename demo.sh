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

# --------------------------------------------------------------------
# Stub terraform binary that returns random drift so the demo is visual
# --------------------------------------------------------------------
BIN_DIR=$(mktemp -d)
STUB="$BIN_DIR/terraform"
cat <<'BASH' > "$STUB"
#!/usr/bin/env bash
# Simple terraform stub that emits random drift
if [[ "$1" == "version" ]]; then
  echo '{"terraform_version":"1.7.5"}'
  exit 0
fi

# Generate 1–5 resource change objects
COUNT=$(( (RANDOM % 5) + 1 ))
echo -n '{"resource_changes":['
for ((i=0;i<COUNT;i++)); do
  echo -n '{"change":{"actions":["update"]}}'
  [[ $i -lt $((COUNT-1)) ]] && echo -n ','
done
echo ']}'

# Exit code 2 signals drift
exit 2
BASH

chmod +x "$STUB"

export PATH="$BIN_DIR:$PATH"

cat <<TOML > terradrift.demo.toml
[profiles.demo.storage]
provider = "mock"
path = "$WORKDIR"
TOML

# Run scan – drift always simulated by stub, exit code 2 expected
RUSTFLAGS='-Awarnings' cargo run -q -p terradrift --bin terradrift -- diff -p demo --config terradrift.demo.toml || true

echo "\nCleanup: $WORKDIR (left for inspection)" 