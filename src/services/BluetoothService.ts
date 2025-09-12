/**
 * Bluetooth Service - b00t Platform Integration Pattern
 * 
 * Handles BLE device discovery, connection, and Zephyr protocol communication
 * Follows error boundary requirements from BEVY_PLATFORM_INTEGRATION_PATTERN.md
 */

import { Notify } from 'quasar'
import type { GameEngine } from '../types/GameEngineExtended'

// TypeScript interfaces matching Rust types
export interface DeviceId {
  id: string
}

export interface DeviceInfo {
  id: DeviceId
  name: string
  device_type: BluetoothDeviceType
  rssi: number
  services: string[]
  manufacturer_data?: string
  is_connected: boolean
  last_seen?: number
  battery_level?: number
}

export interface BluetoothDeviceType {
  SmartCollar?: { collar_type: CollarType }
  FeedingStation?: { capacity_ml: number }
  ToyDispenser?: { toy_count: number }
  ActivityTracker?: { sensors: SensorType[] }
  VirtualDevice?: { emulated_type: BluetoothDeviceType }
  TestDevice?: { device_name: string }
  Unknown?: { service_uuid: string }
}

export enum CollarType {
  TrainingCollar = 'TrainingCollar',
  GPSCollar = 'GPSCollar',
  HealthMonitor = 'HealthMonitor',
  SmartTag = 'SmartTag'
}

export enum SensorType {
  Accelerometer = 'Accelerometer',
  Gyroscope = 'Gyroscope',
  HeartRate = 'HeartRate',
  Temperature = 'Temperature',
  GPS = 'GPS',
  Microphone = 'Microphone'
}

// Bluetooth Events following b00t pattern
export interface BluetoothEvent {
  Bluetooth: {
    action: 'scan' | 'connect' | 'disconnect' | 'send_command' | 'enable_virtual' | 'disable_virtual'
    device_id?: string
    device_filter?: BluetoothDeviceFilter
    duration_ms?: number
    command?: ZephyrCommand
    timeout_ms?: number
  }
}

export interface BluetoothDeviceFilter {
  device_types?: BluetoothDeviceType[]
  min_rssi?: number
  service_uuids?: string[]
  manufacturer_ids?: number[]
  name_patterns?: string[]
}

// Zephyr protocol commands
export interface ZephyrCommand {
  GetDeviceInfo?: Record<string, never>
  GetBatteryLevel?: Record<string, never>
  SetLEDState?: { r: number, g: number, b: number }
  Reboot?: Record<string, never>
  CollarCommands?: { command: CollarCommand }
  FeedingCommands?: { command: FeedingCommand }
  TrackerCommands?: { command: TrackerCommand }
  RawCommand?: { 
    service_uuid: string
    characteristic_uuid: string
    data: number[]
  }
}

export interface CollarCommand {
  Vibrate?: { intensity: number, duration_ms: number }
  PlaySound?: { sound_id: number, volume: number }
  SetTrainingMode?: { mode: TrainingMode }
  GetLocation?: Record<string, never>
  GetHealthMetrics?: Record<string, never>
}

export interface FeedingCommand {
  DispenseFood?: { amount_grams: number }
  GetFoodLevel?: Record<string, never>
  SetFeedingSchedule?: { schedule: FeedingSchedule }
  GetFeedingHistory?: Record<string, never>
}

export interface TrackerCommand {
  GetActivityData?: { start_time: number, end_time: number }
  StartActivityTracking?: Record<string, never>
  StopActivityTracking?: Record<string, never>
  GetSensorData?: { sensor: SensorType }
  CalibrateDevice?: Record<string, never>
}

export enum TrainingMode {
  Positive = 'Positive',
  Correction = 'Correction',
  Alert = 'Alert',
  Silent = 'Silent'
}

export interface FeedingSchedule {
  times: string[]  // "HH:MM" format
  portions: number[]  // grams
}

// Bluetooth response types
export interface ZephyrResponse {
  DeviceInfo?: { 
    firmware_version: string
    hardware_version: string
    serial_number: string
  }
  BatteryLevel?: { percentage: number, voltage_mv: number }
  LocationData?: { 
    latitude: number
    longitude: number
    accuracy_m: number
  }
  SensorData?: {
    sensor: SensorType
    values: number[]
    timestamp: number
  }
  FoodLevel?: { current_grams: number, capacity_grams: number }
  Success?: Record<string, never>
  Error?: { code: number, message: string }
}

