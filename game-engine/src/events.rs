// Event bridge system for Bevy <-> TypeScript communication
// Using specta for type-safe serialization and TypeScript bindings

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
// Note: Using manual type sync instead of specta for simplicity
use wasm_bindgen::prelude::*;
use web_sys::CustomEvent;

// Simple console logging macros for WASM
macro_rules! console_log {
    ($($arg:tt)*) => {
        web_sys::console::log_1(&format!($($arg)*).into())
    };
}

macro_rules! console_warn {
    ($($arg:tt)*) => {
        web_sys::console::warn_1(&format!($($arg)*).into())
    };
}

macro_rules! console_error {
    ($($arg:tt)*) => {
        web_sys::console::error_1(&format!($($arg)*).into())
    };
}

/// Events that Bevy sends to TypeScript
#[derive(Debug, Clone, Serialize, Deserialize, Event)]
#[serde(tag = "type")]
pub enum BevyToJsEvent {
    /// Request to play audio with completion callback
    PlayAudio {
        /// Unique ID for this audio request
        request_id: String,
        /// Sound file path or ID
        sound_id: String,
        /// Volume (0.0 to 1.0)
        volume: f32,
    },
    /// Request Bluetooth scan
    BluetoothScan {
        request_id: String,
        device_filter: String,
    },
    /// Test event for development
    TestEvent {
        request_id: String,
        message: String,
    },
}

/// Events that TypeScript sends back to Bevy
#[derive(Debug, Clone, Serialize, Deserialize, Event)]
#[serde(tag = "type")]
pub enum JsToBevyEvent {
    /// Audio playback completed (success or failure)
    AudioCompleted {
        /// Request ID that was sent with PlayAudio
        request_id: String,
        /// Whether playback succeeded
        success: bool,
        /// Error message if failed
        error_message: Option<String>,
        /// Duration of audio file in seconds
        duration_seconds: Option<f32>,
    },
    /// Bluetooth scan completed
    BluetoothScanCompleted {
        request_id: String,
        success: bool,
        devices_found: Vec<String>,
        error_message: Option<String>,
    },
    /// Test event response
    TestEventResponse {
        request_id: String,
        response_data: String,
    },
    /// User gesture notification for AudioContext activation
    UserGesture {
        request_id: String,
        timestamp: f64,
    },
    /// Shared settings pushed from JS (web) to Bevy
    SettingsUpdated {
        request_id: String,
        settings: SharedSettings,
    },
}

/// Resource to track pending requests
#[derive(Resource, Default)]
pub struct PendingRequests {
    pub audio_requests: std::collections::HashMap<String, AudioRequest>,
    pub bluetooth_requests: std::collections::HashMap<String, BluetoothRequest>,
}

#[derive(Debug, Clone)]
pub struct AudioRequest {
    pub sound_id: String,
    pub volume: f32,
    pub timestamp: f64,
}

#[derive(Debug, Clone)]  
pub struct BluetoothRequest {
    pub device_filter: String,
    pub timestamp: f64,
}

/// System to dispatch Bevy events to JavaScript
pub fn dispatch_bevy_to_js_events(
    mut bevy_to_js_events: EventReader<BevyToJsEvent>,
    mut pending_requests: ResMut<PendingRequests>,
) {
    for event in bevy_to_js_events.read() {
        // Track the request
        match event {
            BevyToJsEvent::PlayAudio { request_id, sound_id, volume } => {
                pending_requests.audio_requests.insert(request_id.clone(), AudioRequest {
                    sound_id: sound_id.clone(),
                    volume: *volume,
                    timestamp: js_sys::Date::now(),
                });
            }
            BevyToJsEvent::BluetoothScan { request_id, device_filter } => {
                pending_requests.bluetooth_requests.insert(request_id.clone(), BluetoothRequest {
                    device_filter: device_filter.clone(),
                    timestamp: js_sys::Date::now(),
                });
            }
            _ => {}
        }

        // Dispatch to JavaScript
        if let Err(e) = send_event_to_js(event) {
            console_error!("Failed to send event to JS: {:?}", e);
        }
    }
}

/// System to handle JavaScript responses
pub fn handle_js_to_bevy_events(
    mut js_to_bevy_events: EventReader<JsToBevyEvent>,
    mut pending_requests: ResMut<PendingRequests>,
    mut shared_settings: ResMut<SharedSettings>,
) {
    for event in js_to_bevy_events.read() {
        match event {
            JsToBevyEvent::AudioCompleted { request_id, success, error_message, duration_seconds } => {
                if let Some(request) = pending_requests.audio_requests.remove(request_id) {
                    let elapsed = js_sys::Date::now() - request.timestamp;
                    console_log!(
                        "Audio completed: {} ({}ms) - Success: {}, Duration: {:?}s", 
                        request.sound_id, elapsed as u32, success, duration_seconds
                    );
                    if let Some(error) = error_message {
                        console_warn!("Audio error: {}", error);
                    }
                } else {
                    console_warn!("Received audio completion for unknown request: {}", request_id);
                }
            }
            JsToBevyEvent::BluetoothScanCompleted { request_id, success, devices_found, error_message } => {
                if let Some(request) = pending_requests.bluetooth_requests.remove(request_id) {
                    let elapsed = js_sys::Date::now() - request.timestamp;
                    console_log!(
                        "Bluetooth scan completed: {} ({}ms) - Success: {}, Devices: {:?}", 
                        request.device_filter, elapsed as u32, success, devices_found
                    );
                    if let Some(error) = error_message {
                        console_warn!("Bluetooth error: {}", error);
                    }
                }
            }
            JsToBevyEvent::TestEventResponse { request_id, response_data } => {
                console_log!("Test event response: {} -> {}", request_id, response_data);
            }
            JsToBevyEvent::UserGesture { request_id, timestamp } => {
                console_log!("👆 User gesture received: {} at {}", request_id, timestamp);
                // This will be handled by the audio system
            }
            JsToBevyEvent::SettingsUpdated { request_id, settings } => {
                // Update settings resource but FORCE music_enabled to false
                let mut updated_settings = settings.clone();
                updated_settings.music_enabled = false; // TODO: Temporarily force music off
                *shared_settings = updated_settings;
                console_log!(
                    "⚙️ Settings updated ({}): music_enabled={} (FORCED OFF), bgm_volume={}, sfx_volume={}",
                    request_id,
                    shared_settings.music_enabled,
                    shared_settings.bgm_volume,
                    shared_settings.sfx_volume
                );
            }
        }
    }
}

