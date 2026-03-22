import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { ConnectionParams, ConnectionStatus, ConnectionStats } from '../types/ipc';
import { tauriInvoke } from '../api/tauri';
import { useSessionStore } from './session';

export const useConnectionStore = defineStore('connection', () => {
  const sessionStore = useSessionStore();

  const sessionStatuses = ref<Map<string, ConnectionStatus>>(new Map());
  const sessionConfigs = ref<Map<string, ConnectionParams>>(new Map());
  const sessionStats = ref<Map<string, ConnectionStats>>(new Map());
  const sessionErrors = ref<Map<string, string>>(new Map());

  const status = computed(() => {
    const activeId = sessionStore.activeTab?.sessionId;
    return activeId ? sessionStatuses.value.get(activeId) ?? 'disconnected' : 'disconnected';
  });

  const config = computed(() => {
    const activeId = sessionStore.activeTab?.sessionId;
    if (!activeId) return null;
    return sessionConfigs.value.get(activeId) ?? null;
  });

  const stats = computed(() => {
    const activeId = sessionStore.activeTab?.sessionId;
    const defaultStats: ConnectionStats = { bytes_sent: 0, bytes_received: 0, packets_sent: 0, packets_received: 0 };
    return activeId ? sessionStats.value.get(activeId) ?? defaultStats : defaultStats;
  });

  const error = computed(() => {
    const activeId = sessionStore.activeTab?.sessionId;
    return activeId ? sessionErrors.value.get(activeId) ?? null : null;
  });

  const isConnected = computed(() => status.value === 'connected');
  const isConnecting = computed(() => status.value === 'connecting');
  const hasError = computed(() => status.value === 'error');

  function getSessionStatus(sessionId: string): ConnectionStatus {
    return sessionStatuses.value.get(sessionId) ?? 'disconnected';
  }

  function getSessionConfig(sessionId: string): ConnectionParams | undefined {
    return sessionConfigs.value.get(sessionId);
  }

  function getSessionStats(sessionId: string): ConnectionStats {
    const defaultStats: ConnectionStats = { bytes_sent: 0, bytes_received: 0, packets_sent: 0, packets_received: 0 };
    return sessionStats.value.get(sessionId) ?? defaultStats;
  }

  function getSessionError(sessionId: string): string | null {
    return sessionErrors.value.get(sessionId) ?? null;
  }

  function setSessionStatus(sessionId: string, status: ConnectionStatus): void {
    sessionStatuses.value.set(sessionId, status);
  }

  function setSessionStats(sessionId: string, stats: ConnectionStats): void {
    sessionStats.value.set(sessionId, stats);
  }

  function setSessionError(sessionId: string, error: string | null): void {
    if (error) {
      sessionErrors.value.set(sessionId, error);
    } else {
      sessionErrors.value.delete(sessionId);
    }
  }

  function removeSession(sessionId: string): void {
    sessionStatuses.value.delete(sessionId);
    sessionConfigs.value.delete(sessionId);
    sessionStats.value.delete(sessionId);
    sessionErrors.value.delete(sessionId);
  }

  async function connect(params: ConnectionParams, sessionId?: string) {
    if (!sessionId) return;

    sessionErrors.value.delete(sessionId);
    sessionStatuses.value.set(sessionId, 'connecting');
    sessionConfigs.value.set(sessionId, params);

    const result = await tauriInvoke<string>('connect', { params });
    if (result.success && result.data) {
      sessionStatuses.value.set(sessionId, 'connected');
      const sessionStore = useSessionStore();
      sessionStore.connectTab(sessionId, result.data);
    } else {
      sessionStatuses.value.set(sessionId, 'error');
      sessionErrors.value.set(sessionId, result.error || 'Connection failed');
    }
  }

  async function disconnect(sessionId?: string) {
    if (!sessionId) return;

    sessionErrors.value.delete(sessionId);
    const result = await tauriInvoke<void>('disconnect', { sessionId });
    if (result.success) {
      sessionStatuses.value.set(sessionId, 'disconnected');
      sessionConfigs.value.delete(sessionId);
      sessionStats.value.delete(sessionId);
    } else {
      sessionStatuses.value.set(sessionId, 'error');
      sessionErrors.value.set(sessionId, result.error || 'Disconnect failed');
    }
  }

  async function getStatus(sessionId?: string) {
    if (!sessionId) return;
    const result = await tauriInvoke<ConnectionStatus>('get_connection_status', { sessionId });
    if (result.success && result.data) {
      sessionStatuses.value.set(sessionId, result.data);
    }
  }

  async function writeData(sessionId: string, data: number[]) {
    return await tauriInvoke<void>('write_data', { sessionId, data });
  }

  async function writeText(sessionId: string, text: string) {
    return await tauriInvoke<void>('write_text', { sessionId, text });
  }

  return {
    sessionStatuses,
    sessionConfigs,
    sessionStats,
    sessionErrors,
    status,
    config,
    stats,
    error,
    isConnected,
    isConnecting,
    hasError,
    getSessionStatus,
    getSessionConfig,
    getSessionStats,
    getSessionError,
    setSessionStatus,
    setSessionStats,
    setSessionError,
    removeSession,
    connect,
    disconnect,
    getStatus,
    writeData,
    writeText
  };
});
