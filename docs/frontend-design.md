# 前端设计文档

## 1. 架构概览

### 核心设计原则

**Tab = Connection = Session（1:1 映射）**

- 一个标签页 = 一个连接 = 一个会话
- 用户不需要关心"后端会话"概念
- 简化心智模型，避免同步问题

### 架构图

```
┌─────────────────────────────────────────────────────────┐
│                   Frontend (Vue 3)                       │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │   TabBar    │  │ TerminalPane│  │  StatusBar   │      │
│  │  (NTabs)    │  │  (xterm.js) │  │  (状态显示)  │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Pinia Stores                        │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────┐  │    │
│  │  │sessionStore │  │terminalStore│  │connectSt│  │    │
│  │  │(标签页管理) │  │(终端UI状态) │  │(连接管理)│  │    │
│  │  └─────────────┘  └─────────────┘  └─────────┘  │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Composables                          │    │
│  │  ┌──────────────────────────────────────────┐    │    │
│  │  │ useTauriEvents (事件监听)                 │    │    │
│  │  └──────────────────────────────────────────┘    │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                         │
                         │ Tauri IPC
                         ▼
┌─────────────────────────────────────────────────────────┐
│                   Backend (Rust)                         │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │ Connection  │  │   Session   │  │   Logger    │      │
│  │   Manager   │  │   Manager   │  │   Manager   │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
└─────────────────────────────────────────────────────────┘
```

## 2. 数据模型

### 2.1 TabState（标签页状态）

```typescript
interface TabState {
  id: string                    // 唯一标识（UUID）
  sessionId: string | null      // 后端会话 ID（连接后才有）
  title: string                 // 显示名称（用户可编辑）
  isActive: boolean             // 是否为活动标签页
  isConnecting: boolean         // 是否正在连接
}
```

**设计意图**：
- `id` 是前端标签页的唯一标识
- `sessionId` 是后端会话的标识，连接成功后才有效
- `title` 默认为 "New Session"，用户可重命名
- 标签页状态和连接状态分离，连接状态在 `connectionStore` 管理

**为什么需要两个 ID？**

| 阶段 | 使用的 ID | 原因 |
|------|----------|------|
| 未连接时 | tabId | sessionId 为 null，需要临时标识 |
| 连接过程中 | tabId | sessionId 还没有，需要标识正在连接的标签页 |
| 连接成功后 | sessionId | 后端会话已建立，使用正式标识 |
| 事件路由 | sessionId | 后端事件使用 sessionId，需要查找对应标签页 |

**职责分离**：
- `tabId`：标识前端标签页（生命周期：创建 → 关闭）
- `sessionId`：标识后端会话（生命周期：连接 → 断开）

### 2.2 TerminalUIState（终端 UI 状态）

```typescript
interface TerminalUIState {
  showTimestamps: boolean        // 是否显示时间戳
  isSearchOpen: boolean          // 搜索栏是否打开
  isConfigPanelOpen: boolean     // 配置面板是否打开
}
```

**设计意图**：
- 每个标签页独立的 UI 状态
- 存储在 `terminalStore` 的 Map 中
- 与标签页 ID 一一对应

### 2.3 SessionInfo（后端会话信息）

```typescript
interface SessionInfo {
  id: string
  name: string
  connection_type: string
  status: ConnectionStatus
  created_at: string
  last_activity?: string
  stats: ConnectionStats
  logging_enabled: boolean
  log_file_path?: string
}
```

**设计意图**：
- 从后端获取的会话信息
- 用于显示会话列表和状态
- 通过 `list_sessions` 命令获取

## 3. Store 设计

### 3.1 sessionStore（会话和标签页管理）

**职责**：
- 管理前端标签页（tabs, activeTabId）
- 管理后端会话（sessions, activeSessionId）
- 提供标签页操作（addTab, closeTab, renameTab）

