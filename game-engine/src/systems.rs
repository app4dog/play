use bevy::prelude::*;
use web_sys::HtmlAudioElement;
use crate::components::*;
use crate::effects::{CritterExplodeEvent, trigger_critter_explosion};
use crate::resources::*;
use crate::game::*;
use web_sys::console;
use rand::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
// use bevy::log::info;
// use bevy::log;

macro_rules! console_log {
    ($($t:tt)*) => (console::log_1(&format!($($t)*).into()))
}

/// Setup camera system
pub fn setup_camera(mut commands: Commands, game_config: Res<GameConfig>) {
    commands.spawn(Camera2d);
    
    console_log!("📷 Camera setup with bounds: {}x{}", game_config.screen_bounds.x, game_config.screen_bounds.y);
}

/// Setup UI system
pub fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|parent| {
            // Score display
            parent
                .spawn((
                    Text::new("Score: 0"),
                    TextFont {
                        font_size: 40.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ))
                .insert(ScoreDisplay);
        });
}

/// Initialize critter registry with real data - fail fast if data is missing!
/// Shared slot for async loader result: Ok((final_catalog_ron, base_url)) or Err(message)
static REGISTRY_CATALOG_RESULT: std::sync::Mutex<Option<Result<(String, String, std::collections::HashMap<String, (String, String)>), String>>> = std::sync::Mutex::new(None);

#[derive(Resource, Default)]
pub struct RegistryLoadStatus {
    pub started: bool,
    pub completed: bool,
    pub error: Option<String>,
}

/// Startup: kick off async fetch of catalog + critter RON files
pub fn initialize_critter_registry(
    mut load_status: ResMut<RegistryLoadStatus>,
) {
    if load_status.started { return; }
    load_status.started = true;

    console_log!("📦 Fetching critter catalog and RON packages...");

    spawn_local(async {
        let result = load_and_compose_catalog().await
            .map_err(|e| format!("failed to load catalog: {:?}", e));
        if let Ok(mut slot) = REGISTRY_CATALOG_RESULT.lock() {
            *slot = Some(result);
        }
    });
}

