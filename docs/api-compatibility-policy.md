# Forge API Compatibility Policy

Last updated: 2026-02-08

## Scope

This policy defines compatibility expectations for Forge public surfaces:

- Rust public API exposed by `forge::runtime::*`
- Runtime event protocol and persistence formats
- Documented behavior contracts in `docs/forge-1.0-contracts-and-conformance.md`

Internal modules and undocumented behavior are out of scope unless explicitly marked stable.

## Versioning Model

Forge follows Semantic Versioning:

- `MAJOR`: incompatible API or behavior changes
- `MINOR`: backward-compatible features
- `PATCH`: backward-compatible fixes and docs/tests changes

## Pre-1.0 (`0.y.z`) Rules

Before `1.0.0`, API evolution is faster, but changes must still be managed:

- Breaking changes are allowed only with:
  - a migration note
  - a changelog entry with impact scope
  - explicit mention in PR risk section
- Non-breaking additions should prefer `MINOR` updates.
- Security or correctness fixes may ship immediately with migration notes if needed.

## 1.x Rules

From `1.0.0` onward:

- `PATCH`:
  - no public API removals or signature changes
  - no wire format breakages
- `MINOR`:
  - additive API only
  - existing behavior contracts must remain valid
- `MAJOR`:
  - required for removals, incompatible signature changes, incompatible protocol/schema changes, or semantic contract breaks

## Stable Surface Boundaries

The 1.0 stable contract surfaces are defined in:

- `docs/forge-1.0-contracts-and-conformance.md`

Any change to those surfaces is treated as semver-sensitive and must include compatibility analysis.

## Change Classification

Each PR touching public/runtime contracts must classify itself as one of:

- `compatible`
- `compatible-with-deprecation`
- `breaking`

`breaking` changes require MAJOR version planning (or pre-1.0 migration documentation).

## Required Artifacts For Public Changes

When public or contract surfaces change, PRs must include:

1. tests (unit/integration/contract) covering old and new behavior where applicable
2. changelog entry in `CHANGELOG.md`
3. migration notes in `docs/upgrading.md` for user-visible behavior changes

## Compatibility Exceptions

Emergency fixes may temporarily violate additive-only evolution before `1.0.0`, but must still include:

- clear rationale
- mitigation/rollback notes
- follow-up issue for long-term compatibility cleanup
