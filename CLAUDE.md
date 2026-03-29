# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**embedded-debugger** is a cross-platform embedded board debugging tool built with Tauri (Rust backend + Vue 3 frontend). It supports serial port and Telnet connections.

- **Backend**: Rust with Tokio, serial2-tokio, layered architecture (Connection → Session → IPC)
- **Frontend**: Vue 3 + TypeScript + Naive UI + xterm.js
- **Architecture**: Tab = Connection = Session (1:1 mapping)

## Build Commands

### Rust Backend

```bash
# Build the project
cargo build

# Run all tests (unit + integration)
cargo test

# Run tests for a specific module
cargo test connection::tests
cargo test session::tests
cargo test logger::tests
cargo test command::tests

# Run a single test by name
cargo test test_connection_error_open_failed

# Run integration tests only
cargo test --test command_integration
cargo test --test logger_integration
cargo test --test telnet_integration

# Run with output visible
cargo test -- --nocapture

# Check code without building
cargo check

# Lint with clippy
cargo clippy -- -D warnings
```

### Frontend (BUN REQUIRED - NOT npm/npx)

```bash
cd frontend

# Install dependencies
bun install

# Development server
bun run dev

# Production build (includes type check)
bun run build

# Preview production build
bun run preview

# Type checking
bunx tsc --noEmit
```

### Tauri App

```bash
# Development mode (hot reload)
cd src-tauri && cargo tauri dev

# Build production app
cd src-tauri && cargo tauri build
```

## Code Style Guidelines

### Rust Conventions

**Imports**: Standard library first, then external crates, then local modules.

**Error Handling**:
- Use `thiserror` for error enums with structured variants
- Each module defines its own error type (e.g., `ConnectionError`, `LoggerError`)
- Error variants carry context: `OpenFailed { port: String, reason: String }`
- Include `code()` method returning `&'static str` for IPC error codes
- Use `#[from]` for automatic error conversion where appropriate

**Naming**:
- Modules: `snake_case` (connection, session_manager)
- Types/Structs: `PascalCase` (ConnectionError, SerialConfig)
- Constants: `SCREAMING_SNAKE_CASE` (EVENT_PREFIX, DEFAULT_TIMEOUT)
- Methods: `snake_case` (connect, read, write)

**Type Definitions**:
- Derive `Debug, Clone` for all data types
- Derive `Copy` for small enums without heap data
- Use `serde` derives for IPC types: `Serialize, Deserialize`
- Implement `Default` where sensible (configs, stats)

### Testing Patterns (TDD - STRICTLY ENFORCED)

**TDD Workflow**:
1. Write test FIRST
2. Run test (expect failure)
3. Implement minimal code to pass
4. Refactor if needed
5. All tests must pass before commit

**Test Structure (Given/When/Then)**:
```rust
#[test]
fn test_feature_scenario() {
    // Given: setup context
    let config = SerialConfig::default();

    // When: action
    let result = validate_config(&config);

    // Then: assertions
    assert!(result.is_ok());
    assert_eq!(result.unwrap().port, "");
}
```

**Unit Tests**:
- Place in same file as code under `#[cfg(test)] mod tests`
- Test all error variants and edge cases
- Test boundary values (empty strings, max values, special chars)

**Integration Tests**:
- Place in `tests/` directory
- Use `tests/common.rs` for shared helpers
- Use `tempfile::TempDir` for filesystem tests

## High-Level Architecture

### Backend Layering (Connection → Session → IPC)

