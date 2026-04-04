use crate::particle::{ParticleConfig, ParticleSystem};
use crate::types::Vec2;

// -- construction --

#[test]
fn test_new_particle_system_empty() {
    let ps = ParticleSystem::new();
    assert_eq!(ps.particle_count(), 0);
}

#[test]
fn test_default_config() {
    let cfg = ParticleConfig::default();
    assert!(cfg.spawn_rate > 0.0);
    assert!(cfg.lifetime.0 > 0.0);
    assert!(cfg.texture.is_none());
}

// -- add_emitter --

#[test]
fn test_add_emitter_returns_index() {
    let mut ps = ParticleSystem::new();
    let idx0 = ps.add_emitter(ParticleConfig::default(), Vec2::ZERO);
    let idx1 = ps.add_emitter(ParticleConfig::default(), Vec2::new(10.0, 10.0));
    assert_eq!(idx0, 0);
    assert_eq!(idx1, 1);
}

// -- update spawns particles --

#[test]
fn test_update_spawns_particles() {
    let mut ps = ParticleSystem::new();
    ps.add_emitter(ParticleConfig {
        spawn_rate: 100.0,
        ..ParticleConfig::default()
    }, Vec2::ZERO);
    ps.update(0.1); // should spawn ~10 particles
    assert!(ps.particle_count() > 0);
}

#[test]
fn test_inactive_emitter_no_spawn() {
    let mut ps = ParticleSystem::new();
    let idx = ps.add_emitter(ParticleConfig {
        spawn_rate: 100.0,
        ..ParticleConfig::default()
    }, Vec2::ZERO);
    // deactivate by removing
    ps.remove_emitter(idx);
    ps.update(1.0);
    assert_eq!(ps.particle_count(), 0);
}

// -- particle lifetime --

#[test]
fn test_particles_die_after_lifetime() {
    let mut ps = ParticleSystem::new();
    ps.add_emitter(ParticleConfig {
        spawn_rate: 100.0,
        lifetime: (0.1, 0.1), // very short
        ..ParticleConfig::default()
    }, Vec2::ZERO);
    ps.update(0.05); // spawn some (100 * 0.05 = 5 particles)
    let count = ps.particle_count();
    assert!(count > 0);
    ps.remove_emitter(0); // stop spawning
    ps.update(0.2); // age them past lifetime
    ps.update(0.0); // retain pass removes dead particles
    assert_eq!(ps.particle_count(), 0);
}

// -- set_emitter_position --

#[test]
fn test_set_emitter_position() {
    let mut ps = ParticleSystem::new();
    ps.add_emitter(ParticleConfig::default(), Vec2::ZERO);
    ps.set_emitter_position(0, Vec2::new(100.0, 200.0));
    // no panic, and spawned particles should come from new position
    ps.update(0.1);
    assert!(ps.particle_count() > 0);
}

#[test]
fn test_set_emitter_position_invalid_index() {
    let mut ps = ParticleSystem::new();
    ps.set_emitter_position(99, Vec2::new(10.0, 10.0)); // should not panic
}

// -- remove_emitter --

#[test]
fn test_remove_emitter() {
    let mut ps = ParticleSystem::new();
    ps.add_emitter(ParticleConfig::default(), Vec2::ZERO);
    ps.add_emitter(ParticleConfig::default(), Vec2::new(10.0, 0.0));
    ps.remove_emitter(0);
    // should have 1 emitter left — spawning still works
    ps.update(0.5);
    assert!(ps.particle_count() > 0);
}

#[test]
fn test_remove_emitter_invalid_index() {
    let mut ps = ParticleSystem::new();
    ps.remove_emitter(99); // should not panic
}

// -- zero spawn rate --

#[test]
fn test_zero_spawn_rate_no_particles() {
    let mut ps = ParticleSystem::new();
    ps.add_emitter(ParticleConfig {
        spawn_rate: 0.0,
        ..ParticleConfig::default()
    }, Vec2::ZERO);
    ps.update(1.0);
    assert_eq!(ps.particle_count(), 0);
}
