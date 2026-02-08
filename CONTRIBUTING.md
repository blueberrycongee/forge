# Contributing

Thanks for your interest in contributing to Forge.

## Quick Start
- Open an issue for discussion before large changes.
- Keep pull requests small and focused.
- Add tests where behavior changes.
- For public API/runtime contract changes, update `CHANGELOG.md` and `docs/upgrading.md`.

## Development
- This repo is a Rust library. If you add a Cargo workspace, document it here.
- Run formatting and tests before submitting.
- Run:
  - `cargo fmt --all -- --check`
  - `cargo test --all-targets`
  - `cargo clippy --all-targets --all-features -- -D warnings`

## Code Review
- Explain the why, not just the what.
- Include reproduction steps for bug fixes.
- Classify compatibility impact: `compatible`, `compatible-with-deprecation`, or `breaking`.

## Compatibility Governance
- Follow `docs/api-compatibility-policy.md` for semver and stable surface rules.
- Follow `docs/deprecation-policy.md` for deprecation windows and removal requirements.
- Keep `docs/forge-1.0-contracts-and-conformance.md` synchronized with behavior changes.