```
src/
├── connection/          # Low-level connection management
│   ├── mod.rs          # ConnectionError + re-exports
│   ├── traits.rs       # Connection trait definition
│   ├── types.rs        # Config/Status/Stats types
│   ├── serial.rs       # SerialConnection impl
│   └── telnet.rs       # TelnetConnection impl
├── session/            # Session lifecycle management
│   ├── mod.rs          # SessionError + re-exports
│   ├── state.rs        # SessionState enum
│   ├── types.rs        # Session types
│   ├── manager.rs      # SessionManager impl
│   └── connection_registry.rs  # Connection-to-Session mapping
├── logger/             # Logging subsystem
│   ├── mod.rs          # LoggerError + re-exports
│   ├── traits.rs       # Logger trait
│   └── file.rs         # FileLogger impl
├── command/            # Command file parsing
│   ├── mod.rs          # CommandError + re-exports
│   ├── parser.rs       # CommandParser trait + impl
│   └── manager.rs      # CommandManager impl
└── error.rs            # Global Error type wrapping all module errors
```

### Tauri IPC Bridge (src-tauri/src/)

```
src-tauri/src/
├── main.rs             # Tauri app entry, command handlers
├── state.rs            # AppState (SessionManager + Logger registry)
├── ipc.rs              # IPC type definitions (CommandRequest, CommandResponse)
├── data_streamer.rs    # Background task for streaming connection data to frontend
└── commands/           # Tauri command handlers
    ├── mod.rs
    ├── connection.rs   # connect, disconnect, write_data, get_status
    ├── session.rs      # create_session, close_session, list_sessions
    ├── logging.rs      # start_logging, stop_logging, get_log_status
    └── command.rs      # execute_command, load_command_file
```

### Frontend Architecture

```
frontend/src/
├── components/         # Vue components
│   ├── MainLayout.vue  # Terminal-first layout
│   ├── TabBar.vue      # Tab management with connection status
│   ├── TerminalPane.vue # Per-tab container with polling
│   ├── Terminal.vue    # xterm.js wrapper
│   ├── ConfigPanel.vue # Floating connection config
│   ├── SearchBar.vue   # Terminal search
│   └── StatusBar.vue   # Connection status & stats
├── stores/             # Pinia stores
│   ├── session.ts      # Tab + session management
│   ├── terminal.ts     # Terminal UI state + data buffers
│   └── connection.ts   # Connection state per session
├── composables/        # Reusable logic
│   └── useTauriEvents.ts # Tauri event listeners
├── types/              # TypeScript types
│   └── ipc.ts          # IPC type definitions
└── api/                # Tauri command wrappers
    └── tauri.ts
```

### Key Design Decisions

1. **Tab = Connection = Session (1:1 mapping)**: Each tab represents one connection/session. Simplifies mental model and avoids synchronization issues.

2. **Dual ID System**: Frontend tabs use `tabId` (UUID), backend sessions use `sessionId`. `tabId` exists before connection; `sessionId` assigned after successful connection.

3. **State Isolation by Session**: Connection state, terminal UI state, and stats are stored in Maps keyed by `sessionId` to prevent cross-tab pollution.

4. **16ms Data Polling**: TerminalPane polls data buffers every 16ms and writes to xterm.js in batches, reducing DOM operations.

5. **Event-Driven Architecture**: Backend emits events (`data_received`, `status_changed`) which frontend listens to via `useTauriEvents` composable.

## Dependencies

### Rust Backend
- `tokio` - Async runtime
- `serial2-tokio` - Cross-platform serial port
- `mini-telnet` - Telnet client
- `thiserror` - Error derive macros
- `serde` / `serde_json` - Serialization
- `tauri` - Desktop app framework

### Frontend
- `vue` 3.4+ with Composition API
- `pinia` - State management
- `xterm.js` - Terminal emulator
- `naive-ui` - Component library
- `@tauri-apps/api` - Tauri IPC
- **BUN as JavaScript runtime** (NOT npm/npx)

## Important Constraints

- **JavaScript Runtime**: Bun is REQUIRED. Never use npm, yarn, or npx.
- **TDD Workflow**: Write tests FIRST, then implement. All tests must pass before commit.
- **Error Context**: Include relevant data in error variants (port names, timeouts).
- **IPC Safety**: All frontend-bound types must be serializable (derive Serialize, Deserialize).
