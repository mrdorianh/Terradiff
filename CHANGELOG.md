# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0-rc1] - 2025-06-11
### Added
- CLI `terradrift diff` with mock, S3, GCS, Azure providers.
- Parallel orchestrator with bounded concurrency.
- Streaming JSON parser with early exit for fast drift detection.
- Slack webhook sink with optional PLAN_URL link.
- Automatic Terraform binary management & caching.
- GitHub Actions CI (lint, tests, coverage, SBOM, static MUSL build).
- Unit & integration tests including deterministic e2e stub.
- Man page generation (`just man`).
- Demo scripts: `demo.sh` (random drift), `live_demo.sh` (real profile).
- Example env file (`env.example`).
- Colourful TTY table output with drift emoji via `tabled` crate.

### Changed
- README expanded with installation, quick start, live demo docs.

### Fixed
- Nested runtime panic removed by switching to async BufReader.

### Security
- Static binary scanned via Trivy in CI.

[0.1.0-rc1]: https://github.com/yourorg/terradrift/releases/tag/v0.1.0-rc1 