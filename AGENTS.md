# AGENTS.md - Embedded Debugger

## Project Overview

**embedded-debugger**: Cross-platform embedded board debugging tool
- **Backend**: Rust + Tauri (serial/Telnet connections, session management)
- **Frontend**: Vue 3 + TypeScript + Naive UI + xterm.js
- **Architecture**: Layered backend (Connection → Session → IPC), Tauri bridge, Vue frontend

## Build & Test Commands

### Rust Backend
```bash
# Build entire project
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
cargo test test_parse_real_command_file

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

### Tauri App
```bash
# Development mode (hot reload)
cd src-tauri && cargo tauri dev

# Build production app
cd src-tauri && cargo tauri build
```

### Frontend
```bash
cd frontend
bun install
bun run dev        # Development server
bun run build      # Production build (includes type check)
bun run preview    # Preview production build

# Type checking
bunx tsc --noEmit

# Run Vite commands directly
bunx --bun vite
```

## Code Style Guidelines

### Rust Conventions

**Imports:**
- Standard library first, then external crates, then local modules
- Use `pub use` for re-exports at module boundaries
- Prefer explicit imports over glob (`use std::io::Error` not `use std::io::*`)

**Error Handling:**
- Use `thiserror` for error enums with structured variants
- Each module defines its own error type (e.g., `ConnectionError`, `LoggerError`)
- Error variants carry context: `OpenFailed { port: String, reason: String }`
- Include `code()` method returning `&'static str` for IPC error codes
- Use `#[from]` for automatic error conversion where appropriate
- Global `Error` type in `src/error.rs` wraps module errors via `#[from]`

**Naming:**
- Modules: `snake_case` (connection, session_manager)
- Types/Structs: `PascalCase` (ConnectionError, SerialConfig)
- Constants: `SCREAMING_SNAKE_CASE` (EVENT_PREFIX, DEFAULT_TIMEOUT)
- Methods: `snake_case` (connect, read, write)

**Documentation:**
- Module-level: `//! Module description`
- Public items: `/// Description` doc comments
- Include examples in complex public APIs

**Type Definitions:**
- Derive `Debug, Clone` for all data types
- Derive `Copy` for small enums without heap data
- Use `serde` derives for IPC types: `Serialize, Deserialize`
- Implement `Default` where sensible (configs, stats)

### Testing Patterns

**TDD Workflow (STRICTLY ENFORCED):**
1. Write test FIRST
2. Run test (expect failure)
3. Implement minimal code to pass
4. Refactor if needed
5. All tests must pass before commit

**Test Structure (Given/When/Then):**
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

**Unit Tests:**
- Place in same file as code under `#[cfg(test)] mod tests`
- Test all error variants and edge cases
- Test boundary values (empty strings, max values, special chars)

**Integration Tests:**
- Place in `tests/` directory
- Use `tests/common.rs` for shared helpers
- Use `tempfile::TempDir` for filesystem tests

**Test Helpers:**
```rust
// tests/common.rs
pub fn create_test_dir() -> TempDir { ... }
pub fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf { ... }
```

### Module Organization

```
src/
├── lib.rs              # Public API exports
├── error.rs            # Global error type
├── connection/
│   ├── mod.rs          # ConnectionError + tests
│   ├── traits.rs       # Connection trait definition
│   ├── types.rs        # Config/Status/Stats types
│   ├── serial.rs       # SerialConnection impl
│   └── telnet.rs       # TelnetConnection impl
├── session/
│   ├── mod.rs          # SessionError + tests
│   ├── state.rs        # SessionState enum
│   └── manager.rs      # SessionManager impl
├── logger/
│   ├── mod.rs          # LoggerError + tests
│   ├── traits.rs       # Logger trait
│   └── file.rs         # FileLogger impl
└── command/
    ├── mod.rs          # CommandError + tests
    ├── parser.rs       # CommandParser trait + impl
    └── manager.rs      # CommandManager impl
```

### IPC Types (src-tauri)

- Commands: Frontend → Backend operations (connect, disconnect, write)
- Events: Backend → Frontend notifications (data_received, status_changed)
- All IPC types must derive `Serialize, Deserialize`
- Use `#[serde(tag = "type", rename_all = "snake_case")]` for enums

## Key Dependencies

### Rust
- `tokio` - Async runtime
- `serial2-tokio` - Cross-platform serial port
- `mini-telnet` - Telnet client
- `thiserror` - Error derive macros
- `serde` / `serde_json` - Serialization
- `async-trait` - Async trait support
- `uuid` - Session IDs
- `chrono` - Timestamps

### Frontend (Phase 3)
- `vue` 3.4+ with Composition API
- `pinia` - State management
- `xterm.js` - Terminal emulator
- `naive-ui` - Component library
- `@tauri-apps/api` - Tauri IPC

