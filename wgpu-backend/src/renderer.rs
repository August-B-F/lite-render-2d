use std::collections::HashMap;

use lite_render_2d_core::{
    BlendMode, Camera2D, Color, DrawParams, DrawStyle, FilterMode, FontHandle, LineParams, Path,
    Rect, Renderer, RendererError, RoundedRect, SpriteParams, StrokeParams, TextParams,
    TextureHandle, TextureParams, Transform2D, Vec2, WrapMode,
};

use crate::batch::Batcher;
use crate::shaders;

struct TextureInfo {
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

pub struct WgpuRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,

    clear_color: Color,
    w: u32,
    h: u32,
    proj: [f32; 16],

    // shape pipeline
    shape_pipeline: wgpu::RenderPipeline,
    shape_bind_group: wgpu::BindGroup,

    // sprite pipeline
    sprite_pipeline: wgpu::RenderPipeline,
    sprite_bind_group_layout: wgpu::BindGroupLayout,

    // shared
    proj_uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_size: u64,

    batcher: Batcher,
    textures: HashMap<u64, TextureInfo>,
    next_tex_id: u64,
}

// y-down screen ortho, origin top-left
fn screen_ortho(w: u32, h: u32) -> [f32; 16] {
    let w = w as f32;
    let h = h as f32;
    [
        2.0 / w, 0.0,      0.0,  0.0,
        0.0,    -2.0 / h,  0.0,  0.0,
        0.0,     0.0,      -1.0, 0.0,
       -1.0,     1.0,       0.0, 1.0,
    ]
}

