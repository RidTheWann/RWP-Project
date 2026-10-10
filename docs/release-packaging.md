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
1. the source-tree `Cargo.lock` is committed and validated with `cargo --locked`;
2. the Windows NSIS bundle is built successfully in CI;
3. the generated installer is retained as a CI artifact;
4. a Windows acceptance test launches the installed application and exercises a real live-session capture with a user-supplied PresentMon executable.

The first two items are build/release gates; the final item remains a Windows hardware/runtime acceptance test and does not block unrelated repository work.
