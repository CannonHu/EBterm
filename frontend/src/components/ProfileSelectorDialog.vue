<script setup lang="ts">
import { ref, watch, computed, h } from 'vue'
import {
  NModal,
  NList,
  NListItem,
  NEmpty,
  NButton,
  NAlert,
  NIcon,
  NSpin,
  useMessage,
  NThing
} from 'naive-ui'
import {
  Server,
  Wifi,
  ArrowRight,
  Close
} from '@vicons/carbon'
import { profileStorage } from '../services/profileStorage'
import type { SavedProfile } from '../types/ipc'

interface Props {
  visible: boolean
}

const props = defineProps<Props>()

interface Emits {
  (e: 'update:visible', value: boolean): void
  (e: 'connect', profile: SavedProfile): void
  (e: 'open-config-panel'): void
}

const emit = defineEmits<Emits>()

const message = useMessage()

// Reactive state
const profiles = ref<SavedProfile[]>([])
const isLoading = ref(false)
const isConnecting = ref(false)
const connectionError = ref<string | null>(null)
const selectedProfile = ref<SavedProfile | null>(null)

// Computed: sorted profiles by saved time (newest first)
const sortedProfiles = computed(() => {
  return [...profiles.value].sort((a, b) => {
    return new Date(b.savedAt).getTime() - new Date(a.savedAt).getTime()
  })
})

// Watch for dialog visibility changes
watch(
  () => props.visible,
  (visible) => {
    if (visible) {
      loadProfiles()
      selectedProfile.value = null
      connectionError.value = null
      isConnecting.value = false
    }
  }
)

// Load profiles from storage
async function loadProfiles() {
  isLoading.value = true
  try {
    const names = await profileStorage.listProfiles()
    const loadedProfiles: SavedProfile[] = []

    for (const name of names) {
      const profile = await profileStorage.getProfile(name)
      if (profile) {
        loadedProfiles.push(profile)
      }
    }

    profiles.value = loadedProfiles
  } catch (error) {
    console.error('Failed to load profiles:', error)
    message.error('Failed to load saved profiles')
  } finally {
    isLoading.value = false
  }
}

// Format date for display
function formatSavedAt(savedAt: string): string {
  const date = new Date(savedAt)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 1) return 'Just now'
  if (diffMins < 60) return `${diffMins}m ago`
  if (diffHours < 24) return `${diffHours}h ago`
  if (diffDays < 7) return `${diffDays}d ago`

  return date.toLocaleDateString()
}

// Get icon for connection type
function getConnectionTypeIcon(type: string) {
  return type === 'serial' ? Server : Wifi
}

// Handle profile selection
function handleSelectProfile(profile: SavedProfile) {
  selectedProfile.value = profile
}

// Handle double click to connect
function handleDoubleClick(profile: SavedProfile) {
  selectedProfile.value = profile
  handleConnect()
}

// Handle connect button click
async function handleConnect() {
  if (!selectedProfile.value) {
    message.warning('Please select a profile')
    return
  }

  isConnecting.value = true
  connectionError.value = null

  try {
    emit('connect', selectedProfile.value)
    // Parent component handles closing the dialog
  } catch (error) {
    console.error('Connection failed:', error)
    connectionError.value = error instanceof Error ? error.message : 'Connection failed'
    isConnecting.value = false
  }
}

// Handle cancel button click
function handleCancel() {
  if (isConnecting.value) return
  emit('update:visible', false)
}

// Handle opening config panel (when no profiles)
function handleOpenConfigPanel() {
  emit('open-config-panel')
  emit('update:visible', false)
}

// Handle closing error alert
function handleCloseError() {
  connectionError.value = null
}
</script>

<template>
  <NModal
    :show="visible"
    preset="card"
    title="Select Profile"
    size="small"
    style="width: 480px; max-width: 90vw;"
    :mask-closable="!isConnecting"
    :close-on-esc="!isConnecting"
    @update:show="(val) => emit('update:visible', val)"
  >
    <!-- Error Alert -->
    <NAlert
      v-if="connectionError"
      type="error"
      closable
      style="margin-bottom: 16px;"
      @close="handleCloseError"
    >
      {{ connectionError }}
    </NAlert>

    <!-- Loading State -->
    <div v-if="isLoading" class="loading-container">
      <NSpin size="medium" />
      <p class="loading-text">Loading profiles...</p>
    </div>

    <!-- Empty State -->
    <div v-else-if="profiles.length === 0" class="empty-container">
      <NEmpty description="No saved profiles">
        <template #extra>
          <NButton size="small" @click="handleOpenConfigPanel">
            Go to Config Panel
          </NButton>
        </template>
      </NEmpty>
    </div>

    <!-- Profile List -->
    <div v-else class="profile-list-container">
      <NList hoverable clickable style="background: transparent;">
        <NListItem
          v-for="profile in sortedProfiles"
          :key="profile.name"
          :class="{ 'is-selected': selectedProfile?.name === profile.name }"
          @click="handleSelectProfile(profile)"
          @dblclick="handleDoubleClick(profile)"
        >
          <NThing>
            <template #avatar>
              <NIcon :component="getConnectionTypeIcon(profile.params.type)" />
            </template>
            <template #header>
              {{ profile.name }}
            </template>
            <template #description>
              <span class="profile-meta">
                {{ profile.params.type === 'serial'
                  ? `${profile.params.port} @ ${profile.params.baud_rate}`
                  : `${profile.params.host}:${profile.params.port}`
                }}
              </span>
            </template>
            <template #header-extra>
              <span class="saved-time">{{ formatSavedAt(profile.savedAt) }}</span>
            </template>
          </NThing>
        </NListItem>
      </NList>
    </div>

    <!-- Footer -->
    <template #footer>
      <div class="dialog-footer">
        <NButton
          quaternary
          :disabled="isConnecting"
          @click="handleCancel"
        >
          Cancel
        </NButton>
        <NButton
          type="primary"
          :disabled="!selectedProfile"
          :loading="isConnecting"
          @click="handleConnect"
        >
          <template #icon>
            <NIcon :component="ArrowRight" />
          </template>
          Connect
        </NButton>
      </div>
    </template>
  </NModal>
</template>

<style scoped>
.loading-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  gap: 12px;
}

.loading-text {
  margin: 0;
  color: var(--n-text-color-3);
  font-size: 14px;
}

.empty-container {
  padding: 32px 24px;
}

.profile-list-container {
  max-height: 400px;
  overflow-y: auto;
}

:deep(.n-list-item) {
  transition: background-color 0.2s ease;
}

:deep(.n-list-item.is-selected) {
  background-color: var(--n-option-color-active);
}

:deep(.n-list-item.is-selected:hover) {
  background-color: var(--n-option-color-active);
}

.profile-meta {
  font-size: 12px;
  color: var(--n-text-color-3);
}

.saved-time {
  font-size: 12px;
  color: var(--n-text-color-3);
  white-space: nowrap;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
</style>
