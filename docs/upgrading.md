# Forge Upgrade Guide

Last updated: 2026-02-08

This guide tracks user-facing upgrade actions across releases.

## How To Use This Guide

1. Start from your current Forge version.
2. Apply steps for each target release in order.
3. Run your integration tests and conformance checks after each step.

## Unreleased

No upgrade actions yet.

## Upgrade Checklist Template

Use this checklist when publishing a release with behavior changes:

1. API changes:
   - any renamed/removed/moved item
   - replacement API or fallback path
2. Runtime semantics:
   - event protocol changes
   - checkpoint/resume behavior changes
   - permission/tool lifecycle behavior changes
3. Persistence and wire format:
   - schema version notes
   - backward/forward compatibility guarantees
4. Operational impact:
   - config/env changes
   - rollout/rollback guidance
