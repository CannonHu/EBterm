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
  DataBase,
  Wifi,
  ArrowRight,
  CheckmarkOutline
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

// Computed: sorted profiles in reverse order (newest first)
// Profiles are loaded from JSON in insertion order, so we reverse to show newest first
const sortedProfiles = computed(() => {
  return [...profiles.value].reverse()
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

// Get icon for connection type
function getConnectionTypeIcon(type: string) {
  return type === 'serial' ? DataBase : Wifi
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
            <template #header-extra v-if="selectedProfile?.name === profile.name">
              <NIcon :component="CheckmarkOutline" class="selected-icon" />
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

:deep(.n-list-item.n-list-item--clickable:hover),
:deep(.n-list-item:hover),
:deep(.n-list-item--selected),
:deep(.n-list-item.is-selected) {
  background-color: var(--n-option-color-hover);
  transition: background-color 0.2s ease;
}

.profile-meta {
  font-size: 12px;
  color: var(--n-text-color-3);
}

.selected-icon {
  color: var(--n-primary-color);
  font-size: 19px;
  margin-right: 8px;
  display: inline-flex;
  align-items: center;
  height: 100%;
  vertical-align: middle;
}

/* Removed saved-time CSS */

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
</style>
