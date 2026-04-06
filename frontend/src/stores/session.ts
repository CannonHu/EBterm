import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { TabState } from '../types/ipc';
import { useTerminalStore } from './terminal';
import { useConnectionStore } from './connection';

export const useSessionStore = defineStore('session', () => {
  const tabs = ref<TabState[]>([]);
  const activeTabId = ref<string | null>(null);

  const activeTab = computed(() => tabs.value.find(t => t.id === activeTabId.value) || null);

  const activeTabHasConnection = computed(() => {
    const tab = activeTab.value;
    return !!tab?.sessionId;
  });

  function addTab(): string {
    const id = crypto.randomUUID();
    const newTab: TabState = {
      id,
      sessionId: null,
      title: 'New Session',
      isActive: true,
      isConnecting: false
    };
    tabs.value.push(newTab);
    setActiveTab(id);
    return id;
  }

  function closeTab(tabId: string): void {
    const index = tabs.value.findIndex(t => t.id === tabId);
    if (index === -1) return;

    const tab = tabs.value[index];
    const isClosingActive = tabId === activeTabId.value;

    if (tab.sessionId) {
      const terminalStore = useTerminalStore();
      terminalStore.removeState(tabId);
      const connectionStore = useConnectionStore();
      connectionStore.removeTab(tabId);
    }

    tabs.value.splice(index, 1);

    if (isClosingActive) {
      if (tabs.value.length > 0) {
        const newIndex = Math.min(index, tabs.value.length - 1);
        setActiveTab(tabs.value[newIndex].id);
      } else {
        addTab();
      }
    }
  }

  function setActiveTab(tabId: string): void {
    activeTabId.value = tabId;
    tabs.value.forEach(t => {
      t.isActive = t.id === tabId;
    });
  }

  function renameTab(tabId: string, newName: string): void {
    const tab = tabs.value.find(t => t.id === tabId);
    if (tab) {
      tab.title = newName;
    }
  }

  function connectTab(tabId: string, sessionId: string): void {
    const tab = tabs.value.find(t => t.id === tabId);
    if (tab) {
      tab.sessionId = sessionId;
      tab.isConnecting = false;
    }
  }

  function disconnectTab(tabId: string): void {
    const tab = tabs.value.find(t => t.id === tabId);
    if (tab) {
      tab.sessionId = null;
      tab.isConnecting = false;
    }
  }

  function updateTabConnecting(tabId: string, isConnecting: boolean): void {
    const tab = tabs.value.find(t => t.id === tabId);
    if (tab) {
      tab.isConnecting = isConnecting;
    }
  }

  addTab();

  return {
    tabs,
    activeTabId,
    activeTab,
    activeTabHasConnection,
    addTab,
    closeTab,
    setActiveTab,
    renameTab,
    connectTab,
    disconnectTab,
    updateTabConnecting
  };
});
