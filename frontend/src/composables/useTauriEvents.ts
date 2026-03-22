import { listen, type Event, type UnlistenFn } from '@tauri-apps/api/event';
import { onMounted, onUnmounted } from 'vue';
import { useConnectionStore } from '../stores/connection';
import { useSessionStore } from '../stores/session';
import { useTerminalStore } from '../stores/terminal';
import type { ConnectionStatus, IpcError } from '../types/ipc';

interface DataReceivedPayload {
  session_id: string;
  data: number[];
}

interface StatusChangedPayload {
  session_id: string;
  status: ConnectionStatus;
}

interface ErrorOccurredPayload {
  session_id: string;
  error: IpcError;
}

export function useTauriEvents() {
  const connectionStore = useConnectionStore();
  const sessionStore = useSessionStore();
  const terminalStore = useTerminalStore();

  let unlistenDataReceived: UnlistenFn | null = null;
  let unlistenStatusChanged: UnlistenFn | null = null;
  let unlistenErrorOccurred: UnlistenFn | null = null;

  async function setupListeners() {
    unlistenDataReceived = await listen<DataReceivedPayload>('data_received', (event) => {
      const { session_id, data } = event.payload;
      const text = new TextDecoder().decode(new Uint8Array(data));
      terminalStore.emitData(session_id, text);
    });

    unlistenStatusChanged = await listen<StatusChangedPayload>('status_changed', (event) => {
      const { session_id, status } = event.payload;
      connectionStore.setSessionStatus(session_id, status);
      const tab = sessionStore.tabs.find(t => t.sessionId === session_id);
      if (tab) {
        if (status === 'connected') {
          sessionStore.connectTab(tab.id, session_id);
        }
      }
    });

    unlistenErrorOccurred = await listen<ErrorOccurredPayload>('error_occurred', (event) => {
      const { session_id, error } = event.payload;
      connectionStore.setSessionError(session_id, error.message);
    });
  }

  function cleanup() {
    unlistenDataReceived?.();
    unlistenStatusChanged?.();
    unlistenErrorOccurred?.();
  }

  onMounted(() => {
    setupListeners();
  });

  onUnmounted(() => {
    cleanup();
  });

  return {
    cleanup
  };
}
