# 架构简化计划：从过度设计到奥卡姆剃刀

**目标**: 将代码库从 ~5000 行减少到 ~2000 行，提高可维护性
**核心原则**: Minimum is Best - 删除不必要的抽象层

---

## 第一阶段：删除 Session 层（删除约 1000 行）

### 问题分析

当前架构：`SessionManager → Session → ConnectionRegistry → Connection`

实际需要的架构：`ConnectionManager → Connection`

Session 层只做了两件事：
1. 存储一个 `connection_index`（指向 ConnectionRegistry 的索引）
2. 维护一些元数据（创建时间、名称等）

这些完全可以由 ConnectionManager 直接管理。

### 删除步骤

1. **删除文件**:
   - `src/session/manager.rs` (约 800 行)
   - `src/session/mod.rs` (约 100 行)
   - `src/session/types.rs` (约 50 行)
   - `src/session/state.rs` (约 30 行)

2. **创建简化的 ConnectionManager**:

```rust
// src/connection/manager.rs
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct ConnectionManager {
    connections: RwLock<HashMap<String, ConnectionEntry>>,
}

struct ConnectionEntry {
    id: String,
    name: String,
    connection: ConnectionHandle,
    created_at: Instant,
    // 其他元数据...
}
```

3. **更新 Tauri 层**: 直接使用 ConnectionManager，移除所有 Session 相关的调用

### 验收标准
- [ ] `src/session/` 目录被删除
- [ ] 所有测试通过
- [ ] 功能与之前完全一致
- [ ] 代码行数减少约 1000 行

---

## 第二阶段：简化 ConnectionRegistry（删除约 500 行）

### 问题分析

当前设计：
```rust
pub struct ConnectionRegistry {
    connections: Vec<Option<ConnectionHandle>>,  // 扁平化存储
    session_to_index: HashMap<SessionId, usize>,   // 会话到索引映射
    free_indices: Vec<usize>,                      // 空闲索引（LIFO）
}
```

这是典型的过早优化。声称为了"内存复用"和"O(1) 访问"，但：
- HashMap 已经是 O(1)
- 现代堆分配器比你的 free_indices 更高效
- 增加 150 行代码换来 0.1% 的性能提升（假设有的话）

### 简化方案

```rust
// 原方案（约 200 行）
pub struct ConnectionRegistry {
    connections: Vec<Option<ConnectionHandle>>,
    session_to_index: HashMap<SessionId, usize>,
    free_indices: Vec<usize>,
}

// 简化方案（约 50 行）
pub type ConnectionRegistry = HashMap<SessionId, ConnectionHandle>;
```

### 删除文件
- `src/session/connection_registry.rs` (约 300 行)

### 验收标准
- [ ] ConnectionRegistry 简化为 HashMap 别名
- [ ] 所有相关代码更新
- [ ] 测试通过
- [ ] 代码行数减少约 500 行

---

## 第三阶段：简化 DataStreamer（删除约 300 行）

### 问题分析

当前：`data_streamer.rs` 300+ 行代码

```rust
pub struct DataStreamer {
    session_id: SessionId,
    config: DataStreamerConfig,  // 批处理配置
    shutdown_tx: broadcast::Sender<()>,
}

impl DataStreamer {
    pub async fn start(&self, app: tauri::AppHandle, registry: Arc<RwLock<ConnectionRegistry>>) {
        // 300 行：批处理、刷新逻辑、错误处理...
    }
}
```

### 问题根本：DataStreamer 批处理逻辑失效

**设计矛盾点**：

| 设计意图 | 实际实现 | 效果 |
|----------|----------|------|
| 16ms 批量刷新 | 每次 read 后立即检查条件 | 条件不满足时立即返回 |
| 4KB 批处理阈值 | 依赖 Connection.read 返回值 | 通常小于 1KB |
| 双重缓冲 | DataStreamer 缓冲 + Connection 缓冲 | 实际只缓冲一次 |

**核心问题**：DataStreamer 的设计假设 read() 会阻塞并返回大量数据，但实际上：
- Serial/Telnet read 是非阻塞的
- 每次返回的数据量取决于网络/串口缓冲区
- 导致 DataStreamer 频繁地进行小数据量 flush

### 正确方案：真正的批处理

#### 方案：时间窗口批量发送（推荐）

