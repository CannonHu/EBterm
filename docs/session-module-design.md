# Session 模块设计文档

## 概述

Session 模块是 embedded-debugger 的核心会话管理组件，负责管理调试会话的生命周期、连接池管理和事件分发。

**设计目标**：
- 消除 Session 和 Connection 之间的引用循环
- 实现 O(1) 时间复杂度的连接访问
- 支持内存复用（Vec 空闲槽位复用）
- 简化所有权模型（集中式连接管理）

---

## 架构设计

### 核心组件

```
┌─────────────────────────────────────────────────────────────┐
│                      SessionManager                          │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  sessions: Arc<RwLock<HashMap<SessionId, Session>>>  │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  connection_registry: Arc<RwLock<ConnectionRegistry>>│   │
│  │  ┌────────────────────────────────────────────────┐  │   │
│  │  │  connections: Vec<Option<ConnectionHandle>>    │  │   │
│  │  │  session_to_index: HashMap<SessionId, usize>   │  │   │
│  │  │  free_indices: Vec<usize>                      │  │   │
│  │  └────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  event_sender: mpsc::Sender<SessionEvent>            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 数据结构

#### Session 结构体

```rust
pub struct Session {
    id: SessionId,                           // 会话唯一标识
    metadata: SessionMetadata,               // 会话元数据
    state: SessionState,                     // 会话状态
    event_sender: mpsc::Sender<SessionEvent>, // 事件发送器
    created_at: std::time::Instant,          // 创建时间
    last_activity: std::time::Instant,       // 最后活动时间
    connection_index: Option<usize>,         // 连接索引（关键字段）
}
```

**关键字段**：
- `connection_index: Option<usize>`：指向 ConnectionRegistry 中的连接索引
  - `None`：会话未连接
  - `Some(idx)`：会话已连接，索引为 `idx`

#### ConnectionRegistry 结构体

```rust
pub struct ConnectionRegistry {
    connections: Vec<Option<ConnectionHandle>>,  // 连接存储（扁平化）
    session_to_index: HashMap<SessionId, usize>, // 会话到索引映射
    free_indices: Vec<usize>,                    // 空闲索引（LIFO）
}
```

**关键设计**：
- 使用 `Vec<Option<ConnectionHandle>>` 扁平化存储连接
- `Option` 表示槽位是否被占用（支持空闲槽位复用）
- `free_indices` 使用 LIFO 策略，对缓存友好

---

## ConnectionRegistry 扁平化存储优化

### 优化目标

1. **消除引用循环**：Session 不再直接持有 Connection 引用
2. **O(1) 访问**：通过整数索引直接访问连接
3. **内存复用**：Vec 空闲槽位复用，减少分配开销
4. **简化所有权**：ConnectionRegistry 集中拥有所有连接

### 数据流

#### 连接建立流程

```
create_session()
  ├─ 创建 Session (connection_index = None, state = Created)
  └─ 返回 session_id

connect_session(session_id, config)
  ├─ 验证 session 存在且状态为 Created
  ├─ 创建 Connection 对象
  ├─ 调用 connection.connect()
  ├─ registry.insert(session_id, connection) → 分配索引
  ├─ session.set_connection_index(Some(index))
  └─ 更新状态为 Connected
```

#### 连接断开流程

```
disconnect_session(session_id)
  ├─ 验证 session 存在且状态为 Connected
  ├─ 获取 session.connection_index
  ├─ registry.remove_by_index(index) → 移除连接
  ├─ connection.disconnect()
  ├─ session.set_connection_index(None)
  └─ 更新状态为 Disconnected
```

#### 会话关闭流程

```
close_session(session_id)
  ├─ 获取 session.connection_index 和 state
  ├─ 从 sessions HashMap 中移除 session
  ├─ 更新统计信息
  └─ 如果有 connection_index：
      ├─ registry.remove_by_index(index)
      └─ 如果状态为 Connected，调用 disconnect()
```

### 索引复用机制

```rust
// 插入连接
pub fn insert(&mut self, session_id: SessionId, connection: ConnectionHandle) -> Result<usize, SessionError> {
    if self.session_to_index.contains_key(&session_id) {
        return Err(SessionError::AlreadyExists { id: session_id });
    }

    // 复用空闲索引或创建新索引
    let index = if let Some(free_index) = self.free_indices.pop() {
        self.connections[free_index] = Some(connection);
        free_index
    } else {
        let new_index = self.connections.len();
        self.connections.push(Some(connection));
        new_index
    };

    self.session_to_index.insert(session_id, index);
    Ok(index)
}

