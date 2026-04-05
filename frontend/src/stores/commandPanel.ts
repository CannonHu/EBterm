import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { debounce } from 'lodash-es';
import { open } from '@tauri-apps/plugin-dialog';
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
import { tauriInvoke } from '../api/tauri';
import { useSessionStore } from './session';
import type { CommandResult as Result } from '../types/ipc';

// 内部状态类型
interface CommandPanelState {
  filePath: string | null;
  content: string;
  isDirty: boolean;
  isLoading: boolean;
  isOpen: boolean;
  panelWidth: number;
}

const DEFAULT_WIDTH = 320;
const SAVE_DEBOUNCE_MS = 5000;

function createDefaultState(): CommandPanelState {
  return {
    filePath: null,
    content: '',
    isDirty: false,
    isLoading: false,
    isOpen: false,
    panelWidth: DEFAULT_WIDTH
  };
}

export const useCommandPanelStore = defineStore('commandPanel', () => {
  const tabStates = ref<Map<string, CommandPanelState>>(new Map());
  const sessionStore = useSessionStore();

  // Getters
  function getTabState(tabId: string): CommandPanelState {
    if (!tabStates.value.has(tabId)) {
      tabStates.value.set(tabId, createDefaultState());
    }
    return tabStates.value.get(tabId)!;
  }

  // Actions
  function initializeTab(tabId: string): void {
    if (!tabStates.value.has(tabId)) {
      tabStates.value.set(tabId, createDefaultState());
    }
  }

  function removeTab(tabId: string): void {
    tabStates.value.delete(tabId);
  }

  function togglePanel(tabId: string): void {
    const state = getTabState(tabId);
    state.isOpen = !state.isOpen;
  }

  function setPanelWidth(tabId: string, width: number): void {
    const state = getTabState(tabId);
    state.panelWidth = Math.max(200, Math.min(width, 600));
  }

  async function openFile(tabId: string): Promise<Result<void>> {
    const state = getTabState(tabId);

    // 1. 打开文件对话框
    const selected = await open({
      title: 'Open Command File',
      defaultPath: state.filePath || undefined,
      filters: [
        { name: 'Text Files', extensions: ['txt'] },
        { name: 'All Files', extensions: ['*'] }
      ]
    });

    if (!selected) {
      return { success: false, error: 'No file selected' };
    }

    const filePath = selected as string;

    // 2. 读取文件
    state.isLoading = true;
    try {
      const content = await readTextFile(filePath);
      state.isLoading = false;

      // 3. 更新状态
      state.filePath = filePath;
      state.content = content;
      state.isDirty = false;
      state.isOpen = true;

      return { success: true, data: undefined };
    } catch (e) {
      state.isLoading = false;
      return { success: false, error: (e as Error).message || 'Failed to read file' };
    }
  }

  function updateContent(tabId: string, content: string): void {
    const state = getTabState(tabId);
    state.content = content;
    state.isDirty = true;
    debouncedSave(tabId);
  }

  async function saveFile(tabId: string): Promise<Result<void>> {
    const state = getTabState(tabId);

    if (!state.filePath || !state.isDirty) {
      return { success: true, data: undefined };
    }

    state.isLoading = true;
    try {
      await writeTextFile(state.filePath, state.content);
      state.isLoading = false;
      state.isDirty = false;
      return { success: true, data: undefined };
    } catch (e) {
      state.isLoading = false;
      return { success: false, error: (e as Error).message || 'Failed to write file' };
    }
  }

  const debouncedSave = debounce((tabId: string) => {
    const state = getTabState(tabId);
    if (state.isDirty) {
      saveFile(tabId);
    }
  }, SAVE_DEBOUNCE_MS);


  async function sendSelected(tabId: string, selectedText: string): Promise<Result<void>> {
    // 1. 获取当前 Tab 的 session
    const tab = sessionStore.tabs.find(t => t.id === tabId);
    if (!tab?.sessionId) {
      return { success: false, error: 'No active connection' };
    }

    // 2. 检查选中文本
    const trimmed = selectedText.trim();
    if (!trimmed) {
      return { success: false, error: 'No text selected' };
    }

    // 3. 发送命令（自动添加换行符）
    const commandWithNewline = trimmed + '\n';
    return await tauriInvoke<void>('write_text', {
      params: {
        connection_id: tab.sessionId,
        text: commandWithNewline
      }
    });
  }

  return {
    // Getters
    getTabState,
    // Actions
    initializeTab,
    removeTab,
    togglePanel,
    setPanelWidth,
    openFile,
    updateContent,
    saveFile,
    sendSelected
  };
});
