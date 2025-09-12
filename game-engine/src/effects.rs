use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use web_sys::console;

/// Component to mark entities that should explode when despawned
#[derive(Component)]
pub struct ExplodeOnDespawn {
    pub explosion_type: ExplosionType,
}

#[derive(Debug, Clone)]
pub enum ExplosionType {
    ParticleBurst, // Colorful particle explosion effect
    // Future: could add other explosion types like sparkles, confetti, etc.
}

/// Event triggered when a critter should explode
#[derive(Event)]
pub struct CritterExplodeEvent {
    pub position: Vec3,
    pub explosion_type: ExplosionType,
}

/// Resource holding explosion effect assets
#[derive(Resource)]
pub struct ExplosionEffects {
    pub particle_explosion: Handle<EffectAsset>,
}

/// Plugin for explosion effects
pub struct ExplosionEffectsPlugin;

impl Plugin for ExplosionEffectsPlugin {
    fn build(&self, app: &mut App) {
        console::log_1(&"🎆 ExplosionEffectsPlugin::build() starting...".into());
        
        // Always add the event and fallback systems first
        console::log_1(&"🎆 Adding CritterExplodeEvent...".into());
        app.add_event::<CritterExplodeEvent>();
        console::log_1(&"✅ CritterExplodeEvent added".into());
        
        // Use fallback system for now due to WebGL2 vs WebGPU complexity
        console::log_1(&"🎆 Using fallback explosion system (WebGL2 compatible)".into());
        app.add_systems(Update, handle_explosion_events_fallback);
        
        // TODO: Implement proper WebGPU detection and dual-build system
        // For now, fallback provides working explosion events without GPU particles
        
        console::log_1(&"🎆 ExplosionEffectsPlugin setup complete!".into());
    }
}

/// Setup explosion effect assets
fn setup_explosion_effects(
    mut effects: ResMut<Assets<EffectAsset>>,
    mut commands: Commands,
) {
    console::log_1(&"🎆 Setting up ribbon explosion effects...".into());
    
    let mut module = Module::default();

    // Spawn positions over a small sphere for 3D-like explosion
    let init_pos = SetPositionSphereModifier {
        center: module.lit(Vec3::ZERO),
        radius: module.lit(0.3),
        dimension: ShapeDimension::Surface,
    };
    
    // Radial velocity - particles explode outward
    let init_vel = SetVelocitySphereModifier {
        center: module.lit(Vec3::ZERO),
        speed: module.lit(150.0), // Fast initial explosion
    };
    
    // Particle lifetime
    let init_life = SetAttributeModifier::new(Attribute::LIFETIME, module.lit(1.2));

    // Color gradient for ribbons - colorful pet-friendly explosion
    let mut gradient = Gradient::new();
    gradient.add_key(0.0, Vec4::new(1.0, 0.8, 0.2, 1.0)); // Bright yellow-orange start
    gradient.add_key(0.3, Vec4::new(0.9, 0.4, 0.8, 1.0)); // Pink-purple middle
    gradient.add_key(0.7, Vec4::new(0.2, 0.6, 1.0, 0.8)); // Blue transition
    gradient.add_key(1.0, Vec4::new(0.1, 0.1, 0.1, 0.0)); // Fade to transparent

    // Create linear drag and gravity modifiers before consuming module
    let drag_modifier = LinearDragModifier::new(module.lit(0.8));
    let gravity_modifier = AccelModifier::new(module.lit(Vec3::new(0.0, -180.0, 0.0)));
    
    // Build a dramatic particle explosion effect (no ribbons in 0.16, but still impressive!)
    let effect = EffectAsset::new(
        2048, // Max particles for good performance on mobile
        SpawnerSettings::burst(600.0.into(), 0.0.into()), // 600 particles instantly
        module,
    )
    .with_name("critter_explosion")
    .init(init_pos)
    .init(init_vel)
    .init(init_life)
    .update(drag_modifier) // Air resistance to slow down particles  
    .update(gravity_modifier) // Gravity for natural fall
    .render(ColorOverLifetimeModifier {
        gradient,
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    });

    let handle = effects.add(effect);
    commands.insert_resource(ExplosionEffects {
        particle_explosion: handle,
    });
    
    console::log_1(&"✨ Particle explosion effect ready!".into());
}

/// Handle explosion events by spawning particle effects
fn handle_explosion_events(
    mut explosion_events: EventReader<CritterExplodeEvent>,
    explosion_effects: Res<ExplosionEffects>,
    mut commands: Commands,
) {
    // DEBUG: Log when explosion events are received
    let event_count = explosion_events.len();
    if event_count > 0 {
        console::log_1(&format!("🎆 Processing {} explosion events", event_count).into());
    }
    
    for event in explosion_events.read() {
        match event.explosion_type {
            ExplosionType::ParticleBurst => {
                console::log_1(&format!("🎆 Spawning particle explosion at ({:.1}, {:.1}, {:.1})", 
                    event.position.x, event.position.y, event.position.z).into());
                
                commands.spawn((
                    ParticleEffect::new(explosion_effects.particle_explosion.clone()),
                    Transform::from_translation(event.position),
                ));
            }
        }
    }
}

/// Fallback explosion handler for WebGL/incompatible hardware
fn handle_explosion_events_fallback(
    mut explosion_events: EventReader<CritterExplodeEvent>,
    mut commands: Commands,
) {
    // DEBUG: Log when explosion events are received  
    let event_count = explosion_events.len();
    if event_count > 0 {
        console::log_1(&format!("🎆 Processing {} explosion events (FALLBACK)", event_count).into());
    }
    
    for event in explosion_events.read() {
        match event.explosion_type {
            ExplosionType::ParticleBurst => {
                console::log_1(&format!("🎆 FALLBACK: Simple explosion effect at ({:.1}, {:.1}, {:.1})", 
                    event.position.x, event.position.y, event.position.z).into());
                
                // TODO: Add simple sprite-based explosion effect
                // For now, just log that the explosion happened
                console::log_1(&"✨ Fallback explosion complete! (No GPU particles, but critter still despawns)".into());
            }
        }
    }
}

/// Trigger explosion for a critter at given position
pub fn trigger_critter_explosion(
    position: Vec3,
    explosion_events: &mut EventWriter<CritterExplodeEvent>,
) {
    explosion_events.write(CritterExplodeEvent {
        position,
        explosion_type: ExplosionType::ParticleBurst,
    });
}