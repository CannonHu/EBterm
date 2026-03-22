<script setup lang="ts">
import { ref, h } from 'vue'
import { useSessionStore } from '../stores/session'
import { useTerminalStore } from '../stores/terminal'
import { NButton, NTabs, NTabPane, NDropdown, NInput, NIcon, NModal } from 'naive-ui'
import { Add, Close, Edit } from '@vicons/carbon'

const sessionStore = useSessionStore()
const terminalStore = useTerminalStore()

const showRenameModal = ref(false)
const renameTabId = ref<string | null>(null)
const renameValue = ref('')
const showDropdown = ref(false)
const dropdownX = ref(0)
const dropdownY = ref(0)
const contextTabId = ref<string | null>(null)

const connectionOptions = [
  { label: 'Serial Port', key: 'serial' },
  { label: 'Telnet', key: 'telnet' },
]

const contextMenuOptions = [
  { label: 'Rename', key: 'rename', icon: () => h(NIcon, null, { default: () => h(Edit) }) },
  { type: 'divider', key: 'd1' },
  { label: 'Close', key: 'close', icon: () => h(NIcon, null, { default: () => h(Close) }) },
  { label: 'Close Others', key: 'closeOthers' },
  { label: 'Close All', key: 'closeAll' },
]

const getStatusColor = (tab: typeof sessionStore.tabs[0]) => {
  if (tab.isConnecting) return '#f59e0b'
  if (tab.sessionId) return '#22c55e'
  return '#6b7280'
}

function handleTabChange(tabId: string) {
  sessionStore.setActiveTab(tabId)
}

function handleClose(tabId: string) {
  sessionStore.closeTab(tabId)
}

function handleAdd(type: string) {
  const tab = sessionStore.activeTab;
  if (tab) {
    terminalStore.toggleConfigPanel(tab.id);
  }
}

function handleContextMenu(e: MouseEvent, tabId: string) {
  e.preventDefault()
  contextTabId.value = tabId
  dropdownX.value = e.clientX
  dropdownY.value = e.clientY
  showDropdown.value = true
}

function handleMenuSelect(key: string) {
  showDropdown.value = false
  const tabId = contextTabId.value
  if (!tabId) return

  switch (key) {
    case 'rename':
      renameTabId.value = tabId
      const tab = sessionStore.tabs.find(t => t.id === tabId)
      if (tab) {
        renameValue.value = tab.title
      }
      showRenameModal.value = true
      break
    case 'close':
      sessionStore.closeTab(tabId)
      break
    case 'closeOthers':
      sessionStore.tabs.filter(t => t.id !== tabId).forEach(t => sessionStore.closeTab(t.id))
      break
    case 'closeAll':
      [...sessionStore.tabs].forEach(t => sessionStore.closeTab(t.id))
      break
  }
}

function confirmRename() {
  if (renameTabId.value && renameValue.value.trim()) {
    sessionStore.renameTab(renameTabId.value, renameValue.value.trim())
    showRenameModal.value = false
    renameTabId.value = null
    renameValue.value = ''
  }
}

function cancelRename() {
  showRenameModal.value = false
  renameTabId.value = null
  renameValue.value = ''
}
</script>

<template>
  <div class="tab-bar">
    <div class="tabs-wrapper">
      <n-tabs
        :value="sessionStore.activeTabId"
        type="card"
        size="small"
        :closable="true"
        @update:value="handleTabChange"
        @close="handleClose"
      >
        <n-tab-pane
          v-for="tab in sessionStore.tabs"
          :key="tab.id"
          :name="tab.id"
          :tab="tab.title"
        >
          <template #tab>
            <div class="tab-label" @contextmenu.prevent="(e: MouseEvent) => handleContextMenu(e, tab.id)">
              <span
                class="status-dot"
                :style="{ backgroundColor: getStatusColor(tab) }"
              />
              <span class="tab-title">{{ tab.title }}</span>
            </div>
          </template>
        </n-tab-pane>
      </n-tabs>
    </div>

    <n-dropdown :options="connectionOptions" @select="handleAdd">
      <n-button size="small" quaternary circle class="add-btn">
        <template #icon>
          <n-icon><Add /></n-icon>
        </template>
      </n-button>
    </n-dropdown>

    <n-dropdown
      :show="showDropdown"
      :x="dropdownX"
      :y="dropdownY"
      :options="contextMenuOptions"
      placement="bottom-start"
      @select="handleMenuSelect"
      @clickoutside="showDropdown = false"
    />

    <n-modal
      v-model:show="showRenameModal"
      preset="dialog"
      title="Rename Tab"
      positive-text="Confirm"
      negative-text="Cancel"
      @positive-click="confirmRename"
      @negative-click="cancelRename"
    >
      <n-input v-model:value="renameValue" placeholder="Enter new name" @keyup.enter="confirmRename" />
    </n-modal>
  </div>
</template>

<style scoped>
.tab-bar {
  display: flex;
  align-items: center;
  height: 40px;
  padding: 0 8px;
  background: var(--n-color);
  border-bottom: 1px solid var(--n-border-color);
  gap: 8px;
}

.tabs-wrapper {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.tabs-wrapper :deep(.n-tabs) {
  --n-tab-gap: 4px;
}

.tabs-wrapper :deep(.n-tabs-nav) {
  padding: 0;
}

.tabs-wrapper :deep(.n-tabs-tab) {
  padding: 6px 12px;
  border-radius: 4px 4px 0 0;
}

.tab-label {
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
  transition: background-color 0.2s ease;
}

.tab-title {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.add-btn {
  flex-shrink: 0;
}
</style>
