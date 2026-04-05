<script setup lang="ts">
import { onMounted, onUnmounted, computed } from 'vue'
import TabBar from '../components/TabBar.vue'
import TerminalPane from '../components/TerminalPane.vue'
import StatusBar from '../components/StatusBar.vue'
import CommandPanel from '../components/CommandPanel.vue'
import { useSessionStore } from '../stores/session'
import { useTerminalStore } from '../stores/terminal'
import { useTauriEvents } from '../composables/useTauriEvents'

const sessionStore = useSessionStore()
const terminalStore = useTerminalStore()
const { cleanup } = useTauriEvents()
const activeTabId = computed(() => sessionStore.activeTabId)

// 检查当前标签页是否已连接（有 sessionId）
function canOpenSearch(): boolean {
  if (!activeTabId.value) return false
  const activeTab = sessionStore.tabs.find(t => t.id === activeTabId.value)
  return !!(activeTab?.sessionId)
}

// 全局快捷键：Ctrl/Cmd + F 打开搜索
const handleKeydown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'f') {
    e.preventDefault()
    if (canOpenSearch()) {
      terminalStore.openSearch(activeTabId.value!)
    }
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div class="main-layout">
    <TabBar />
    <div class="content-area">
      <div class="terminal-area">
        <TerminalPane
          v-for="tab in sessionStore.tabs"
          v-show="tab.id === sessionStore.activeTabId"
          :key="tab.id"
          :tab-id="tab.id"
          :session-id="tab.sessionId"
        />
      </div>
      <CommandPanel
        v-if="activeTabId"
        :tab-id="activeTabId as string"
        class="command-panel-wrapper"
      />
    </div>
    <StatusBar />
  </div>
</template>

<style scoped>
.main-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
  background: var(--n-color-embedded);
}

.content-area {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.terminal-area {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.command-panel-wrapper {
  flex-shrink: 0;
}
</style>