/// Send event to JavaScript via CustomEvent
fn send_event_to_js(event: &BevyToJsEvent) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("No window object")?;
    let event_data = serde_json::to_string(event)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
    
    let custom_event = CustomEvent::new_with_event_init_dict(
        "bevy-to-js-event",
        &{
            let mut init = web_sys::CustomEventInit::new();
            init.set_detail(&JsValue::from_str(&event_data));
            init
        },
    )?;
    
    window.dispatch_event(&custom_event)?;
    console_log!("Dispatched event to JS: {}", event_data);
    Ok(())
}

/// JavaScript interface to send events back to Bevy
#[wasm_bindgen]
pub fn send_js_to_bevy_event(event_json: &str) -> Result<(), JsValue> {
    let event: JsToBevyEvent = serde_json::from_str(event_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse event: {}", e)))?;
    
    // TODO: We need a way to inject events into the Bevy world from WASM
    // For now, we'll use a global queue and poll it in a system
    INCOMING_JS_EVENTS.with(|queue| {
        queue.borrow_mut().push(event);
    });
    
    Ok(())
}

// Thread-local queue for incoming JS events
thread_local! {
    static INCOMING_JS_EVENTS: std::cell::RefCell<Vec<JsToBevyEvent>> = std::cell::RefCell::new(Vec::new());
}

/// System to poll JavaScript events and add them to Bevy's event system
pub fn poll_js_events(mut js_to_bevy_writer: EventWriter<JsToBevyEvent>) {
    INCOMING_JS_EVENTS.with(|queue| {
        let mut events = queue.borrow_mut();
        for event in events.drain(..) {
            js_to_bevy_writer.write(event);
        }
    });
}

/// Plugin to add event bridge systems to Bevy app
pub struct EventBridgePlugin;

impl Plugin for EventBridgePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<BevyToJsEvent>()
            .add_event::<JsToBevyEvent>()
            .init_resource::<SharedSettings>()
            .init_resource::<PendingRequests>()
            .add_systems(Update, (
                poll_js_events,
                dispatch_bevy_to_js_events,
                handle_js_to_bevy_events,
            ).chain());
    }
}

/// Shared settings resource synchronized from JS
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct SharedSettings {
    pub music_enabled: bool,
    pub bgm_volume: f32,
    pub sfx_volume: f32,
}

impl Default for SharedSettings {
    fn default() -> Self {
        Self {
            music_enabled: false, // TODO: Temporarily disabled - was: true
            bgm_volume: 0.6,
            sfx_volume: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::AppExit;
    use std::time::Duration;

    #[test]
    fn test_audio_event_serialization() {
        let event = BevyToJsEvent::PlayAudio {
            request_id: "test-123".to_string(),
            sound_id: "yipee.mp3".to_string(),
            volume: 0.8,
        };
        
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: BevyToJsEvent = serde_json::from_str(&serialized).unwrap();
        
        match deserialized {
            BevyToJsEvent::PlayAudio { request_id, sound_id, volume } => {
                assert_eq!(request_id, "test-123");
                assert_eq!(sound_id, "yipee.mp3");
                assert_eq!(volume, 0.8);
            }
            _ => panic!("Wrong event type after deserialization"),
        }
    }

    #[test]
    fn test_audio_completion_event() {
        let event = JsToBevyEvent::AudioCompleted {
            request_id: "test-456".to_string(),
            success: true,
            error_message: None,
            duration_seconds: Some(2.5),
        };
        
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: JsToBevyEvent = serde_json::from_str(&serialized).unwrap();
        
        match deserialized {
            JsToBevyEvent::AudioCompleted { request_id, success, duration_seconds, .. } => {
                assert_eq!(request_id, "test-456");
                assert_eq!(success, true);
                assert_eq!(duration_seconds, Some(2.5));
            }
            _ => panic!("Wrong event type after deserialization"),
        }
    }

    // Integration test with Bevy app
    #[test] 
    fn test_event_bridge_integration() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            EventBridgePlugin,
        ));

        // Send an audio event
        let event = BevyToJsEvent::PlayAudio {
            request_id: "integration-test".to_string(),
            sound_id: "test.mp3".to_string(),
            volume: 1.0,
        };
        
        app.world_mut().send_event(event);
        app.update();

        // Check that pending request was tracked
        let pending = app.world().resource::<PendingRequests>();
        assert!(pending.audio_requests.contains_key("integration-test"));

        // Simulate JS response
        let response = JsToBevyEvent::AudioCompleted {
            request_id: "integration-test".to_string(),
            success: true,
            error_message: None,
            duration_seconds: Some(1.5),
        };

        // Add to queue and poll
        INCOMING_JS_EVENTS.with(|queue| {
            queue.borrow_mut().push(response);
        });
        
        app.update();

        // Check that request was removed from pending
        let pending = app.world().resource::<PendingRequests>();
        assert!(!pending.audio_requests.contains_key("integration-test"));
    }
}