// Error types
export class BluetoothError extends Error {
  constructor(
    message: string,
    public code?: string,
    public deviceId?: string
  ) {
    super(message)
    this.name = 'BluetoothError'
  }
}

/**
 * Bluetooth Service Implementation
 * 
 * ✅ COMPLIANT with b00t pattern:
 * - Error boundaries for all async operations
 * - Generic error/crash logging
 * - Error metrics tracking
 * - Structured logging with context
 * - No silent failures
 */
export class BluetoothService {
  private gameEngine: GameEngine | null = null
  private isScanning = false
  private connectedDevices = new Map<string, DeviceInfo>()
  private discoveredDevices = new Map<string, DeviceInfo>()
  private virtualNetworkEnabled = false
  private errorCounts = new Map<string, number>()
  private readonly maxRetries = 3

  // ✅ MANDATORY: Generic error/crash logger
  private logError(context: string, error: Error, additionalData?: unknown) {
    console.error(`🔥 b00t platform integration error [${context}]:`, {
      error: error.message,
      stack: error.stack,
      timestamp: new Date().toISOString(),
      context,
      additionalData
    })
  }

  // ✅ REQUIRED: Error metrics tracking
  private incrementErrorCount(context: string) {
    const currentCount = this.errorCounts.get(context) || 0
    this.errorCounts.set(context, currentCount + 1)
    console.log(`📊 Error count for ${context}: ${currentCount + 1}`)
  }

  // ✅ REQUIRED: Error boundary wrapper
  private createErrorBoundary<T>(
    handler: () => Promise<T>,
    context: string
  ): Promise<T | null> {
    return handler().catch(error => {
      this.logError(context, error)
      this.incrementErrorCount(`${context}_failed`)
      
      // Optional: Error telemetry
      this.reportErrorTelemetry(context, error)
      
      return null
    })
  }

  private reportErrorTelemetry(context: string, error: Error) {
    // Optional telemetry reporting
    console.log(`📊 Bluetooth telemetry: ${context} failed`, { 
      error: error.message,
      deviceCount: this.connectedDevices.size
    })
  }

  /**
   * Initialize Bluetooth service with game engine reference
   */
  async initialize(gameEngine: GameEngine): Promise<void> {
    await this.createErrorBoundary(() => Promise.resolve().then(() => {
      this.gameEngine = gameEngine
      console.log('🔵 BluetoothService initialized')
      
      // Check for browser Bluetooth support
      if (typeof navigator !== 'undefined' && 'bluetooth' in navigator) {
        console.log('🔵 Browser Bluetooth API available')
      } else {
        console.log('🔵 Browser Bluetooth API not available - using virtual network only')
      }
    }), 'bluetooth_initialize')
  }

  /**
   * Start scanning for Bluetooth devices
   */
  async startScan(options?: {
    duration_ms?: number
    device_filter?: BluetoothDeviceFilter
  }): Promise<string | null> {
    return await this.createErrorBoundary(() => Promise.resolve().then(() => {
      if (this.isScanning) {
        throw new BluetoothError('Scan already in progress')
      }

      console.log('🔵 Starting Bluetooth scan', options)
      
      if (this.gameEngine) {
        const requestId = this.gameEngine.start_bluetooth_scan(options?.duration_ms)
        this.isScanning = true
        
        // Set scan timeout
        if (options?.duration_ms) {
          setTimeout(() => {
            this.stopScan().catch(err => {
              console.warn('Auto-stop scan failed:', err)
            })
          }, options.duration_ms)
        }
        
        return requestId
      } else {
        throw new BluetoothError('Game engine not initialized')
      }
    }), 'bluetooth_start_scan')
  }

  /**
   * Stop scanning for devices
   */
  async stopScan(): Promise<void> {
    await this.createErrorBoundary(() => Promise.resolve().then(() => {
      if (!this.isScanning) {
        return
      }

      console.log('🔵 Stopping Bluetooth scan')
      
      if (this.gameEngine) {
        this.gameEngine.stop_bluetooth_scan()
        this.isScanning = false
      }
    }), 'bluetooth_stop_scan')
  }

