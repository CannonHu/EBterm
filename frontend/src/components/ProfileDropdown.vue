<script setup lang="ts">
import { ref, h, computed } from 'vue'
import { NDropdown, NButton, NIcon } from 'naive-ui'
import { TrashCan } from '@vicons/carbon'

interface Props {
  profiles: string[]
}

const props = defineProps<Props>()

interface Emits {
  (e: 'load', name: string): void
  (e: 'delete', name: string): void
}

const emit = defineEmits<Emits>()

const hoveredProfile = ref<string | null>(null)

// Transform profiles into NDropdown options with custom render
const dropdownOptions = computed(() => {
  return props.profiles.map((name) => ({
    key: name,
    label: name,
    // Custom render function for each option
    render: () => h(
      'div',
      {
        class: 'profile-option',
        onMouseenter: () => { hoveredProfile.value = name },
        onMouseleave: () => { hoveredProfile.value = null }
      },
      [
        h('span', { class: 'profile-name' }, name),
        // Delete button - visible on hover
        hoveredProfile.value === name
          ? h(
              NButton,
              {
                size: 'tiny',
                quaternary: true,
                circle: true,
                class: 'delete-btn',
                onClick: (e: MouseEvent) => {
                  e.stopPropagation()
                  handleDelete(name)
                }
              },
              {
                icon: () => h(NIcon, null, { default: () => h(TrashCan) })
              }
            )
          : null
      ]
    )
  }))
})

// Handle profile selection
function handleSelect(key: string) {
  emit('load', key)
}

// Handle delete
function handleDelete(name: string) {
  emit('delete', name)
  hoveredProfile.value = null
}
</script>

<template>
  <div v-if="profiles.length > 0" class="profile-dropdown">
    <NDropdown
      trigger="click"
      :options="dropdownOptions"
      @select="handleSelect"
    >
      <NButton size="small" quaternary>
        <slot>Load Profile</slot>
      </NButton>
    </NDropdown>
  </div>
</template>

<style scoped>
.profile-dropdown {
  display: inline-flex;
}
</style>

<style>
/* Global styles for dropdown options */
.profile-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  min-width: 160px;
  cursor: pointer;
}

.profile-option:hover {
  background-color: var(--n-option-color-hover);
}

.profile-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-right: 8px;
}

.delete-btn {
  flex-shrink: 0;
  opacity: 0.7;
  transition: opacity 0.2s ease;
}

.delete-btn:hover {
  opacity: 1;
  color: var(--n-error-color);
}
</style>
