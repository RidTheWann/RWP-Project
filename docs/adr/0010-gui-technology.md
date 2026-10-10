# ADR-0010: Native GUI technology

## Status

Accepted.

## Decision

Use Tauri 2 for the Windows native renderer.

Reasons:

- the current Tauri 2.12 release documents an MSRV of Rust 1.90, which is compatible with the repository MSRV of Rust 1.91;
- it provides a native Windows window while using the system WebView2 renderer;
- the presentation layer can remain framework-light with vanilla HTML/CSS/JavaScript;
- the analysis workspace remains independent of GUI dependencies.

The first renderer slice intentionally does not add IPC or live capture controls. It provides a deterministic visual shell only. Live session orchestration is an M8 concern.

## Rejected for this milestone

egui/eframe was not selected because current egui 0.36.2 requires Rust 1.95, which would force an unnecessary MSRV increase.

## Consequences

The project gains Tauri/WebView2 dependencies only in the native UI crate. Core, capture, system, analysis, diagnostics, report, and CLI crates remain GUI-independent.

Packaging is initially disabled. Installer/bundling policy is deferred until end-to-end live capture UX exists.