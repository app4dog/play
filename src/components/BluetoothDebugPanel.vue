<template>
  <q-card class="bluetooth-debug-panel">
    <q-card-section class="bg-blue-1">
      <div class="row items-center no-wrap">
        <q-icon name="bluetooth" color="blue" size="md" class="q-mr-sm" />
        <div class="text-h6">🔵 Bluetooth Debug Panel</div>
        <q-space />
        <q-chip 
          :color="status.virtualNetworkEnabled ? 'green' : 'grey'" 
          text-color="white" 
          dense
        >
          {{ status.virtualNetworkEnabled ? 'Virtual Network' : 'Real Network' }}
        </q-chip>
      </div>
    </q-card-section>

    <q-separator />

    <!-- Status Section -->
    <q-card-section>
      <div class="text-subtitle2 q-mb-sm">Status</div>
      <div class="row q-gutter-sm">
        <q-chip 
          :color="status.scanning ? 'orange' : 'grey'" 
          text-color="white" 
          dense
        >
          Scanning {{ status.scanning ? 'ON' : 'OFF' }}
        </q-chip>
        <q-chip 
          :color="connectedDevices.length > 0 ? 'green' : 'grey'" 
          text-color="white" 
          dense
        >
          Connected: {{ connectedDevices.length }}
        </q-chip>
        <q-chip 
          :color="discoveredDevices.length > 0 ? 'blue' : 'grey'" 
          text-color="white" 
          dense
        >
          Discovered: {{ discoveredDevices.length }}
        </q-chip>
      </div>
    </q-card-section>

    <q-separator />

    <!-- Controls Section -->
    <q-card-section>
      <div class="text-subtitle2 q-mb-sm">Network Controls</div>
      <div class="row q-gutter-sm q-mb-md">
        <q-btn 
          color="primary" 
          label="Enable Virtual Network" 
          size="sm" 
          @click="enableVirtualNetwork"
          :disable="status.virtualNetworkEnabled"
        />
        <q-btn 
          color="grey" 
          label="Disable Virtual Network" 
          size="sm" 
          @click="disableVirtualNetwork"
          :disable="!status.virtualNetworkEnabled"
        />
      </div>

      <div class="text-subtitle2 q-mb-sm">Scan Controls</div>
      <div class="row q-gutter-sm">
        <q-btn 
          color="orange" 
          label="Start Scan" 
          size="sm" 
          @click="startScan"
          :disable="status.scanning"
        />
        <q-btn 
          color="grey" 
          label="Stop Scan" 
          size="sm" 
          @click="stopScan"
          :disable="!status.scanning"
        />
        <q-btn 
          color="red" 
          label="Clear All" 
          size="sm" 
          @click="clearDevices"
        />
      </div>
    </q-card-section>

    <q-separator />

    <!-- Discovered Devices Section -->
    <q-card-section>
      <div class="text-subtitle2 q-mb-sm">
        Discovered Devices ({{ discoveredDevices.length }})
      </div>
      
      <div v-if="discoveredDevices.length === 0" class="text-grey-6 text-center q-py-md">
        No devices discovered. Start a scan to find devices.
      </div>
      
      <div v-else>
        <q-list separator>
          <q-item 
            v-for="device in discoveredDevices" 
            :key="device.id.id"
            class="device-item"
          >
            <q-item-section avatar>
              <q-avatar 
                :color="getDeviceColor(device.device_type)" 
                text-color="white" 
                size="sm"
              >
                <q-icon :name="getDeviceIcon(device.device_type)" />
              </q-avatar>
            </q-item-section>

            <q-item-section>
              <q-item-label>{{ device.name }}</q-item-label>
              <q-item-label caption>
                {{ device.id.id }} | {{ getDeviceTypeString(device.device_type) }}
              </q-item-label>
              <q-item-label caption class="text-grey-6">
                RSSI: {{ device.rssi }}dBm
                <span v-if="device.battery_level">, Battery: {{ device.battery_level }}%</span>
              </q-item-label>
            </q-item-section>

            <q-item-section side>
              <div class="column q-gutter-xs">
                <q-btn 
                  v-if="!device.is_connected"
                  color="green" 
                  label="Connect" 
                  size="xs" 
                  @click="connectDevice(device.id.id)"
                />
                <q-btn 
                  v-else
                  color="red" 
                  label="Disconnect" 
                  size="xs" 
                  @click="disconnectDevice(device.id.id)"
                />
                
                <q-btn 
                  color="blue" 
                  label="Test" 
                  size="xs" 
                  @click="testDevice(device)"
                  :disable="!device.is_connected"
                />
              </div>
            </q-item-section>
          </q-item>
        </q-list>
      </div>
    </q-card-section>

    <q-separator />

    <!-- Device Testing Section -->
    <q-card-section v-if="connectedDevices.length > 0">
      <div class="text-subtitle2 q-mb-sm">Device Testing</div>
      
      <div class="row q-gutter-sm q-mb-md">
        <q-btn 
          color="purple" 
          label="Test Virtual Collar" 
          size="sm" 
          @click="testVirtualCollar"
        />
        <q-btn 
          color="green" 
          label="Test Virtual Feeder" 
          size="sm" 
          @click="testVirtualFeeder"
        />
      </div>

      <!-- Custom Command Testing -->
      <div class="text-subtitle2 q-mb-sm">Custom Commands</div>
      <div class="row q-gutter-sm items-center">
        <q-select
          v-model="selectedDevice"
          :options="connectedDeviceOptions"
          option-label="name"
          option-value="id"
          label="Device"
          style="min-width: 200px"
          dense
        />
        <q-select
          v-model="selectedCommand"
          :options="commandOptions"
          label="Command"
          style="min-width: 150px"
          dense
        />
        <q-btn 
          color="indigo" 
          label="Send" 
          size="sm" 
          @click="sendCustomCommand"
          :disable="!selectedDevice || !selectedCommand"
        />
      </div>
    </q-card-section>

    <q-separator />

    <!-- Response Log Section -->
    <q-card-section>
      <div class="row items-center q-mb-sm">
        <div class="text-subtitle2">Response Log</div>
        <q-space />
        <q-btn 
          flat 
          dense 
          icon="clear" 
          size="sm" 
          @click="clearLog"
          class="text-grey-6"
        />
      </div>
      
      <div class="response-log">
        <div 
          v-for="(logEntry, index) in responseLog" 
          :key="index"
          class="log-entry text-caption"
          :class="`log-${logEntry.type}`"
        >
          <span class="log-timestamp">{{ formatTimestamp(logEntry.timestamp) }}</span>
          <span class="log-message">{{ logEntry.message }}</span>
        </div>
        
        <div v-if="responseLog.length === 0" class="text-grey-6 text-center q-py-md">
          No responses logged yet. Send commands to see responses here.
        </div>
      </div>
    </q-card-section>
  </q-card>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { bluetoothService, type DeviceInfo, type BluetoothDeviceType } from '../services/BluetoothService'