**状态**：
```typescript
{
  // 后端会话管理
  sessions: SessionInfo[]
  activeSessionId: string | null
  activeSession: SessionInfo | null

  // 前端标签页管理
  tabs: TabState[]
  activeTabId: string | null
  activeTab: TabState | null
}
```

**方法**：
```typescript
// 后端会话
loadSessions(): Promise<void>
renameSession(id, name): Promise<void>
setActiveSession(id): void

// 前端标签页
addTab(): void
closeTab(tabId): void
setActiveTab(tabId): void
renameTab(tabId, newName): void
connectTab(tabId, sessionId): void
disconnectTab(tabId): void
updateTabConnecting(tabId, isConnecting): void
```

### 3.2 terminalStore（终端 UI 状态管理）

**职责**：
- 管理每个标签页的终端 UI 状态
- 提供事件发射器，用于数据传输
- 提供 UI 状态操作

**状态**：
```typescript
{
  states: Map<string, TerminalUIState>  // 标签页 ID → UI 状态
}
```

**方法**：
```typescript
initState(id): void
removeState(id): void
getState(id): TerminalUIState
onData(id, listener): () => void  // 订阅数据事件，返回取消订阅函数
emitData(id, data): void          // 发射数据事件
toggleTimestamps(id): void
openSearch(id): void
closeSearch(id): void
toggleConfigPanel(id): void
closeConfigPanel(id): void
```

### 3.3 connectionStore（连接管理）

**职责**：
- 管理每个会话的连接状态（按 sessionId 隔离）
- 提供连接操作
- 跟踪每个会话的连接参数和统计

**状态**：
```typescript
{
  sessionStatuses: Map<string, ConnectionStatus>    // 每个会话的状态
  sessionConfigs: Map<string, ConnectionParams>     // 每个会话的配置
  sessionStats: Map<string, ConnectionStats>        // 每个会话的统计
  sessionErrors: Map<string, string>                // 每个会话的错误
}
```

**计算属性**（根据当前活动标签页返回）：
```typescript
status: ConnectionStatus      // 当前标签页的状态
config: ConnectionParams      // 当前标签页的配置
stats: ConnectionStats        // 当前标签页的统计
error: string | null          // 当前标签页的错误
isConnected: boolean
isConnecting: boolean
hasError: boolean
```

**方法**：
```typescript
connect(params, sessionId): Promise<void>
disconnect(sessionId): Promise<void>
getStatus(sessionId): Promise<void>
getSessionStatus(sessionId): ConnectionStatus
getSessionConfig(sessionId): ConnectionParams | undefined
getSessionStats(sessionId): ConnectionStats
getSessionError(sessionId): string | null
setSessionStatus(sessionId, status): void
setSessionStats(sessionId, stats): void
setSessionError(sessionId, error): void
removeSession(sessionId): void
```

## 4. 组件设计

### 4.1 组件层次

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

### 4.2 TabBar.vue

**职责**：
- 显示标签页列表
- 提供标签页操作（新建、关闭、重命名）
- 显示连接状态指示

**关键实现**：
```typescript
// 连接状态颜色
function getStatusColor(tab: TabState): string {
  if (tab.isConnecting) return '#f59e0b'  // 黄色：连接中
  if (tab.sessionId) return '#22c55e'     // 绿色：已连接
  return '#6b7280'                        // 灰色：未连接
}
```

### 4.3 TerminalPane.vue

**职责**：
- 作为单个标签页的容器
- 管理终端组件的生命周期
- 处理数据轮询

**数据流**：
```typescript
// 16ms 轮询从 dataBuffers 读取数据
dataPollInterval = window.setInterval(() => {
  const data = terminalStore.flushData(props.sessionId);
  for (const chunk of data) {
    terminalRef.value.write(chunk);
  }
}, 16);
```

### 4.4 Terminal.vue

**职责**：
- 封装 xterm.js 终端
- 处理终端输入输出
- 支持搜索和时间戳