/// Update: if async result is ready, insert CritterRegistry
pub fn try_initialize_registry_from_cache(
    mut commands: Commands,
    mut load_status: ResMut<RegistryLoadStatus>,
) {
    if load_status.completed { return; }

    let Some(result) = REGISTRY_CATALOG_RESULT.lock().ok().and_then(|mut g| g.take()) else { return; };
    match result {
        Ok((catalog_ron, base_url, sounds_map)) => {
            match CritterRegistry::from_ron(&catalog_ron, base_url.clone()) {
                Ok(registry) => {
                    // Build critter summaries BEFORE moving registry into resources
                    let mut list: Vec<crate::CritterSummary> = Vec::new();
                    for (id, critter) in registry.catalog.critters.iter() {
                        let path = critter.sprite.path.clone();
                        let url = if path.starts_with("http://") || path.starts_with("https://") {
                            path
                        } else if base_url.is_empty() {
                            format!("/{}", path.trim_start_matches('/'))
                        } else {
                            format!("{}/{}", base_url.trim_end_matches('/'), path.trim_start_matches('/'))
                        };
                        let species = match critter.species {
                            critter_keeper::CritterSpecies::Bird => "Bird",
                            critter_keeper::CritterSpecies::Bunny => "Bunny",
                        }.to_string();

                        // Frame layout and idle animation extraction
                        let frame_layout = &critter.sprite.frame_layout;
                        let frame_width = frame_layout.frame_size.0 as f32;
                        let frame_height = frame_layout.frame_size.1 as f32;
                        let idle_anim = critter
                            .sprite
                            .animations
                            .get("idle")
                            .or_else(|| critter.sprite.animations.values().next())
                            .expect("No animations found in critter");
                        let idle_fps = idle_anim.fps as f32;

                        // Build grid coordinates for all frames (DRY with engine logic)
                        let coords = {
                            match &frame_layout.layout {
                                critter_keeper::LayoutType::Grid { cols, rows } => {
                                    let mut coordinates = Vec::new();
                                    for row in 0..*rows {
                                        let inv_row = rows - 1 - row;
                                        for col in 0..*cols {
                                            coordinates.push((
                                                col as f32 * frame_width,
                                                inv_row as f32 * frame_height,
                                            ));
                                        }
                                    }
                                    coordinates
                                }
                                critter_keeper::LayoutType::Horizontal => {
                                    (0..frame_layout.frame_count)
                                        .map(|i| (i as f32 * frame_width, 0.0))
                                        .collect()
                                }
                                critter_keeper::LayoutType::Vertical => {
                                    (0..frame_layout.frame_count)
                                        .map(|i| {
                                            let inv_i = (frame_layout.frame_count - 1 - i) as f32;
                                            (0.0, inv_i * frame_height)
                                        })
                                        .collect()
                                }
                            }
                        };
                        // Map idle frame indices to coordinates
                        let mut idle_coords: Vec<(f32, f32)> = Vec::new();
                        for idx in idle_anim.frames.iter() {
                            let i = (*idx as usize).min(coords.len().saturating_sub(1));
                            idle_coords.push(coords[i]);
                        }

                        // Stats as source-of-truth values
                        let stats = &critter.stats;

                        list.push(crate::CritterSummary {
                            id: id.clone(),
                            name: critter.name.clone(),
                            species,
                            sprite_url: url,
                            frame_width,
                            frame_height,
                            idle_fps,
                            idle_frame_coords: idle_coords,
                            stat_base_speed: stats.base_speed as f32,
                            stat_energy: stats.energy as f32,
                            stat_happiness_boost: stats.happiness_boost as f32,
                        });
                    }
                    // Now move registry into resources
                    commands.insert_resource(registry);
                    // Convert sounds_map into CritterSounds resource
                    let mut cs = CritterSounds::default();
                    for (id, (entry, success)) in sounds_map.into_iter() {
                        cs.sounds.insert(id, CritterSoundSet { entry, success });
                    }
                    commands.insert_resource(cs);
                    // Publish critter list snapshots for UI
                    crate::set_available_critters(list);
                    load_status.completed = true;
                    console_log!("✅ CritterRegistry initialized (base: {})", base_url);
                }
                Err(err) => {
                    load_status.error = Some(format!("from_ron error: {}", err));
                    console_log!("❌ CritterRegistry::from_ron failed: {}", err);
                }
            }
        }
        Err(msg) => {
            load_status.error = Some(msg.clone());
            console_log!("❌ Critter catalog load failed: {}", msg);
        }
    }
}

async fn fetch_text(url: &str) -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await?;
    let resp: web_sys::Response = resp_value.dyn_into()?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!("HTTP {} for {}", resp.status(), url)));
    }
    let text_promise = resp.text()?;
    let text = wasm_bindgen_futures::JsFuture::from(text_promise).await?;
    Ok(text.as_string().unwrap_or_default())
}

async fn load_and_compose_catalog() -> Result<(String, String, std::collections::HashMap<String, (String, String)>), JsValue> {
    // Base paths
    let base_dir = "/critters/";
    let catalog_url = "/critters/catalog.ron";

    // Compute base_url for CritterConfig (origin + trailing slash)
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let origin = window.location().origin().map_err(|_| JsValue::from_str("origin error"))?;
    let base_url = if origin.ends_with('/') { origin } else { format!("{}/", origin) };

    let catalog_text = fetch_text(catalog_url).await?;
    let mut sounds_map: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();

    // Parse pointer entries: "id": "file.ron"
    let mut entries: Vec<(String, String)> = Vec::new();
    for raw_line in catalog_text.lines() {
        let line = raw_line.trim();
        if !line.contains(":") || !line.contains(".ron") { continue; }
        // Extract first quoted = id, last quoted = file
        let (id, file) = match (line.find('"'), line.rfind('"')) {
            (Some(first_q), Some(last_q)) if last_q > first_q => {
                let rest = &line[first_q+1..];
                if let Some(end_id_rel) = rest.find('"') {
                    let id = &rest[..end_id_rel];
                    let left = &line[..last_q];
                    if let Some(start_file) = left.rfind('"') {
                        let file = &line[start_file+1..last_q];
                        (id.to_string(), file.to_string())
                    } else { continue }
                } else { continue }
            }
            _ => continue,
        };
        if file.ends_with(".ron") { entries.push((id, file)); }
    }

    // Fetch each critter RON and build final embedded catalog
    let mut final_catalog = String::from("(\n    critters: {\n");
    for (id, file) in entries {
        let url = if file.starts_with('/') { file.clone() } else { format!("{}{}", base_dir, file) };
        let ron_text = fetch_text(&url).await?;
        // Extract optional sounds mapping: sounds: (entry: "...", success: "...")
        let entry_pat = regex_lite::Regex::new("entry\\s*:\\s*\"([^\"]+)\"").unwrap();
        let success_pat = regex_lite::Regex::new("success\\s*:\\s*\"([^\"]+)\"").unwrap();
        let entry = entry_pat.captures(&ron_text).and_then(|c| c.get(1)).map(|m| m.as_str().to_string());
        let success = success_pat.captures(&ron_text).and_then(|c| c.get(1)).map(|m| m.as_str().to_string());
        if let (Some(e), Some(s)) = (entry, success) {
            sounds_map.insert(id.clone(), (e, s));
        }
        final_catalog.push_str(&format!("        \"{}\": {},\n", id, ron_text.trim()));
    }
    final_catalog.push_str("    }\n)");

    Ok((final_catalog, base_url, sounds_map))
}

