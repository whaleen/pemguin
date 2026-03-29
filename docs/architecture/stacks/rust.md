# Stack: Rust

For Rust projects — TUI apps, API servers, CLIs.

## Tools

- **Language**: Rust (stable)
- **Build**: Cargo
- **Workspace**: Cargo workspaces for multi-crate projects

## Key Commands

```bash
cargo run            # run
cargo build          # debug build
cargo build --release  # optimized build
cargo test           # tests
cargo clippy         # lint
cargo fmt            # format
```

## Common Crates

| Purpose | Crate |
|---------|-------|
| TUI | ratatui, crossterm |
| Async | tokio |
| HTTP server | axum |
| HTTP client | reqwest |
| Serialization | serde, serde_json |
| CLI args | clap |
| Logging | tracing, tracing-subscriber |
| Error handling | anyhow, thiserror |

## Project Structure

```
src/
  main.rs        — entry point, stays minimal
  lib.rs         — core logic
Cargo.toml
```

For workspaces:
```
apps/
  my-app/
    src/
    Cargo.toml
Cargo.toml         — workspace root
```

## Conventions

- `main.rs` stays minimal — logic lives in `lib.rs` or modules
- Use `anyhow` for application errors, `thiserror` for library errors
- Format with `cargo fmt` before committing
