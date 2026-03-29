<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import {
  NDrawer,
  NDrawerContent,
  NForm,
  NFormItem,
  NRadioGroup,
  NRadioButton,
  NSelect,
  NInput,
  NInputNumber,
  NButton,
  useMessage
} from 'naive-ui'
import { useConnectionStore } from '../stores/connection'
import { tauriInvoke } from '../api/tauri'
import { profileStorage } from '../services/profileStorage'
import SaveProfileDialog from './SaveProfileDialog.vue'
import ProfileDropdown from './ProfileDropdown.vue'
import type {
  ConnectionParams,
  SerialParams,
  TelnetParams,
  SerialPortInfo,
  DataBits,
  Parity,
  StopBits,
  FlowControl,
  SavedProfile
} from '../types/ipc'

interface Props {
  visible: boolean
  sessionId: string
}

const props = defineProps<Props>()

interface Emits {
  (e: 'update:visible', value: boolean): void
  (e: 'connected', sessionId: string): void
  (e: 'disconnected', sessionId: string): void
}

const emit = defineEmits<Emits>()

const message = useMessage()
const connectionStore = useConnectionStore()

// Profile management
const profiles = ref<string[]>([])
const isSaveDialogVisible = ref(false)

// Connection type
const connectionType = ref<'serial' | 'telnet'>('serial')

// Serial port list
const serialPorts = ref<SerialPortInfo[]>([])
const isLoadingPorts = ref(false)

// Serial form data
const serialForm = ref<SerialParams>({
  port: '',
  baud_rate: 115200,
  data_bits: 'eight',
  parity: 'none',
  stop_bits: 'one',
  flow_control: 'none'
})

// Telnet form data
const telnetForm = ref<TelnetParams>({
  host: '',
  port: 23,
  connect_timeout_secs: 10
})

// Connection state
const isConnecting = computed(() => connectionStore.isConnecting)
const isConnected = computed(() => connectionStore.isConnected)

// Select options
const baudRateOptions = [
  { label: '9600', value: 9600 },
  { label: '19200', value: 19200 },
  { label: '38400', value: 38400 },
  { label: '57600', value: 57600 },
  { label: '115200', value: 115200 }
]

const dataBitsOptions: { label: string; value: DataBits }[] = [
  { label: '7', value: 'seven' },
  { label: '8', value: 'eight' }
]

const parityOptions: { label: string; value: Parity }[] = [
  { label: 'None', value: 'none' },
  { label: 'Odd', value: 'odd' },
  { label: 'Even', value: 'even' }
]

const stopBitsOptions: { label: string; value: StopBits }[] = [
  { label: '1', value: 'one' },
  { label: '2', value: 'two' }
]

const flowControlOptions: { label: string; value: FlowControl }[] = [
  { label: 'None', value: 'none' },
  { label: 'Software', value: 'software' },
  { label: 'Hardware', value: 'hardware' }
]

// Load serial ports
async function loadSerialPorts() {
  isLoadingPorts.value = true
  try {
    const result = await tauriInvoke<SerialPortInfo[]>('list_serial_ports')
    if (result.success && result.data) {
      serialPorts.value = result.data
    } else {
      message.error('Failed to load serial ports')
    }
  } catch (error) {
    message.error(`Error loading serial ports: ${error}`)
  } finally {
    isLoadingPorts.value = false
  }
}

// Form validation
function validateForm(): boolean {
  if (connectionType.value === 'serial') {
    if (!serialForm.value.port) {
      message.error('Please select a serial port')
      return false
    }
  } else {
    if (!telnetForm.value.host) {
      message.error('Please enter a host address')
      return false
    }
    if (!telnetForm.value.port || telnetForm.value.port < 1 || telnetForm.value.port > 65535) {
      message.error('Please enter a valid port (1-65535)')
      return false
    }
  }
  return true
}

// Handle connect
async function handleConnect() {
  if (!validateForm()) return

  let params: ConnectionParams
  if (connectionType.value === 'serial') {
    params = { type: 'serial', ...serialForm.value }
  } else {
    params = { type: 'telnet', ...telnetForm.value }
  }

  try {
    await connectionStore.connect(params, props.sessionId)
    if (connectionStore.isConnected) {
      message.success('Connected successfully')
      emit('connected', props.sessionId)
      closePanel()
    } else if (connectionStore.hasError) {
      message.error(connectionStore.error || 'Connection failed')
    }
  } catch (error) {
    message.error(`Connection error: ${error}`)
  }
}

// Handle disconnect
async function handleDisconnect() {
  try {
    await connectionStore.disconnect(props.sessionId)
    if (!connectionStore.isConnected) {
      message.success('Disconnected')
      emit('disconnected', props.sessionId)
    }
  } catch (error) {
    message.error(`Disconnect error: ${error}`)
  }
}

// Close panel
function closePanel() {
  emit('update:visible', false)
}

// Watch for visible changes to load serial ports
watch(
  () => props.visible,
  (visible) => {
    if (visible && connectionType.value === 'serial') {
      loadSerialPorts()
    }
  }
)

// Load profiles
async function loadProfiles() {
  try {
    const profileList = await profileStorage.listProfiles()
    profiles.value = profileList
  } catch (error) {
    console.error('Failed to load profiles:', error)
  }
}

// Handle save profile
async function handleSaveProfile(name: string, params: any) {
  try {
    await profileStorage.saveProfile(name, params)
    message.success(`Profile '${name}' saved successfully`)
    await loadProfiles()
  } catch (error) {
    if (error instanceof Error) {
      if (error.message.includes('Maximum 100 profiles')) {
        message.error('Maximum 100 profiles allowed')
      } else {
        message.error(`Failed to save profile: ${error.message}`)
      }
    } else {
      message.error('Failed to save profile')
    }
  }
}

