<template>
  <div
    v-if="isOpen"
    class="command-panel"
    :style="{ width: panelWidth + 'px' }"
  >
    <!-- Header -->
    <div class="panel-header">
      <span class="panel-title">Command Panel</span>
      <div class="panel-actions">
        <n-button text size="small" @click="togglePanel(tabId)">
          <template #icon>
            <n-icon><chevron-right /></n-icon>
          </template>
        </n-button>
      </div>
    </div>

    <!-- Toolbar -->
    <div class="panel-toolbar">
      <n-button size="small" @click="handleOpenFile" :loading="isLoading">
        <template #icon>
          <n-icon><folder-open /></n-icon>
        </template>
        Open
      </n-button>
      <n-button
        size="small"
        type="primary"
        :disabled="!hasSelection"
        @click="handleSendSelected"
        :loading="isSending"
      >
        <template #icon>
          <n-icon><send /></n-icon>
        </template>
        Send Line
      </n-button>
    </div>

    <!-- Editor -->
    <div class="panel-content">
      <div v-if="!filePath" class="empty-state">
        <n-empty description="No file open">
          <template #extra>
            <n-button @click="handleOpenFile">Open .txt File</n-button>
          </template>
        </n-empty>
      </div>
      <textarea
        v-else
        ref="textareaRef"
        v-model="content"
        class="command-textarea"
        spellcheck="false"
        :disabled="isLoading"
      />
    </div>

    <!-- Footer -->
    <div v-if="filePath" class="panel-footer">
      <span class="file-name" :class="{ dirty: isDirty }">
        {{ fileName }}{{ isDirty ? ' •' : '' }}
      </span>
      <span class="line-count">{{ lineCount }} lines</span>
    </div>

    <!-- Resize handle -->
    <div class="resize-handle" @mousedown="startResize" />
  </div>

  <!-- 完全收起，不占用空间 -->
  <div v-else></div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { NButton, NIcon, NEmpty, useMessage } from 'naive-ui';
import { ChevronRight, FolderOpen, Send } from '@vicons/carbon';
import { useCommandPanelStore } from '../stores/commandPanel';
import { useSessionStore } from '../stores/session';

const props = defineProps<{
  tabId: string;
}>();

const store = useCommandPanelStore();
const sessionStore = useSessionStore();
const message = useMessage();

const textareaRef = ref<HTMLTextAreaElement | null>(null);
const isSending = ref(false);
const isResizing = ref(false);

// Computed
const state = computed(() => store.getTabState(props.tabId));
const isOpen = computed(() => state.value.isOpen);
const filePath = computed(() => state.value.filePath);
const isDirty = computed(() => state.value.isDirty);
const isLoading = computed(() => state.value.isLoading);
const panelWidth = computed(() => state.value.panelWidth);

const content = computed({
  get: () => state.value.content,
  set: (val) => store.updateContent(props.tabId, val)
});

const fileName = computed(() => {
  if (!filePath.value) return '';
  const parts = filePath.value.split(/[/\\]/);
  return parts[parts.length - 1];
});

const lineCount = computed(() => {
  return content.value.split('\n').length;
});

const hasSelection = computed(() => {
  const textarea = textareaRef.value;
  if (!textarea) return false;
  // 始终允许发送，没有选中时发送光标所在行
  return true;
});

// Methods
function togglePanel(tabId: string) {
  store.togglePanel(tabId);
}

async function handleOpenFile() {
  const result = await store.openFile(props.tabId);
  if (!result.success) {
    message.error(result.error || 'Failed to open file');
  }
}


async function handleSendSelected() {
  const textarea = textareaRef.value;
  if (!textarea) return;

  let textToSend = '';
  const selectionStart = textarea.selectionStart;
  const selectionEnd = textarea.selectionEnd;

  if (selectionStart !== selectionEnd) {
    // 有选中内容，发送选中的内容
    textToSend = textarea.value.substring(selectionStart, selectionEnd);
  } else {
    // 没有选中内容，发送光标所在行
    const text = textarea.value;
    const lineStart = text.lastIndexOf('\n', selectionStart - 1) + 1;
    const lineEnd = text.indexOf('\n', selectionStart);
    textToSend = text.substring(lineStart, lineEnd === -1 ? text.length : lineEnd);
  }

  isSending.value = true;
  const result = await store.sendSelected(props.tabId, textToSend);
  isSending.value = false;

  if (result.success) {
    message.success('Sent');
  } else {
    message.error(result.error || 'Failed to send');
  }
}

// Resize logic
function startResize(e: MouseEvent) {
  isResizing.value = true;
  const startX = e.clientX;
  const startWidth = panelWidth.value;

  function onMouseMove(e: MouseEvent) {
    const delta = startX - e.clientX;
    store.setPanelWidth(props.tabId, startWidth + delta);
  }

  function onMouseUp() {
    isResizing.value = false;
    document.removeEventListener('mousemove', onMouseMove);
    document.removeEventListener('mouseup', onMouseUp);
  }

  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
}

function handleF10Key(event: KeyboardEvent) {
  if (event.key === 'F10' && isOpen.value && filePath.value) {
    event.preventDefault();
    handleSendSelected();
  }
}

// Initialize
onMounted(() => {
  store.initializeTab(props.tabId);
  window.addEventListener('keydown', handleF10Key);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleF10Key);
});

// Watch for tab close
watch(() => sessionStore.tabs.find(t => t.id === props.tabId), (tab) => {
  if (!tab) {
    store.removeTab(props.tabId);
  }
}, { immediate: true });
</script>

<style scoped>
.command-panel {
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--border-color, #e0e0e0);
  background: var(--bg-color, #ffffff);
  position: relative;
  height: 100%;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-color, #e0e0e0);
}

.panel-title {
  font-size: 14px;
  font-weight: 500;
}

.panel-actions {
  display: flex;
  gap: 4px;
}

.panel-toolbar {
  display: flex;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-color, #e0e0e0);
}

.panel-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.command-textarea {
  flex: 1;
  width: 100%;
  border: none;
  outline: none;
  resize: none;
  padding: 8px 12px;
  font-family: 'Menlo', 'Monaco', 'Consolas', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.5;
  background: var(--bg-color, #ffffff);
  color: var(--text-color, #333333);
}

.command-textarea:focus {
  background: var(--bg-color-hover, #f5f5f5);
}

.panel-footer {
  display: flex;
  justify-content: space-between;
  padding: 6px 12px;
  font-size: 12px;
  color: var(--text-color-secondary, #666666);
  border-top: 1px solid var(--border-color, #e0e0e0);
}

.file-name.dirty {
  color: var(--warning-color, #f59e0b);
}

.resize-handle {
  position: absolute;
  left: -3px;
  top: 0;
  bottom: 0;
  width: 6px;
  cursor: col-resize;
}

</style>
