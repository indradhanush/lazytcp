# AGENTS Guide for `tcpdump-tui`

## Purpose

This repository hosts a Rust TUI prototype built with `ratatui` for interactive packet monitoring.

The core problem to solve:
- `tcpdump` and `tshark` are powerful but cumbersome for iterative live exploration.
- Changing filters quickly by host, destination, protocol, and traffic type should be first-class.

The near-term goal is a focused operator flow:
1. Start capture from a selected interface or input source.
2. Show a live packet list with useful summaries.
3. Apply, refine, and clear filters without restarting the app.
4. Drill into selected packet details while capture continues.

Do not broaden scope into a full Wireshark replacement.

## Technology Constraints

- Language: Rust
- UI framework: `ratatui`
- Keep shell/process integration explicit and auditable when invoking external capture tools.

## Expected Repository Layout

- `src/main.rs`
  - Terminal lifecycle (raw mode, alternate screen), event loop, and app run loop wiring.
- `src/app.rs`
  - UI-facing state (`App`), selection/navigation behavior, and filter edit state.
- `src/domain.rs`
  - Typed model for packets, endpoints, protocols, and filter expressions.
- `src/capture.rs`
  - Capture backend abstraction and runtime integration (for `tcpdump`/`tshark` adapters or future native capture).
- `src/ui.rs`
  - `ratatui` rendering for capture pane, packet list, filter input, and packet detail.
- `plans/`
  - All planning artifacts live here (for example: `plans/20260309-v1-foundation.org`).
- Primary Ratatui reference for implementation patterns: `~/github.com/ratatui/examples`
- Read [RATATUI.md](./RATATUI.md) as a secondary local reference when working on ratatui APIs.

## Navigation Order for New Work

When making changes, read files in this order:
1. `plans/` (relevant plan file for the current task, if present)
2. `src/domain.rs`
3. `src/capture.rs`
4. `src/app.rs`
5. `src/ui.rs`
6. `src/main.rs`

This keeps domain and capture contracts stable before UI changes.

## V1 Scope Guardrails (Do Not Drift)

- Local, single-session capture workflow first.
- Interactive filtering is the primary feature.
- Filter dimensions in scope: host/source/destination/protocol/port/traffic class.
- Live filter updates must not require restarting the TUI.
- Avoid broad dashboards, remote orchestration, or multi-node packet correlation unless explicitly requested.

## Iterative Change Workflow (Required)

Use small, verifiable slices:
1. Implement one narrow behavior.
2. Run formatting and checks.
3. Run tests (or add them first for domain/capture logic changes).
4. Confirm no regressions before expanding scope.

## Plan Scope and Precedence

- Plan mode is required for non-trivial work (3+ steps, multi-file implementation, architecture decisions, or production-risk behavior changes).
- All plans must live in the `plans/` subdirectory.
- Plan files must be Org mode files (`.org`).
- Plan filenames must start with a date prefix in `YYYYMMDD-` format.
- No-plan exception is allowed for lightweight tasks when all are true:
  - 1-2 steps only,
  - command-only or one small localized edit,
  - no architectural/design decision,
  - no production-risk behavior change.
- For lightweight tasks, execute directly and run the minimum verification commands from `Verification Policy`.
- This repository `AGENTS.md` overrides global planning defaults for work in `tcpdump-tui`.
- If instruction conflicts remain unclear, ask Dhanush before proceeding.

## Verification Policy (Required)

- Run Rust verification commands when a change affects Rust source, dependencies, build behavior, or runtime behavior.
- Do not run cargo verification for non-Rust-only changes (for example: `.github/workflows/*`, `Makefile`, docs, plan files) unless explicitly requested.
- Do not mark work complete until verification passes, or explicitly document why a command could not be run.
- Plan files should not duplicate verification command checklists; they should only record verification outcomes.

Minimum Rust verification commands (when applicable):

```bash
cargo fmt
cargo test
cargo check
```

For interactive verification:

```bash
cargo run
```

## Commit Discipline (Required)

- Use the user-level subagent `git-commit-agent` for commit workflows.
  - Agent path: `~/.claude/agents/git-commit-agent.md`
  - Default invocation pattern: "Use `git-commit-agent` to commit current changes."
- Commit frequently in small, reviewable increments.
- Use conventional commit format (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
- Prefer one logical concern per commit; avoid mixing unrelated changes.
- Before each commit, ensure applicable `Verification Policy` commands have been run for the current changes.
- If the tree is dirty, stage and commit the relevant finished slice before starting a new one.

## Testing Guidance

- Add unit tests in `src/domain.rs` for filter parsing/normalization and match behavior.
- Add tests in `src/capture.rs` for backend argument construction and parsing boundaries.
- Keep capture-facing tests deterministic (mock process output unless explicitly testing integration behavior).
- If UI behavior changes, keep baseline keyboard navigation (`q`, `j/down`, `k/up`) stable unless requirements change.

## Change Strategy

- Prefer evolving `domain.rs` and `capture.rs` contracts before UI refactors.
- Keep `ui.rs` as a pure rendering layer driven by `App` state.
- Keep error handling explicit and always restore terminal state on all exits.
- Avoid new dependencies unless necessary for the accepted scope.