// Reactive state
const status = ref({
  scanning: false,
  connectedDevices: 0,
  discoveredDevices: 0,
  virtualNetworkEnabled: false
})

const discoveredDevices = ref<DeviceInfo[]>([])
const connectedDevices = ref<DeviceInfo[]>([])
const selectedDevice = ref<DeviceInfo | null>(null)
const selectedCommand = ref<string>('')

const responseLog = ref<Array<{
  timestamp: Date
  type: 'info' | 'success' | 'error' | 'command'
  message: string
}>>([])

// Command options for testing
const commandOptions = [
  'battery',
  'info',
  'vibrate',
  'get_food_level',
  'dispense_food',
  'get_activity'
]

// Computed properties
const connectedDeviceOptions = computed(() => 
  connectedDevices.value.map(device => ({
    ...device,
    id: device.id.id,
    name: device.name
  }))
)

// Methods
const logMessage = (type: 'info' | 'success' | 'error' | 'command', message: string) => {
  responseLog.value.unshift({
    timestamp: new Date(),
    type,
    message
  })
  
  // Keep only last 50 entries
  if (responseLog.value.length > 50) {
    responseLog.value = responseLog.value.slice(0, 50)
  }
}

// Helper function to format errors for template literals
const formatError = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message
  }
  if (typeof error === 'string') {
    return error
  }
  return String(error)
}

