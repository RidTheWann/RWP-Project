# Release packaging

## M10 packaging contract

The native Tauri application is packaged for Windows as an NSIS installer.

Packaging configuration:
- Tauri bundling is enabled in `crates/rwp-ui-native/src-tauri/tauri.conf.json`.
- The NSIS target is the explicit Windows installer target.
- The existing `icons/icon.ico` is used as the installer/application icon.
- Product identity remains `RidTheWann Performance` with identifier `dev.ridthewann.performance`.

## External capture dependency

PresentMon remains an external executable selected by the user through the live-session configuration. M10 does not silently redistribute or download a PresentMon binary.

This preserves the existing out-of-process capture boundary and the product requirement that capture has no DLL injection, hooks, or overlay dependency.

## Release gate

A release candidate is not considered ready until:
1. the source-tree `Cargo.lock` is present and validated with `cargo --locked`;
2. the Windows NSIS bundle is built successfully (locally per `docs/local-gates.md`; CI retired);
3. the generated installer is retained as a local artifact with its SHA-256 (binaries are never committed);
4. a Windows acceptance test launches the installed application and exercises a real live-session capture with a user-supplied PresentMon executable.

The first two items are build/release gates; the final item remains a Windows hardware/runtime acceptance test and does not block unrelated repository work.

## Unsigned alpha: SmartScreen expectation

Alpha installers are unsigned (signing awaits a certificate), so Windows
SmartScreen shows an "unknown publisher" warning on first launch. This is
expected and must not be bypassed with repackaging tricks: the honest path is
a certificate plus reputation buildup in the signing phase. The installer is
per-user (no elevation needed) and preserves `%LOCALAPPDATA%\RidTheWann`
user data on uninstall.

## Local installer verification (Fase 6)

- Default bundle (`downloadBootstrapper` mode): 2.11 MiB, per-user silent
  install/uninstall verified, launched app ran 20 s with zero connections
  for its PID.
- Release bundle (`offlineInstaller` per ADR-0029): 208.94 MiB with the
  WebView2 runtime embedded, per-user silent install/uninstall verified,
  launched app ran 20 s with zero connections for its PID.
- Hashes for both bundles are recorded in `docs/autopilot/STATE.md`.
