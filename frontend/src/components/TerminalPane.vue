<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { NButton } from 'naive-ui'
import Terminal from './Terminal.vue'
import ConfigPanel from './ConfigPanel.vue'
import SearchBar from './SearchBar.vue'
import { useSessionStore } from '../stores/session'
import { useTerminalStore } from '../stores/terminal'
import { useConnectionStore } from '../stores/connection'
import type { TabState } from '../types/ipc'

interface Props {
  tabId: string
  sessionId: string | null
}

const props = defineProps<Props>()

const sessionStore = useSessionStore()
const terminalStore = useTerminalStore()
const connectionStore = useConnectionStore()

const terminalRef = ref<InstanceType<typeof Terminal> | null>(null)

const isActive = computed(() => sessionStore.activeTabId === props.tabId)
const currentTab = computed<TabState | undefined>(() =>
  sessionStore.tabs.find(tab => tab.id === props.tabId)
)

const isConfigPanelOpen = computed(() => {
  if (!currentTab.value) return false
  const state = terminalStore.getState(currentTab.value.id)
  return state?.isConfigPanelOpen ?? false
})

const isSearchOpen = computed(() => {
  if (!currentTab.value) return false
  const state = terminalStore.getState(currentTab.value.id)
  return state?.isSearchOpen ?? false
})

const showTimestamp = computed(() => {
  if (!currentTab.value) return false
  const state = terminalStore.getState(currentTab.value.id)
  return state?.showTimestamps ?? false
})

const searchMatchCount = ref(0)
const currentSearchMatch = ref(0)
const searchQuery = ref('')
let unsubscribeData: (() => void) | null = null

function subscribeToData() {
  if (unsubscribeData) {
    unsubscribeData()
    unsubscribeData = null
  }
  if (props.sessionId) {
    unsubscribeData = terminalStore.onData(props.sessionId, (data) => {
      terminalRef.value?.write(data)
    })
  }
}

function openConfig() {
  if (!currentTab.value) return
  terminalStore.toggleConfigPanel(currentTab.value.id)
}

function closeConfig() {
  if (!currentTab.value) return
  terminalStore.closeConfigPanel(currentTab.value.id)
  // Note: blur() is already called in ConfigPanel.closePanel()
}

function closeSearch() {
  if (!currentTab.value) return
  terminalStore.closeSearch(currentTab.value.id)
  searchQuery.value = ''
  searchMatchCount.value = 0
  currentSearchMatch.value = 0
  terminalRef.value?.clearSearch()
}

function handleSearchResults(result: { matchCount: number; currentMatch: number }) {
  searchMatchCount.value = result.matchCount
  currentSearchMatch.value = result.currentMatch
}

async function handleConnected(sessionId: string) {
  await sessionStore.connectTab(props.tabId, sessionId)
}

async function handleDisconnected() {
  await sessionStore.disconnectTab(props.tabId)
}

function handleTerminalData(data: string) {
  if (!props.sessionId) {
    return;
  }
  connectionStore.writeText(props.sessionId, data)
}

function handleTerminalReady() {
  // Terminal is ready
}

function handleSearch(query: string) {
  searchQuery.value = query
  if (!terminalRef.value) return
  if (query) {
    // search() will trigger onDidChangeResults which calls handleSearchResults
    // Don't set searchMatchCount here - wait for the event
    terminalRef.value.search(query)
  } else {
    searchMatchCount.value = 0
    currentSearchMatch.value = 0
  }
}

function handleSearchNext() {
  if (!terminalRef.value || !searchQuery.value) return
  terminalRef.value.search(searchQuery.value)
  // currentMatch is updated automatically via searchResults event
}

function handleSearchPrevious() {
  if (!terminalRef.value || !searchQuery.value) return
  terminalRef.value.searchPrevious(searchQuery.value)
  // currentMatch is updated automatically via searchResults event
}

function writeToTerminal(data: string) {
  terminalRef.value?.write(data)
}

function clearTerminal() {
  terminalRef.value?.clear()
}

onMounted(() => {
  subscribeToData()
})

watch(() => props.sessionId, () => {
  subscribeToData()
})

onUnmounted(() => {
  if (unsubscribeData) {
    unsubscribeData()
  }
})

watch(
  () => isConfigPanelOpen.value,
  (isOpen) => {
    console.log('[TerminalPane] isConfigPanelOpen changed:', isOpen)
  }
)

defineExpose({
  write: writeToTerminal,
  clear: clearTerminal,
  focus: () => terminalRef.value?.focus()
})
</script>

<template>
  <div class="terminal-pane" :class="{ 'is-active': isActive }">
    <div v-if="!sessionId" class="welcome-view">
      <div class="welcome-content">
        <svg class="welcome-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="4" y="4" width="16" height="16" rx="2"/>
          <path d="M8 8l4 4-4 4"/>
          <path d="M14 16h2"/>
        </svg>
        <h3>Not Connected</h3>
        <p>Connect to a device to start your session</p>
        <NButton
          type="primary"
          size="large"
          @click="openConfig"
        >
          Connect to Device
        </NButton>
      </div>
    </div>

    <template v-else>
      <Terminal
        ref="terminalRef"
        :session-id="sessionId"
        :show-timestamp="showTimestamp"
        @data="handleTerminalData"
        @ready="handleTerminalReady"
        @search-results="handleSearchResults"
      />
      <SearchBar
        :visible="isSearchOpen"
        :match-count="searchMatchCount"
        :current-match="currentSearchMatch"
        @update:visible="closeSearch"
        @search="handleSearch"
        @next="handleSearchNext"
        @previous="handleSearchPrevious"
      />
    </template>

    <ConfigPanel
      :visible="isConfigPanelOpen"
      :tab-id="props.tabId"
      @update:visible="closeConfig"
      @connected="handleConnected"
      @disconnected="handleDisconnected"
    />
  </div>
</template>

<style scoped>
.terminal-pane {
  width: 100%;
  height: 100%;
  position: relative;
  background: var(--n-color-modal);
  overflow: hidden;
}

.terminal-pane.is-active {
  background: var(--n-color-modal);
}

.welcome-view {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--n-color-modal) 0%, var(--n-color-popover) 100%);
}

.welcome-content {
  text-align: center;
  padding: 48px;
}

.welcome-icon {
  width: 64px;
  height: 64px;
  margin: 0 auto 24px;
  color: var(--n-primary-color);
  opacity: 0.8;
}

.welcome-content h3 {
  margin: 0 0 8px;
  font-size: 20px;
  font-weight: 500;
  color: var(--n-text-color);
}

.welcome-content p {
  margin: 0 0 24px;
  font-size: 14px;
  color: var(--n-text-color-3);
}
</style>
