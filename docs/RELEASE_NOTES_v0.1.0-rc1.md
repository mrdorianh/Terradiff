# Terradrift v0.1.0-rc1 – Release Candidate

_2025-06-18_

Terradrift is a blazing-fast, zero-dependency Terraform drift detector written in Rust.
This release candidate bundles every feature planned for the 0.1 GA.  If no critical
bugs are reported during the next week, it will be promoted to **v0.1.0** unchanged.

## 🔥 Highlights

* **Multi-cloud state back-ends** – `mock`, **S3**, **Google Cloud Storage**, **Azure Blob**
* **Streaming plan parser** – detects drift in huge workspaces with <100 MB RAM.
* **Parallel orchestration** – scan hundreds of workspaces in seconds with `--jobs`.
* **Slack alerts** – fail CI with exit 2 _and_ notify your team instantly.
* **Colourful TUI table** – emoji-powered summary when a TTY is present.
* **Static MUSL binaries** – minimal attack surface, SBOM & Trivy-scanned.

## 🚀 Install

```bash
curl -sSL https://github.com/yourorg/terradrift/releases/download/v0.1.0-rc1/terradrift-x86_64-unknown-linux-musl.tar.gz | tar -xz
sudo mv terradrift /usr/local/bin/
```

Or build from source (needs Rust 1.78+):

```bash
cargo install --git https://github.com/yourorg/terradrift --tag v0.1.0-rc1 terradrift
```

## 📝 Changelog (since 0.0.x)

See `CHANGELOG.md` for the full list.  Key additions:

- CLI `terradrift diff` & `version` commands
- Automatic Terraform binary management & caching
- Provider features: mock, S3, GCS, Azure
- Parallel orchestrator & streaming parser
- Slack webhook sink with rich formatting
- CI pipeline: lint, tests, coverage, SBOM, static build
- Demo scripts (`demo.sh`, `live_demo.sh`), man page generation
- Colourful table output with drift emoji 🎉

## ⚠️ Known Issues

* Azure provider depends on preview SDK; API changes may occur.
* Very large `tfstate` files (>300 MiB) not yet covered by e2e tests.

## 📣 Call for Feedback

Please try Terradrift in your CI pipelines and open an issue with any
feedback, bug reports, or performance metrics.

---

Signed, **Terradrift Core** 