# Unreleased (post-0.4.0-beta)

Version `0.4.0` in tree (`0.4.0-beta` released as GitHub pre-release; `0.1.0-alpha`, `0.2.0-alpha`, `0.3.0-alpha` records frozen). User-facing changes accumulate in `CHANGELOG.md` under
`## Unreleased`; this note tracks known issues for the unreleased tree.

## New since 0.4.0-beta

No user-facing changes yet since `0.4.0-beta`.

## Known issues (unreleased tree)

- Live-capture validation on reference hardware with real PresentMon v2
  (GUI and `rwp capture` CLI: `doctor` with a versioned or version-unknown
  executable (SHA-256 recorded), timed
  capture, `analyze` → `report --format md` → two-session `compare
  --same-scenario`, manifest hashes, Ctrl+C/no-orphan check) remains
  `BLOCKED_ON_USER`. Synthetic fixtures prove tooling only.
- `rwp capture` Ctrl+C persists the partial session with `interrupted=true`
  (exit 0 with a stderr notice); a repeat signal still ends the process and
  the shared kill-on-close Job Object terminates PresentMon (no orphan). A
  killed stop can leave a PresentMon ETW session open: check with
  `logman query -ets` and rerun with `--stop-existing-session` (see
  `docs/troubleshooting.md`). Live-Ctrl+C on real PresentMon is still
  `BLOCKED_ON_USER`.
- `--process-name` resolution fails closed when zero or multiple live
  processes match; pass `--process-id` in that case.
- Installer remains unsigned (SmartScreen warning expected); signing awaits a
  certificate. WebView2 first-run provisioning beyond the offline bundle path
  is unverified.

Provenance: local gate on the pinned 1.99.0 toolchain (`docs/local-gates.md`);
per-batch outputs in `docs/autopilot/STATE.md`. Release history:
`0.1.0-alpha.md`, `0.2.0-alpha.md`, `0.3.0-alpha.md`, `0.4.0-beta.md`; no version bump for unreleased work.