  /**
   * Connect to a Bluetooth device
   */
  async connectDevice(deviceId: string): Promise<string | null> {
    return await this.createErrorBoundary(() => Promise.resolve().then(() => {
      console.log(`🔵 Connecting to device: ${deviceId}`)
      
      if (this.gameEngine) {
        const requestId = this.gameEngine.connect_bluetooth_device(deviceId)
        
        // Simulate connection success for virtual devices
        if (this.virtualNetworkEnabled && deviceId.startsWith('virtual_')) {
          setTimeout(() => {
            this.onDeviceConnected(deviceId)
          }, 500) // Simulate connection delay
        }
        
        return requestId
      } else {
        throw new BluetoothError('Game engine not initialized')
      }
    }), 'bluetooth_connect_device')
  }

  /**
   * Disconnect from a Bluetooth device
   */
  async disconnectDevice(deviceId: string): Promise<void> {
    await this.createErrorBoundary(() => Promise.resolve().then(() => {
      console.log(`🔵 Disconnecting from device: ${deviceId}`)
      
      if (this.gameEngine) {
        this.gameEngine.disconnect_bluetooth_device(deviceId)
        this.connectedDevices.delete(deviceId)
      }
    }), 'bluetooth_disconnect_device')
  }

  /**
   * Send a command to a Bluetooth device using Zephyr protocol
   */
  async sendCommand(
    deviceId: string, 
    command: ZephyrCommand,
    timeoutMs: number = 5000
  ): Promise<string | null> {
    return await this.createErrorBoundary(() => Promise.resolve().then(() => {
      console.log(`🔵 Sending command to ${deviceId} (timeout: ${timeoutMs}ms):`, command)
      
      if (!this.connectedDevices.has(deviceId)) {
        throw new BluetoothError(`Device ${deviceId} not connected`, 'NOT_CONNECTED', deviceId)
      }
      
      if (this.gameEngine) {
        const commandJson = JSON.stringify(command)
        const requestId = this.gameEngine.send_bluetooth_command(deviceId, commandJson)
        
        return requestId
      } else {
        throw new BluetoothError('Game engine not initialized')
      }
    }), 'bluetooth_send_command')
  }

  /**
   * Enable virtual Bluetooth network for testing
   */
  async enableVirtualNetwork(): Promise<void> {
    await this.createErrorBoundary(() => Promise.resolve().then(() => {
      console.log('🔵 Enabling virtual Bluetooth network')
      
      if (this.gameEngine) {
        this.gameEngine.enable_virtual_bluetooth()
        this.virtualNetworkEnabled = true
        
        // Simulate device discovery after enabling
        setTimeout(() => {
          this.simulateVirtualDeviceDiscovery()
        }, 1000)
        
        Notify.create({
          type: 'positive',
          message: '🔵 Virtual Bluetooth network enabled',
          position: 'top'
        })
      }
    }), 'bluetooth_enable_virtual')
  }

  /**
   * Disable virtual Bluetooth network
   */
  async disableVirtualNetwork(): Promise<void> {
    await this.createErrorBoundary(() => Promise.resolve().then(() => {
      console.log('🔵 Disabling virtual Bluetooth network')
      
      if (this.gameEngine) {
        this.gameEngine.disable_virtual_bluetooth()
        this.virtualNetworkEnabled = false
        
        // Clear virtual devices
        for (const [deviceId] of this.discoveredDevices.entries()) {
          if (deviceId.startsWith('virtual_')) {
            this.discoveredDevices.delete(deviceId)
            this.connectedDevices.delete(deviceId)
          }
        }
        
        Notify.create({
          type: 'info',
          message: '🔵 Virtual Bluetooth network disabled',
          position: 'top'
        })
      }
    }), 'bluetooth_disable_virtual')
  }

  /**
   * Get current Bluetooth status
   */
  getStatus() {
    if (this.gameEngine) {
      return this.gameEngine.get_bluetooth_status()
    }
    
    return {
      scanning: this.isScanning,
      connectedDevices: this.connectedDevices.size,
      discoveredDevices: this.discoveredDevices.size,
      virtualNetworkEnabled: this.virtualNetworkEnabled,
      errorCounts: Object.fromEntries(this.errorCounts)
    }
  }

