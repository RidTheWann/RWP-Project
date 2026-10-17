# Known Limitations

What RWP cannot do today, stated plainly so no result is over-read.

## Frame generation

Parser rows carry `SwapChainAddress` and `FrameType`; analysis groups by the
dominant swap chain and reports what it discards. Generated (interpolated)
frames are labeled, not measured as displayed work: there is no
display-side (photon/VSYNC) measurement, so generated-frame benefit claims
are out of scope.

## Multi-GPU and vendor gaps

The target adapter is selected per-process (never blind index 0), with a
vendor-neutral per-PID GPU Engine path plus NVIDIA details only from the
matched adapter. AMD/Intel-specific clock, temperature, power, and VRAM
fields may stay absent on those vendors; absent stays absent, never zero.
Multi-adapter correlation is unvalidated.

## Anti-cheat and probing

The GUI opt-out disables RWP's extra process/thread probing, but PresentMon
still observes the target process out-of-process. The opt-out reduces
inspection; it is not an anti-cheat compatibility guarantee, and no such
claim is tested.

## Non-conclusions

- Evidence scores are heuristics, not probabilities; 60/100 does not mean
  60% likely.
- A/B comparison is descriptive-only and never causal proof.
- `Unknown` means insufficient evidence, not a clean bill of health.
- `Equivalent` means inside the practical margin, not identical.
- Single-device numbers (sampler overhead, duty-cycle probes) describe one
  box, not hardware in general.

## Deferred categories and inputs

- The shipped pipeline ends at Explain: no Recommend/Optimize output exists.
  Remediation text shown for session failures is troubleshooting, not
  optimization, and the capability order in `docs/product-spec.md` is an
  ordering invariant, not a claim that those stages exist.
- `streaming_overhead` never fires: no encoder telemetry exists to attribute
  against; background activity alone is not used as a proxy.
- ETW and thread-cycle-time cross-checks need an admin session and have not
  run here.
- Real PresentMon fixtures, multi-device runs, and the hardware acceptance
  matrix are outstanding external work.

## Distribution and platform

- The installer is unsigned (SmartScreen warning expected); signing awaits a
  certificate.
- WebView2 runtime ships inside the release bundle (offline installer mode);
  first-run provisioning beyond that path is unverified.
- Live capture (GUI and `rwp capture` CLI) is Windows-only; analysis of
  persisted sessions is platform-neutral. The CLI reuses the GUI session
  orchestration and manifest layout, but live-capture validation on reference
  hardware with real PresentMon remains outstanding.
- A Ctrl+C-interrupted `rwp capture` persists a partial session and exits 0
  with an `interrupted: partial session saved at <dir>` stderr notice (owner
  decision, see `docs/cli-errors.md`); a repeat signal still ends the process
  without persisting. Installing the Ctrl+C handler requires a console.
