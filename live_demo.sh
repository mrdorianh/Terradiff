#!/usr/bin/env bash
# live_demo.sh — run Terradrift against a real profile with safety checks
#
# Usage:
#   TD_PROFILE=prod SLACK_WEBHOOK_URL=… ./live_demo.sh
#
# Required env vars:
#   TD_PROFILE              — name of profile in terradrift.toml
# Optional env vars:
#   SLACK_WEBHOOK_URL       — Slack channel webhook for alert
#   PLAN_URL                — Link added to Slack message
#   TERRADRIFT_TF_CACHE     — Directory to cache Terraform binaries
#
set -euo pipefail

if [[ -z "${TD_PROFILE:-}" ]]; then
  echo "❌ TD_PROFILE env var not set (e.g. export TD_PROFILE=prod)" >&2
  exit 1
fi

BIN="$(dirname "$0")/target/release/terradrift"
if [[ ! -x "$BIN" ]]; then
  echo "🔧 Building release binary…"
  # Compile with optional provider features when requested via TD_FEATURES env var.
  FEATURES="${TD_FEATURES:-}"
  if [[ -n "$FEATURES" ]]; then
    echo "cargo build --release --features \"$FEATURES\""
    cargo build -q -p terradrift --release --features "$FEATURES"
  else
    echo "cargo build --release (no extra features)"
    cargo build -q -p terradrift --release
  fi
fi

CFG=${TD_CONFIG:-terradrift.toml}
if [[ ! -f "$CFG" ]]; then
  echo "❌ $CFG not found in repo root" >&2
  exit 1
fi

printf "\n👉 Running Terradrift diff for profile %s\n\n" "$TD_PROFILE"
RUSTFLAGS='-Awarnings' "$BIN" diff -p "$TD_PROFILE" --config "$CFG"
EXIT=$?

if [[ $EXIT -eq 0 ]]; then
  echo "✅ No drift detected"
elif [[ $EXIT -eq 2 ]]; then
  echo "🚨 Drift detected! (exit 2)"
else
  echo "❌ Terradrift failed with exit code $EXIT" >&2
fi

exit $EXIT 