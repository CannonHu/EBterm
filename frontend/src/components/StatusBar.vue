<template>
  <div class="status-bar">
    <div class="left-section">
      <span class="status-dot" :class="connectionStatusClass" />
      <span class="session-name">{{ sessionName }}</span>
    </div>
    <div class="right-section">
      <span v-if="isLogging" class="log-status" :title="logFilePath">
        {{ shortLogFilePath }}
      </span>
      <NButton size="small" quaternary @click="toggleTimestamp">
        <template #icon>
          <ClockIcon :class="{ 'icon-active': showTimestamp }" />
        </template>
      </NButton>
      <NButton size="small" quaternary :disabled="!canUseSearch" @click="openSearch">
        <template #icon>
          <SearchIcon />
        </template>
      </NButton>
      <NButton
        size="small"
        quaternary
        :disabled="!canUseLogging"
        @click="toggleLogging"
        :class="{ 'logging-active': isLogging }"
      >
        <template #icon>
          <SaveIcon :class="{ 'icon-active': isLogging }" />
        </template>
      </NButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NButton } from 'naive-ui';
import { Time as ClockIcon, Search as SearchIcon, Save as SaveIcon } from '@vicons/carbon';
import { documentDir } from '@tauri-apps/api/path';
import { useConnectionStore } from '../stores/connection';
import { useSessionStore } from '../stores/session';
import { useTerminalStore } from '../stores/terminal';

const connectionStore = useConnectionStore();
const sessionStore = useSessionStore();
const terminalStore = useTerminalStore();

const connectionStatusClass = computed(() => {
  switch (connectionStore.status) {
    case 'connected':
      return 'status-connected';
    case 'connecting':
      return 'status-connecting';
    case 'error':
      return 'status-error';
    default:
      return 'status-disconnected';
  }
});

const sessionName = computed(() => {
  const tab = sessionStore.activeTab;
  if (!tab) return 'No Tab';
  return tab.title;
});

const showTimestamp = computed(() => {
  const state = terminalStore.activeTerminalState;
  return state?.showTimestamps ?? false;
});

// 当前标签页是否已连接（有 sessionId）
const canUseSearch = computed(() => {
  const activeTabId = sessionStore.activeTabId;
  if (!activeTabId) return false;
  const activeTab = sessionStore.tabs.find(t => t.id === activeTabId);
  return !!(activeTab?.sessionId);
});

const canUseLogging = computed(() => canUseSearch.value);

const isLogging = computed(() => connectionStore.isLogging);

const logFilePath = computed(() => {
  return connectionStore.loggingStatus.file_path ?? '';
});

const shortLogFilePath = computed(() => {
  const path = logFilePath.value;
  if (!path) return '';
  return path.split(/[\\/]/).pop() || path;
});

function toggleTimestamp(): void {
  const tabId = sessionStore.activeTabId;
  if (tabId) {
    terminalStore.toggleTimestamps(tabId);
  }
}

function openSearch(): void {
  const tabId = sessionStore.activeTabId;
  if (tabId) {
    terminalStore.openSearch(tabId);
  }
}

async function toggleLogging() {
  const tabId = sessionStore.activeTabId;
  if (!tabId) return;

  if (isLogging.value) {
    await connectionStore.stopLogging(tabId);
  } else {
    const activeTab = sessionStore.tabs.find(t => t.id === tabId);
    const connectionName = activeTab?.title || 'unknown';
    // 清理文件名中的非法字符
    const safeName = connectionName.replace(/[<>:"/\\|?*\s]/g, '_');
    const timeStr = new Date().toISOString().replace(/[:T.]/g, '_').substring(0, 15);
    const fileName = `${safeName}_${timeStr}.log`;
    // 保存到用户文档目录，避免dev模式下触发tauri重新构建
    const docsDir = await documentDir();
    const filePath = `${docsDir}/${fileName}`;
    await connectionStore.startLogging(tabId, filePath);
  }
}
</script>

<style scoped>
.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 28px;
  padding: 0 12px;
  background: var(--n-color-embedded);
  border-top: 1px solid var(--n-border-color);
  font-size: 12px;
  color: var(--n-text-color-2);
}

.left-section {
  display: flex;
  align-items: center;
  gap: 6px;
}

.right-section {
  display: flex;
  align-items: center;
  gap: 4px;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-connected {
  background: #18a058;
}

.status-connecting {
  background: #f0a020;
}

.status-error {
  background: #d03050;
}

.status-disconnected {
  background: rgba(153, 153, 153, 0.5);
}

.session-name {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.icon-active {
  color: #18a058;
}

.log-status {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--n-text-color-3);
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-right: 8px;
}

.logging-active .n-button__icon {
  color: #d03050;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}
</style>