/// Asset loading system
pub fn load_game_assets(
    asset_server: Res<AssetServer>,
    mut asset_collection: ResMut<AssetCollection>,
) {
    // Load sprite sheets using HTTPS URLs with WebAssetPlugin
    console_log!("🎨 Starting asset loading with HTTPS URLs...");
    
    // Use HTTPS URLs with WebAssetPlugin configured properly
    asset_collection.bird_sprite = asset_server.load("https://play.app4.dog:9000/assets/sprites/bird-animation.png");
    console_log!("🐦 Bird sprite handle created: {:?}", asset_collection.bird_sprite);
    
    asset_collection.bunny_sprite = asset_server.load("https://play.app4.dog:9000/assets/sprites/bunny-sprite-sheet.png");  
    console_log!("🐰 Bunny sprite handle created: {:?}", asset_collection.bunny_sprite);
    
    // Load audio (prefer existing OGG in repo)
    asset_collection.positive_sound = asset_server.load("assets/audio/positive/yipee.ogg");
    
    console_log!("✅ Asset loading initiated with HTTPS URLs");
}

/// Enhanced asset loading status monitoring system with detailed error handling
pub fn monitor_asset_loading(
    asset_server: Res<AssetServer>,
    selected_asset: Res<SelectedCritterAsset>,
    mut monitoring_timer: Local<Timer>,
    mut assets_loaded: Local<bool>,
    time: Res<Time>,
) {
    // Only monitor if assets aren't loaded yet
    if *assets_loaded {
        return;
    }
    
    if monitoring_timer.duration().is_zero() {
        *monitoring_timer = Timer::from_seconds(2.0, TimerMode::Repeating);
    }
    
    monitoring_timer.tick(time.delta());
    
    if monitoring_timer.just_finished() {
        if let Some(handle) = &selected_asset.handle {
            let status = asset_server.get_load_state(handle);
            let url = selected_asset.url.clone().unwrap_or_else(|| "(unknown)".to_string());
            match status {
                Some(bevy::asset::LoadState::NotLoaded) => console_log!("🧭 Selected sprite: ⏳ Not loaded yet ({})", url),
                Some(bevy::asset::LoadState::Loading) => console_log!("🧭 Selected sprite: 🔄 Loading... ({})", url),
                Some(bevy::asset::LoadState::Loaded) => {
                    if !*assets_loaded {
                        console_log!("🧭 Selected sprite: ✅ Loaded ({})", url);
                        *assets_loaded = true;
                        console_log!("🎉 Selected critter sprite loaded. Monitoring stopped.");
                    }
                },
                Some(bevy::asset::LoadState::Failed(err)) => console_log!("🧭 Selected sprite: ❌ Failed ({}) - {:?}", url, err),
                None => console_log!("🧭 Selected sprite: (no status) {}", url),
            }
        } else {
            console_log!("🧭 No selected critter sprite to monitor yet.");
        }
    }
}

