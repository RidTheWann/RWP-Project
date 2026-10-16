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

## Redesigned UX (0.5.0-beta candidate)

- [ ] First run checks Windows permission and the user-supplied PresentMon component before Start can be enabled.
- [ ] The ordinary Home view hides PresentMon path, output CSV path, PID, and telemetry tuning.
- [ ] The empty game-list state says to open the game and refresh; application selection remains manual because name + PID do not classify a game.
- [ ] Preparation settings accept 5, 10, and 15 seconds and are stored locally only.
- [ ] Countdown uses a monotonic deadline and starts capture only after reaching zero; cancel never invokes session_start.
- [ ] Minimize RWP during the countdown, return to the game, then restore the window; verify countdown and capture start timing (NOT RUN until Windows smoke test).
- [ ] Recording shows elapsed/requested duration and a progressbar only; no live FPS or verdict is shown.
- [ ] Stop displays a generic early-stop confirmation, requests cancellation once, and keeps the UI in Analyzing until Completed or Failed.
- [ ] Failure view presents friendly localized copy and a single retry/repair action; unknown failure provides a technical-detail copy action.
- [ ] Stable and completion notifications are tested from an installed Windows package; toasts are best-effort and never include process identity.
- [ ] Close the window during countdown and capture; verify Stay, confirmed close, child cleanup and terminal-state races.
- [ ] Compare selector disables unverified sessions; Yes sends same_scenario=true, while Not sure and No both send false.
- [ ] Compare shows only Comparable, LimitedSampleEquivalence, or InconclusiveDifferentScenario and never shows numeric deltas for an inconclusive scenario.
- [ ] Verify UI at 900×600, 1120×720 and 1360×860, 125%/150% display scaling, forced-colors, reduced motion, and keyboard/screen reader paths.

## Capture

- [ ] GUI rejects an invalid PresentMon executable path.
- [ ] GUI rejects a path that is not a regular file.
- [ ] GUI rejects an executable whose basename is not `PresentMon.exe`.
- [ ] GUI reports missing PresentMon permissions with a remediation message.
- [ ] Starting a capture shows the Running state with the target PID.
- [ ] Stop requests cancellation and the capture reaches a terminal state.
- [ ] Closing the application does not leave an unowned PresentMon child process.
- [ ] Interrupted capture is represented as interrupted capture state, not a generic failed analysis.
- [ ] "Notify when stable" is checked by default; unchecking it suppresses only the toast, never the banner.
- [ ] A completed capture shows Stable only when the existing stability.v1 diagnosis fires; an observed stutter may show Not stable, while all other unsupported cases remain Stability unknown. The UI must not infer Stable from absence of a diagnosis.
- [ ] Disabling the notification or running where toasts are refused still leaves the banner as the source of truth.

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
- [ ] Status/history polling shows live data, never "not allowed by ACL" or a stuck IPC ERROR badge.
- [ ] No shell/filesystem/network/updater capability is granted to the main window.
- [ ] Backend rejects invalid numeric/path inputs independently of UI validation.
- [ ] CSP blocks unintended external resource loading.

## Evidence

Record pass/fail, exact build SHA, session manifest SHA-256, PresentMon SHA-256, and remediation for each failed item. Do not attach unredacted diagnostic bundles to public issues.

For the real-PresentMon CLI smoke flow (doctor, import, analyze, report,
compare with redacted summary), follow `smoke-test.md`.
