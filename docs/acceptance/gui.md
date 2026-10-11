# GUI Acceptance Checklist

This checklist is the manual acceptance companion for WP4/WP5. Execute it on a real Windows build using the pinned Rust/toolchain and a real PresentMon v2 capture.

> Fase 4 scope note: items runnable without hardware are covered by automated
> tests (backend command tests plus CLI E2E): process picker listing, file
> dialog wiring, live progress/event payloads, Compare on two persisted
> sessions, and remediation mapping. Items below requiring a real capture,
> PresentMon permissions, a 30-minute run, or hardware telemetry are explicitly
> DEFERRED until reference hardware is available — not half-done.

## Environment

- [ ] Windows 11 test machine recorded.
- [ ] RWP build version and Git SHA recorded.
- [ ] PresentMon v2 executable path recorded.
- [ ] PresentMon version and SHA-256 recorded.
- [ ] Target application/API recorded.
- [ ] GPU/vendor/driver recorded.
- [ ] Network disconnected for the offline-first acceptance pass.

## Capture

- [ ] GUI rejects an invalid PresentMon executable path.
- [ ] GUI rejects a path that is not a regular file.
- [ ] GUI rejects an executable whose basename is not `PresentMon.exe`.
- [ ] GUI reports missing PresentMon permissions with a remediation message.
- [ ] Starting a capture shows the Running state with the target PID.
- [ ] Stop requests cancellation and the capture reaches a terminal state.
- [ ] Closing the application does not leave an unowned PresentMon child process.
- [ ] Interrupted capture is represented as interrupted capture state, not a generic failed analysis.

## Persistence and history

- [ ] Completed capture persists a manifest, capture, report, telemetry, and raw PresentMon CSV.
- [ ] History lists only sessions whose manifest hashes verify.
- [ ] Loading a history entry reproduces the persisted report/dashboard deterministically.
- [ ] Tampering with a persisted file causes verification failure and the session is not loaded.
- [ ] Session-history identifiers cannot traverse outside the history root.

## Compare and reporting

- [ ] Compare accepts two verified sessions and shows deterministic comparison output.
- [ ] Human-readable report defaults to redacted identifying fields.
- [ ] Explicit unredacted output is an intentional operator action.
- [ ] HTML report is self-contained and loads without external assets.
- [ ] JSON report remains machine-readable and versioned.

## IPC/security

- [ ] Webview can invoke only the registered session commands.
- [ ] No shell/filesystem/network/updater capability is granted to the main window.
- [ ] Backend rejects invalid numeric/path inputs independently of UI validation.
- [ ] CSP blocks unintended external resource loading.

## Evidence

Record pass/fail, exact build SHA, session manifest SHA-256, PresentMon SHA-256, and remediation for each failed item. Do not attach unredacted diagnostic bundles to public issues.
