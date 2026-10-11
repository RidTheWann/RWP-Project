# ADR-0030 — No automatic updates

- Status: accepted
- Date: 2026-10-12
- Scope: Fase 6 / RC-10

## Context

An updater would need network access, a signing identity, and release-channel
infrastructure — all absent by design in this phase. PresentMon distribution
is already decided user-supplied (ADR-0028), so no component has an
update channel to manage.

## Decision

RWP ships no updater: no updater plugin dependency, no `updater` section in
`tauri.conf.json`, no update code in the frontend, and no version check on
startup. Operators update by running a newer installer. Reaffirm each release
by confirming the updater plugin is absent from the dependency graph and the
config carries no updater section.

## Consequences

- The application makes no network requests for updates, supporting the RC-10
  offline claim alongside the dependency and runtime evidence.
- Revisit only together with signing identity plus release-channel ownership.
