# Repository Guidelines

## Project Structure & Key References
Workspace crates live in `frontend/`, `backend/`, `shared/`, and `src-tauri/`, compiled via the root `Cargo.toml`. Actor+Relay specs sit in `docs/actors_relays/actor_relay_architecture.md` with migration notes in `docs/actors_relays/novywave/migration_strategy.md`. Domain responsibilities are charted in `.claude/extra/project/domain_map.md`, while full product specs live in `.claude/extra/project/specs/specs.md`. Static fonts ship from `public/`, and waveform fixtures for manual checks are in `test_files/`.

## Actor+Relay Architecture
No raw mutables—domain state must flow through Actors, Relays, or Atoms. Name relays for observed events (`file_loaded_relay`, not `load_file`). Cache current values only inside actor loops, react to relay input, and model concrete domains (`TrackedFiles`, `WaveformTimeline`). Prefer ConnectionMessageActor patterns for cross-component messaging (`.claude/extra/architecture/connection-message-actor-pattern.md`). Never introduce fallback values; expose explicit loading or empty states instead. Revisit `.claude/extra/architecture/actor-relay-patterns.md` and `frontend/src/dataflow/` before touching dataflow code.

## Build, Test, and Development Commands
Run `makers install` once to install MoonZoon, the WASM target, and the Tauri CLI. Use `makers start` for the hot-reload web server (`dev_server.log` captures output). `makers tauri` reuses that server for desktop debugging; `makers tauri-build` emits bundles. Ship web builds with `makers build`; use `makers clean` before regenerating artifacts. Never run `makers start`/`makers build` from automation—coordinate with the dev server owner instead.

## Compilation & MCP Workflow
Rely on the maintainer-run dev server and tail `dev_server.log` until you see "Frontend built". Report every warning or `error[E...]` line explicitly. Use Browser MCP (`@browsermcp/mcp`) for visual verification once compilation is clean. If compilation stalls or fails, stop and surface the log output rather than attempting local rebuilds.

## Coding Style & Testing
Use rustfmt defaults (`cargo fmt --all`), keep modules snake_case and exported types in PascalCase, and prefer derived traits (`#[derive(Default, Clone, Copy, Debug)]`) over manual impls. Avoid suppressing lints (`#[allow]`) except for documented dataflow APIs and preserve existing business logic during refactors. Express absence with `Option` instead of sentinel values. Run `cargo clippy --workspace --all-targets` plus `cargo test --workspace` before PRs; refresh fixtures in `test_files/` when parser logic changes. For UI or Tauri updates, smoke-test with the running `makers start` + `makers tauri` stack to catch console regressions.

## Commit & Pull Request Guidelines
Commits are brief sentence-case summaries (`Implement dock-responsive panel layout`). Explain non-obvious choices in the body and link specs or issues. Pull requests should cover user impact, note completed checks (`cargo fmt`, `cargo clippy`, `cargo test --workspace`), and attach screenshots or GIFs for UI differences.
