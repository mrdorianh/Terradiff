# Terradrift

[![CI](https://github.com/yourorg/terradrift/actions/workflows/ci.yml/badge.svg)](https://github.com/yourorg/terradrift/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-90%25-brightgreen)](#)

Terraform drift detection at ludicrous speed 🚀

## Features
- Ultra-fast parallel drift scans (100 workspaces < 60 s)
- Provider-agnostic state back-ends via features: **s3**, **gcs**, **azure**
- Incremental JSON parser with early exit → low RAM (<150 MB)
- Deterministic JSON summary & optional Slack alert
- Single <8 MB static binary (musl) ready for CI runners

## Installation
### Pre-built binaries (recommended)
Head to the [Releases](https://github.com/yourorg/terradrift/releases) page and grab the archive for your platform.

```bash
chmod +x terradrift
./terradrift --help
```

### Cargo
```bash
cargo install terradrift --features s3,gcs,azure # choose features you need
```

## Quick Start
1. Create `terradrift.toml` next to your Terraform repos:
```toml
[profiles.prod.storage]
provider = "s3"
bucket   = "tfstate-prod-bucket"
prefix   = "states"
```
2. Run a drift scan:
```bash
terradrift diff -p prod -j 8
```
3. Exit codes
- `0` – no drift
- `2` – drift detected (non-blocking in CI)

## Demo (no cloud creds)
```bash
./demo.sh
```
This spins up a temporary directory with mock tfstate files, runs Terradrift, and shows summary JSON.

## Configuration Reference
See [`terradrift.toml.example`](./terradrift.toml.example) for all supported keys.

## Slack Alerts
Set the environment variables before running:
```bash
export SLACK_WEBHOOK_URL="https://hooks.slack.com/services/T000/B000/XXX"
# Optional: link to a hosted plan diff
export PLAN_URL="https://artifacts.example.com/latest-plan.html"
```
A 🚨 alert is sent only when drift is found.

## JSON Summary Schema
Terradrift emits a machine-readable JSON summary to stdout. The schema is published in [`terradrift.schema.json`](./terradrift.schema.json).

## GitHub Actions
```yaml
- uses: actions/checkout@v4
- name: Terradrift
  uses: yourorg/terradrift-action@v1
  with:
    profile: prod
    slack_webhook: ${{ secrets.SLACK_WEBHOOK_URL }}
```

## Development
```bash
# Format & lint
just lint
# Run tests
just test
```

Static release build (x86_64-musl):
```bash
just build-static
```

## License
MIT OR Apache-2.0
