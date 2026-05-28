# `/goal` Prompt: Desktop Startup Release Hardening

Use this prompt from `/home/martinkavik/repos/NovyWave`:

```text
/goal Implement docs/plans/desktop_startup_release_hardening.md end to end.

Work in small, verified jj commits. Follow AGENTS.md strictly: use makers workflows instead of ad-hoc cargo check/build, do not revert unrelated work, and update the plan document after each completed slice with current evidence.

Primary done criteria:
- Fresh packaged launches do not create or load a workspace from bundled resources.
- Windows NSIS/MSI, macOS DMG/app bundle, Linux AppImage/deb/rpm, and Chrome packages have CI startup or payload proof appropriate to the artifact.
- Missing backend, config load failures, root discovery failures, stale plugin artifacts, and Chrome discovery failures produce visible actionable errors plus logs where possible.
- Chrome packages are built from WEB frontend artifacts on every platform and fail CI if Tauri bindings or dev-server URLs leak into shipped frontend files.
- Issue #1 is not closed as fixed until a Windows release artifact built from the fixed commit proves backend presence, window creation, config readiness, platform roots, and visible error handling.

Hard gates:
- Commit each verified slice with jj before moving to the next high-risk area.
- Report every warning or error line from supported makers builds.
- Do not mark the goal complete while any verification gate in the plan remains unproven.
```