## Development Workflow

1. **Always run tests before committing**: `cargo test`
2. **Check clippy warnings**: `cargo clippy -- -D warnings`
3. **Follow TDD**: Write test → Fail → Implement → Pass → Refactor
4. **Error context**: Include relevant data in error variants (port names, timeouts)
5. **IPC safety**: All frontend-bound types must be serializable

## IMPORTANT CONSTRAINTS

### JavaScript Runtime: Bun (NOT npm/yarn/npx)

**This project uses Bun as the JavaScript runtime. ALL frontend commands must use bun:**

| Wrong (npm/yarn/npx) | Correct (bun) |
|---------------------|---------------|
| `npm install` | `bun install` |
| `npm run dev` | `bun run dev` |
| `npm run build` | `bun run build` |
| `npx vite` | `bunx --bun vite` |
| `npx tsc` | `bunx tsc` |
| `npx create-xxx` | `bun create xxx` |

**When delegating to agents, ALWAYS specify in the prompt:**
- "Use bun as the JS runtime, NOT npm/yarn/npx"
- "Run commands with bun/bunx, NOT npx/npm"

## Frontend Architecture

### 核心设计原则

**Tab = Connection = Session（1:1 映射）**

- 一个标签页 = 一个连接 = 一个会话
- 用户不需要关心"后端会话"概念
- 简化心智模型，避免同步问题

### 组件层次

```
App.vue
└── MainLayout.vue
    ├── TabBar.vue
    │   ├── NTabs (标签页列表)
    │   ├── NDropdown (新建按钮)
    │   ├── NDropdown (右键菜单)
    │   └── NModal (重命名对话框)
    ├── TerminalPane.vue (v-for each tab)
    │   ├── Terminal.vue (xterm.js)
    │   ├── SearchBar.vue (搜索栏)
    │   └── ConfigPanel.vue (配置面板)
    └── StatusBar.vue
        ├── 连接状态指示器
        ├── 会话名称
        ├── TX/RX 统计
        ├── 时间戳按钮
        └── 搜索按钮
```

### Store 设计

**sessionStore**（会话和标签页管理）
- 管理前端标签页（tabs, activeTabId）
- 管理后端会话（sessions, activeSessionId）
- 提供标签页操作（addTab, closeTab, renameTab）

**terminalStore**（终端 UI 状态管理）
- 管理每个标签页的终端 UI 状态
- 管理终端数据缓冲区
- 提供 UI 状态操作

**connectionStore**（连接管理）
- 管理连接状态
- 提供连接操作
- 跟踪连接参数

### 数据流

**连接流程**：
```
用户点击 "Connect" 
→ ConfigPanel.handleConnect()
→ connectionStore.connect(params, sessionId)
→ Tauri invoke 'connect'
→ 后端建立连接
→ Tauri emit 'status_changed'
→ useTauriEvents 监听事件
→ sessionStore.connectTab(tabId, sessionId)
→ 更新标签页状态
```

**数据接收流程**：
```
后端接收数据
→ Tauri emit 'data_received'
→ useTauriEvents 监听事件
→ terminalStore.emitData(sessionId, text)
→ TerminalPane 订阅事件
→ terminalRef.write(data)
→ 终端显示数据
```

### 关键设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| Tab = Connection | ✅ 是 | 简化心智模型，避免同步问题 |
| 后端会话管理 | ✅ 保留 | 用于获取会话信息和状态 |
| 标签页标题 | 👤 用户定义 | 用户可自定义，不自动重命名 |
| 数据传输 | 16ms 轮询 | 简单可靠，性能足够 |
| UI 状态存储 | 按标签页隔离 | 避免跨标签页污染 |

详细设计文档：`docs/frontend-design.md`

## Project Status

- ✅ Phase 1: Core traits and types defined
- ✅ Phase 2: Backend implementations (serial, telnet, session, logger, command) - 141 tests passing
- ✅ Phase 3.1: Frontend framework setup (Vue 3 + TypeScript + Naive UI + Pinia + xterm.js)
- 🚧 Phase 3.2: Frontend components implementation - **NEXT**
- ⏳ Phase 4: Integration, cross-platform testing, packaging

### 已完成的前端组件
- ✅ TabBar.vue — 标签页管理
- ✅ TerminalPane.vue — 会话容器
- ✅ Terminal.vue — xterm.js 封装
- ✅ ConfigPanel.vue — 浮动配置面板
- ✅ SearchBar.vue — 搜索栏
- ✅ StatusBar.vue — 底部状态栏
- ✅ MainLayout.vue — 终端优先布局
- ✅ useTauriEvents.ts — Tauri 事件监听
- ✅ sessionStore — 会话和标签页管理
- ✅ terminalStore — 终端 UI 状态管理
- ✅ connectionStore — 连接管理
