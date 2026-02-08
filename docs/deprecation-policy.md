# Forge Deprecation Policy

Last updated: 2026-02-08

## Goal

Deprecations provide a predictable migration path before removals.

## What Can Be Deprecated

- Public Rust items (functions, structs, enums, trait methods, modules)
- Runtime behavior flags/config options
- Event or persistence fields (only with compatibility-preserving rollout)

## Required Steps

For any deprecation:

1. mark API with `#[deprecated(since = "...", note = "...")]` where applicable
2. add replacement guidance in API docs and `docs/upgrading.md`
3. add changelog entry in `CHANGELOG.md`
4. keep contract/conformance coverage for old behavior until removal

## Minimum Support Window

### Pre-1.0 (`0.y.z`)

- Keep deprecated APIs for at least one `MINOR` release whenever practical.
- If immediate removal is required, document migration in the same release.

### 1.x

- Keep deprecated APIs for at least two `MINOR` releases before removal.
- Removal occurs only in a `MAJOR` release, except severe security cases.

## Wire Format and Persistence Deprecation

- Event/checkpoint/session schema changes must remain backward-compatible during deprecation.
- Versioned records must continue to decode older versions within the supported window.
- Removal of compatibility decode paths requires MAJOR release planning.

## Exceptions

Immediate removal is allowed for critical security or data-corruption risks, but requires:

- explicit security rationale
- migration or mitigation instructions
- post-incident compatibility note in `CHANGELOG.md`

## Enforcement

PRs introducing deprecations must include:

- tests validating old/new paths during transition
- explicit removal target version
- upgrade-note updates
