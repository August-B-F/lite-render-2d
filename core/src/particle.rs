use crate::renderer::Renderer;
use crate::texture::TextureHandle;
use crate::types::{BlendMode, Color, DrawParams, DrawStyle, SpriteParams, Transform2D, Vec2};

#[derive(Clone, Debug)]
pub struct ParticleConfig {
    pub spawn_rate: f32,
    pub lifetime: (f32, f32),
    pub velocity: (Vec2, Vec2),
    pub size: (f32, f32),
    pub color_start: Color,
    pub color_end: Color,
    pub gravity: Vec2,
    pub texture: Option<TextureHandle>,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            spawn_rate: 10.0,
            lifetime: (1.0, 2.0),
            velocity: (Vec2::new(-20.0, -50.0), Vec2::new(20.0, -10.0)),
            size: (4.0, 1.0),
            color_start: Color::WHITE,
            color_end: Color::new(1.0, 1.0, 1.0, 0.0),
            gravity: Vec2::new(0.0, 98.0),
            texture: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParticleEmitter {
    pub config: ParticleConfig,
    pub position: Vec2,
    pub active: bool,
    spawn_accum: f32,
}

impl ParticleEmitter {
    pub fn new(config: ParticleConfig, position: Vec2) -> Self {
        Self {
            config,
            position,
            active: true,
            spawn_accum: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
struct Particle {
    pos: Vec2,
    vel: Vec2,
    age: f32,
    lifetime: f32,
    size_start: f32,
    size_end: f32,
    color_start: Color,
    color_end: Color,
    texture: Option<TextureHandle>,
}

// simple xorshift rng - good enough for partcles
fn cheap_rand(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed as f32) / (u32::MAX as f32)
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub struct ParticleSystem {
    emitters: Vec<ParticleEmitter>,
    particles: Vec<Particle>,
    rng_seed: u32,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self {
            emitters: Vec::new(),
            particles: Vec::new(),
            rng_seed: 12345,
        }
    }
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_emitter(&mut self, config: ParticleConfig, position: Vec2) -> usize {
        let idx = self.emitters.len();
        self.emitters.push(ParticleEmitter::new(config, position));
        idx
    }

    pub fn set_emitter_position(&mut self, idx: usize, pos: Vec2) {
        if let Some(e) = self.emitters.get_mut(idx) {
            e.position = pos;
        }
    }

    pub fn remove_emitter(&mut self, idx: usize) {
        if idx < self.emitters.len() {
            self.emitters.remove(idx);
        }
    }

    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    pub fn update(&mut self, dt: f32) {
        // remove dead particles
        self.particles.retain(|p| p.age < p.lifetime);

        // spawn new particles from active emitters
        for emitter in &mut self.emitters {
            if !emitter.active { continue; }

            emitter.spawn_accum += emitter.config.spawn_rate * dt;
            while emitter.spawn_accum >= 1.0 {
                emitter.spawn_accum -= 1.0;

                let cfg = &emitter.config;
                let t1 = cheap_rand(&mut self.rng_seed);
                let t2 = cheap_rand(&mut self.rng_seed);
                let t3 = cheap_rand(&mut self.rng_seed);

                let vel = Vec2::new(
                    lerp_f32(cfg.velocity.0.x, cfg.velocity.1.x, t1),
                    lerp_f32(cfg.velocity.0.y, cfg.velocity.1.y, t2),
                );
                let lifetime = lerp_f32(cfg.lifetime.0, cfg.lifetime.1, t3);

                self.particles.push(Particle {
                    pos: emitter.position,
                    vel,
                    age: 0.0,
                    lifetime,
                    size_start: cfg.size.0,
                    size_end: cfg.size.1,
                    color_start: cfg.color_start,
                    color_end: cfg.color_end,
                    texture: cfg.texture,
                });
            }
        }

        // update alive particles
        for p in &mut self.particles {
            p.age += dt;
            // find gravity from first emitter with matching texture (simple approach)
            let grav = self.emitters.first()
                .map(|e| e.config.gravity)
                .unwrap_or(Vec2::ZERO);
            p.vel.x += grav.x * dt;
            p.vel.y += grav.y * dt;
            p.pos.x += p.vel.x * dt;
            p.pos.y += p.vel.y * dt;
        }
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        for p in &self.particles {
            let t = (p.age / p.lifetime).min(1.0);
            let size = lerp_f32(p.size_start, p.size_end, t);
            let color = p.color_start.lerp(p.color_end, t);

            match p.texture {
                Some(tex) => {
                    renderer.draw_sprite(tex, SpriteParams {
                        transform: Transform2D {
                            pos: Vec2::new(p.pos.x - size * 0.5, p.pos.y - size * 0.5),
                            scale: Vec2::new(size, size),
                            rotation: 0.0,
                        },
                        tint: color,
                        src_rect: None,
                        flip_x: false,
                        flip_y: false,
                        blend: BlendMode::Alpha,
                        z_index: 0,
                        opacity: color.a,
                    });
                }
                None => {
                    renderer.draw_circle(
                        p.pos,
                        size * 0.5,
                        DrawParams {
                            style: DrawStyle::Fill(color),
                            blend: BlendMode::Alpha,
                            z_index: 0,
                            opacity: 1.0,
                        },
                    );
                }
            }
        }
    }
}