/// Critter movement system with screen wrapping and position tracking
pub fn critter_movement_system(
    time: Res<Time>,
    mut critter_query: Query<(&mut Transform, &mut CritterMovement), With<Critter>>,
    game_config: Res<GameConfig>,
    mut frame_counter: Local<u32>,
) {
    *frame_counter += 1;
    
    for (mut transform, mut movement) in &mut critter_query {
        let old_pos = transform.translation;
        
        // Update position based on velocity
        transform.translation += movement.velocity.extend(0.0) * time.delta_secs();
        
        // Log position every 60 frames (roughly 1 second at 60fps)
        if *frame_counter % 60 == 0 {
            console_log!("📍 Critter position: ({:.1}, {:.1}, {:.1}) velocity: ({:.1}, {:.1})", 
                transform.translation.x, transform.translation.y, transform.translation.z,
                movement.velocity.x, movement.velocity.y);
        }
        
        // Screen wrapping with margins
        let margin = 50.0;
        let half_width = game_config.screen_bounds.x / 2.0;
        let half_height = game_config.screen_bounds.y / 2.0;
        
        let pos = &mut transform.translation;
        
        // Horizontal wrapping (left-right)
        if pos.x > half_width + margin {
            pos.x = -half_width - margin;
        } else if pos.x < -half_width - margin {
            pos.x = half_width + margin;
        }
        
        // Vertical wrapping (top-bottom) 
        if pos.y > half_height + margin {
            pos.y = -half_height - margin;
        } else if pos.y < -half_height - margin {
            pos.y = half_height + margin;
        }
        
        // Move towards target if set (overrides continuous movement)
        if let Some(target) = movement.target_position {
            let direction = (target - transform.translation.xy()).normalize_or_zero();
            let distance = transform.translation.xy().distance(target);
            
            if distance > 5.0 {
                movement.velocity = direction * movement.max_speed;
            } else {
                movement.velocity = Vec2::ZERO;
                movement.target_position = None;
                
                // Resume random movement after reaching target
                let mut rng = thread_rng();
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let speed = rng.gen_range(30.0..80.0);
                movement.velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);
            }
        }
        
        // Occasionally change direction for more interesting movement
        if thread_rng().gen_ratio(1, 180) { // ~1/3 chance per second at 60fps
            let mut rng = thread_rng();
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed = rng.gen_range(30.0..80.0);
            movement.velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);
        }
    }
}

/// Critter interaction system - handles real pet interactions with game critters
pub fn critter_interaction_system(
    mut commands: Commands,
    mut interaction_events: EventReader<CritterInteractionEvent>,
    critter_query: Query<(Entity, &Critter, &Transform, Option<&SpriteAnimation>)>,
    mut game_progress_events: EventWriter<GameProgressEvent>,
    mut game_state: ResMut<GameState>,
    asset_server: Res<AssetServer>,
    critter_sounds: Option<Res<CritterSounds>>,
    mut audio_gate: ResMut<AudioGate>,
    mut explosion_events: EventWriter<CritterExplodeEvent>,
) {
    // DEBUG: Log when interaction events are received
    let event_count = interaction_events.len();
    if event_count > 0 {
        console::log_1(&format!("🎯 Processing {} critter interaction events", event_count).into());
    }
    for event in interaction_events.read() {
        if let Ok((entity, critter, transform, anim)) = critter_query.get(event.critter_entity) {
            match event.interaction_type {
                InteractionType::Tap => {
                    // Unlock audio due to user gesture
                    audio_gate.enabled = true;
                    
                    // 🎆 TRIGGER EXPLOSION EFFECT before despawning!
                    trigger_critter_explosion(transform.translation, &mut explosion_events);
                    console::log_1(&format!("🎆 Ribbon explosion triggered at ({:.1}, {:.1})", 
                        transform.translation.x, transform.translation.y).into());
                    
                    // When critter is tapped, it disappears and gives points
                    commands.entity(entity).despawn();
                    
                    // Clear current critter from game state if it was this one
                    if game_state.current_critter_id == Some(entity) {
                        game_state.current_critter_id = None;
                    }
                    
                    game_progress_events.write(GameProgressEvent {
                        score_change: 50, // Higher score for successfully catching a critter
                        achievement: Some(format!("{} caught!", critter.name)),
                    });
                    // Play success sound from catalog (if present)
                    if let (Some(sounds_res), Some(anim)) = (&critter_sounds, anim) {
                        if let Some(set) = sounds_res.sounds.get(&anim.critter_id) {
                            let success_path = &set.success;
                            // Prefer relative paths to respect BASE_URL/subpaths
                            let url = if success_path.starts_with("http") {
                                success_path.clone()
                            } else {
                                success_path.trim_start_matches('/').to_string()
                            };
                            if let Ok(audio) = HtmlAudioElement::new_with_src(&url) {
                                // Attempt to play and surface any async errors
                                match audio.play() {
                                    Ok(promise) => {
                                        let url_c = url.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            if let Err(e) = wasm_bindgen_futures::JsFuture::from(promise).await {
                                                console_log!("❌ Audio play rejected for {}: {:?}", url_c, e);
                                            }
                                        });
                                        console_log!("🔊 Success sound playing (web): {}", url);
                                    }
                                    Err(err) => {
                                        console_log!("❌ audio.play() error for {}: {:?}", url, err);
                                    }
                                }
                            } else {
                                console_log!("❌ Failed to create HtmlAudioElement for {}", url);
                            }
                        }
                    }
                    
                    console_log!("🎯 {} was caught and disappeared!", critter.name);
                }
                InteractionType::Swipe(_) => {
                    // 🎆 TRIGGER EXPLOSION EFFECT for swipe too!
                    trigger_critter_explosion(transform.translation, &mut explosion_events);
                    
                    // Swipe still makes critters disappear but gives fewer points
                    commands.entity(entity).despawn();
                    
                    if game_state.current_critter_id == Some(entity) {
                        game_state.current_critter_id = None;
                    }
                    
                    game_progress_events.write(GameProgressEvent {
                        score_change: 25,
                        achievement: None,
                    });
                    
                    console_log!("💨 {} was swiped away with ribbons!", critter.name);
                }
                InteractionType::Hold => {
                    // 🎆 TRIGGER EXPLOSION EFFECT for hold too!
                    trigger_critter_explosion(transform.translation, &mut explosion_events);
                    
                    // Hold interaction also removes critter
                    commands.entity(entity).despawn();
                    
                    if game_state.current_critter_id == Some(entity) {
                        game_state.current_critter_id = None;
                    }
                    
                    game_progress_events.write(GameProgressEvent {
                        score_change: 30,
                        achievement: None,
                    });
                    
                    console_log!("✋ {} was held and exploded into ribbons!", critter.name);
                }
            }
        }
    }
}

