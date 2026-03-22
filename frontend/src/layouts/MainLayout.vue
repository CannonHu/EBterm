<script setup lang="ts">
import TabBar from '../components/TabBar.vue'
import TerminalPane from '../components/TerminalPane.vue'
import StatusBar from '../components/StatusBar.vue'
import { useSessionStore } from '../stores/session'
import { useTauriEvents } from '../composables/useTauriEvents'

const sessionStore = useSessionStore()
const { cleanup } = useTauriEvents()
</script>

<template>
  <div class="main-layout">
    <TabBar />
    <div class="terminal-area">
      <TerminalPane
        v-for="tab in sessionStore.tabs"
        v-show="tab.id === sessionStore.activeTabId"
        :key="tab.id"
        :tab-id="tab.id"
        :session-id="tab.sessionId"
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

.terminal-area {
  flex: 1;
  position: relative;
  overflow: hidden;
}
</style>
