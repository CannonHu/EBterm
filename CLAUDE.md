# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

**toy-term** is a cross-platform terminal debugging tool built with Tauri (Rust backend + Vue 3 frontend). Supports serial port and Telnet connections, with profile management for quick connection switching.

- **Backend**: Rust with Tokio, serial2-tokio, layered architecture (Connection → IPC)
- **Frontend**: Vue 3 + TypeScript + Naive UI + xterm.js
- **Architecture**: Tab = Connection = Context (1:1 mapping)

## Build Commands

### Rust Backend

```bash
# Build the project
cargo build

# Run all tests (unit + integration)
cargo test

# Run tests for a specific module
cargo test connection::tests
cargo test logger::tests
cargo test command::tests

# Run a single test by name
cargo test test_connection_error_open_failed

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
- Modules: `snake_case` (connection, command)
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

### Backend Layering (Connection → IPC)

```
src/
├── connection/          # Low-level connection management
│   ├── mod.rs          # ConnectionError + re-exports
│   ├── types.rs        # Connection trait + config/status/stats types
│   ├── serial.rs       # SerialConnection impl
│   ├── telnet.rs       # TelnetConnection impl
│   └── discovery.rs    # Serial port auto-discovery
├── logger/             # Logging subsystem
│   ├── mod.rs          # LoggerError + re-exports
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
├── state.rs            # AppState (Connection registry + Command manager)
├── ipc.rs              # IPC type definitions
├── connection_context.rs # Per-connection context (connection + stream + logger)
├── data_streamer.rs    # Background task for streaming connection data to frontend
└── commands/           # Tauri command handlers
    ├── mod.rs
    ├── connection.rs   # connect, disconnect, write_data, get_status
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
│   ├── ConfigPanel.vue # Connection config dialog
│   ├── SearchBar.vue   # Terminal search
│   ├── StatusBar.vue   # Connection status & stats
│   ├── ProfileSelectorDialog.vue # Connection profile selection
│   └── SaveProfileDialog.vue # Save current connection as profile
├── stores/             # Pinia stores
│   ├── session.ts      # Tab + connection management
│   ├── terminal.ts     # Terminal UI state + data buffers
│   ├── connection.ts   # Connection state per tab
│   └── ui.ts           # Global UI state
├── composables/        # Reusable logic
│   ├── useTauriEvents.ts # Tauri event listeners
│   └── useTerminal.ts  # Terminal operations
├── services/
│   └── profileStorage.ts # Connection profile persistence
├── router/
│   └── index.ts        # App router
├── types/              # TypeScript types
│   └── ipc.ts          # IPC type definitions
└── api/                # Tauri command wrappers
    └── tauri.ts
```

### Key Design Decisions

1. **Simplified Architecture**: Removed Session layer for lower overhead, now use direct Connection → Context mapping.
2. **Tab = Connection = Context (1:1 mapping)**: Each tab represents one connection context. Simplifies mental model and avoids synchronization issues.
3. **Serial Auto-Discovery**: Automatically scans for available serial ports across all platforms.
4. **Profile Management**: Save and load connection configurations for quick access to frequently used connections.
5. **State Isolation**: Connection state, terminal UI state, and stats are isolated per tab to prevent cross-tab pollution.
6. **16ms Data Polling**: TerminalPane polls data buffers every 16ms and writes to xterm.js in batches, reducing DOM operations.
7. **Event-Driven Architecture**: Backend emits events (`data_received`, `status_changed`) which frontend listens to via `useTauriEvents` composable.

## Dependencies

### Rust Backend
| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime + TCP stream for Telnet |
| `serial2-tokio` | Cross-platform serial port |
| `thiserror` | Error handling |
| `serde` / `serde_json` / `toml` | Serialization |
| `async-trait` | Async trait support |
| `tracing` | Structured logging |
| `uuid` | Unique ID generation |
| `regex` / `once_cell` | Pattern matching + lazy static |

> Telnet实现使用原生tokio TcpStream（无额外Telnet库依赖），适配嵌入式设备常用的裸TCP透传模式

### Frontend
| Package | Purpose |
|---------|---------|
| `vue` 3.4+ | UI framework (Composition API) |
| `pinia` | State management |
| `vue-router` | Client-side routing |
| `naive-ui` | Component library |
| `@vueuse/core` | Vue composables collection |
| `xterm.js` | Terminal emulator |
| `xterm-addon-fit` / `xterm-addon-search` | xterm.js extensions |
| `@tauri-apps/api` | Tauri IPC |
| `@tauri-apps/plugin-dialog` / `plugin-fs` | Tauri system plugins |
| **Bun** | JavaScript runtime (no npm/yarn)

## Important Constraints

- **JavaScript Runtime**: Bun is REQUIRED. Never use npm, yarn, or npx.
- **TDD Workflow**: Write tests FIRST, then implement. All tests must pass before commit.
- **Error Context**: Include relevant data in error variants (port names, timeouts).
- **IPC Safety**: All frontend-bound types must be serializable (derive Serialize, Deserialize).