/// Game state management system
pub fn game_state_system(
    mut game_state: ResMut<GameState>,
    mut game_progress_events: EventReader<GameProgressEvent>,
) {
    for event in game_progress_events.read() {
        game_state.score = (game_state.score as i32 + event.score_change).max(0) as u32;
        
        // Level progression
        let new_level = (game_state.score / 100) + 1;
        if new_level > game_state.level {
            game_state.level = new_level;
            // info!("🎉 Level up! New level: {}", game_state.level);
        }
        
        if let Some(achievement) = &event.achievement {
            // info!("🏆 Achievement unlocked: {}", achievement);
        }
    }
}

/// UI update system
pub fn ui_update_system(
    game_state: Res<GameState>,
    mut score_query: Query<&mut Text, With<ScoreDisplay>>,
) {
    if game_state.is_changed() {
        for mut text in &mut score_query {
            text.0 = format!("Score: {} | Level: {}", game_state.score, game_state.level);
        }
    }
}

/// Critter loading system - handles selection of critter type from Vue frontend
pub fn critter_loading_system(
    mut load_events: EventReader<LoadCritterEvent>,
    mut game_state: ResMut<GameState>,
    critter_registry: Option<Res<CritterRegistry>>,
) {
    for event in load_events.read() {
        // Use canonical ID field
        let critter_id = &event.id;
        if let Some(reg) = &critter_registry {
            if reg.catalog.critters.contains_key(critter_id) {
                game_state.selected_critter_id = Some(critter_id.clone());
                console_log!("🐶 Critter ID {} selected for spawning", critter_id);
            } else {
                console_log!("⚠️ Unknown critter ID: {}", critter_id);
            }
        } else {
            console_log!("⏳ CritterRegistry not ready yet; deferring selection for {}", critter_id);
        }
    }
}

