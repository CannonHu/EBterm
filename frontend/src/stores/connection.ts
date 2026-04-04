import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { ConnectionParams, ConnectionStatus, ConnectionStats, LoggingStatus } from '../types/ipc';
import { tauriInvoke } from '../api/tauri';
import { useSessionStore } from './session';

export const useConnectionStore = defineStore('connection', () => {
  const sessionStore = useSessionStore();

  // Map sessionId to tabId for reverse lookup
  const sessionIdToTabId = ref<Map<string, string>>(new Map());

  // Store connection state by tabId
  const tabStatuses = ref<Map<string, ConnectionStatus>>(new Map());
  const tabConfigs = ref<Map<string, ConnectionParams>>(new Map());
  const tabStats = ref<Map<string, ConnectionStats>>(new Map());
  const tabErrors = ref<Map<string, string>>(new Map());
  const tabLoggingStatuses = ref<Map<string, LoggingStatus>>(new Map());

  const status = computed(() => {
    const activeTabId = sessionStore.activeTabId;
    return activeTabId ? tabStatuses.value.get(activeTabId) ?? 'disconnected' : 'disconnected';
  });

  const config = computed(() => {
    const activeTabId = sessionStore.activeTabId;
    if (!activeTabId) return null;
    return tabConfigs.value.get(activeTabId) ?? null;
  });

  const stats = computed(() => {
    const activeTabId = sessionStore.activeTabId;
    const defaultStats: ConnectionStats = { bytes_sent: 0, bytes_received: 0, packets_sent: 0, packets_received: 0 };
    return activeTabId ? tabStats.value.get(activeTabId) ?? defaultStats : defaultStats;
  });

  const error = computed(() => {
    const activeTabId = sessionStore.activeTabId;
    return activeTabId ? tabErrors.value.get(activeTabId) ?? null : null;
  });

  const isConnected = computed(() => status.value === 'connected');
  const isConnecting = computed(() => status.value === 'connecting');
  const hasError = computed(() => status.value === 'error');

  const loggingStatus = computed(() => {
    const activeTabId = sessionStore.activeTabId;
    const defaultStatus: LoggingStatus = { enabled: false, bytes_logged_input: 0, bytes_logged_output: 0 };
    return activeTabId ? tabLoggingStatuses.value.get(activeTabId) ?? defaultStatus : defaultStatus;
  });

  const isLogging = computed(() => loggingStatus.value.enabled);

  function getTabStatus(tabId: string): ConnectionStatus {
    return tabStatuses.value.get(tabId) ?? 'disconnected';
  }

  function getTabConfig(tabId: string): ConnectionParams | undefined {
    return tabConfigs.value.get(tabId);
  }

  function getTabStats(tabId: string): ConnectionStats {
    const defaultStats: ConnectionStats = { bytes_sent: 0, bytes_received: 0, packets_sent: 0, packets_received: 0 };
    return tabStats.value.get(tabId) ?? defaultStats;
  }

  function getTabError(tabId: string): string | null {
    return tabErrors.value.get(tabId) ?? null;
  }

  function setTabStatus(tabId: string, status: ConnectionStatus): void {
    tabStatuses.value.set(tabId, status);
  }

  function setTabStats(tabId: string, stats: ConnectionStats): void {
    tabStats.value.set(tabId, stats);
  }

  function setTabError(tabId: string, error: string | null): void {
    if (error) {
      tabErrors.value.set(tabId, error);
    } else {
      tabErrors.value.delete(tabId);
    }
  }

  function getTabLoggingStatus(tabId: string): LoggingStatus {
    const defaultStatus: LoggingStatus = { enabled: false, bytes_logged_input: 0, bytes_logged_output: 0 };
    return tabLoggingStatuses.value.get(tabId) ?? defaultStatus;
  }

  function setTabLoggingStatus(tabId: string, status: LoggingStatus): void {
    tabLoggingStatuses.value.set(tabId, status);
  }

  function removeTab(tabId: string): void {
    const tab = sessionStore.tabs.find(t => t.id === tabId);
    const sessionId = tab?.sessionId;
    if (sessionId) {
      sessionIdToTabId.value.delete(sessionId);
    }
    tabStatuses.value.delete(tabId);
    tabConfigs.value.delete(tabId);
    tabStats.value.delete(tabId);
    tabErrors.value.delete(tabId);
    tabLoggingStatuses.value.delete(tabId);
  }

  async function connect(params: ConnectionParams, tabId?: string) {
    if (!tabId) return;

    tabErrors.value.delete(tabId);
    tabStatuses.value.set(tabId, 'connecting');
    tabConfigs.value.set(tabId, params);

    const result = await tauriInvoke<string>('connect', { params });

    if (result.success && result.data) {
      const sessionId = result.data;
      tabStatuses.value.set(tabId, 'connected');
      sessionIdToTabId.value.set(sessionId, tabId);
      sessionStore.connectTab(tabId, sessionId);
    } else {
      tabStatuses.value.set(tabId, 'error');
      tabErrors.value.set(tabId, result.error || 'Connection failed');
    }
  }

  async function disconnect(tabId?: string) {
    if (!tabId) return;

    const tab = sessionStore.tabs.find(t => t.id === tabId);
    const sessionId = tab?.sessionId;
    if (!sessionId) return;

    tabErrors.value.delete(tabId);
    // 停止日志记录
    const loggingStatus = getTabLoggingStatus(tabId);
    if (loggingStatus.enabled) {
      await tauriInvoke<void>('stop_logging', { connectionId: sessionId });
      setTabLoggingStatus(tabId, { ...loggingStatus, enabled: false });
    }
    const result = await tauriInvoke<void>('disconnect', { connectionId: sessionId });
    if (result.success) {
      tabStatuses.value.set(tabId, 'disconnected');
      tabConfigs.value.delete(tabId);
      tabStats.value.delete(tabId);
      sessionIdToTabId.value.delete(sessionId);
    } else {
      tabStatuses.value.set(tabId, 'error');
      tabErrors.value.set(tabId, result.error || 'Disconnect failed');
    }
  }

  async function getStatus(sessionId?: string) {
    if (!sessionId) return;
    const result = await tauriInvoke<ConnectionStatus>('get_connection_status', { params: { connection_id: sessionId } });
    if (result.success && result.data) {
      const tabId = sessionIdToTabId.value.get(sessionId);
      if (tabId) {
        tabStatuses.value.set(tabId, result.data);
      }
    }
  }

  async function writeText(sessionId: string, text: string) {
    return await tauriInvoke<void>('write_text', { params: { connection_id: sessionId, text } });
  }

  async function startLogging(tabId: string, filePath: string) {
    const tab = sessionStore.tabs.find(t => t.id === tabId);
    const sessionId = tab?.sessionId;
    if (!sessionId) return { success: false, error: 'No active session' };

    const result = await tauriInvoke<void>('start_logging', {
      connectionId: sessionId,
      filePath: filePath
    });

    if (result.success) {
      setTabLoggingStatus(tabId, {
        enabled: true,
        file_path: filePath,
        bytes_logged_input: 0,
        bytes_logged_output: 0,
        started_at: new Date().toISOString()
      });
    }

    return result;
  }

  async function stopLogging(tabId: string) {
    const tab = sessionStore.tabs.find(t => t.id === tabId);
    const sessionId = tab?.sessionId;
    if (!sessionId) return { success: false, error: 'No active session' };

    const result = await tauriInvoke<void>('stop_logging', { connectionId: sessionId });

    if (result.success) {
      const currentStatus = getTabLoggingStatus(tabId);
      setTabLoggingStatus(tabId, {
        ...currentStatus,
        enabled: false
      });
    }

    return result;
  }

  // Methods that work with sessionId instead of tabId (for event handlers)
  function setSessionStatus(sessionId: string, status: ConnectionStatus): void {
    const tabId = sessionIdToTabId.value.get(sessionId);
    if (tabId) {
      tabStatuses.value.set(tabId, status);
    }
  }

  function setSessionError(sessionId: string, error: string | null): void {
    const tabId = sessionIdToTabId.value.get(sessionId);
    if (tabId) {
      if (error) {
        tabErrors.value.set(tabId, error);
      } else {
        tabErrors.value.delete(tabId);
      }
    }
  }

  function removeSession(sessionId: string): void {
    const tabId = sessionIdToTabId.value.get(sessionId);
    if (tabId) {
      removeTab(tabId);
    }
  }

  return {
    tabStatuses,
    tabConfigs,
    tabStats,
    tabErrors,
    status,
    config,
    stats,
    error,
    isConnected,
    isConnecting,
    hasError,
    getTabStatus,
    getTabConfig,
    getTabStats,
    getTabError,
    setTabStatus,
    setTabStats,
    setTabError,
    removeTab,
    setSessionStatus,
    setSessionError,
    removeSession,
    connect,
    disconnect,
    getStatus,
    writeText,
    loggingStatus,
    isLogging,
    getTabLoggingStatus,
    setTabLoggingStatus,
    startLogging,
    stopLogging
  };
});