// 移除连接
pub fn remove_by_index(&mut self, index: usize) -> Option<(SessionId, ConnectionHandle)> {
    let session_id = self.session_to_index.iter()
        .find(|(_, idx)| **idx == index)
        .map(|(sid, _)| sid.clone())?;

    if let Some(sid) = session_id {
        self.session_to_index.remove(&sid);
        let connection = std::mem::replace(&mut self.connections[index], None)?;
        self.free_indices.push(index);  // 回收索引
        Some((sid, connection))
    } else {
        None
    }
}
```

---

## 会话状态机

### 状态定义

```rust
pub enum SessionState {
    Created,        // 会话已创建，未连接
    Connecting,     // 正在连接（保留状态）
    Connected,      // 已连接
    Disconnecting,  // 正在断开（保留状态）
    Disconnected,   // 已断开
    Error(String),  // 发生错误
}
```

### 状态转换

```
Created ──(connect_session)──> Connected
Connected ──(disconnect_session)──> Disconnected
Disconnected ──(connect_session)──> Connected
Any State ──(close_session)──> [Session Removed]
```

---

## 并发安全

### 锁设计

- `sessions: Arc<RwLock<HashMap<SessionId, Session>>>`：会话存储
- `connection_registry: Arc<RwLock<ConnectionRegistry>>`：连接注册表
- `stats: Arc<RwLock<SessionManagerStats>>`：统计信息

### 锁使用规则

1. **锁顺序**：在任何方法中，获取多个锁时，必须按以下顺序：
   - sessions → registry → stats

2. **锁释放**：在获取下一个锁之前，必须释放前一个锁
   ```rust
   let mut sessions = self.sessions.write().await;
   // ... 使用 sessions
   drop(sessions);  // 显式释放

   let mut registry = self.connection_registry.write().await;
   // ... 使用 registry
   ```

3. **读写分离**：优先使用 `read()` 读锁，仅在必要时使用 `write()` 写锁

---

## 事件系统

### 事件类型

```rust
pub enum SessionEvent {
    Created(SessionId),
    StateChanged(SessionId, SessionState),
    DataReceived(SessionId, Vec<u8>),
    DataSent(SessionId, usize),
    Error(SessionId, String),
    Closed(SessionId),
}
```

### 事件分发

- 使用 `mpsc::channel(100)` 进行事件分发
- 事件发送不阻塞主流程（使用 `let _ = sender.send(...).await`）
- 支持多个事件订阅者（通过 `subscribe_events` 方法）

---

## 错误处理

### SessionError 定义

```rust
pub enum SessionError {
    NotFound { id: String },
    AlreadyExists { id: String },
    NotConnected { id: String },
    InvalidState { id: String, state: String },
    CreationFailed { reason: String },
    DestructionFailed { id: String, reason: String },
    MaxSessionsReached { max: usize },
    Generic(String),
}
```

### 错误码

每个错误变体提供 `code()` 方法，返回静态字符串错误码，用于 IPC 通信。

---

## 测试覆盖

### 单元测试

- `connection_registry.rs`：19 个测试
  - 基本操作（insert, get, remove）
  - 索引复用
  - 边界条件
  - 并发安全

- `manager.rs`：5 个测试
  - 会话创建和列表
  - 会话关闭
  - 最大会话数限制
  - 错误处理

- `mod.rs`：15 个测试
  - SessionError 各变体
  - 错误码
  - 边界条件

### 测试场景

| 场景 | 测试覆盖 |
|------|---------|
| 创建会话 | ✅ |
| 连接会话 | ⚠️ 需要实际连接测试 |
| 断开会话 | ⚠️ 需要实际连接测试 |
| 关闭会话 | ✅ |
| 写入数据 | ⚠️ 需要实际连接测试 |
| 错误处理 | ✅ |
| 最大会话数 | ✅ |

---

## 性能特性

### 时间复杂度

| 操作 | 时间复杂度 | 说明 |
|------|-----------|------|
| create_session | O(1) | HashMap 插入 |
| connect_session | O(1) | Vec 索引访问 |
| disconnect_session | O(n) | remove_by_index 需要遍历 |
| close_session | O(n) | remove_by_index 需要遍历 |
| write_session_data | O(1) | Vec 索引访问 |
| get_session | O(1) | HashMap 查找 |

### 空间复杂度

- Session 存储：O(n) - n 为会话数
- ConnectionRegistry：O(m) - m 为最大连接数（包括空闲槽位）
- 空闲槽位复用减少内存碎片

---

## 未来改进

### 已知限制

1. `remove_by_index` 是 O(n) 操作，可通过反向映射优化
2. `subscribe_events` 方法未完全实现
3. 缺少会话超时自动清理机制

### 改进建议

1. **优化 remove_by_index**：
   ```rust
   // 添加反向映射
   index_to_session: HashMap<usize, SessionId>
   ```

2. **实现 subscribe_events**：
   ```rust
   pub async fn subscribe_events(&self) -> broadcast::Receiver<SessionEvent> {
       self.event_sender.subscribe()
   }
   ```

3. **添加会话超时清理**：
   ```rust
   pub async fn cleanup_expired_sessions(&self) -> usize {
       // 清理超过 session_timeout_secs 的会话
   }
   ```

---

## 附录

### 相关文件

- `/src/session/manager.rs`：SessionManager 实现
- `/src/session/connection_registry.rs`：ConnectionRegistry 实现
- `/src/session/mod.rs`：SessionError 定义
- `/src/session/state/state.rs`：SessionState 定义

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2024-03-22 | 初始版本，实现扁平化存储优化 |

---

**文档维护者**: embedded-debugger team  
**最后更新**: 2024-03-22
