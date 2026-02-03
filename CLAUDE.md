# CLAUDE.md

Guidelines for working on Radial.

## Project Context

Radial is a task state-management CLI for LLM agents.

## Code Style

### Error Handling
- Use `anyhow` for application errors (`anyhow::Result`, `anyhow::bail!`, `anyhow::Context`)
- Create new custom errors with `thiserror` when the error is specific and especially repeated
- Add context to errors: `.context("failed to open database")`
- Reserve `thiserror` for library-style typed errors only if needed later

### Idiomatic Rust
- Prefer iterators over manual loops
- Use `?` for early returns, not `.unwrap()` (except in tests)
- Favor `impl Into<T>` and `AsRef<T>` for flexible APIs
- Use `Default` trait where appropriate
- Destructure structs and enums explicitly
- Keep functions small and focused

### Comments
- Only comment *why*, not *what*
- Complex logic or non-obvious decisions get comments
- No commented-out code
- No obvious comments like `// create a new goal`

### Logging
- Use `tracing` crate for structured logging
- Add logging at key decision points and state transitions
- Use appropriate levels:
  - `error!` — something failed
  - `warn!` — something unexpected but recoverable
  - `info!` — key operations (goal created, task completed)
  - `debug!` — detailed flow for debugging
  - `trace!` — very verbose, rarely used

Example:
```rust
use tracing::{info, debug, error};

info!(goal_id = %id, "goal created");
debug!(task_id = %id, state = ?new_state, "task state transition");
error!(error = ?e, "failed to write to database");
```

## Documentation
- When writing in longer form documentation, use clear and concise language.
- Aim to provide a clear understanding of exactly what the code does as the north star.
- Use examples to illustrate usage and expected behavior.
- Always aim to write in clean and proper markdown formatting. Do not use emojis.
- Ensure that all documentation is up-to-date and accurate.
- Follow the [Google Style Guide](https://developers.google.com/style) for documentation.
- Aim to not use language like "obvious", "simple", or "trivial" when describing code or process. 

## Workflow

### Build & Test Always
After making changes:
```bash
cargo build
cargo test
```

Do not move on until both pass.

### Clippy-Driven Development
Lean heavily into clippy. Run frequently:
```bash
cargo clippy -- -W clippy::pedantic
```

Fix warnings as they come up, not later. Clippy suggestions often lead to more idiomatic code.

Common clippy allows (if too noisy):
```rust
#![allow(clippy::module_name_repetitions)]
```

### Formatting
Always format before committing:
```bash
cargo fmt
```

## Dependencies

Core deps for MVP:
- `clap` (derive feature) — CLI parsing
- `rusqlite` (bundled feature) — SQLite persistence  
- `nanoid` — short IDs
- `chrono` — timestamps
- `anyhow` — error handling
- `tracing` + `tracing-subscriber` — logging

## File Structure

```
radial/
├── src/
│   ├── main.rs         # CLI entry point, clap setup
│   ├── db.rs           # SQLite persistence layer
│   ├── models.rs       # Goal, Task, Contract, Result structs
│   └── commands/       # Subcommand implementations
│       ├── mod.rs
│       ├── init.rs
│       ├── goal.rs
│       ├── task.rs
│       └── status.rs
├── Cargo.toml
└── CLAUDE.md
```

## Testing

- Unit tests go in the same file as the code (`#[cfg(test)]`)
- Integration tests in `tests/` directory
- Use `tempfile` crate for tests that need a database
- Test the happy path first, then edge cases
