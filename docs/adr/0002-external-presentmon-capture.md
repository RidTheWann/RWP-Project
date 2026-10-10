# ADR-0002 — External PresentMon capture

- Status: Accepted (reconstructed)
- Context: Capture must not inject DLLs, hook the target process, or require an overlay.
- Decision: Run PresentMon as a separate process and consume its emitted data through the capture boundary. The engine does not load capture code into the target application.
- Consequences: Capture lifecycle, executable identity, stderr, cancellation, and schema compatibility are explicit boundaries; PresentMon remains an external dependency.
