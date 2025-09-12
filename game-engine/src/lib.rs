use bevy::prelude::*;
use bevy_web_asset::WebAssetPlugin;
use wasm_bindgen::prelude::*;
use web_sys::console;
use std::sync::Mutex;
use std::collections::VecDeque;

mod audio;
mod bluetoothle;
mod components;
mod effects;
mod events;
mod game;
mod resources;
mod systems;

use audio::{PlatformAudioPlugin, send_audio_response_to_bevy};
use bluetooth::{BluetoothPlugin, BluetoothRequest, BluetoothResponse, DeviceId, BluetoothDeviceFilter, create_test_virtual_devices};
use events::{EventBridgePlugin, BevyToJsEvent, send_js_to_bevy_event};
use game::{GamePlugin, LoadCritterEvent, SpawnCritterEvent};
use systems::process_click_on_critters;

// Event queues for communication between WASM interface and Bevy
static LOAD_CRITTER_QUEUE: Mutex<VecDeque<LoadCritterEvent>> = Mutex::new(VecDeque::new());
static INTERACTION_QUEUE: Mutex<VecDeque<(String, f32, f32, f32, f32)>> = Mutex::new(VecDeque::new());
static AUDIO_EVENT_QUEUE: Mutex<VecDeque<BevyToJsEvent>> = Mutex::new(VecDeque::new());
static NATIVE_AUDIO_QUEUE: Mutex<VecDeque<audio::AudioRequest>> = Mutex::new(VecDeque::new());
static BLUETOOTH_REQUEST_QUEUE: Mutex<VecDeque<BluetoothRequest>> = Mutex::new(VecDeque::new());
static BLUETOOTH_RESPONSE_QUEUE: Mutex<VecDeque<BluetoothResponse>> = Mutex::new(VecDeque::new());

// Shared critter list snapshot for UI consumption
#[derive(Clone, Debug)]
pub struct CritterSummary {
    pub id: String,
    pub name: String,
    pub species: String,
    pub sprite_url: String,
    // Preview/animation metadata (idle)
    pub frame_width: f32,
    pub frame_height: f32,
    pub idle_fps: f32,
    pub idle_frame_coords: Vec<(f32, f32)>, // ordered list of (x,y) for idle frames
    // Raw critter stats for display
    pub stat_base_speed: f32,
    pub stat_energy: f32,
    pub stat_happiness_boost: f32,
}

