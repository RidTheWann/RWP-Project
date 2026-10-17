# Owner Smoke-Test Kit (Real PresentMon)

This is the manual acceptance companion for real-PresentMon evidence (WP4/WP9,
RC-06/RC-07/RC-12). It stays `BLOCKED_ON_USER` until run on reference hardware
with a real PresentMon v2 capture. Synthetic fixtures prove the kit works but
are never acceptance evidence.

> If the input CSV lives under `crates/**/fixtures`, every generated summary
> header says `SYNTHETIC FIXTURE - NOT ACCEPTANCE EVIDENCE`. Do not paste a
> synthetic summary as hardware proof.

## What the kit does

`tools/smoke-test.ps1` (PowerShell 5.1+ compatible, no admin, no network, no
new tools) runs:

1. `rwp doctor [--presentmon <path>]` and records the PresentMon SHA-256 plus
   the doctor-reported version when available.
2. `import-presentmon <csv> --process-id <pid> --qpc-frequency-hz <hz>
   --output-dir <OutDir>/session-imported`, or notes that GUI capture was used.
3. `analyze --session <imported> --output-dir <OutDir>/session-analyzed`.
4. `report --session <analyzed> --format md` (redacted by default).
5. `compare --baseline <dir> --candidate <dir> --same-scenario` only when two
   session dirs are given via `-Baseline`/`-Candidate`.

Every step's command, exit code, and output is recorded under `OutDir`
(default `smoke-out/`, git-ignored). A redacted `summary.md` replaces the
Windows username, full user-profile paths, and machine name with `<user>`,
`<profile>`, and `<machine>`. Prove redaction any time with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/smoke-test.ps1 -SelfCheck
```

Expected: three `SELFCHECK sample:` lines plus
`SELFCHECK RESULT: PASS (username/profile/machine redacted)`.

## Owner steps (real PresentMon)

1. Install Intel PresentMon v2. Note the exact version and the file SHA-256:
   `Get-FileHash -LiteralPath <path>\PresentMon.exe -Algorithm SHA256`.
2. From the repo root, run the kit (examples; adjust PID/QPC to your capture):
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File tools/smoke-test.ps1 `
     -PresentMon C:\Tools\PresentMon.exe `
     -CsvPath C:\Captures\run.csv `
     -ProcessId 1234 `
     -QpcFrequencyHz 10000000 `
     -OutDir smoke-out
   ```
   For a GUI live capture instead of a CSV, omit `-CsvPath`; the kit notes
   `GUI capture was used` and still runs `doctor`. To analyze a GUI session
   dir manually, use `rwp analyze`/`report`/`compare` per `docs/cli-reference.md`.
   GUI cancel check (graceful stop without a console, all `BLOCKED_ON_USER`
   until run on reference hardware; never fake): start a capture in the
   release GUI, cancel it mid-capture, and confirm (a) no console window ever
   appears, (b) the capture reaches the interrupted (not failed) state,
   (c) no `PresentMon.exe` remains in Task Manager afterwards, (d)
   `logman query -ets` in an elevated prompt shows no leftover PresentMon
   trace session, and (e) the partial `presentmon.csv` parses with
   `interrupted=true` in the persisted report. Record each observation in
   STATE before calling AUD-03 done on real hardware.
3. For a live CLI capture with real PresentMon (`BLOCKED_ON_USER` until run on
   reference hardware; never fake):
   ```powershell
   rwp doctor --presentmon C:\Tools\PresentMon.exe
   rwp capture --presentmon C:\Tools\PresentMon.exe --process-id 1234 --duration-seconds 60 --output-dir sessions/live
   rwp report --session sessions/live --format md
   ```
   By process name (fails closed on zero/multiple matches; then use `--process-id`):
   ```powershell
   rwp capture --presentmon C:\Tools\PresentMon.exe --process-name Game.exe --duration-seconds 60 --interval-ms 50 --output-dir sessions/live
   ```
   With probing disabled and an explicit refresh rate (otherwise auto-detect):
   ```powershell
   rwp capture --presentmon C:\Tools\PresentMon.exe --process-id 1234 --duration-seconds 60 --no-process-probing --refresh-rate-hz 144 --output-dir sessions/live
   ```
   Safety check: while `rwp capture` runs, confirm PresentMon appears as a
   child in Task Manager; press Ctrl+C once and confirm the CLI prints the
   `interrupted: partial session saved` notice, exits 0, and no
   `PresentMon.exe` remains afterwards (a repeat signal still ends the
   process; the shared kill-on-close Job Object terminates PresentMon either
   way). Record the PresentMon path/version/SHA-256, the rwp binary
   SHA-256, the session `manifest.json` hash, and the redacted `report --format
   md` output in STATE before calling RC-06/RC-07 done.
   Ctrl+C drill (60 s capture, interrupt at about 20 s; all `BLOCKED_ON_USER`
   until run on reference hardware; never fake): start
   `rwp capture --presentmon C:\Tools\PresentMon.exe --process-id 1234
   --duration-seconds 60 --output-dir sessions/live-interrupted`, press Ctrl+C
   once at about 20 s, and confirm (a) the CLI prints
   `interrupted: partial session saved at sessions/live-interrupted` on
   stderr and exits 0, (b) the session persists with `interrupted=true` in
   both manifest and report, (c) no `PresentMon.exe` remains in Task Manager,
   and (d) `logman query -ets` shows no leftover PresentMon trace session.
   Pressing Ctrl+C a second time while the stop is in progress must still end
   the process (default handling; Job Object kills PresentMon). Record the
   exit code, the notice text, and the manifest hash in STATE.
   2 s measurement (owner data for the graceful-wait value, `BLOCKED_ON_USER`
   until measured on real PresentMon, ideally under load): time from the
   Ctrl+C keypress to the `interrupted: partial session saved` notice (or to
   process exit on the second signal) with a stopwatch across several runs,
   including one run with a busy target, and paste the samples in STATE. The
   code keeps the documented `GRACEFUL_STOP_TIMEOUT` (2 s); change it only
   with measured data, never blind.
4. For a two-session A/A or A/B comparison, re-run or pass existing dirs:
   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File tools/smoke-test.ps1 `
     -Baseline smoke-out/session-analyzed `
     -Candidate smoke-out-second/session-analyzed `
     -OutDir smoke-out-compare
   ```
   The kit appends `compare --same-scenario` output to the new summary.
5. Open `OutDir/summary.md`. Confirm the header is
   `# RWP SMOKE TEST SUMMARY (redacted)` (not the synthetic warning), that
   `<user>`/`<profile>`/`<machine>` appear, and that no raw username, profile
   path, or machine name remains.
