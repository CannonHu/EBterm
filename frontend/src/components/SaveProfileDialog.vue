<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  NModal,
  NInput,
  NButton,
  NForm,
  NFormItem,
  useMessage
} from 'naive-ui'

interface Props {
  visible: boolean
  params: any
}

const props = defineProps<Props>()

interface Emits {
  (e: 'update:visible', value: boolean): void
  (e: 'save', name: string, params: any): void
}

const emit = defineEmits<Emits>()

const message = useMessage()

// Form state
const profileName = ref('')
const isSubmitting = ref(false)

// Reset form when dialog opens
watch(
  () => props.visible,
  (visible) => {
    if (visible) {
      profileName.value = ''
      isSubmitting.value = false
    }
  }
)

// Validate profile name
function validateName(name: string): string | null {
  const trimmed = name.trim()
  if (!trimmed) {
    return 'Profile name is required'
  }
  if (trimmed.length > 100) {
    return 'Profile name must be 100 characters or less'
  }
  return null
}

// Handle save
function handleSave() {
  console.log('[SaveProfileDialog] handleSave props.params:', JSON.stringify(props.params))
  const error = validateName(profileName.value)
  if (error) {
    message.error(error)
    return
  }

  isSubmitting.value = true
  const name = profileName.value.trim()
  emit('save', name, props.params)
  emit('update:visible', false)
  isSubmitting.value = false
}

// Handle cancel
function handleCancel() {
  emit('update:visible', false)
}

// Handle enter key
function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSave()
  }
}
</script>

<template>
  <NModal
    :show="visible"
    preset="card"
    title="Save Profile"
    size="small"
    :mask-closable="!isSubmitting"
    @update:show="(val) => emit('update:visible', val)"
  >
    <NForm>
      <NFormItem label="Profile Name" required>
        <NInput
          v-model:value="profileName"
          placeholder="Enter profile name"
          :disabled="isSubmitting"
          :maxlength="100"
          show-count
          clearable
          @keydown="handleKeyDown"
        />
      </NFormItem>
    </NForm>

    <template #footer>
      <div class="dialog-footer">
        <NButton quaternary :disabled="isSubmitting" @click="handleCancel">
          Cancel
        </NButton>
        <NButton type="primary" :loading="isSubmitting" @click="handleSave">
          Save
        </NButton>
      </div>
    </template>
  </NModal>
</template>

<style scoped>
.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
</style>
