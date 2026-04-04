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
import { useSessionStore } from '../stores/session'
import { tauriInvoke } from '../api/tauri'
import { profileStorage } from '../services/profileStorage'
import SaveProfileDialog from './SaveProfileDialog.vue'
/* ProfileDropdown removed - moved to TabBar */
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
  tabId: string
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
const sessionStore = useSessionStore()

// Profile management
/* profiles ref removed - Load Profile moved to TabBar */
const isSaveDialogVisible = ref(false)

// Connection type
const connectionType = ref<'serial' | 'telnet'>('telnet')

// Connection name
const connectionName = ref<string>('')

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

// Save dialog params - computed to ensure reactivity
const saveDialogParams = computed(() => ({
  type: connectionType.value,
  ...(connectionType.value === 'serial' ? serialForm.value : telnetForm.value)
}))

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

  // Calculate tab title before building params (user input or auto-generated)
  let tabTitle = ''
  if (connectionType.value === 'serial') {
    tabTitle = connectionName.value || serialForm.value.port
  } else {
    tabTitle = connectionName.value || `${telnetForm.value.host}:${telnetForm.value.port}`
  }

  // Build params with name always present
  let params: ConnectionParams
  if (connectionType.value === 'serial') {
    params = { type: 'serial', ...serialForm.value, name: tabTitle }
  } else {
    params = { type: 'telnet', ...telnetForm.value, name: tabTitle }
  }

  try {
    const activeTab = sessionStore.activeTab
    const previousSessionId = activeTab?.sessionId

    await connectionStore.connect(params, props.tabId)

    // Get the sessionId that was just set
    const currentTab = sessionStore.activeTab
    const newSessionId = currentTab?.sessionId

    if (connectionStore.isConnected && newSessionId && newSessionId !== previousSessionId) {
      // Set tab title directly (name is always present)
      sessionStore.renameTab(props.tabId, tabTitle)

      message.success('Connected successfully')
      emit('connected', newSessionId)
      closePanel()
    } else if (connectionStore.hasError) {
      message.error(connectionStore.error || 'Connection failed')
    } else {
      message.warning('Connection state unclear')
    }
  } catch (error) {
    console.error('[ConfigPanel] Connection error:', error)
    message.error(`Connection error: ${error}`)
  }
}

// Handle disconnect
async function handleDisconnect() {
  try {
    await connectionStore.disconnect(props.tabId)
    if (!connectionStore.isConnected) {
      message.success('Disconnected')
      emit('disconnected', props.tabId)
    }
  } catch (error) {
    message.error(`Disconnect error: ${error}`)
  }
}

// Close panel
function closePanel() {
  // Remove focus before closing to prevent button staying in focused state
  document.activeElement?.blur()
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

/* loadProfiles removed - Load Profile moved to TabBar */

// Handle save profile
async function handleSaveProfile(name: string, params: any) {
  try {
    await profileStorage.saveProfile(name, params)
    message.success(`Profile '${name}' saved successfully`)
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

// Load serial ports on mount if panel is visible
onMounted(() => {
  if (props.visible && connectionType.value === 'serial') {
    loadSerialPorts()
  }
})
</script>

<template>
  <NDrawer
    :show="visible"
    placement="top"
    :height="325"
    :mask-closable="!isConnecting"
    :trap-focus="false"
  >
    <NDrawerContent title="Connection Configuration">
      <NForm label-placement="left" label-width="120" :disabled="isConnecting">
        <!-- Connection Type and Name -->
        <div class="type-name-grid">
          <NFormItem label="Type">
            <NRadioGroup v-model:value="connectionType">
              <NRadioButton value="telnet">Telnet</NRadioButton>
              <NRadioButton value="serial">Serial</NRadioButton>
            </NRadioGroup>
          </NFormItem>

          <NFormItem label="Name">
            <NInput
              v-model:value="connectionName"
              placeholder="e.g. My Device"
              clearable
            />
          </NFormItem>
        </div>

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
                placeholder="Enter Host IP address"
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
          <!-- Load Profile moved to TabBar "+" menu -->
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
      :params="saveDialogParams"
      @update:visible="isSaveDialogVisible = $event"
      @save="handleSaveProfile"
    />
  </NDrawer>
</template>

<style scoped>
.type-name-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px 24px;
  margin-bottom: 16px;
}

.type-name-grid :deep(.n-form-item) {
  margin-bottom: 0 !important;
}

.serial-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px 24px;
}

.serial-grid :deep(.n-form-item) {
  margin-bottom: 0 !important;
}

.telnet-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px 24px;
}

.empty-ports {
  padding: 12px;
  text-align: center;
  color: var(--n-text-color-disabled);
  font-size: 13px;
}

/* 移除drawer footer默认的上下padding */
:deep(.n-drawer-footer) {
  padding-top: 10px !important;
  padding-bottom: 10px !important;
}

.drawer-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.drawer-footer :deep(.n-button) {
  height: 33px !important;
}
</style>
