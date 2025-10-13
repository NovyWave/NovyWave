# Repository Guidelines

## Project Structure & Key References
Workspace crates live in `frontend/`, `backend/`, `shared/`, and `src-tauri/`, compiled via the root `Cargo.toml`. Actor+Relay specs sit in `docs/actors_relays/actor_relay_architecture.md` with migration notes in `docs/actors_relays/novywave/migration_strategy.md`. Domain responsibilities are charted in `.claude/extra/project/domain_map.md`, while full product specs live in `.claude/extra/project/specs/specs.md`. Static fonts ship from `public/`, and waveform fixtures for manual checks are in `test_files/`.

## Actor+Relay Architecture
No raw mutables—domain state must flow through Actors, Relays, or Atoms. Name relays for observed events (`file_loaded_relay`, not `load_file`). Cache current values only inside actor loops, react to relay input, and model concrete domains (`TrackedFiles`, `WaveformTimeline`). Prefer ConnectionMessageActor patterns for cross-component messaging (`.claude/extra/architecture/connection-message-actor-pattern.md`). Never introduce fallback values; expose explicit loading or empty states instead. Revisit `.claude/extra/architecture/actor-relay-patterns.md` and `frontend/src/dataflow/` before touching dataflow code.

## Build, Test, and Development Commands
Run `makers install` once to install MoonZoon, the WASM target, and the Tauri CLI. Use `makers start` for the hot-reload web server (`dev_server.log` captures output). `makers tauri` reuses that server for desktop debugging; `makers tauri-build` emits bundles. Ship web builds with `makers build`; use `makers clean` before regenerating artifacts. Never run `makers start`/`makers build` from automation—coordinate with the dev server owner instead.

## Compilation & MCP Workflow
Rely on the maintainer-run dev server and tail `dev_server.log` until you see "Frontend built"—scan just the newest chunk rather than dumping the full file. Report every warning or `error[E...]` line explicitly. Use Browser MCP (`@browsermcp/mcp`) for visual verification once compilation is clean. If compilation stalls or fails, stop and surface the minimal log excerpt showing the failure rather than attempting local rebuilds.
When debugging, keep console output lean: add temporary `println!/log!` sparingly and remove them before finishing.

Do not run `cargo check`, `cargo build`, or other cargo compilation commands locally unless this document explicitly instructs you to. Treat `dev_server.log` as the source of truth for build status and share relevant excerpts instead of running ad-hoc cargo builds.

Always review the latest `dev_server.log` after making changes. If it shows any compilation errors or warnings, either fix them immediately or add a TODO documenting the follow-up work before proceeding.

## Coding Style & Testing
Use rustfmt defaults (`cargo fmt --all`), keep modules snake_case and exported types in PascalCase, and prefer derived traits (`#[derive(Default, Clone, Copy, Debug)]`) over manual impls. Avoid suppressing lints (`#[allow]`) except for documented dataflow APIs and preserve existing business logic during refactors. Express absence with `Option` instead of sentinel values. Run `cargo clippy --workspace --all-targets` plus `cargo test --workspace` before PRs; refresh fixtures in `test_files/` when parser logic changes. For UI or Tauri updates, smoke-test with the running `makers start` + `makers tauri` stack to catch console regressions.

## Commit & Pull Request Guidelines
Commits are brief sentence-case summaries (`Implement dock-responsive panel layout`). Explain non-obvious choices in the body and link specs or issues. Pull requests should cover user impact, note completed checks (`cargo fmt`, `cargo clippy`, `cargo test --workspace`), and attach screenshots or GIFs for UI differences.

## Agent Notes
- When the user asks to "create todos", log them in the Codex CLI todo tool instead of modifying repository files (e.g., avoid writing TODOs into `.novywave`).
- Never fully revert or reset ongoing work unless the user explicitly requests and confirms it; prefer incremental corrections that preserve recent changes.
- Avoid blocking config saves on extra "timeline ready" flags or similar coordination in `frontend/src/config.rs`; that pattern has repeatedly left the backend stuck logging `timeline query start` without ever sending `UnifiedSignalResponse`. Restore timeline state by replaying the saved values through existing relays without delaying the config saver.
- When restoring timeline state, guard the bounds listener so a transient `None` range doesn’t wipe the restored viewport—set a flag once config playback runs and ignore the empty bounds ping unless no variables remain.
- Plugin builds are managed via `makers build_plugins` / `makers watch_plugins`. The team runs the watcher (output in `dev_plugins.log`), and agents read that log rather than spamming the main terminal.
- Reload watcher debugging playbook (Oct 2025): if `ReloadWaveformFiles` isn’t triggering a frontend refresh, instrument the browser console to verify (1) the DownMsg lands (`ConnectionMessageActor` log), (2) the relay subscriber count, and (3) whether `TrackedFiles::reload_existing_paths` fires. If the relay shows `0` subscribers or the listener drops before the event arrives, stop using a separate relay/actor and handle the DownMsg directly inside `ConnectionMessageActor` by calling `process_selected_file_paths`, which reuses the dialog’s add/reload path and avoids timing races.