/// Random critter spawning system
pub fn critter_spawning_system(
    mut commands: Commands,
    mut spawn_events: EventReader<SpawnCritterEvent>,
    mut game_state: ResMut<GameState>,
    critter_registry: Option<Res<CritterRegistry>>,
    asset_server: Res<AssetServer>,
    mut selected_asset: ResMut<SelectedCritterAsset>,
    critter_sounds: Option<Res<CritterSounds>>,
    audio_gate: Res<AudioGate>,
) {
    for event in spawn_events.read() {
        // Only spawn if we have a selected critter ID and no current critter
        if let (Some(ref critter_id), None) = (&game_state.selected_critter_id, game_state.current_critter_id) {
            if let Some(reg) = &critter_registry {
                if let Some(critter_data) = reg.catalog.critters.get(critter_id) {
                    // Build absolute URL for sprite
                    let path = critter_data.sprite.path.clone();
                    let url = if path.starts_with("http://") || path.starts_with("https://") {
                        path
                    } else {
                        let origin = web_sys::window()
                            .and_then(|w| w.location().origin().ok())
                            .unwrap_or_else(|| String::from(""));
                        if origin.is_empty() { format!("/{}", path.trim_start_matches('/')) }
                        else { format!("{}/{}", origin.trim_end_matches('/'), path.trim_start_matches('/')) }
                    };
                    let sprite_handle: Handle<Image> = asset_server.load(url.clone());
                    let status = asset_server.get_load_state(&sprite_handle);
                    console_log!("🖼️ Using sprite URL {} status: {:?}", url, status);
                    let use_fallback = matches!(status, Some(bevy::asset::LoadState::Failed(_)));

                    // Update selected asset for monitoring
                    selected_asset.handle = Some(sprite_handle.clone());
                    selected_asset.url = Some(url.clone());

                console_log!("🖼️ Spawning sprite at position ({}, {}) with scale 0.5", event.position.x, event.position.y);
                
                // Compute initial frame rect immediately to avoid flashing full sheet
                let frame_layout = &critter_data.sprite.frame_layout;
                let frame_coordinates = generate_grid_coordinates(&frame_layout);
                let idle_animation = critter_data.sprite.animations.get("idle").unwrap_or(
                    critter_data.sprite.animations.values().next().expect("No animations found")
                );
                let first_index = if !idle_animation.frames.is_empty() { idle_animation.frames[0] } else { 0 };
                let initial_rect = frame_coordinates.get(first_index as usize).map(|coords| Rect {
                    min: Vec2::new(coords.0, coords.1),
                    max: Vec2::new(coords.0 + frame_layout.frame_size.0 as f32, coords.1 + frame_layout.frame_size.1 as f32),
                });

                // Determine animation FPS from critter data (speed up a bit)
                let base_fps = idle_animation.fps.max(1.0);
                let speed_multiplier: f32 = 1.75; // global speed-up factor
                let target_fps = (base_fps * speed_multiplier).clamp(1.0, 60.0);

                // Spawn critter entity with maximum visibility
                let critter_entity = commands.spawn((
                    Sprite {
                        image: if use_fallback { Default::default() } else { sprite_handle },
                        color: if use_fallback { 
                            Color::srgb(0.0, 1.0, 1.0) // Bright cyan for fallback sprite
                        } else { 
                            Color::srgb(1.0, 1.0, 1.0) // White for normal sprite
                        },
                        rect: initial_rect,
                        custom_size: Some(Vec2::new(200.0, 200.0)), // Force size
                        ..default()
                    },
                    Transform::from_translation(event.position.extend(100.0)) // Much higher Z for visibility
                        .with_scale(Vec3::splat(1.0)), // Full scale for maximum visibility
                    Critter {
                        name: critter_data.name.clone(),
                        species: match critter_data.species {
                            critter_keeper::CritterSpecies::Bird => CritterSpecies::Bird,
                            critter_keeper::CritterSpecies::Bunny => CritterSpecies::Bunny,
                        },
                        personality: CritterPersonality {
                            playfulness: critter_data.stats.happiness_boost,
                            curiosity: 0.7,
                            obedience: 0.6, // Default value
                        },
                        energy: critter_data.stats.energy,
                        happiness: 0.5,
                    },
                    CritterMovement {
                        velocity: {
                            let mut rng = thread_rng();
                            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                            let speed = rng.gen_range(30.0..80.0); // Random movement speed
                            Vec2::new(angle.cos() * speed, angle.sin() * speed)
                        },
                        max_speed: critter_data.stats.base_speed,
                        acceleration: 100.0,
                        target_position: None,
                    },
                    SpriteAnimation {
                        timer: Timer::from_seconds(1.0 / target_fps, TimerMode::Repeating),
                        frame_count: critter_data.sprite.frame_layout.frame_count as usize,
                        current_frame: 0,
                        repeat: true,
                        critter_id: critter_id.clone(),
                    },
                )).id();
                
                // Play entry sound from catalog-defined path (if present)
                if audio_gate.enabled {
                    if let Some(sounds_res) = &critter_sounds {
                        if let Some(set) = sounds_res.sounds.get(critter_id) {
                            let entry_path = &set.entry;
                            // Prefer relative paths to respect BASE_URL/subpaths
                            let url = if entry_path.starts_with("http") {
                                entry_path.clone()
                            } else {
                                entry_path.trim_start_matches('/').to_string()
                            };
                            if let Ok(audio) = HtmlAudioElement::new_with_src(&url) {
                                match audio.play() {
                                    Ok(promise) => {
                                        let url_c = url.clone();
                                        wasm_bindgen_futures::spawn_local(async move {
                                            if let Err(e) = wasm_bindgen_futures::JsFuture::from(promise).await {
                                                console_log!("❌ Audio play rejected for {}: {:?}", url_c, e);
                                            }
                                        });
                                        console_log!("🔊 Entry sound playing (web): {}", url);
                                    }
                                    Err(err) => {
                                        console_log!("❌ audio.play() error for {}: {:?}", url, err);
                                    }
                                }
                            } else {
                                console_log!("❌ Failed to create HtmlAudioElement for {}", url);
                            }
                        }
                    }
                }

                game_state.current_critter_id = Some(critter_entity);
                console_log!("🎭 Spawned {} at ({}, {})", critter_data.name, event.position.x, event.position.y);
                }
            }
        }
    }
}

