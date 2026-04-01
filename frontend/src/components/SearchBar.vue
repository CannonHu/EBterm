<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { NInput, NButton } from 'naive-ui'

interface Props {
  visible: boolean
  matchCount: number
  currentMatch: number
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void
  (e: 'search', query: string): void
  (e: 'next'): void
  (e: 'previous'): void
}>()

const query = ref('')
const inputRef = ref<InstanceType<typeof NInput> | null>(null)

watch(() => props.visible, (visible) => {
  if (visible) {
    nextTick(() => {
      inputRef.value?.focus()
    })
  }
})

function handleInput(value: string) {
  query.value = value
  emit('search', value)
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    e.preventDefault()
    if (e.shiftKey) {
      emit('previous')
    } else {
      emit('next')
    }
  } else if (e.key === 'Escape') {
    e.preventDefault()
    closeSearch()
  }
}

function closeSearch() {
  query.value = ''
  emit('update:visible', false)
}
</script>

<template>
  <div v-if="visible" class="search-bar">
    <div class="search-bar-content">
      <NInput
        ref="inputRef"
        v-model:value="query"
        size="small"
        placeholder="Search..."
        clearable
        class="search-input"
        @update:value="handleInput"
        @keydown="handleKeydown"
      />
      <span class="match-info">
        {{ matchCount > 0 ? `${currentMatch} of ${matchCount}` : 'No matches' }}
      </span>
      <NButton
        size="small"
        quaternary
        :disabled="matchCount === 0"
        @click="$emit('previous')"
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
          <path d="M7.41 15.41L12 10.83l4.59 4.58L18 14l-6-6-6 6z"/>
        </svg>
      </NButton>
      <NButton
        size="small"
        quaternary
        :disabled="matchCount === 0"
        @click="$emit('next')"
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
          <path d="M7.41 8.59L12 13.17l4.59-4.58L18 10l-6 6-6-6 1.41-1.41z"/>
        </svg>
      </NButton>
      <NButton
        size="small"
        quaternary
        @click="closeSearch"
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
          <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
        </svg>
      </NButton>
    </div>
  </div>
</template>

<style scoped>
.search-bar {
  position: absolute;
  top: 12px;
  right: 12px;
  z-index: 100;
}

.search-bar-content {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  background: rgba(245, 245, 245, 0.95);
  backdrop-filter: blur(8px);
  border: 1px solid rgba(0, 0, 0, 0.15);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.search-input {
  width: 180px;
}

.match-info {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.6);
  white-space: nowrap;
  min-width: 60px;
  text-align: center;
  padding: 0 4px;
}

/* Reduce button size and center icon for more compact layout */
.search-bar-content :deep(.n-button) {
  padding: 0 4px;
}
</style>