  /**
   * Get discovered devices
   */
  getDiscoveredDevices(): DeviceInfo[] {
    return Array.from(this.discoveredDevices.values())
  }

  /**
   * Get connected devices
   */
  getConnectedDevices(): DeviceInfo[] {
    return Array.from(this.connectedDevices.values())
  }

  // Event handlers (called by game engine or WebBluetooth API)
  private onDeviceDiscovered(device: DeviceInfo) {
    this.discoveredDevices.set(device.id.id, device)
    console.log(`🔵 Device discovered: ${device.name} (${device.id.id})`)
    
    Notify.create({
      type: 'info',
      message: `📱 Device found: ${device.name}`,
      timeout: 3000,
      position: 'top-right'
    })
  }

  private onDeviceConnected(deviceId: string) {
    const device = this.discoveredDevices.get(deviceId)
    if (device) {
      device.is_connected = true
      this.connectedDevices.set(deviceId, device)
      console.log(`🔵 Device connected: ${deviceId}`)
      
      Notify.create({
        type: 'positive',
        message: `✅ Connected to ${device.name}`,
        timeout: 3000,
        position: 'top-right'
      })
    }
  }

  private onDeviceDisconnected(deviceId: string) {
    const device = this.connectedDevices.get(deviceId)
    if (device) {
      device.is_connected = false
      this.connectedDevices.delete(deviceId)
      console.log(`🔵 Device disconnected: ${deviceId}`)
      
      Notify.create({
        type: 'warning',
        message: `⚠️ Disconnected from ${device.name}`,
        timeout: 3000,
        position: 'top-right'
      })
    }
  }

  private simulateVirtualDeviceDiscovery() {
    // Simulate virtual device discovery
    const virtualDevices: DeviceInfo[] = [
      {
        id: { id: 'virtual_collar_001' },
        name: 'Test Smart Collar',
        device_type: { SmartCollar: { collar_type: CollarType.TrainingCollar } },
        rssi: -45,
        services: ['uuid_collar_service'],
        manufacturer_data: 'ZephyrCollar_v2.1',
        is_connected: false,
        last_seen: Date.now(),
        battery_level: 85
      },
      {
        id: { id: 'virtual_feeder_001' },
        name: 'Test Feeding Station',
        device_type: { FeedingStation: { capacity_ml: 2000 } },
        rssi: -38,
        services: ['uuid_feeder_service'],
        manufacturer_data: 'ZephyrFeeder_v1.5',
        is_connected: false,
        last_seen: Date.now(),
        battery_level: 92
      }
    ]

    for (const device of virtualDevices) {
      this.onDeviceDiscovered(device)
    }
  }

  /**
   * Test commands for virtual devices
   */
  async testVirtualCollar(): Promise<void> {
    await this.createErrorBoundary(async () => {
      const deviceId = 'virtual_collar_001'
      
      // Test battery command
      await this.sendCommand(deviceId, { GetBatteryLevel: {} })
      
      // Test vibration
      await this.sendCommand(deviceId, {
        CollarCommands: {
          command: {
            Vibrate: { intensity: 50, duration_ms: 1000 }
          }
        }
      })
      
      Notify.create({
        type: 'positive',
        message: '🔵 Virtual collar test commands sent',
        position: 'top'
      })
    }, 'test_virtual_collar')
  }

  async testVirtualFeeder(): Promise<void> {
    await this.createErrorBoundary(async () => {
      const deviceId = 'virtual_feeder_001'
      
      // Test food level
      await this.sendCommand(deviceId, {
        FeedingCommands: {
          command: { GetFoodLevel: {} }
        }
      })
      
      // Test dispense food
      await this.sendCommand(deviceId, {
        FeedingCommands: {
          command: {
            DispenseFood: { amount_grams: 50 }
          }
        }
      })
      
      Notify.create({
        type: 'positive',
        message: '🔵 Virtual feeder test commands sent',
        position: 'top'
      })
    }, 'test_virtual_feeder')
  }
}

// Export singleton instance
export const bluetoothService = new BluetoothService()