const enableVirtualNetwork = async () => {
  try {
    await bluetoothService.enableVirtualNetwork()
    status.value.virtualNetworkEnabled = true
    logMessage('success', '✅ Virtual Bluetooth network enabled')
    
    // Auto-refresh devices after enabling
    setTimeout(() => {
      refreshDevices()
    }, 1000)
  } catch (error) {
    logMessage('error', `❌ Failed to enable virtual network: ${formatError(error)}`)
  }
}

const disableVirtualNetwork = async () => {
  try {
    await bluetoothService.disableVirtualNetwork()
    status.value.virtualNetworkEnabled = false
    discoveredDevices.value = []
    connectedDevices.value = []
    logMessage('info', 'ℹ️ Virtual Bluetooth network disabled')
  } catch (error) {
    logMessage('error', `❌ Failed to disable virtual network: ${formatError(error)}`)
  }
}

const startScan = async () => {
  try {
    const requestId = await bluetoothService.startScan({ duration_ms: 10000 })
    status.value.scanning = true
    logMessage('info', `🔍 Started Bluetooth scan (${requestId})`)
    
    // Auto-stop after 10 seconds
    setTimeout(() => {
      stopScan().catch(err => {
        console.warn('Auto-stop scan failed:', err)
      })
    }, 10000)
  } catch (error) {
    logMessage('error', `❌ Failed to start scan: ${formatError(error)}`)
  }
}

const stopScan = async () => {
  try {
    await bluetoothService.stopScan()
    status.value.scanning = false
    logMessage('info', '⏹️ Stopped Bluetooth scan')
  } catch (error) {
    logMessage('error', `❌ Failed to stop scan: ${formatError(error)}`)
  }
}

const clearDevices = () => {
  discoveredDevices.value = []
  connectedDevices.value = []
  selectedDevice.value = null
  logMessage('info', '🗑️ Cleared all devices')
}

const connectDevice = async (deviceId: string) => {
  try {
    const requestId = await bluetoothService.connectDevice(deviceId)
    logMessage('info', `🔗 Connecting to device ${deviceId} (${requestId})`)
  } catch (error) {
    logMessage('error', `❌ Failed to connect to ${deviceId}: ${formatError(error)}`)
  }
}

const disconnectDevice = async (deviceId: string) => {
  try {
    await bluetoothService.disconnectDevice(deviceId)
    logMessage('info', `🔌 Disconnected from device ${deviceId}`)
  } catch (error) {
    logMessage('error', `❌ Failed to disconnect from ${deviceId}: ${formatError(error)}`)
  }
}

const testDevice = async (device: DeviceInfo) => {
  const deviceTypeStr = getDeviceTypeString(device.device_type)
  
  if (deviceTypeStr.includes('Collar')) {
    await testVirtualCollar()
  } else if (deviceTypeStr.includes('Feeder')) {
    await testVirtualFeeder()
  } else {
    // Generic test
    await sendCommand(device.id.id, 'info')
  }
}

const testVirtualCollar = async () => {
  try {
    await bluetoothService.testVirtualCollar()
    logMessage('command', '🎮 Virtual collar test commands sent')
  } catch (error) {
    logMessage('error', `❌ Virtual collar test failed: ${formatError(error)}`)
  }
}

const testVirtualFeeder = async () => {
  try {
    await bluetoothService.testVirtualFeeder()
    logMessage('command', '🍽️ Virtual feeder test commands sent')
  } catch (error) {
    logMessage('error', `❌ Virtual feeder test failed: ${formatError(error)}`)
  }
}

const sendCustomCommand = async () => {
  if (!selectedDevice.value || !selectedCommand.value) return
  
  await sendCommand(selectedDevice.value.id.id, selectedCommand.value)
}

const sendCommand = async (deviceId: string, command: string) => {
  try {
    const requestId = await bluetoothService.sendCommand(
      deviceId,
      { [command]: {} as Record<string, never> },
      5000
    )
    logMessage('command', `📤 Sent "${command}" to ${deviceId} (${requestId})`)
  } catch (error) {
    logMessage('error', `❌ Failed to send command "${command}": ${formatError(error)}`)
  }
}