```rust
pub async fn start_batch_streamer(
    session_id: String,
    connection: ConnectionHandle,
    app: tauri::AppHandle,
) {
    const BATCH_INTERVAL_MS: u64 = 16;  // 16ms 批处理窗口
    const MAX_BATCH_SIZE: usize = 16384;  // 最大批量 16KB

    tokio::spawn(async move {
        let mut batch_buffer = Vec::with_capacity(MAX_BATCH_SIZE);
        let mut last_batch_time = Instant::now();
        let mut read_buf = [0u8; 4096];  // 单次 read buffer

        loop {
            // 使用 timeout 实现定时批处理
            let timeout_duration = Duration::from_millis(BATCH_INTERVAL_MS)
                .saturating_sub(last_batch_time.elapsed());

            match tokio::time::timeout(
                timeout_duration,
                connection.read(&mut read_buf)
            ).await {
                // 在 timeout 前读取到数据
                Ok(Ok(0)) => break,  // 连接关闭
                Ok(Ok(n)) => {
                    batch_buffer.extend_from_slice(&read_buf[..n]);

                    // 如果 batch 已满，立即发送
                    if batch_buffer.len() >= MAX_BATCH_SIZE {
                        send_batch(&app, &session_id, &batch_buffer).await;
                        batch_buffer.clear();
                        last_batch_time = Instant::now();
                    }
                }
                Ok(Err(_)) => break,  // 读取错误

                // Timeout：即使没有数据也要发送 batch
                Err(_) => {
                    if !batch_buffer.is_empty() {
                        send_batch(&app, &session_id, &batch_buffer).await;
                        batch_buffer.clear();
                    }
                    last_batch_time = Instant::now();
                }
            }
        }
    });
}

async fn send_batch(app: &tauri::AppHandle, session_id: &str, data: &[u8]) {
    let _ = app.emit("data_received", DataReceivedEvent {
        session_id: session_id.to_string(),
        data: data.to_vec(),
    });
}
```

**关键改进**：
1. **强制 16ms 批处理窗口**：使用 `tokio::time::timeout` 确保即使数据量小，也会等待 16ms
2. **真正的批量发送**：16ms 内累积的所有数据一次性发送，而非逐字节
3. **上限保护**：16KB 上限防止内存无限增长

**预期效果**：
- 低速场景（9600 baud）：每 16ms 发送一次，约 60 IPC/秒（可接受）
- 高速场景（1M baud）：16KB 上限触发频繁发送，但仍比逐字节高效 1000 倍

#### 删除内容
- `src-tauri/src/data_streamer.rs` (~350 行)
- **简化后：~120 行（真正的批处理实现）**
- **净减少：~230 行**

---

## 第四阶段：统一事件系统（删除约 400 行）

### 问题分析

当前有三套事件机制：

```rust
// 1. mpsc channel
event_sender: mpsc::Sender<SessionEvent>

// 2. Callbacks
pub on_connected: Option<Box<dyn Fn(SessionId, Arc<RwLock<ConnectionRegistry>>)>>
pub on_disconnected: Option<Box<dyn Fn(SessionId)>>
pub on_error: Option<Box<dyn Fn(SessionId, String)>>

// 3. Tauri Events
app.emit("data_received", ...)
```

### 问题
- 同样的事件需要在三个地方处理
- 维护成本增加 3 倍
- 难以追踪事件流

### 统一方案

**只使用 Tauri 事件**：

```rust
// 删除以下：
// - mpsc::channel
// - SessionCallbacks 结构
// - 所有回调注册逻辑

// 保留：
// Tauri 的事件系统（已经是跨进程通信的标准方式）
```

### 删除文件
- `src/session/callbacks.rs` (如果不存在，则从 manager.rs 中删除相关代码)
- 删除所有 `event_sender` 相关代码

### 验收标准
- [ ] 删除 mpsc channel
- [ ] 删除 Callback 机制
- [ ] 只保留 Tauri 事件
- [ ] 测试通过
- [ ] 代码行数减少约 400 行

---

## 第五阶段：合并状态枚举（删除约 200 行）

### 问题分析

```rust
// Session 状态
enum SessionState {
    Created,        // 会话已创建，未连接
    Connecting,     // 正在连接（保留状态）
    Connected,      // 已连接
    Disconnecting,  // 正在断开（保留状态）
    Disconnected,   // 已断开
    Error(String),  // 发生错误
}

// Connection 状态
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}
```

**为什么需要两个？** Session 只是一个包装，状态应该由 Connection 决定。

### 简化方案

```rust
// 只保留 ConnectionStatus
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

// Session 直接暴露 Connection 的状态
impl Session {
    pub fn status(&self) -> ConnectionStatus {
        self.connection.status()
    }
}
```

### 验收标准
- [ ] 删除 SessionState 枚举
- [ ] 统一使用 ConnectionStatus
- [ ] 测试通过
- [ ] 代码行数减少约 200 行

---

## 总结

### 预期收益

| 阶段 | 删除代码 | 简化效果 |
|------|----------|----------|
| 删除 Session 层 | ~1000 行 | 消除 1 层抽象 |
| 简化 ConnectionRegistry | ~500 行 | 删除 150 行复杂逻辑 |
| 简化 DataStreamer | ~300 行 | 300 行 → 20 行 |
| 统一事件系统 | ~400 行 | 3 种 → 1 种 |
| 合并状态枚举 | ~200 行 | 2 个 → 1 个 |
| **总计** | **~2400 行** | **50% 代码减少** |

### 实施建议

1. **按顺序执行**：每个阶段都依赖前一个阶段
2. **每个阶段都要完整测试**：确保功能不变
3. **保持版本控制**：每个阶段一个 commit，便于回滚
4. **不要追求完美**：简化 80% 比不简化 100% 更好

### 最终目标

```
之前：SessionManager → Session → ConnectionRegistry → Connection → DataStreamer (5000 行)
之后：ConnectionManager → Connection (2000 行)
```

这不仅是代码量的减少，更是心智负担的降低。新的开发者可以在 1 小时内理解整个架构，而不是 1 周。