6. Paste into `docs/autopilot/STATE.md` (see template below): each step's
   command + exit code, the PresentMon path/version/SHA-256, the rwp binary
   SHA-256, the session manifest hashes, and the redacted summary. Never paste
   raw logs with PII into public issues.

## What to paste into STATE.md (results template)

```markdown
- <date> Owner smoke test (real PresentMon): `rwp doctor --presentmon <path>`
  EXIT <code> (presentmon major_version=<v> supported=<bool>; both null when
  the binary carries no version resource — version-unknown is advisory, not
  a failure); PresentMon
  SHA256 `<hash>`; rwp binary SHA256 `<hash>`.
- `import-presentmon <csv> --process-id <pid> --qpc-frequency-hz <hz>`
  EXIT <code> -> <session-dir> (manifest SHA256 `<hash>`).
- `analyze --session <in> --output-dir <out>` EXIT <code>.
- `report --session <analyzed> --format md` EXIT <code> (redacted; interrupted=<bool>).
- `compare --baseline <a> --candidate <b> --same-scenario` EXIT <code>
  (comparability=<value>) — only when two sessions were compared.
- Redacted summary header: `# RWP SMOKE TEST SUMMARY (redacted)` (or the
  synthetic warning when fixtures were used). Real-PresentMon acceptance:
  <DONE | BLOCKED_ON_USER with reason>.
```

Manifest hashes: after `analyze`, hash `manifest.json` inside the analyzed
session dir and record it. The kit's `run.log` already lists every command and
exit code; copy from there.

## Anonymizing a CSV before sharing it

Never share a raw capture with PII or sensitive process lists. To produce a
shareable sample:

1. Copy only the needed rows (a few hundred frames is plenty for a parser
   repro) into a new file. Keep the header exactly.
2. Replace the `Application` column with a neutral name (for example
   `Game.exe`) unless the exact process name is the point of the report.
3. Confirm all rows share one `ProcessID` or document why PIDs change; the
   parser rejects PID changes by design.
4. Note any `CPUStartQPC` rows that move backwards (real multi-threaded output
   does this occasionally); the parser drops and counts them instead of
   failing, and PID changes are still rejected by design.
5. Delete any trailing partial line if the capture was killed mid-flush, or
   keep it only to repro tolerant-tail handling (note which mode you used).
6. Run `import-presentmon` on the anonymized copy and confirm it parses before
   sending it. Attach only the anonymized file plus the redacted `summary.md`.

Fixtures under `crates/**/fixtures` are anonymized and minimal; they prove
tooling only. `crates/rwp-capture/tests/fixtures/real-capture-v2-anonymized.csv`
follows this policy on a real 2026-10-17 capture (Application renamed,
windowed to 320 rows); the raw capture stays local and is the acceptance
evidence, never the fixture.

## Evidence status

- Synthetic-fixture run (this machine, no PresentMon): proven via
  `tools/smoke-test.ps1 -CsvPath
  crates/rwp-cli/tests/fixtures/minimal-presentmon-v2.csv`; summary header is
  `SYNTHETIC FIXTURE - NOT ACCEPTANCE EVIDENCE`; redaction self-check passes.
- Real PresentMon v2 flow: DONE 2026-10-17 (elevated live CLI capture,
  PresentMon 2.6.0): `doctor` EXIT 0 (permissions ok under elevation), two
  60 s `dwm.exe` captures EXIT 0, `analyze`/`report`/`compare` EXIT 0 with
  manifest hashes recorded in `docs/acceptance/2026-10-17.md`; anonymized
  regression fixture landed at
  `crates/rwp-capture/tests/fixtures/real-capture-v2-anonymized.csv`.
  Hardware acceptance matrix and the GUI cancel drill remain
  `BLOCKED_ON_USER`. Nothing was faked.

See also: `gui.md` (manual GUI checklist) and `../release/README.md` (local
release artifacts index).
