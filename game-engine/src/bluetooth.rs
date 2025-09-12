use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use web_sys::console;
use js_sys;

/// Component to mark entities that should explode when despawned
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub device_type: BluetoothLEDeviceType,
    pub rssi: i16,
    pub services: Vec<String>,
    pub manufacturer_data: Option<String>,
    pub is_connected: bool,
    pub last_seen: Option<f64>,
    pub battery_level: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BluetoothLEDeviceType {
    // Pet training devices
    SmartCollar { collar_type: CollarType },
    FeedingStation { capacity_ml: u32 },
    ToyDispenser { toy_count: u8 },
    ActivityTracker { sensors: Vec<SensorType> },
    
    // Testing/Virtual devices
    VirtualDevice { emulated_type: Box<BluetoothLEDeviceType> },
    TestDevice { device_name: String },
    
    // Generic/Unknown
    Unknown { service_uuid: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollarType {
    TrainingCollar,
    GPSCollar,
    HealthMonitor,
    SmartTag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SensorType {
    Accelerometer,
    Gyroscope,
    HeartRate,
    Temperature,
    GPS,
    Microphone,
}

/// BluetoothLE connection states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BluetoothLEConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Pairing,
    Paired,
    Error(String),
}

/// Resource holding BluetoothLE state following b00t pattern
#[derive(Resource)]
pub struct BluetoothLEManager {
    pub scanning: bool,
    pub connected_devices: HashMap<DeviceId, DeviceInfo>,
    pub discovered_devices: HashMap<DeviceId, DeviceInfo>,
    pub pending_requests: HashMap<String, BluetoothLERequest>,
    pub connection_states: HashMap<DeviceId, BluetoothLEConnectionState>,
    
    // Error handling
    pub last_error: Option<BluetoothLEError>,
    pub error_count: u32,
    pub retry_backoff: Duration,
    
    // Virtual network for testing
    pub virtual_network_enabled: bool,
    pub virtual_devices: HashMap<DeviceId, VirtualDevice>,
    pub virtual_command_log: Vec<VirtualCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualDevice {
    pub info: DeviceInfo,
    pub command_handlers: HashMap<String, VirtualCommandHandler>,
    pub state: HashMap<String, serde_json::Value>,
    pub auto_responses: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualCommandHandler {
    pub command_pattern: String,
    pub response_template: String,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualCommand {
    pub timestamp: f64,
    pub device_id: DeviceId,
    pub command: String,
    pub response: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BluetoothLEError {
    AdapterNotFound,
    DeviceNotFound { device_id: String },
    ConnectionFailed { reason: String },
    PairingFailed { reason: String },
    ServiceDiscoveryFailed,
    CommandTimeout { command: String },
    PlatformError { message: String },
}

impl std::fmt::Display for BluetoothLEError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BluetoothLEError::AdapterNotFound => write!(f, "BluetoothLE adapter not found"),
            BluetoothLEError::DeviceNotFound { device_id } => write!(f, "Device not found: {}", device_id),
            BluetoothLEError::ConnectionFailed { reason } => write!(f, "Connection failed: {}", reason),
            BluetoothLEError::PairingFailed { reason } => write!(f, "Pairing failed: {}", reason),
            BluetoothLEError::ServiceDiscoveryFailed => write!(f, "Service discovery failed"),
            BluetoothLEError::CommandTimeout { command } => write!(f, "Command timeout: {}", command),
            BluetoothLEError::PlatformError { message } => write!(f, "Platform error: {}", message),
        }
    }
}

impl std::error::Error for BluetoothLEError {}

/// Events for BluetoothLE requests (Bevy -> Platform)
#[derive(Event, Debug, Clone, Serialize, Deserialize)]
pub enum BluetoothLERequest {
    // Device discovery
    StartScan { 
        duration_ms: Option<u32>, 
        device_filter: Option<BluetoothLEDeviceFilter> 
    },
    StopScan,
    
    // Connection management
    Connect { device_id: DeviceId },
    Disconnect { device_id: DeviceId },
    Pair { device_id: DeviceId, pin: Option<String> },
    
    // Device communication (Zephyr protocol)
    SendCommand { 
        device_id: DeviceId, 
        command: ZephyrCommand,
        timeout_ms: Option<u32>,
    },
    
    // Virtual network (testing)
    EnableVirtualNetwork,
    DisableVirtualNetwork,
    RegisterVirtualDevice { device: VirtualDevice },
    RemoveVirtualDevice { device_id: DeviceId },
    SimulateDeviceCommand { device_id: DeviceId, command: String },
}

#[derive(Event, Debug, Clone, Serialize, Deserialize)]
pub enum BluetoothLEResponse {
    // Scan results
    ScanStarted,
    ScanStopped,
    DeviceDiscovered { device: DeviceInfo },
    
    // Connection events
    Connected { device_id: DeviceId },
    Disconnected { device_id: DeviceId, reason: Option<String> },
    Paired { device_id: DeviceId },
    PairingFailed { device_id: DeviceId, error: String },
    
    // Command responses (from Zephyr devices)
    CommandResponse { 
        device_id: DeviceId, 
        command: ZephyrCommand,
        response: ZephyrResponse,
        latency_ms: u32,
    },
    CommandFailed { 
        device_id: DeviceId, 
        command: ZephyrCommand, 
        error: String 
    },
    
    // Virtual network responses
    VirtualNetworkEnabled,
    VirtualNetworkDisabled,
    VirtualDeviceRegistered { device_id: DeviceId },
    VirtualCommandExecuted { 
        device_id: DeviceId, 
        command: String, 
        response: String 
    },
    
    // Errors
    Error { error: BluetoothLEError },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothLEDeviceFilter {
    pub device_types: Option<Vec<BluetoothLEDeviceType>>,
    pub min_rssi: Option<i16>,
    pub service_uuids: Option<Vec<String>>,
    pub manufacturer_ids: Option<Vec<u16>>,
    pub name_patterns: Option<Vec<String>>,
}

/// Zephyr device communication protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZephyrCommand {
    // Universal commands (all devices)
    GetDeviceInfo,
    GetBatteryLevel,
    SetLEDState { r: u8, g: u8, b: u8 },
    Reboot,
    
    // Smart collar specific
    CollarCommands {
        command: CollarCommand,
    },
    
    // Feeding station specific
    FeedingCommands {
        command: FeedingCommand,
    },
    
    // Activity tracker specific
    TrackerCommands {
        command: TrackerCommand,
    },
    
    // Custom/Raw command
    RawCommand { 
        service_uuid: String, 
        characteristic_uuid: String, 
        data: Vec<u8> 
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollarCommand {
    Vibrate { intensity: u8, duration_ms: u32 },
    PlaySound { sound_id: u8, volume: u8 },
    SetTrainingMode { mode: TrainingMode },
    GetLocation,
    GetHealthMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedingCommand {
    DispenseFood { amount_grams: u32 },
    GetFoodLevel,
    SetFeedingSchedule { schedule: FeedingSchedule },
    GetFeedingHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackerCommand {
    GetActivityData { start_time: u64, end_time: u64 },
    StartActivityTracking,
    StopActivityTracking,
    GetSensorData { sensor: SensorType },
    CalibrateDevice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrainingMode {
    Positive,
    Correction,
    Alert,
    Silent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedingSchedule {
    pub times: Vec<String>, // "HH:MM" format
    pub portions: Vec<u32>, // grams
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZephyrResponse {
    DeviceInfo { 
        firmware_version: String, 
        hardware_version: String, 
        serial_number: String 
    },
    BatteryLevel { percentage: u8, voltage_mv: u16 },
    LocationData { 
        latitude: f64, 
        longitude: f64, 
        accuracy_m: f32 
    },
    SensorData { 
        sensor: SensorType, 
        values: Vec<f32>, 
        timestamp: u64 
    },
    FoodLevel { current_grams: u32, capacity_grams: u32 },
    Success,
    Error { code: u16, message: String },
}

impl Default for BluetoothLEManager {
    fn default() -> Self {
        Self {
            scanning: false,
            connected_devices: HashMap::new(),
            discovered_devices: HashMap::new(),
            pending_requests: HashMap::new(),
            connection_states: HashMap::new(),
            last_error: None,
            error_count: 0,
            retry_backoff: Duration::from_millis(100),
            virtual_network_enabled: false,
            virtual_devices: HashMap::new(),
            virtual_command_log: Vec::new(),
        }
    }
}

impl BluetoothLEManager {
    /// Handle errors following b00t pattern
    pub fn handle_error(&mut self, error: BluetoothLEError) {
        console::log_1(&format!("🔵 BluetoothLE error: {}", error).into());
        
        self.last_error = Some(error.clone());
        self.error_count += 1;
        
        // Exponential backoff
        self.retry_backoff = Duration::from_millis(
            100 * 2_u64.pow(self.error_count.min(10))
        );
    }
    
    pub fn should_retry(&self) -> bool {
        self.error_count < 3 && 
        self.retry_backoff.as_millis() < 30000 // Max 30s backoff
    }
    
    /// Register a virtual device for testing
    pub fn register_virtual_device(&mut self, device: VirtualDevice) {
        let device_id = device.info.id.clone();
        console::log_1(&format!("🔵 Registering virtual device: {:?}", device_id).into());
        
        self.virtual_devices.insert(device_id.clone(), device.clone());
        self.discovered_devices.insert(device_id, device.info);
    }
    
    /// Execute a command on a virtual device
    pub fn execute_virtual_command(&mut self, device_id: &DeviceId, command: &str) -> Option<String> {
        if let Some(virtual_device) = self.virtual_devices.get_mut(device_id) {
            // Log the command
            self.virtual_command_log.push(VirtualCommand {
                timestamp: js_sys::Date::now(),
                device_id: device_id.clone(),
                command: command.to_string(),
                response: None,
            });
            
            // Find matching handler
            for (pattern, handler) in &virtual_device.command_handlers {
                if command.contains(pattern) {
                    let response = handler.response_template.clone();
                    
                    // Update last command response in log
                    if let Some(last_cmd) = self.virtual_command_log.last_mut() {
                        last_cmd.response = Some(response.clone());
                    }
                    
                    console::log_1(&format!("🔵 Virtual device {} responded: {}", device_id.0, response).into());
                    return Some(response);
                }
            }
            
            // Default response
            Some("OK".to_string())
        } else {
            None
        }
    }
}

/// BluetoothLE Plugin following b00t pattern
pub struct BluetoothLEPlugin;

impl Plugin for BluetoothLEPlugin {
    fn build(&self, app: &mut App) {
        console::log_1(&"🔵 BluetoothLEPlugin::build() starting...".into());
        
        app
            .init_resource::<BluetoothLEManager>()
            .add_event::<BluetoothLERequest>()
            .add_event::<BluetoothLEResponse>()
            .add_systems(Update, (
                handle_bluetoothle_requests,
                process_bluetoothle_responses,
                bluetoothle_connection_monitor,
                virtual_network_system,
            ));
        
        console::log_1(&"🔵 BluetoothLEPlugin setup complete!".into());
    }
}

/// Handle BluetoothLE requests from game logic
fn handle_bluetoothle_requests(
    mut bt: ResMut<BluetoothLEManager>,
    mut requests: EventReader<BluetoothLERequest>,
    mut responses: EventWriter<BluetoothLEResponse>,
) {
    for request in requests.read() {
        console::log_1(&format!("🔵 Processing BluetoothLE request: {:?}", request).into());
        
        match request {
            BluetoothLERequest::StartScan { duration_ms, device_filter } => {
                bt.scanning = true;
                responses.write(BluetoothLEResponse::ScanStarted);
                
                // If virtual network is enabled, simulate device discovery
                if bt.virtual_network_enabled {
                    for (_, virtual_device) in &bt.virtual_devices {
                        responses.write(BluetoothLEResponse::DeviceDiscovered { 
                            device: virtual_device.info.clone() 
                        });
                    }
                }
            },
            
            BluetoothLERequest::StopScan => {
                bt.scanning = false;
                responses.write(BluetoothLEResponse::ScanStopped);
            },
            
            BluetoothLERequest::Connect { device_id } => {
                bt.connection_states.insert(device_id.clone(), BluetoothLEConnectionState::Connecting);
                
                if bt.virtual_network_enabled && bt.virtual_devices.contains_key(device_id) {
                    // Simulate successful virtual connection
                    bt.connection_states.insert(device_id.clone(), BluetoothLEConnectionState::Connected);
                    
                    // Clone device info to avoid borrowing issues
                    let device_info = bt.virtual_devices.get(device_id).map(|d| d.info.clone());
                    if let Some(info) = device_info {
                        bt.connected_devices.insert(device_id.clone(), info);
                    }
                    responses.write(BluetoothLEResponse::Connected { device_id: device_id.clone() });
                } else {
                    // Real device connection would be handled by TypeScript bridge
                    console::log_1(&format!("🔵 Real device connection requested: {:?}", device_id).into());
                }
            },
            
            BluetoothLERequest::SendCommand { device_id, command, timeout_ms } => {
                if bt.virtual_network_enabled {
                    // Handle virtual device command
                    let command_str = format!("{:?}", command);
                    if let Some(response) = bt.execute_virtual_command(device_id, &command_str) {
                        // Simulate Zephyr response
                        let zephyr_response = match command {
                            ZephyrCommand::GetBatteryLevel => {
                                ZephyrResponse::BatteryLevel { percentage: 85, voltage_mv: 3700 }
                            },
                            ZephyrCommand::GetDeviceInfo => {
                                ZephyrResponse::DeviceInfo {
                                    firmware_version: "1.0.0".to_string(),
                                    hardware_version: "v2.1".to_string(),
                                    serial_number: "VRT001".to_string(),
                                }
                            },
                            _ => ZephyrResponse::Success,
                        };
                        
                        responses.write(BluetoothLEResponse::CommandResponse {
                            device_id: device_id.clone(),
                            command: command.clone(),
                            response: zephyr_response,
                            latency_ms: 50, // Simulate low latency
                        });
                    }
                } else {
                    console::log_1(&format!("🔵 Real device command: {:?} -> {:?}", device_id, command).into());
                }
            },
            
            BluetoothLERequest::EnableVirtualNetwork => {
                bt.virtual_network_enabled = true;
                responses.write(BluetoothLEResponse::VirtualNetworkEnabled);
                console::log_1(&"🔵 Virtual BluetoothLE network enabled".into());
            },
            
            BluetoothLERequest::RegisterVirtualDevice { device } => {
                let device_id = device.info.id.clone();
                bt.register_virtual_device(device.clone());
                responses.write(BluetoothLEResponse::VirtualDeviceRegistered { device_id });
            },
            
            _ => {
                console::log_1(&format!("🔵 Unhandled BluetoothLE request: {:?}", request).into());
            }
        }
    }
}

/// Process BluetoothLE responses (placeholder for future expansion)
fn process_bluetoothle_responses(
    mut responses: EventReader<BluetoothLEResponse>,
) {
    for response in responses.read() {
        console::log_1(&format!("🔵 BluetoothLE response: {:?}", response).into());
    }
}

/// Monitor BluetoothLE connections and handle timeouts
fn bluetoothle_connection_monitor(
    _bt: Res<BluetoothLEManager>,
) {
    // Connection timeout monitoring would go here
    // For now, just a placeholder
}

/// Virtual network system for testing
fn virtual_network_system(
    bt: Res<BluetoothLEManager>,
) {
    // Keep virtual devices "alive" - send periodic heartbeats, etc.
    if bt.virtual_network_enabled && !bt.virtual_devices.is_empty() {
        // This would handle periodic virtual device simulation
    }
}

/// Helper function to create common virtual devices for testing
pub fn create_test_virtual_devices() -> Vec<VirtualDevice> {
    vec![
        // Virtual smart collar
        VirtualDevice {
            info: DeviceInfo {
                id: DeviceId("virtual_collar_001".to_string()),
                name: "Test Smart Collar".to_string(),
                device_type: BluetoothLEDeviceType::SmartCollar { 
                    collar_type: CollarType::TrainingCollar 
                },
                rssi: -45,
                services: vec!["uuid_collar_service".to_string()],
                manufacturer_data: Some("ZephyrCollar_v2.1".to_string()),
                is_connected: false,
                last_seen: Some(js_sys::Date::now()),
                battery_level: Some(85),
            },
            command_handlers: [
                ("GetBatteryLevel".to_string(), VirtualCommandHandler {
                    command_pattern: "GetBatteryLevel".to_string(),
                    response_template: "85%".to_string(),
                    delay_ms: 100,
                }),
                ("Vibrate".to_string(), VirtualCommandHandler {
                    command_pattern: "Vibrate".to_string(),
                    response_template: "Vibrating".to_string(),
                    delay_ms: 50,
                }),
            ].into(),
            state: HashMap::new(),
            auto_responses: true,
        },
        
        // Virtual feeding station
        VirtualDevice {
            info: DeviceInfo {
                id: DeviceId("virtual_feeder_001".to_string()),
                name: "Test Feeding Station".to_string(),
                device_type: BluetoothLEDeviceType::FeedingStation { capacity_ml: 2000 },
                rssi: -38,
                services: vec!["uuid_feeder_service".to_string()],
                manufacturer_data: Some("ZephyrFeeder_v1.5".to_string()),
                is_connected: false,
                last_seen: Some(js_sys::Date::now()),
                battery_level: Some(92),
            },
            command_handlers: [
                ("GetFoodLevel".to_string(), VirtualCommandHandler {
                    command_pattern: "GetFoodLevel".to_string(),
                    response_template: "1200g/2000g".to_string(),
                    delay_ms: 75,
                }),
                ("DispenseFood".to_string(), VirtualCommandHandler {
                    command_pattern: "DispenseFood".to_string(),
                    response_template: "Food dispensed".to_string(),
                    delay_ms: 200,
                }),
            ].into(),
            state: HashMap::new(),
            auto_responses: true,
        },
    ]
}

/// Trigger BluetoothLE device scan
pub fn trigger_bluetoothle_scan(
    bt_requests: &mut EventWriter<BluetoothLERequest>,
    filter: Option<BluetoothLEDeviceFilter>,
    duration_ms: Option<u32>,
) {
    bt_requests.write(BluetoothLERequest::StartScan {
        duration_ms,
        device_filter: filter,
    });
}

/// Trigger device connection
pub fn connect_to_device(
    bt_requests: &mut EventWriter<BluetoothLERequest>,
    device_id: DeviceId,
) {
    bt_requests.write(BluetoothLERequest::Connect { device_id });
}