# ADR-0029 — WebView2 runtime install mode

- Status: accepted
- Date: 2026-10-12
- Scope: Fase 6 / RC-09

## Context

The Tauri NSIS bundle needs a WebView2 runtime on the target machine. The
default mode downloads a bootstrapper at install time, which requires network
access during setup and contradicts the offline-first product invariant. The
tested default bundle works when WebView2 is already present but its download
path is unverified here.

## Decision

Release bundles set `bundle.windows.webviewInstallMode` to
`offlineInstaller`: the full WebView2 standalone installer is embedded, so
setup completes with no network access. The size cost (~150 MiB added to the
bundle) is accepted for the offline-first guarantee. `fixedRuntime` (vendoring
a specific runtime build) is rejected: it freezes the runtime version and
shifts servicing burden onto RWP releases.

## Consequences

- Install/uninstall verification must be re-run against the offline bundle
  (recorded in STATE); the earlier default-mode evidence covers the
  application itself, not the runtime provisioning path.
- Bundle size grows by roughly two orders of magnitude; this is documented in
  the release notes rather than optimized away.
