<template>
  <div class="status-bar">
    <div class="left-section">
      <span class="status-dot" :class="connectionStatusClass" />
      <span class="session-name">{{ sessionName }}</span>
    </div>
    <div class="right-section">
      <span class="stats">TX: {{ formattedBytesSent }} | RX: {{ formattedBytesReceived }}</span>
      <NButton size="tiny" quaternary @click="toggleTimestamp">
        <template #icon>
          <ClockIcon :class="{ 'icon-active': showTimestamp }" />
        </template>
      </NButton>
      <NButton size="tiny" quaternary @click="openSearch">
        <template #icon>
          <SearchIcon />
        </template>
      </NButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NButton } from 'naive-ui';
import { Clock as ClockIcon, Search as SearchIcon } from '@vicons/ionicons5';
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

const formattedBytesSent = computed(() => formatBytes(connectionStore.stats.bytes_sent));
const formattedBytesReceived = computed(() => formatBytes(connectionStore.stats.bytes_received));

const showTimestamp = computed(() => {
  const state = terminalStore.activeTerminalState;
  return state?.showTimestamps ?? false;
});

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

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
</script>

<style scoped>
.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 24px;
  padding: 0 8px;
  background: var(--n-color-embedded);
  border-top: 1px solid var(--n-border-color);
  font-size: 11px;
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
  width: 8px;
  height: 8px;
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
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.stats {
  font-family: var(--n-mono-font);
  margin-right: 4px;
}

.icon-active {
  color: #18a058;
}
</style>
</content>