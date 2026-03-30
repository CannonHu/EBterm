<script setup lang="ts">
import { ref, h } from 'vue'
import { useSessionStore } from '../stores/session'
import { useTerminalStore } from '../stores/terminal'
import { useConnectionStore } from '../stores/connection'
import { NButton, NTabs, NTabPane, NDropdown, NInput, NIcon, NModal, useMessage } from 'naive-ui'
import { Add, Close, Edit } from '@vicons/carbon'
import ProfileSelectorDialog from '../components/ProfileSelectorDialog.vue'
import type { SavedProfile } from '../types/ipc'

const sessionStore = useSessionStore()
const terminalStore = useTerminalStore()
const connectionStore = useConnectionStore()
const message = useMessage()

const showRenameModal = ref(false)
const renameTabId = ref<string | null>(null)
const renameValue = ref('')
const showDropdown = ref(false)
const dropdownX = ref(0)
const dropdownY = ref(0)
const contextTabId = ref<string | null>(null)

const showProfileSelector = ref(false)
const isConnectingFromProfile = ref(false)

const connectionOptions = [
  { label: 'Serial Port', key: 'serial' },
  { label: 'Telnet', key: 'telnet' },
  { type: 'divider', key: 'd1' },
  { label: 'Load from Profile...', key: 'profile' },
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
  if (type === 'profile') {
    showProfileSelector.value = true
    return
  }
  
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

async function handleProfileConnect(profile: SavedProfile) {
  isConnectingFromProfile.value = true
  
  try {
    // 1. Create new tab
    const newTabId = sessionStore.addTab()
    sessionStore.setActiveTab(newTabId)
    
    // 2. Set tab title to profile name
    sessionStore.renameTab(newTabId, profile.name)
    
    // 3. Connect using connectionStore
    await connectionStore.connect(profile.params, newTabId)
    
    if (connectionStore.isConnected) {
      // Connection success, close dialog
      showProfileSelector.value = false
      message.success(`Connected using profile "${profile.name}"`)
    } else if (connectionStore.hasError) {
      // Connection failed, show error (keep tab open)
      message.error(`Connection failed: ${connectionStore.error}`)
    }
  } catch (error) {
    console.error('Profile connection error:', error)
    message.error(`Connection error: ${error}`)
    // Reset state and close dialog on failure
    showProfileSelector.value = false
  } finally {
    isConnectingFromProfile.value = false
  }
}

function handleOpenConfigPanel() {
  const tab = sessionStore.activeTab
  if (tab) {
    terminalStore.openConfigPanel(tab.id)
  } else {
    // If no tab, create new tab
    const newTabId = sessionStore.addTab()
    sessionStore.setActiveTab(newTabId)
    terminalStore.openConfigPanel(newTabId)
  }
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

    <ProfileSelectorDialog
      :visible="showProfileSelector"
      @update:visible="showProfileSelector = $event"
      @connect="handleProfileConnect"
      @open-config-panel="handleOpenConfigPanel"
    />
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