fn f32_as_bytes(data: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

impl WgpuRenderer {
    pub fn draw_calls(&self) -> u32 {
        self.batcher.draw_calls()
    }
}

impl Renderer for WgpuRenderer {
    fn new(window: &winit::window::Window) -> Result<Self, RendererError>
    where
        Self: Sized,
    {
        let instance = wgpu::Instance::default();

        // safety: the caller (examples) keeps window alive longer than the renderer
        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::from_window(window)
                .map_err(|e| RendererError::Surface(e.to_string()))?;
            instance
                .create_surface_unsafe(target)
                .map_err(|e| RendererError::Surface(e.to_string()))?
        };

        let (adapter, device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
                .ok_or_else(|| RendererError::ContextCreation("no suitable adapter".into()))?;

            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default(), None)
                .await
                .map_err(|e| RendererError::ContextCreation(e.to_string()))?;

            Ok::<_, RendererError>((adapter, device, queue))
        })?;

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // projection uniform buffer (64 bytes = mat4x4<f32>)
        let proj_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("proj_uniform"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // -- shape pipeline --
        let shape_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shape_shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::SHAPE_SHADER.into()),
        });

        let shape_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shape_bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let shape_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shape_bg"),
            layout: &shape_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: proj_uniform_buffer.as_entire_binding(),
            }],
        });

        let shape_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shape_pl"),
                bind_group_layouts: &[&shape_bind_group_layout],
                push_constant_ranges: &[],
            });

        let blend_state = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let shape_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shape_pipeline"),
            layout: Some(&shape_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shape_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 48, // 12 floats
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 2 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 32, shader_location: 3 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 36, shader_location: 4 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 40, shader_location: 5 },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shape_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(blend_state),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // -- sprite pipeline --
        let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::SPRITE_SHADER.into()),
        });

        let sprite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite_bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let sprite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sprite_pl"),
                bind_group_layouts: &[&sprite_bind_group_layout],
                push_constant_ranges: &[],
            });

        let sprite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&sprite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sprite_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 36, // 9 floats
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 2 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 32, shader_location: 3 },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sprite_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(blend_state),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // initial vertex buffer (64KB)
        let vertex_buffer_size: u64 = 65536;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex_buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            surface_config,
            clear_color: Color::BLACK,
            w: size.width.max(1),
            h: size.height.max(1),
            proj: screen_ortho(size.width.max(1), size.height.max(1)),
            shape_pipeline,
            shape_bind_group,
            sprite_pipeline,
            sprite_bind_group_layout,
            proj_uniform_buffer,
            vertex_buffer,
            vertex_buffer_size,
            batcher: Batcher::new(),
            textures: HashMap::new(),
            next_tex_id: 1,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);
        self.w = w;
        self.h = h;
        self.proj = screen_ortho(w, h);
        self.surface_config.width = w;
        self.surface_config.height = h;
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn set_camera(&mut self, camera: &Camera2D) {
        self.proj = camera.projection_matrix();
    }

    fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    fn set_blend_mode(&mut self, _mode: BlendMode) {}

    fn begin_frame(&mut self) -> Result<(), RendererError> {
        self.batcher.clear();
        Ok(())
    }

    fn push_transform(&mut self, _transform: Transform2D) {}
    fn pop_transform(&mut self) {}
    fn reset_transform(&mut self) {}

    fn push_clip_rect(&mut self, _rect: Rect) {}
    fn pop_clip_rect(&mut self) {}

    fn draw_rect(&mut self, rect: Rect, params: DrawParams) {
        let (color, mode, stroke_w) = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                (c, 0.0_f32, 0.0_f32)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut c = sp.color;
                c.a *= params.opacity;
                (c, 1.0, sp.thickness)
            }
            _ => return,
        };

        let x = rect.pos.x;
        let y = rect.pos.y;
        let w = rect.size.x;
        let h = rect.size.y;

        #[rustfmt::skip]
        let verts: [f32; 72] = [
            x,     y,     0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x + w, y,     w,   0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x + w, y + h, w,   h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x,     y,     0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x + w, y + h, w,   h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x,     y + h, 0.0, h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
        ];

        self.batcher.push_shape(&verts);
    }

    fn draw_rounded_rect(&mut self, _rrect: RoundedRect, _params: DrawParams) {}

    fn draw_circle(&mut self, center: Vec2, radius: f32, params: DrawParams) {
        let (color, mode, stroke_w_norm) = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                (c, 2.0_f32, 0.0_f32)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut c = sp.color;
                c.a *= params.opacity;
                let norm = 1.0 - sp.thickness / radius;
                (c, 3.0, norm)
            }
            _ => return,
        };

        let pad = 2.0;
        let ext = radius + pad;
        let cx = center.x;
        let cy = center.y;
        let ln = ext / radius;

        #[rustfmt::skip]
        let verts: [f32; 72] = [
            cx - ext, cy - ext, -ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx + ext, cy - ext,  ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx + ext, cy + ext,  ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx - ext, cy - ext, -ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx + ext, cy + ext,  ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx - ext, cy + ext, -ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
        ];

        self.batcher.push_shape(&verts);
    }

    fn draw_ellipse(&mut self, _center: Vec2, _radii: Vec2, _params: DrawParams) {}
    fn draw_arc(
        &mut self,
        _center: Vec2,
        _radius: f32,
        _start_angle: f32,
        _end_angle: f32,
        _params: DrawParams,
    ) {
    }
    fn draw_polygon(&mut self, _points: &[Vec2], _params: DrawParams) {}
    fn draw_triangle(&mut self, _a: Vec2, _b: Vec2, _c: Vec2, _params: DrawParams) {}

    fn draw_line(&mut self, from: Vec2, to: Vec2, params: LineParams) {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }

        let half = params.thickness * 0.5;
        let nx = -dy / len * half;
        let ny = dx / len * half;

        let mut color = params.color;
        color.a *= params.opacity;
        let mode = 4.0_f32;

        #[rustfmt::skip]
        let verts: [f32; 72] = [
            from.x + nx, from.y + ny, 0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            from.x - nx, from.y - ny, 0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            to.x + nx,   to.y + ny,   0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            to.x + nx,   to.y + ny,   0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            from.x - nx, from.y - ny, 0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            to.x - nx,   to.y - ny,   0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
        ];

        self.batcher.push_shape(&verts);
    }

    fn draw_polyline(&mut self, _points: &[Vec2], _params: LineParams) {}
    fn draw_path(&mut self, _path: &Path, _params: DrawParams) {}
    fn stroke_path(&mut self, _path: &Path, _params: StrokeParams) {}

    fn load_texture(
        &mut self,
        data: &[u8],
        params: TextureParams,
    ) -> Result<TextureHandle, RendererError> {
        let img = image::load_from_memory(data)
            .map_err(|e| RendererError::Texture(e.to_string()))?
            .into_rgba8();
        let (w, h) = img.dimensions();
        let pixels = img.into_raw();

        let tex_size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite_tex"),
            size: tex_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            tex_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let filter = match params.filter {
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
            FilterMode::Linear => wgpu::FilterMode::Linear,
        };
        let address_mode = match params.wrap {
            WrapMode::Clamp => wgpu::AddressMode::ClampToEdge,
            WrapMode::Repeat => wgpu::AddressMode::Repeat,
        };

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite_sampler"),
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            mag_filter: filter,
            min_filter: filter,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite_bg"),
            layout: &self.sprite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.proj_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let id = self.next_tex_id;
        self.next_tex_id += 1;
        self.textures.insert(id, TextureInfo { bind_group, width: w, height: h });
        Ok(TextureHandle::new(id))
    }

    fn unload_texture(&mut self, handle: TextureHandle) {
        self.textures.remove(&handle.id());
    }

    fn draw_sprite(&mut self, handle: TextureHandle, params: SpriteParams) {
        let info = match self.textures.get(&handle.id()) {
            Some(i) => i,
            None => return,
        };

        let tw = info.width as f32;
        let th = info.height as f32;

        let t = &params.transform;
        let sx = t.scale.x * tw;
        let sy = t.scale.y * th;
        let cos = t.rotation.cos();
        let sin = t.rotation.sin();

        let transform = |px: f32, py: f32| -> (f32, f32) {
            let x = cos * sx * px + (-sin * sy) * py + t.pos.x;
            let y = sin * sx * px + cos * sy * py + t.pos.y;
            (x, y)
        };

        let (x0, y0) = transform(0.0, 0.0);
        let (x1, y1) = transform(1.0, 0.0);
        let (x2, y2) = transform(1.0, 1.0);
        let (x3, y3) = transform(0.0, 1.0);

        let (uv_min_x, uv_min_y, uv_max_x, uv_max_y) = match params.src_rect {
            Some(r) => (
                r.pos.x / tw,
                r.pos.y / th,
                (r.pos.x + r.size.x) / tw,
                (r.pos.y + r.size.y) / th,
            ),
            None => (0.0, 0.0, 1.0, 1.0),
        };

        let bake_uv = |mut u: f32, mut v: f32| -> (f32, f32) {
            if params.flip_x { u = 1.0 - u; }
            if params.flip_y { v = 1.0 - v; }
            let u = uv_min_x + u * (uv_max_x - uv_min_x);
            let v = uv_min_y + v * (uv_max_y - uv_min_y);
            (u, v)
        };

        let (u0, v0) = bake_uv(0.0, 0.0);
        let (u1, v1) = bake_uv(1.0, 0.0);
        let (u2, v2) = bake_uv(1.0, 1.0);
        let (u3, v3) = bake_uv(0.0, 1.0);

        let r = params.tint.r;
        let g = params.tint.g;
        let b = params.tint.b;
        let a = params.tint.a;
        let op = params.opacity;

        #[rustfmt::skip]
        let verts: [f32; 54] = [
            x0, y0, u0, v0, r, g, b, a, op,
            x1, y1, u1, v1, r, g, b, a, op,
            x2, y2, u2, v2, r, g, b, a, op,
            x0, y0, u0, v0, r, g, b, a, op,
            x2, y2, u2, v2, r, g, b, a, op,
            x3, y3, u3, v3, r, g, b, a, op,
        ];

        self.batcher.push_sprite(handle.id(), &verts);
    }

    fn load_font(&mut self, _data: &[u8]) -> Result<FontHandle, RendererError> {
        Ok(FontHandle::new(0))
    }

    fn unload_font(&mut self, _handle: FontHandle) {}
    fn draw_text(&mut self, _text: &str, _params: &TextParams) {}

    fn measure_text(&self, _text: &str, _params: &TextParams) -> Vec2 {
        Vec2::ZERO
    }

    fn end_frame(&mut self) -> Result<(), RendererError> {
        // upload projection
        self.queue.write_buffer(
            &self.proj_uniform_buffer,
            0,
            f32_as_bytes(&self.proj),
        );

        // acquire surface texture
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| RendererError::Surface(e.to_string()))?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // calculate total vertex data needed
        let shape_bytes = self.batcher.shape_buf.len() * 4;
        let mut sprite_total_bytes = 0u64;
        let mut sprite_offsets: Vec<(u64, u64, u64)> = Vec::new(); // (tex_id, offset, byte_len)
        for (&tex_id, buf) in &self.batcher.sprite_runs {
            if buf.is_empty() {
                continue;
            }
            let byte_len = (buf.len() * 4) as u64;
            sprite_offsets.push((tex_id, sprite_total_bytes, byte_len));
            sprite_total_bytes += byte_len;
        }

        let total_needed = (shape_bytes as u64) + sprite_total_bytes;

        // grow vertex buffer if needed
        if total_needed > self.vertex_buffer_size && total_needed > 0 {
            let new_size = total_needed.next_power_of_two();
            self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vertex_buffer"),
                size: new_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.vertex_buffer_size = new_size;
        }

        // upload all vertex data before the render pass
        let shape_byte_len = shape_bytes as u64;
        if !self.batcher.shape_buf.is_empty() {
            self.queue.write_buffer(
                &self.vertex_buffer,
                0,
                f32_as_bytes(&self.batcher.shape_buf),
            );
        }

        // upload sprite data after shapes
        let sprite_base_offset = shape_byte_len;
        for (&tex_id, buf) in &self.batcher.sprite_runs {
            if buf.is_empty() {
                continue;
            }
            let offset_in_list = sprite_offsets
                .iter()
                .find(|(id, _, _)| *id == tex_id)
                .unwrap();
            let buffer_offset = sprite_base_offset + offset_in_list.1;
            self.queue.write_buffer(
                &self.vertex_buffer,
                buffer_offset,
                f32_as_bytes(buf),
            );
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        {
            let c = self.clear_color;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: c.r as f64,
                            g: c.g as f64,
                            b: c.b as f64,
                            a: c.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            // draw shapes
            if !self.batcher.shape_buf.is_empty() {
                let vert_count = (self.batcher.shape_buf.len() / 12) as u32;
                pass.set_pipeline(&self.shape_pipeline);
                pass.set_bind_group(0, &self.shape_bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(0..shape_byte_len));
                pass.draw(0..vert_count, 0..1);
                self.batcher.add_draw_call();
            }

            // draw sprites
            for &(tex_id, offset, byte_len) in &sprite_offsets {
                let info = match self.textures.get(&tex_id) {
                    Some(i) => i,
                    None => continue,
                };
                let vert_count = (byte_len / 36) as u32; // 9 floats * 4 bytes
                let buf_start = sprite_base_offset + offset;
                let buf_end = buf_start + byte_len;
                pass.set_pipeline(&self.sprite_pipeline);
                pass.set_bind_group(0, &info.bind_group, &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(buf_start..buf_end));
                pass.draw(0..vert_count, 0..1);
                self.batcher.add_draw_call();
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