const refreshDevices = () => {
  discoveredDevices.value = bluetoothService.getDiscoveredDevices()
  connectedDevices.value = bluetoothService.getConnectedDevices()
  
  const btStatus = bluetoothService.getStatus()
  status.value = {
    scanning: btStatus.scanning || false,
    connectedDevices: btStatus.connectedDevices || 0,
    discoveredDevices: btStatus.discoveredDevices || 0,
    virtualNetworkEnabled: btStatus.virtualNetworkEnabled || false
  }
}

const clearLog = () => {
  responseLog.value = []
  logMessage('info', '🗑️ Response log cleared')
}

// Utility functions  
const getDeviceColor = (deviceType: BluetoothDeviceType): string => {
  if (deviceType.SmartCollar) return 'purple'
  if (deviceType.FeedingStation) return 'green'
  if (deviceType.ToyDispenser) return 'orange'
  if (deviceType.ActivityTracker) return 'blue'
  if (deviceType.VirtualDevice) return 'indigo'
  if (deviceType.TestDevice) return 'cyan'
  return 'grey'
}

const getDeviceIcon = (deviceType: BluetoothDeviceType): string => {
  if (deviceType.SmartCollar) return 'pets'
  if (deviceType.FeedingStation) return 'restaurant'
  if (deviceType.ToyDispenser) return 'sports_esports'
  if (deviceType.ActivityTracker) return 'fitness_center'
  if (deviceType.VirtualDevice) return 'computer'
  if (deviceType.TestDevice) return 'bug_report'
  return 'device_unknown'
}

const getDeviceTypeString = (deviceType: BluetoothDeviceType): string => {
  if (deviceType.SmartCollar) return `Smart Collar (${deviceType.SmartCollar.collar_type})`
  if (deviceType.FeedingStation) return `Feeding Station (${deviceType.FeedingStation.capacity_ml}ml)`
  if (deviceType.ToyDispenser) return `Toy Dispenser (${deviceType.ToyDispenser.toy_count} toys)`
  if (deviceType.ActivityTracker) return `Activity Tracker`
  if (deviceType.VirtualDevice) return `Virtual Device`
  if (deviceType.TestDevice) return `Test Device (${deviceType.TestDevice.device_name})`
  if (deviceType.Unknown) return `Unknown (${deviceType.Unknown.service_uuid})`
  return 'Unknown Device'
}

const formatTimestamp = (timestamp: Date): string => {
  return timestamp.toLocaleTimeString('en-US', { 
    hour12: false, 
    hour: '2-digit', 
    minute: '2-digit', 
    second: '2-digit' 
  })
}

// Lifecycle
onMounted(() => {
  // Initialize Bluetooth service (this would typically be done in main app)
  // For now, just refresh the current state
  refreshDevices()
  
  // Set up periodic refresh
  setInterval(refreshDevices, 2000) // Refresh every 2 seconds
  
  logMessage('info', '🔵 Bluetooth Debug Panel initialized')
})
</script>

<style scoped lang="scss">
.bluetooth-debug-panel {
  min-width: 400px;
  max-width: 600px;
}

.device-item {
  border-left: 3px solid transparent;
  transition: border-color 0.3s ease;
  
  &:hover {
    border-left-color: #1976d2;
  }
}

.response-log {
  max-height: 200px;
  overflow-y: auto;
  background: #fafafa;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  padding: 8px;
  font-family: 'Courier New', monospace;
}

.log-entry {
  padding: 2px 0;
  display: flex;
  
  &.log-info .log-timestamp {
    color: #2196f3;
  }
  
  &.log-success .log-timestamp {
    color: #4caf50;
  }
  
  &.log-error .log-timestamp {
    color: #f44336;
  }
  
  &.log-command .log-timestamp {
    color: #9c27b0;
  }
}

.log-timestamp {
  min-width: 70px;
  font-weight: bold;
  margin-right: 8px;
}

.log-message {
  flex: 1;
}
</style>