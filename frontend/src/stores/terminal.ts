import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { TerminalUIState } from '../types/ipc';
import { useSessionStore } from './session';

type DataListener = (data: string) => void;

export const useTerminalStore = defineStore('terminal', () => {
  const states = ref<Map<string, TerminalUIState>>(new Map());
  const dataListeners = ref<Map<string, Set<DataListener>>>(new Map());

  const sessionStore = useSessionStore();

  const activeTerminalState = computed(() => {
    const activeId = sessionStore.activeTabId;
    return activeId ? states.value.get(activeId) : null;
  });

  function initState(id: string): void {
    const defaultState: TerminalUIState = {
      showTimestamps: false,
      isSearchOpen: false,
      isConfigPanelOpen: false
    };
    states.value.set(id, defaultState);
  }

  function removeState(id: string): void {
    states.value.delete(id);
    dataListeners.value.delete(id);
  }

  function getState(id: string): TerminalUIState {
    if (!states.value.has(id)) {
      initState(id);
    }
    return states.value.get(id)!;
  }

  function onData(id: string, listener: DataListener): () => void {
    if (!dataListeners.value.has(id)) {
      dataListeners.value.set(id, new Set());
    }
    dataListeners.value.get(id)!.add(listener);
    return () => {
      dataListeners.value.get(id)?.delete(listener);
    };
  }

  function emitData(id: string, data: string): void {
    const listeners = dataListeners.value.get(id);
    if (listeners) {
      listeners.forEach(listener => listener(data));
    }
  }

  function toggleTimestamps(id: string): void {
    const state = getState(id);
    state.showTimestamps = !state.showTimestamps;
  }

  function openSearch(id: string): void {
    const state = getState(id);
    state.isSearchOpen = true;
  }

  function closeSearch(id: string): void {
    const state = getState(id);
    state.isSearchOpen = false;
  }

  function toggleConfigPanel(id: string): void {
    const state = getState(id);
    state.isConfigPanelOpen = !state.isConfigPanelOpen;
  }

  function openConfigPanel(id: string): void {
    const state = getState(id);
    state.isConfigPanelOpen = true;
  }

  function closeConfigPanel(id: string): void {
    const state = getState(id);
    state.isConfigPanelOpen = false;
  }

  return {
    states,
    activeTerminalState,
    initState,
    removeState,
    getState,
    onData,
    emitData,
    toggleTimestamps,
    openSearch,
    closeSearch,
    toggleConfigPanel,
    openConfigPanel,
    closeConfigPanel
  };
});