static CRITTER_LIST: Mutex<Vec<CritterSummary>> = Mutex::new(Vec::new());
static CRITTERS_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn set_available_critters(list: Vec<CritterSummary>) {
    if let Ok(mut g) = CRITTER_LIST.lock() {
        *g = list;
        CRITTERS_READY.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

// Enable better panic messages in development
#[cfg(feature = "console_error_panic_hook")]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

// Main entry point for WASM
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    set_panic_hook();

    let build_timestamp = env!("BUILD_TIMESTAMP");
    console::log_1(&format!("🐕 App4.Dog Game Engine Starting... [v2024-EXPLOSION-FIX] Built: {}", build_timestamp).into());
    
    App::new()
        .add_plugins(WebAssetPlugin::default())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        canvas: Some("#game-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
        )
        .add_plugins(GamePlugin)
        .add_plugins(EventBridgePlugin)
        .add_plugins(PlatformAudioPlugin)
        .add_plugins(BluetoothPlugin)
        .add_plugins(effects::ExplosionEffectsPlugin)
        .add_systems(Update, (
            process_load_critter_queue,
            process_interaction_queue,
            process_audio_event_queue,
            process_native_audio_queue,
            process_bluetooth_request_queue,
            process_bluetooth_response_queue,
        ))
        .run();
}

// JavaScript interface for game control
#[wasm_bindgen]
pub struct GameEngine {
    // Future: store game state reference
}

#[wasm_bindgen]
impl GameEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> GameEngine {
        console::log_1(&"🎮 GameEngine initialized".into());
        GameEngine {}
    }

    #[wasm_bindgen]
    pub fn start_game(&self) {
        console::log_1(&"🚀 Game starting...".into());
        // Future: trigger game start event
    }

    #[wasm_bindgen]
    pub fn pause_game(&self) {
        console::log_1(&"⏸️ Game paused".into());
        // Future: pause game systems
    }

    #[wasm_bindgen]
    pub fn reset_game(&self) {
        console::log_1(&"🔄 Game reset".into());
        // Future: reset game state
    }

    #[wasm_bindgen]
    pub fn handle_interaction(&self, interaction_type: &str, x: f32, y: f32, dir_x: f32, dir_y: f32) {
        console::log_1(&format!("🐾 Pet interaction received: {} at ({}, {}) with direction ({}, {})", 
            interaction_type, x, y, dir_x, dir_y).into());
        
        // Queue the interaction for processing by Bevy
        if let Ok(mut queue) = INTERACTION_QUEUE.lock() {
            queue.push_back((interaction_type.to_string(), x, y, dir_x, dir_y));
        }
    }

    #[wasm_bindgen]
    pub fn load_critter(&self, critter_id: u32, name: &str, species: &str) {
        console::log_1(&format!("🐶 Loading critter: ID={}, Name={}, Species={}", 
            critter_id, name, species).into());
        
        // Queue the critter load event for processing by Bevy
        if let Ok(mut queue) = LOAD_CRITTER_QUEUE.lock() {
            queue.push_back(LoadCritterEvent {
                critter_id,
                name: name.to_string(),
                species: species.to_string(),
                id: name.to_string(), // back-compat bridge (deprecated)
            });
        }
    }

    /// Preferred API: load a critter by canonical ID string
    #[wasm_bindgen]
    pub fn load_critter_by_id(&self, id: &str) {
        console::log_1(&format!("🐶 Loading critter by id: {}", id).into());
        if let Ok(mut queue) = LOAD_CRITTER_QUEUE.lock() {
            queue.push_back(LoadCritterEvent {
                critter_id: 0,
                name: String::new(),
                species: String::new(),
                id: id.to_string(),
            });
        }
    }

    #[wasm_bindgen]
    pub fn get_critter_info(&self) -> js_sys::Object {
        // Return current critter information as JS object
        let info = js_sys::Object::new();
        js_sys::Reflect::set(&info, &"id".into(), &1.into()).unwrap();
        js_sys::Reflect::set(&info, &"name".into(), &"Default Critter".into()).unwrap();
        js_sys::Reflect::set(&info, &"species".into(), &"dog".into()).unwrap();
        js_sys::Reflect::set(&info, &"happiness".into(), &0.8.into()).unwrap();
        js_sys::Reflect::set(&info, &"energy".into(), &1.0.into()).unwrap();
        
        info
    }

    #[wasm_bindgen]
    pub fn unload_critter(&self) {
        console::log_1(&"🚪 Unloading current critter".into());
        // Future: cleanup current critter entity
    }

    /// Test the new event bridge by playing audio via TypeScript
    #[wasm_bindgen]
    pub fn play_audio_via_bridge(&self, sound_id: &str, volume: f32) -> String {
        let request_id = format!("audio-{}", js_sys::Date::now() as u64);
        console::log_1(&format!("🎵 Requesting audio via bridge: {} (request_id: {})", sound_id, request_id).into());
        
        // We need to trigger this from within a Bevy system, so we'll use the same pattern as other events
        if let Ok(mut queue) = AUDIO_EVENT_QUEUE.lock() {
            queue.push_back(BevyToJsEvent::PlayAudio {
                request_id: request_id.clone(),
                sound_id: sound_id.to_string(),
                volume,
            });
        }
        
        request_id
    }

    /// Play audio using the new b00t AudioPlugin pattern
    #[wasm_bindgen]
    pub fn play_audio_native(&self, sound_id: &str, volume: Option<f32>) -> String {
        let request_id = audio::AudioManager::generate_request_id();
        console::log_1(&format!("🎵 Playing audio via AudioPlugin: {} (request_id: {})", sound_id, request_id).into());
        
        // Queue audio request for the AudioPlugin to process
        if let Ok(mut queue) = NATIVE_AUDIO_QUEUE.lock() {
            queue.push_back(audio::AudioRequest::Play {
                request_id: request_id.clone(),
                sound_id: sound_id.to_string(),
                context: audio::AudioContext::Test,
                volume: volume.unwrap_or(0.8),
                loop_audio: false,
            });
        }
        
        request_id
    }
    
    /// Play enter area sound
    #[wasm_bindgen]
    pub fn play_enter_sound(&self) -> String {
        console::log_1(&"🚪 Playing enter sound".into());
        self.play_audio_native("enter_area", Some(0.8))
    }
    
    /// Play exit area sound
    #[wasm_bindgen]
    pub fn play_exit_sound(&self) -> String {
        console::log_1(&"🚪 Playing exit sound".into());
        self.play_audio_native("exit_area", Some(0.7))
    }

    /// Start Bluetooth device scan
    #[wasm_bindgen]
    pub fn start_bluetooth_scan(&self, duration_ms: Option<u32>) -> String {
        let request_id = format!("bt-scan-{}", js_sys::Date::now() as u64);
        console::log_1(&format!("🔵 Starting Bluetooth scan (request_id: {})", request_id).into());
        
        if let Ok(mut queue) = BLUETOOTH_REQUEST_QUEUE.lock() {
            queue.push_back(BluetoothRequest::StartScan {
                duration_ms,
                device_filter: None, // Can be extended later
            });
        }
        
        request_id
    }

    /// Stop Bluetooth device scan
    #[wasm_bindgen]
    pub fn stop_bluetooth_scan(&self) {
        console::log_1(&"🔵 Stopping Bluetooth scan".into());
        
        if let Ok(mut queue) = BLUETOOTH_REQUEST_QUEUE.lock() {
            queue.push_back(BluetoothRequest::StopScan);
        }
    }

    /// Connect to a Bluetooth device
    #[wasm_bindgen]
    pub fn connect_bluetooth_device(&self, device_id: &str) -> String {
        let request_id = format!("bt-connect-{}", js_sys::Date::now() as u64);
        console::log_1(&format!("🔵 Connecting to device: {} (request_id: {})", device_id, request_id).into());
        
        if let Ok(mut queue) = BLUETOOTH_REQUEST_QUEUE.lock() {
            queue.push_back(BluetoothRequest::Connect {
                device_id: DeviceId(device_id.to_string()),
            });
        }
        
        request_id
    }

    /// Disconnect from a Bluetooth device
    #[wasm_bindgen]
    pub fn disconnect_bluetooth_device(&self, device_id: &str) {
        console::log_1(&format!("🔵 Disconnecting from device: {}", device_id).into());
        
        if let Ok(mut queue) = BLUETOOTH_REQUEST_QUEUE.lock() {
            queue.push_back(BluetoothRequest::Disconnect {
                device_id: DeviceId(device_id.to_string()),
            });
        }
    }

    /// Enable virtual Bluetooth network for testing
    #[wasm_bindgen]
    pub fn enable_virtual_bluetooth(&self) {
        console::log_1(&"🔵 Enabling virtual Bluetooth network".into());
        
        if let Ok(mut queue) = BLUETOOTH_REQUEST_QUEUE.lock() {
            queue.push_back(BluetoothRequest::EnableVirtualNetwork);
            
            // Register test devices
            for virtual_device in create_test_virtual_devices() {
                queue.push_back(BluetoothRequest::RegisterVirtualDevice {
                    device: virtual_device,
                });
            }
        }
    }

    /// Disable virtual Bluetooth network
    #[wasm_bindgen]
    pub fn disable_virtual_bluetooth(&self) {
        console::log_1(&"🔵 Disabling virtual Bluetooth network".into());
        
        if let Ok(mut queue) = BLUETOOTH_REQUEST_QUEUE.lock() {
            queue.push_back(BluetoothRequest::DisableVirtualNetwork);
        }
    }

    /// Send a command to a Bluetooth device (Zephyr protocol)
    #[wasm_bindgen]
    pub fn send_bluetooth_command(&self, device_id: &str, command_json: &str) -> String {
        let request_id = format!("bt-cmd-{}", js_sys::Date::now() as u64);
        console::log_1(&format!("🔵 Sending command to {}: {} (request_id: {})", 
            device_id, command_json, request_id).into());
        
        // Parse command JSON - for now use a simple command
        // In practice this would deserialize from JSON to ZephyrCommand
        if let Ok(mut queue) = BLUETOOTH_REQUEST_QUEUE.lock() {
            // Simplified command parsing for demo
            let command = if command_json.contains("battery") {
                bluetooth::ZephyrCommand::GetBatteryLevel
            } else if command_json.contains("info") {
                bluetooth::ZephyrCommand::GetDeviceInfo
            } else {
                bluetooth::ZephyrCommand::GetDeviceInfo // Default
            };
            
            queue.push_back(BluetoothRequest::SendCommand {
                device_id: DeviceId(device_id.to_string()),
                command,
                timeout_ms: Some(5000),
            });
        }
        
        request_id
    }

    /// Get Bluetooth status and discovered devices
    #[wasm_bindgen]
    pub fn get_bluetooth_status(&self) -> js_sys::Object {
        let status = js_sys::Object::new();
        
        // This would read from Bluetooth manager state
        // For now, return basic status
        js_sys::Reflect::set(&status, &"scanning".into(), &false.into()).unwrap();
        js_sys::Reflect::set(&status, &"connectedDevices".into(), &0.into()).unwrap();
        js_sys::Reflect::set(&status, &"discoveredDevices".into(), &0.into()).unwrap();
        js_sys::Reflect::set(&status, &"virtualNetworkEnabled".into(), &false.into()).unwrap();
        
        status
    }
}