/// Auto-spawning system - randomly spawns critters every few seconds
pub fn auto_spawn_system(
    time: Res<Time>,
    mut timer: Local<Timer>,
    mut spawn_events: EventWriter<SpawnCritterEvent>,
    game_state: Res<GameState>,
    game_config: Res<GameConfig>,
) {
    if timer.duration().is_zero() {
        *timer = Timer::from_seconds(3.0, TimerMode::Repeating); // Spawn every 3 seconds
    }
    
    timer.tick(time.delta());
    
    if timer.just_finished() && game_state.current_critter_id.is_none() && game_state.selected_critter_id.is_some() {
        let mut rng = thread_rng();
        
        // ALWAYS spawn at center for debugging
        let x = 0.0;
        let y = 0.0;
        
        console_log!("🎯 FORCED CENTER SPAWN at (0, 0) for debugging");
        
        spawn_events.write(SpawnCritterEvent {
            position: Vec2::new(x, y),
        });
        
        console_log!("🎲 Auto-spawning critter at random position ({}, {})", x, y);
    }
}

/// Click detection system - finds which critter (if any) was clicked based on position
pub fn process_click_on_critters(
    click_position: Vec2,
    critter_query: Query<(Entity, &Transform), With<Critter>>,
    mut interaction_events: EventWriter<CritterInteractionEvent>,
) {
    for (entity, transform) in &critter_query {
        let critter_pos = transform.translation.xy();
        let critter_size = 50.0; // Approximate clickable area radius (adjustable)
        
        if click_position.distance(critter_pos) <= critter_size {
            interaction_events.write(CritterInteractionEvent {
                critter_entity: entity,
                interaction_type: InteractionType::Tap,
                position: click_position,
            });
            
            console_log!("🎯 Click detected on critter at ({}, {})", critter_pos.x, critter_pos.y);
            return; // Only interact with the first critter found
        }
    }
}

