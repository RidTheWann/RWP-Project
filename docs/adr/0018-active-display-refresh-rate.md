# ADR-0018 — Active-display refresh-rate source

- **Status:** accepted
- **Date:** 2026-10-09
- **Scope:** WP3 / AUD-08

## Context

The refresh-aware stutter rule requires the active display refresh rate. Supplying a guessed value would make the detector silently change semantics, which violates the missing-data contract.

## Decision

On Windows, the session boundary resolves the target process to visible top-level windows and:

1. prefers the foreground window when it belongs to the target process;
2. otherwise requires all visible target-process windows to resolve to one unambiguous monitor;
3. resolves that monitor to its device name with GetMonitorInfoW;
4. reads the current display mode with EnumDisplaySettingsExW(..., ENUM_CURRENT_SETTINGS, ...);
5. treats a zero/unusable dmDisplayFrequency, no target window, or ambiguous monitor mapping as missing refresh data rather than synthesizing a value.

The resolver returns the Windows display-device name as provenance alongside the refresh rate.

## Consequences

- Refresh-aware analysis can remain deterministic once the observed refresh rate is captured in session provenance.
- Multi-monitor ambiguity is explicit and yields None.
- Fullscreen/windowless or inaccessible applications may legitimately lack refresh data.
- Exact Win32 semantics and the supported hardware matrix remain VERIFY until validated on Windows 11 hardware.

## References

- Microsoft Win32 display/window APIs: EnumDisplaySettingsExW, GetMonitorInfoW, MonitorFromWindow, EnumWindows.