**暴露方法**：
```typescript
defineExpose({
  write,
  writeln,
  clear,
  clearAll,
  focus,
  blur,
  getDimensions,
  fit,
  search,
  searchPrevious,
  getSearchMatches
})
```

### 4.5 ConfigPanel.vue

**职责**：
- 提供连接配置表单
- 支持 Serial 和 Telnet 两种连接类型
- 处理连接/断开操作

**关键实现**：
```typescript
async function handleConnect() {
  const params = connectionType.value === 'serial' 
    ? serialForm.value 
    : telnetForm.value;
  
  await connectionStore.connect(params, props.sessionId);
  emit('connected', props.sessionId);
}
```

### 4.6 StatusBar.vue

**职责**：
- 显示连接状态
- 显示会话名称
- 显示 TX/RX 统计
- 提供时间戳和搜索按钮

**关键实现**：
```typescript
const sessionName = computed(() => {
  const tab = sessionStore.activeTab;
  return tab?.title ?? 'No Tab';
});
```

## 5. 数据流

### 5.1 连接流程

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

### 5.2 数据接收流程

```
后端接收数据
→ Tauri emit 'data_received'
→ useTauriEvents 监听事件
→ terminalStore.emitData(sessionId, text)
→ TerminalPane 订阅事件
→ terminalRef.write(data)
→ 终端显示数据
```

### 5.3 数据发送流程

```
用户在终端输入
→ Terminal.onData()
→ TerminalPane.handleTerminalData()
→ Tauri invoke 'write_data'
→ 后端发送数据
```

## 6. 关键设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| Tab = Connection | ✅ 是 | 简化心智模型，避免同步问题 |
| 后端会话管理 | ✅ 保留 | 用于获取会话信息和状态 |
| 标签页标题 | 👤 用户定义 | 用户可自定义，不自动重命名 |
| 数据传输 | 事件驱动 | 实时性好，代码简洁 |
| UI 状态存储 | 按标签页隔离 | 避免跨标签页污染 |
| 连接状态存储 | 按 sessionId 隔离 | 支持多标签页独立连接 |

## 7. 代码规范

### 7.1 组件命名

- 使用 PascalCase：`TerminalPane.vue`、`ConfigPanel.vue`
- 文件名与组件名一致

### 7.2 Store 命名

- 使用 camelCase：`sessionStore`、`terminalStore`、`connectionStore`
- 文件名使用 snake_case：`session.ts`、`terminal.ts`

### 7.3 类型定义

- 在 `types/ipc.ts` 中定义所有 IPC 类型
- 使用 interface 而不是 type
- 类型名使用 PascalCase

### 7.4 事件处理

- 使用 `@update:visible` 而不是 `@close`
- 事件名使用 camelCase
- 事件参数使用对象而不是多个参数

## 8. 测试策略

### 8.1 单元测试

- Store 测试：使用 Pinia 的 `defineStore` 测试
- 组件测试：使用 Vue Test Utils
- 工具函数测试：使用 Vitest

### 8.2 集成测试

- Tauri IPC 测试：使用模拟的 Tauri API
- 数据流测试：验证完整流程

### 8.3 E2E 测试

- 使用 Playwright 测试完整用户流程
- 测试连接、断开、数据收发

## 9. 性能优化

### 9.1 数据缓冲

- 使用 `dataBuffers` 缓冲终端数据
- 16ms 批量写入，减少 DOM 操作

### 9.2 虚拟滚动

- xterm.js 内置虚拟滚动
- 只渲染可见行

### 9.3 事件节流

- ResizeObserver 使用 100ms 防抖
- 搜索使用即时反馈

## 10. 错误处理

### 10.1 连接错误

- 显示错误消息
- 更新连接状态
- 允许重试

### 10.2 数据错误

- 记录错误日志
- 继续接收后续数据
- 不阻塞用户操作

### 10.3 UI 错误

- 使用 Naive UI 的 NMessage 显示错误
- 提供友好的错误信息
- 允许用户重试操作
