/**
 * Extended GameEngine type with Bluetooth methods
 */
import type { GameEngine as BaseGameEngine } from '../../game-engine/pkg/app4dog_game_engine'

export interface GameEngine extends BaseGameEngine {
  // Bluetooth methods
  start_bluetooth_scan(duration_ms?: number): string
  stop_bluetooth_scan(): void
  connect_bluetooth_device(device_id: string): string
  disconnect_bluetooth_device(device_id: string): void
  send_bluetooth_command(device_id: string, command_json: string): string
  enable_virtual_bluetooth(): void
  disable_virtual_bluetooth(): void
  get_bluetooth_status(): {
    scanning: boolean
    connectedDevices: number
    discoveredDevices: number
    virtualNetworkEnabled: boolean
  }
}