/// Free functions to allow UI to query available critters without holding a GameEngine instance
#[wasm_bindgen]
pub fn critters_ready() -> bool {
    CRITTERS_READY.load(std::sync::atomic::Ordering::SeqCst)
}

#[wasm_bindgen]
pub fn get_available_critters() -> js_sys::Array {
    let arr = js_sys::Array::new();
    if let Ok(g) = CRITTER_LIST.lock() {
        for c in g.iter() {
            let o = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&o, &"id".into(), &c.id.clone().into());
            let _ = js_sys::Reflect::set(&o, &"name".into(), &c.name.clone().into());
            let _ = js_sys::Reflect::set(&o, &"species".into(), &c.species.clone().into());
            let _ = js_sys::Reflect::set(&o, &"sprite".into(), &c.sprite_url.clone().into());
            // Animation/preview fields
            let _ = js_sys::Reflect::set(&o, &"frameWidth".into(), &c.frame_width.into());
            let _ = js_sys::Reflect::set(&o, &"frameHeight".into(), &c.frame_height.into());
            let _ = js_sys::Reflect::set(&o, &"idleFps".into(), &c.idle_fps.into());
            let frames_arr = js_sys::Array::new();
            for (x, y) in c.idle_frame_coords.iter() {
                let fo = js_sys::Object::new();
                let _ = js_sys::Reflect::set(&fo, &"x".into(), &(*x).into());
                let _ = js_sys::Reflect::set(&fo, &"y".into(), &(*y).into());
                frames_arr.push(&fo);
            }
            let _ = js_sys::Reflect::set(&o, &"frames".into(), &frames_arr);
            // Stats
            let stats = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&stats, &"baseSpeed".into(), &c.stat_base_speed.into());
            let _ = js_sys::Reflect::set(&stats, &"energy".into(), &c.stat_energy.into());
            let _ = js_sys::Reflect::set(&stats, &"happinessBoost".into(), &c.stat_happiness_boost.into());
            let _ = js_sys::Reflect::set(&o, &"stats".into(), &stats);
            arr.push(&o);
        }
    }
    arr
}