// Handle load profile
async function handleLoadProfile(name: string) {
  try {
    const profile = await profileStorage.getProfile(name)
    if (!profile) {
      message.error(`Profile '${name}' not found`)
      return
    }

    const { params, savedAt } = profile

    if (params.type === 'serial') {
      connectionType.value = 'serial'
      serialForm.value = {
        port: params.port,
        baud_rate: params.baud_rate,
        data_bits: params.data_bits,
        parity: params.parity,
        stop_bits: params.stop_bits,
        flow_control: params.flow_control
      }
    } else if (params.type === 'telnet') {
      connectionType.value = 'telnet'
      telnetForm.value = {
        host: params.host,
        port: params.port,
        connect_timeout_secs: params.connect_timeout_secs
      }
    }

    message.success(`Profile '${name}' loaded (saved at ${new Date(savedAt).toLocaleString()})`)
  } catch (error) {
    message.error(`Failed to load profile: ${error}`)
  }
}

// Handle delete profile
async function handleDeleteProfile(name: string) {
  try {
    await profileStorage.deleteProfile(name)
    message.success(`Profile '${name}' deleted`)
    await loadProfiles()
  } catch (error) {
    message.error(`Failed to delete profile: ${error}`)
  }
}

// Load serial ports on mount if panel is visible
onMounted(() => {
  if (props.visible && connectionType.value === 'serial') {
    loadSerialPorts()
  }
  loadProfiles()
})
</script>

<template>
  <NDrawer
    :show="visible"
    placement="top"
    :height="320"
    :mask-closable="!isConnecting"
    @update:show="(val) => emit('update:visible', val)"
  >
    <NDrawerContent title="Connection Configuration" closable @close="closePanel">
      <NForm label-placement="left" label-width="120" :disabled="isConnecting">
        <!-- Connection Type -->
        <NFormItem label="Type">
          <NRadioGroup v-model:value="connectionType">
            <NRadioButton value="serial">Serial</NRadioButton>
            <NRadioButton value="telnet">Telnet</NRadioButton>
          </NRadioGroup>
        </NFormItem>

        <!-- Serial Configuration -->
        <template v-if="connectionType === 'serial'">
          <div class="serial-grid">
            <NFormItem label="Port" required>
              <NSelect
                v-model:value="serialForm.port"
                :options="serialPorts.map((p) => ({ label: p.port_name, value: p.port_name }))"
                placeholder="Select serial port"
                :loading="isLoadingPorts"
                clearable
              >
                <template #empty>
                  <div class="empty-ports">
                    <span v-if="isLoadingPorts">Loading ports...</span>
                    <span v-else>No serial ports found</span>
                  </div>
                </template>
              </NSelect>
            </NFormItem>

            <NFormItem label="Baud Rate">
              <NSelect v-model:value="serialForm.baud_rate" :options="baudRateOptions" />
            </NFormItem>

            <NFormItem label="Data Bits">
              <NSelect v-model:value="serialForm.data_bits" :options="dataBitsOptions" />
            </NFormItem>

            <NFormItem label="Parity">
              <NSelect v-model:value="serialForm.parity" :options="parityOptions" />
            </NFormItem>

            <NFormItem label="Stop Bits">
              <NSelect v-model:value="serialForm.stop_bits" :options="stopBitsOptions" />
            </NFormItem>

            <NFormItem label="Flow Control">
              <NSelect v-model:value="serialForm.flow_control" :options="flowControlOptions" />
            </NFormItem>
          </div>
        </template>

        <!-- Telnet Configuration -->
        <template v-else>
          <div class="telnet-grid">
            <NFormItem label="Host" required>
              <NInput
                v-model:value="telnetForm.host"
                placeholder="Enter host address (e.g., 192.168.1.1)"
                clearable
              />
            </NFormItem>

            <NFormItem label="Port">
              <NInputNumber
                v-model:value="telnetForm.port"
                :min="1"
                :max="65535"
                placeholder="Port number"
              />
            </NFormItem>

            <NFormItem label="Timeout">
              <NInputNumber
                v-model:value="telnetForm.connect_timeout_secs"
                :min="1"
                :max="300"
                placeholder="Connection timeout (seconds)"
              />
            </NFormItem>
          </div>
        </template>
      </NForm>

      <template #footer>
        <div class="drawer-footer">
          <ProfileDropdown :profiles="profiles" @load="handleLoadProfile" @delete="handleDeleteProfile">
            <template #default>Load Profile</template>
          </ProfileDropdown>
          <NButton quaternary @click="closePanel">Cancel</NButton>
          <NButton quaternary @click="isSaveDialogVisible = true">Save Profile</NButton>
          <NButton
            v-if="isConnected"
            type="error"
            :loading="isConnecting"
            @click="handleDisconnect"
          >
            Disconnect
          </NButton>
          <NButton
            v-else
            type="primary"
            :loading="isConnecting"
            :disabled="isConnecting"
            @click="handleConnect"
          >
            Connect
          </NButton>
        </div>
      </template>
    </NDrawerContent>

    <SaveProfileDialog
      :visible="isSaveDialogVisible"
      :params="{
        type: connectionType.value,
        ...(connectionType.value === 'serial' ? serialForm.value : telnetForm.value)
      }"
      @update:visible="isSaveDialogVisible = $event"
      @save="handleSaveProfile"
    />
  </NDrawer>
</template>

<style scoped>
.serial-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px 24px;
}

.telnet-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px 24px;
}

.empty-ports {
  padding: 12px;
  text-align: center;
  color: var(--n-text-color-disabled);
  font-size: 13px;
}

.drawer-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
</style>
