# Unreleased (post-0.1.0-alpha)

Version `0.2.0` in tree (bump from `0.1.0`; alpha record frozen). User-facing changes accumulate in `CHANGELOG.md` under
`## Unreleased`; this note tracks known issues for the unreleased tree.

## New since 0.1.0-alpha

- `rwp capture` CLI for live Windows capture (same `SessionRunner`
  orchestration, session layout, and manifest as the GUI). See
  `docs/cli-reference.md` (`rwp capture --help`) and ADR-0031.

## Known issues (unreleased tree)

- Live-capture validation on reference hardware with real PresentMon v2
  (GUI and `rwp capture` CLI: `doctor` with a versioned executable, timed
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
per-batch outputs in `docs/autopilot/STATE.md`. No tag, no GitHub Release, no
version bump for unreleased work.