/// Expose the JS->Bevy event sending function 
#[wasm_bindgen]
pub fn send_event_to_bevy(event_json: &str) -> Result<(), JsValue> {
    send_js_to_bevy_event(event_json)
}

/// Expose the audio response function from AudioPlugin
#[wasm_bindgen]  
pub fn send_audio_response(response_json: &str) -> Result<(), JsValue> {
    send_audio_response_to_bevy(response_json)
}

// Systems to process the event queues from WASM interface
fn process_load_critter_queue(
    mut load_events: EventWriter<LoadCritterEvent>,
) {
    if let Ok(mut queue) = LOAD_CRITTER_QUEUE.lock() {
        while let Some(event) = queue.pop_front() {
            load_events.write(event);
        }
    }
}

fn process_interaction_queue(
    critter_query: Query<(Entity, &Transform), With<components::Critter>>,
    mut interaction_events: EventWriter<game::CritterInteractionEvent>,
    window_query: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut audio_gate: ResMut<resources::AudioGate>,
) {
    if let Ok(mut queue) = INTERACTION_QUEUE.lock() {
        let queue_size = queue.len();
        if queue_size > 0 {
            console::log_1(&format!("🎯 Processing {} interactions from queue", queue_size).into());
        }
        
        while let Some((interaction_type, screen_x, screen_y, _dir_x, _dir_y)) = queue.pop_front() {
            // Convert screen coordinates to world coordinates
            let Ok(window) = window_query.single() else { continue; };
            let Ok((camera, camera_transform)) = camera_query.single() else { continue; };
            
            // Convert screen position to world position
            let screen_pos = Vec2::new(screen_x, screen_y);
            let world_pos = if let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, screen_pos) {
                world_position
            } else {
                // Fallback: simple conversion assuming centered camera
                Vec2::new(screen_x - window.width() / 2.0, window.height() / 2.0 - screen_y)
            };
            
            console::log_1(&format!("🎯 Click at screen ({}, {}) -> world ({}, {})", 
                screen_x, screen_y, world_pos.x, world_pos.y).into());
            
            // Find the closest critter to the click position  
            // Unlock audio due to user gesture
            audio_gate.enabled = true;
            
            let critter_count = critter_query.iter().count();
            console::log_1(&format!("🎯 Found {} critters in scene", critter_count).into());
            
            for (entity, transform) in &critter_query {
                let critter_pos = transform.translation.xy();
                let critter_size = 100.0; // Larger clickable area radius for easier clicking
                let distance = world_pos.distance(critter_pos);
                
                console::log_1(&format!("🎯 Distance to critter at ({}, {}): {:.1}", 
                    critter_pos.x, critter_pos.y, distance).into());
                
                if distance <= critter_size {
                    let interaction_type_enum = match interaction_type.as_str() {
                        "swipe" => game::InteractionType::Swipe(Vec2::ZERO), // Could use dir_x, dir_y
                        "hold" => game::InteractionType::Hold,
                        _ => game::InteractionType::Tap, // Default to tap
                    };
                    
                    interaction_events.write(game::CritterInteractionEvent {
                        critter_entity: entity,
                        interaction_type: interaction_type_enum,
                        position: world_pos,
                    });
                    
                    console::log_1(&format!("✅ {} interaction sent to critter at ({}, {})", 
                        interaction_type, critter_pos.x, critter_pos.y).into());
                    break; // Only interact with the first critter found
                }
            }
        }
    }
}