/// Sprite animation system - handles frame-by-frame sprite sheet animation using Grid coordinates from critter-keeper
pub fn sprite_animation_system(
    time: Res<Time>,
    mut animation_query: Query<(&mut SpriteAnimation, &mut Sprite), With<Critter>>,
    critter_registry: Option<Res<CritterRegistry>>,
) {
    let Some(critter_registry) = critter_registry else { return; };
    for (mut animation, mut sprite) in &mut animation_query {
        animation.timer.tick(time.delta());
        
        if animation.timer.just_finished() {
            // Move to next frame
            animation.current_frame = (animation.current_frame + 1) % animation.frame_count;
            
            // Look up critter data to get frame layout information
            if let Some(critter_data) = critter_registry.catalog.critters.get(&animation.critter_id) {
                let frame_layout = &critter_data.sprite.frame_layout;
                let idle_animation = critter_data.sprite.animations.get("idle").unwrap_or(
                    critter_data.sprite.animations.values().next().expect("No animations found")
                );
                
                // Generate Grid coordinates for all frames (same logic as Vue component)
                let frame_coordinates = generate_grid_coordinates(&frame_layout);
                
                // Get the current animation frame index from the idle animation sequence
                let animation_frame_index = if !idle_animation.frames.is_empty() {
                    idle_animation.frames[animation.current_frame % idle_animation.frames.len()]
                } else {
                    animation.current_frame
                };
                
                // Get the actual pixel coordinates for this frame
                if let Some(coords) = frame_coordinates.get(animation_frame_index as usize) {
                    let frame_width = frame_layout.frame_size.0 as f32;
                    let frame_height = frame_layout.frame_size.1 as f32;
                    
                    // Set the rect to show only the current frame using Grid coordinates
                    sprite.rect = Some(Rect {
                        min: Vec2::new(coords.0, coords.1),
                        max: Vec2::new(coords.0 + frame_width, coords.1 + frame_height),
                    });
                    // console_log!(
                    //     "🎬 Animating frame {}/{} (anim sequence: {}) - Grid coords: ({}, {}) rect: {:?}",
                    //     animation.current_frame + 1,
                    //     animation.frame_count,
                    //     animation_frame_index,
                    //     coords.0,
                    //     coords.1,
                    //     sprite.rect
                    // );
                } else {
                    console_log!("❌ Invalid frame index {} for critter {}", animation_frame_index, animation.critter_id);
                }
            } else {
                console_log!("❌ Critter data not found for ID: {}", animation.critter_id);
            }
        }
    }
}

/// Generate Grid coordinates for sprite sheet frames (matches Vue component logic)
fn generate_grid_coordinates(frame_layout: &critter_keeper::FrameLayout) -> Vec<(f32, f32)> {
    let frame_width = frame_layout.frame_size.0 as f32;
    let frame_height = frame_layout.frame_size.1 as f32;
    
    match &frame_layout.layout {
        critter_keeper::LayoutType::Grid { cols, rows } => {
            let mut coordinates = Vec::new();
            for row in 0..*rows {
                // Bevy's sprite rect uses bottom-left origin; invert Y from top-left grid
                let inv_row = rows - 1 - row;
                for col in 0..*cols {
                    coordinates.push((
                        col as f32 * frame_width,
                        inv_row as f32 * frame_height
                    ));
                }
            }
            coordinates
        },
        critter_keeper::LayoutType::Horizontal => {
            // Fallback to horizontal layout if needed
            (0..frame_layout.frame_count).map(|i| (i as f32 * frame_width, 0.0)).collect()
        },
        critter_keeper::LayoutType::Vertical => {
            // Fallback to vertical layout if needed
            // Invert Y so index 0 corresponds to top frame, matching Canvas logic
            (0..frame_layout.frame_count)
                .map(|i| {
                    let inv_i = (frame_layout.frame_count - 1 - i) as f32;
                    (0.0, inv_i * frame_height)
                })
                .collect()
        }
    }
}

/// Window size detection system - gets current canvas size and updates game config
pub fn window_resize_system(
    mut game_config: ResMut<GameConfig>,
    mut last_size: Local<Option<Vec2>>,
) {
    // Get canvas size from DOM
    let window = web_sys::window().expect("should have a window");
    let document = window.document().expect("should have a document");
    let canvas = document
        .get_element_by_id("game-canvas")
        .expect("should have game-canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("should be canvas element");
    
    let width = canvas.client_width() as f32;
    let height = canvas.client_height() as f32;
    let current_size = Vec2::new(width, height);
    
    // Only update if size actually changed
    if *last_size != Some(current_size) {
        *last_size = Some(current_size);
        
        // Update screen bounds based on actual canvas size
        game_config.screen_bounds = current_size;
        
        // Update spawn bounds to be slightly smaller than screen bounds
        game_config.pet_spawn_bounds = Vec2::new(width * 0.8, height * 0.8);
        
        console_log!("📏 Canvas size detected: {}x{}, spawn area: {}x{}", 
            width, height, 
            game_config.pet_spawn_bounds.x, game_config.pet_spawn_bounds.y
        );
    }
}