// System to process audio events from WASM interface
fn process_audio_event_queue(
    mut bevy_to_js_events: EventWriter<BevyToJsEvent>,
) {
    if let Ok(mut queue) = AUDIO_EVENT_QUEUE.lock() {
        while let Some(event) = queue.pop_front() {
            bevy_to_js_events.write(event);
        }
    }
}

// System to process native audio requests from WASM interface
fn process_native_audio_queue(
    mut audio_requests: EventWriter<audio::AudioRequest>,
) {
    if let Ok(mut queue) = NATIVE_AUDIO_QUEUE.lock() {
        while let Some(request) = queue.pop_front() {
            audio_requests.write(request);
        }
    }
}

// System to process Bluetooth requests from WASM interface
fn process_bluetooth_request_queue(
    mut bluetooth_requests: EventWriter<BluetoothRequest>,
) {
    if let Ok(mut queue) = BLUETOOTH_REQUEST_QUEUE.lock() {
        while let Some(request) = queue.pop_front() {
            bluetooth_requests.write(request);
        }
    }
}

// System to process Bluetooth responses and forward to WASM interface
fn process_bluetooth_response_queue(
    mut bluetooth_responses: EventReader<BluetoothResponse>,
) {
    for response in bluetooth_responses.read() {
        // Forward responses to JavaScript via event system or store in queue
        // For now, just log them
        console::log_1(&format!("🔵 Bluetooth response: {:?}", response).into());
        
        if let Ok(mut queue) = BLUETOOTH_RESPONSE_QUEUE.lock() {
            queue.push_back(response.clone());
        }
    }
}
