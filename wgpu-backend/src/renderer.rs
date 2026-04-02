use std::collections::HashMap;

use lite_render_2d_core::{
    BlendMode, Camera2D, Color, DrawParams, DrawStyle, FilterMode, FontHandle, FrameStats,
    LineParams, Path, PostEffect, Rect, RenderTargetHandle, Renderer, RendererError, RoundedRect,
    SpriteInstance, SpriteParams, StrokeParams, TextParams, TextureHandle, TextureParams,
    Transform2D, Vec2, WrapMode,
};

use crate::batch::Batcher;
use crate::shaders;

struct TextureInfo {
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

#[allow(dead_code)]
struct WgpuRenderTargetInfo {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    texture_handle_id: u64,
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
    cam: Camera2D,

    // shape pipelines (one per blend mode: Alpha, Additive, Multiply, Screen, PremultAlpha)
    shape_pipelines: [wgpu::RenderPipeline; 5],
    shape_bind_group: wgpu::BindGroup,

    // sprite pipelines (one per blend mode)
    sprite_pipelines: [wgpu::RenderPipeline; 5],
    sprite_bind_group_layout: wgpu::BindGroupLayout,

    // shared
    proj_uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_size: u64,

    batcher: Batcher,
    textures: HashMap<u64, TextureInfo>,
    next_tex_id: u64,
    // font system
    font_system: lite_render_2d_core::font_atlas::FontSystem,
    font_atlas_tex_id: Option<u64>,
    // transform stack
    transform_stack: lite_render_2d_core::transform_stack::TransformStack,
    // clip rect stack
    clip_stack: Vec<[u32; 4]>,
    current_clip: Option<[u32; 4]>,
    // blend mode
    current_blend: BlendMode,
    // dpi scale factor
    scale_factor: f32,
    // post-processing effects
    effect_grayscale_pipeline: wgpu::RenderPipeline,
    effect_invert_pipeline: wgpu::RenderPipeline,
    effect_brightness_pipeline: wgpu::RenderPipeline,
    effect_vignette_pipeline: wgpu::RenderPipeline,
    effect_bgl: wgpu::BindGroupLayout,
    effect_bgl_brightness: wgpu::BindGroupLayout,
    effect_brightness_uniform: wgpu::Buffer,
    // render targets
    render_targets: HashMap<u64, WgpuRenderTargetInfo>,
    next_rt_id: u64,
    active_render_target: Option<u64>,
    saved_viewport: Option<(u32, u32)>,
    saved_proj: Option<[f32; 16]>,
    // sdf text pipelines (one per blend mode)
    sdf_pipelines: [wgpu::RenderPipeline; 5],
    // sdf font system
    sdf_font_system: lite_render_2d_core::sdf_font::SdfFontSystem,
    sdf_atlas_tex_id: Option<u64>,
}

fn intersect_rects(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = (a[0] + a[2]).min(b[0] + b[2]);
    let y1 = (a[1] + a[3]).min(b[1] + b[3]);
    if x1 <= x0 || y1 <= y0 {
        return [0, 0, 0, 0];
    }
    [x0, y0, x1 - x0, y1 - y0]
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

    // flush current batch to a given texture view with explicit viewport dimensions
    fn flush_to_view(&mut self, view: &wgpu::TextureView, clear: Option<Color>, vp_w: u32, vp_h: u32) {
        use crate::batch::CmdKind;

        // upload projection
        self.queue.write_buffer(
            &self.proj_uniform_buffer,
            0,
            f32_as_bytes(&self.proj),
        );

        self.batcher.sort_commands();

        let shape_bytes = (self.batcher.shape_buf.len() * 4) as u64;
        let sprite_bytes = (self.batcher.sprite_buf.len() * 4) as u64;
        let total_needed = shape_bytes + sprite_bytes;

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

        if !self.batcher.shape_buf.is_empty() {
            self.queue.write_buffer(&self.vertex_buffer, 0, f32_as_bytes(&self.batcher.shape_buf));
        }
        if !self.batcher.sprite_buf.is_empty() {
            self.queue.write_buffer(&self.vertex_buffer, shape_bytes, f32_as_bytes(&self.batcher.sprite_buf));
        }

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("flush_encoder"),
        });

        {
            let load_op = match clear {
                Some(c) => wgpu::LoadOp::Clear(wgpu::Color {
                    r: c.r as f64, g: c.g as f64, b: c.b as f64, a: c.a as f64,
                }),
                None => wgpu::LoadOp::Load,
            };

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("flush_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: load_op, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            let mut draw_calls = 0u32;
            let cmds = std::mem::take(&mut self.batcher.commands);
            let mut i = 0;
            while i < cmds.len() {
                let cmd = cmds[i];

                let mut end = i + 1;
                while end < cmds.len() {
                    let next = &cmds[end];
                    if next.kind != cmd.kind || next.blend != cmd.blend || next.clip != cmd.clip || next.z_index != cmd.z_index {
                        break;
                    }
                    let prev = &cmds[end - 1];
                    if next.vert_start != prev.vert_start + prev.vert_len {
                        break;
                    }
                    end += 1;
                }

                let total_vert_len: u32 = cmds[i..end].iter().map(|c| c.vert_len).sum();

                match cmd.clip {
                    Some([x, y, w, h]) => {
                        // clamp scissor to viewport bounds
                        let cw = w.min(vp_w.saturating_sub(x));
                        let ch = h.min(vp_h.saturating_sub(y));
                        pass.set_scissor_rect(x.min(vp_w), y.min(vp_h), cw, ch);
                    }
                    None => pass.set_scissor_rect(0, 0, vp_w, vp_h),
                }

                let blend_idx = cmd.blend as usize;

                match cmd.kind {
                    CmdKind::Shape => {
                        let byte_start = cmd.vert_start as u64 * 4;
                        let byte_end = byte_start + total_vert_len as u64 * 4;
                        let vert_count = total_vert_len / 12;
                        pass.set_pipeline(&self.shape_pipelines[blend_idx]);
                        pass.set_bind_group(0, &self.shape_bind_group, &[]);
                        pass.set_vertex_buffer(0, self.vertex_buffer.slice(byte_start..byte_end));
                        pass.draw(0..vert_count, 0..1);
                        draw_calls += 1;
                    }
                    CmdKind::Sprite { texture_id } => {
                        let info = match self.textures.get(&texture_id) {
                            Some(i) => i,
                            None => { i = end; continue; }
                        };
                        let byte_start = shape_bytes + cmd.vert_start as u64 * 4;
                        let byte_end = byte_start + total_vert_len as u64 * 4;
                        let vert_count = total_vert_len / 9;
                        pass.set_pipeline(&self.sprite_pipelines[blend_idx]);
                        pass.set_bind_group(0, &info.bind_group, &[]);
                        pass.set_vertex_buffer(0, self.vertex_buffer.slice(byte_start..byte_end));
                        pass.draw(0..vert_count, 0..1);
                        draw_calls += 1;
                    }
                    CmdKind::SdfSprite { texture_id } => {
                        let info = match self.textures.get(&texture_id) {
                            Some(i) => i,
                            None => { i = end; continue; }
                        };
                        let byte_start = shape_bytes + cmd.vert_start as u64 * 4;
                        let byte_end = byte_start + total_vert_len as u64 * 4;
                        let vert_count = total_vert_len / 9;
                        pass.set_pipeline(&self.sdf_pipelines[blend_idx]);
                        pass.set_bind_group(0, &info.bind_group, &[]);
                        pass.set_vertex_buffer(0, self.vertex_buffer.slice(byte_start..byte_end));
                        pass.draw(0..vert_count, 0..1);
                        draw_calls += 1;
                    }
                }

                i = end;
            }

            drop(pass);
            for _ in 0..draw_calls {
                self.batcher.add_draw_call();
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn apply_transform_quad(&self, verts: &mut [f32; 72]) {
        if self.transform_stack.is_identity() {
            return;
        }
        for i in 0..6 {
            let base = i * 12;
            let p = self.transform_stack.apply(Vec2::new(verts[base], verts[base + 1]));
            verts[base] = p.x;
            verts[base + 1] = p.y;
        }
    }

    fn push_shape_raw_transformed(&mut self, verts: &mut [f32], z_index: i32, blend: BlendMode) {
        lite_render_2d_core::tessellation::apply_transform(verts, &self.transform_stack);
        self.batcher.push_shape_raw(verts, z_index, blend, self.current_clip);
    }

    fn apply_transform_sprite(&self, verts: &mut [f32; 54]) {
        if self.transform_stack.is_identity() {
            return;
        }
        for i in 0..6 {
            let base = i * 9;
            let p = self.transform_stack.apply(Vec2::new(verts[base], verts[base + 1]));
            verts[base] = p.x;
            verts[base + 1] = p.y;
        }
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
        let scale = window.scale_factor() as f32;
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

        // blend states for each mode (must match BlendMode repr order)
        let blend_states = [
            // Alpha
            wgpu::BlendState {
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
            },
            // Additive
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            // Multiply
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Dst,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            // Screen
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrc,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            // PremultipliedAlpha
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
            },
        ];

        let shape_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: 48,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 2 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 32, shader_location: 3 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 36, shader_location: 4 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 40, shader_location: 5 },
            ],
        };

        // create one shape pipeline per blend mode
        let shape_pipelines = std::array::from_fn(|i| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shape_pipeline"),
                layout: Some(&shape_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shape_shader,
                    entry_point: Some("vs_main"),
                    buffers: std::slice::from_ref(&shape_vertex_layout),
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shape_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend_states[i]),
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
            })
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

        let sprite_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: 36,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 2 },
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 32, shader_location: 3 },
            ],
        };

        // create one sprite pipeline per blend mode
        let sprite_pipelines = std::array::from_fn(|i| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("sprite_pipeline"),
                layout: Some(&sprite_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &sprite_shader,
                    entry_point: Some("vs_main"),
                    buffers: std::slice::from_ref(&sprite_vertex_layout),
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &sprite_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend_states[i]),
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
            })
        });

        // -- sdf text pipeline (same vertex layout and bind group as sprite) --
        let sdf_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sdf_text_shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::SDF_TEXT_SHADER.into()),
        });

        let sdf_pipelines = std::array::from_fn(|i| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("sdf_pipeline"),
                layout: Some(&sprite_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &sdf_shader,
                    entry_point: Some("vs_main"),
                    buffers: std::slice::from_ref(&sprite_vertex_layout),
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &sdf_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend_states[i]),
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
            })
        });

        // -- post-processing effect pipelines --
        let effect_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("effect_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let effect_bgl_brightness = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("effect_bgl_bright"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let effect_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_pl"),
            bind_group_layouts: &[&effect_bgl],
            push_constant_ranges: &[],
        });

        let effect_pl_brightness = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effect_pl_bright"),
            bind_group_layouts: &[&effect_bgl_brightness],
            push_constant_ranges: &[],
        });

        // helper to create an effect pipeline (no vertex buffers, uses vertex_index)
        let make_effect_pipeline = |label: &str, shader_src: &str, layout: &wgpu::PipelineLayout| -> wgpu::RenderPipeline {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
            })
        };

        let effect_grayscale_pipeline = make_effect_pipeline("effect_grayscale", shaders::EFFECT_GRAYSCALE_SHADER, &effect_pl);
        let effect_invert_pipeline = make_effect_pipeline("effect_invert", shaders::EFFECT_INVERT_SHADER, &effect_pl);
        let effect_brightness_pipeline = make_effect_pipeline("effect_brightness", shaders::EFFECT_BRIGHTNESS_SHADER, &effect_pl_brightness);
        let effect_vignette_pipeline = make_effect_pipeline("effect_vignette", shaders::EFFECT_VIGNETTE_SHADER, &effect_pl);

        let effect_brightness_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brightness_uniform"),
            size: 4,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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
            cam: Camera2D::new(size.width as f32 / scale, size.height as f32 / scale),
            proj: screen_ortho(
                (size.width as f32 / scale) as u32,
                (size.height as f32 / scale) as u32,
            ),
            shape_pipelines,
            shape_bind_group,
            sprite_pipelines,
            sprite_bind_group_layout,
            proj_uniform_buffer,
            vertex_buffer,
            vertex_buffer_size,
            batcher: Batcher::new(),
            textures: HashMap::new(),
            next_tex_id: 1,
            font_system: lite_render_2d_core::font_atlas::FontSystem::new(),
            font_atlas_tex_id: None,
            transform_stack: lite_render_2d_core::transform_stack::TransformStack::new(),
            clip_stack: Vec::new(),
            current_clip: None,
            current_blend: BlendMode::Alpha,
            scale_factor: scale,
            effect_grayscale_pipeline,
            effect_invert_pipeline,
            effect_brightness_pipeline,
            effect_vignette_pipeline,
            effect_bgl,
            effect_bgl_brightness,
            effect_brightness_uniform,
            render_targets: HashMap::new(),
            next_rt_id: 1,
            active_render_target: None,
            saved_viewport: None,
            saved_proj: None,
            sdf_pipelines,
            sdf_font_system: lite_render_2d_core::sdf_font::SdfFontSystem::new(),
            sdf_atlas_tex_id: None,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);
        self.w = w;
        self.h = h;
        // projection uses logical pixels, surface uses physical
        let lw = (w as f32 / self.scale_factor) as u32;
        let lh = (h as f32 / self.scale_factor) as u32;
        self.proj = screen_ortho(lw, lh);
        self.surface_config.width = w;
        self.surface_config.height = h;
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn set_camera(&mut self, camera: &Camera2D) {
        self.cam = *camera;
        self.proj = camera.projection_matrix();
    }

    fn camera(&self) -> &Camera2D {
        &self.cam
    }

    fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    fn set_blend_mode(&mut self, mode: BlendMode) {
        self.current_blend = mode;
    }

    fn begin_frame(&mut self) -> Result<(), RendererError> {
        self.batcher.clear();
        Ok(())
    }

    fn push_transform(&mut self, transform: Transform2D) {
        self.transform_stack.push(transform);
    }
    fn pop_transform(&mut self) {
        self.transform_stack.pop();
    }
    fn reset_transform(&mut self) {
        self.transform_stack.reset();
    }

    fn push_clip_rect(&mut self, rect: Rect) {
        let s = self.scale_factor;
        let new_clip = [
            (rect.pos.x.max(0.0) * s) as u32,
            (rect.pos.y.max(0.0) * s) as u32,
            (rect.size.x.max(0.0) * s) as u32,
            (rect.size.y.max(0.0) * s) as u32,
        ];
        let clipped = match self.current_clip {
            Some(parent) => intersect_rects(parent, new_clip),
            None => new_clip,
        };
        self.clip_stack.push(clipped);
        self.current_clip = Some(clipped);
    }

    fn pop_clip_rect(&mut self) {
        self.clip_stack.pop();
        self.current_clip = self.clip_stack.last().copied();
    }

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
            DrawStyle::LinearGradient { color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                (c, 0.0, 0.0)
            }
            DrawStyle::RadialGradient { color_inner, .. } => {
                let mut c = color_inner;
                c.a *= params.opacity;
                (c, 0.0, 0.0)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                (c, 0.0, 0.0)
            }
        };

        let x = rect.pos.x;
        let y = rect.pos.y;
        let w = rect.size.x;
        let h = rect.size.y;

        #[rustfmt::skip]
        let mut verts: [f32; 72] = [
            x,     y,     0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x + w, y,     w,   0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x + w, y + h, w,   h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x,     y,     0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x + w, y + h, w,   h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
            x,     y + h, 0.0, h,    color.r, color.g, color.b, color.a,  mode, stroke_w, w, h,
        ];

        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.apply_transform_quad(&mut verts);
        self.batcher.push_shape(&verts, params.z_index, params.blend, self.current_clip);
    }

    fn draw_rounded_rect(&mut self, rrect: RoundedRect, params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_rounded_rect_fill(rrect, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_rounded_rect_stroke(rrect, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_rounded_rect_fill(rrect, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_rounded_rect_fill(rrect, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

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
            DrawStyle::LinearGradient { color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                (c, 2.0, 0.0)
            }
            DrawStyle::RadialGradient { color_inner, .. } => {
                let mut c = color_inner;
                c.a *= params.opacity;
                (c, 2.0, 0.0)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                (c, 2.0, 0.0)
            }
        };

        let pad = 2.0;
        let ext = radius + pad;
        let cx = center.x;
        let cy = center.y;
        let ln = ext / radius;

        #[rustfmt::skip]
        let mut verts: [f32; 72] = [
            cx - ext, cy - ext, -ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx + ext, cy - ext,  ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx + ext, cy + ext,  ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx - ext, cy - ext, -ln, -ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx + ext, cy + ext,  ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
            cx - ext, cy + ext, -ln,  ln,  color.r, color.g, color.b, color.a,  mode, stroke_w_norm, 0.0, 0.0,
        ];

        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.apply_transform_quad(&mut verts);
        self.batcher.push_shape(&verts, params.z_index, params.blend, self.current_clip);
    }

    fn draw_ellipse(&mut self, center: Vec2, radii: Vec2, params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_ellipse_fill(center, radii, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_ellipse_stroke(center, radii, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_ellipse_fill(center, radii, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_ellipse_fill(center, radii, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_arc(
        &mut self,
        center: Vec2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        params: DrawParams,
    ) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_arc_fill(center, radius, start_angle, end_angle, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_arc_stroke(center, radius, start_angle, end_angle, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_arc_fill(center, radius, start_angle, end_angle, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_arc_fill(center, radius, start_angle, end_angle, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_polygon(&mut self, points: &[Vec2], params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_convex_polygon(points, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_polygon_stroke(points, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_convex_polygon(points, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_convex_polygon(points, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_complex_polygon(&mut self, outer: &[Vec2], holes: &[&[Vec2]], params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                lite_render_2d_core::path_tessellation::tessellate_complex_polygon(outer, holes, c)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::path_tessellation::tessellate_complex_polygon_stroke(outer, holes, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                lite_render_2d_core::path_tessellation::tessellate_complex_polygon(outer, holes, c)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                lite_render_2d_core::path_tessellation::tessellate_complex_polygon(outer, holes, c)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, params: DrawParams) {
        let mut verts = match params.style {
            DrawStyle::Fill(col) => {
                let mut col = col;
                col.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_triangle(a, b, c, col)
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_triangle_stroke(a, b, c, &sp)
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut col = color_start;
                col.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_triangle(a, b, c, col)
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut col = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                col.a *= params.opacity;
                lite_render_2d_core::tessellation::tessellate_triangle(a, b, c, col)
            }
        };
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

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
        let mut verts: [f32; 72] = [
            from.x + nx, from.y + ny, 0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            from.x - nx, from.y - ny, 0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            to.x + nx,   to.y + ny,   0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            to.x + nx,   to.y + ny,   0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            from.x - nx, from.y - ny, 0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
            to.x - nx,   to.y - ny,   0.0, 0.0,  color.r, color.g, color.b, color.a,  mode, 0.0, 0.0, 0.0,
        ];

        self.apply_transform_quad(&mut verts);
        self.batcher.push_shape(&verts, params.z_index, params.blend, self.current_clip);
    }

    fn draw_polyline(&mut self, points: &[Vec2], params: LineParams) {
        let z_index = params.z_index;
        let blend = params.blend;
        let mut params = params;
        params.color.a *= params.opacity;
        let mut verts = lite_render_2d_core::tessellation::tessellate_polyline(points, &params);
        self.push_shape_raw_transformed(&mut verts, z_index, blend);
    }

    fn draw_path(&mut self, path: &Path, params: DrawParams) {
        let color = match params.style {
            DrawStyle::Fill(c) => {
                let mut c = c;
                c.a *= params.opacity;
                c
            }
            DrawStyle::Stroke(ref sp) => {
                let mut sp = *sp;
                sp.color.a *= params.opacity;
                let mut verts = lite_render_2d_core::path_tessellation::tessellate_path_stroke(path, &sp);
                self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
                return;
            }
            DrawStyle::LinearGradient { color_start, .. } | DrawStyle::RadialGradient { color_inner: color_start, .. } => {
                let mut c = color_start;
                c.a *= params.opacity;
                c
            }
            DrawStyle::LinearGradientStops { ref stops, .. } | DrawStyle::RadialGradientStops { ref stops, .. } => {
                let mut c = if stops.is_empty() { Color::WHITE } else { stops[0].color };
                c.a *= params.opacity;
                c
            }
        };
        let mut verts = lite_render_2d_core::path_tessellation::tessellate_path_fill(path, color);
        lite_render_2d_core::tessellation::apply_gradient(&mut verts, &params.style);
        self.push_shape_raw_transformed(&mut verts, params.z_index, params.blend);
    }

    fn stroke_path(&mut self, path: &Path, params: StrokeParams) {
        let mut verts = lite_render_2d_core::path_tessellation::tessellate_path_stroke(path, &params);
        self.push_shape_raw_transformed(&mut verts, 0, BlendMode::Alpha);
    }

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

    fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)> {
        self.textures.get(&handle.id()).map(|i| (i.width, i.height))
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

        // use src_rect dimensions for world-space size (not full texture)
        let (display_w, display_h) = match params.src_rect {
            Some(r) => (r.size.x, r.size.y),
            None => (tw, th),
        };

        let t = &params.transform;
        let sx = t.scale.x * display_w;
        let sy = t.scale.y * display_h;
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
        let mut verts: [f32; 54] = [
            x0, y0, u0, v0, r, g, b, a, op,
            x1, y1, u1, v1, r, g, b, a, op,
            x2, y2, u2, v2, r, g, b, a, op,
            x0, y0, u0, v0, r, g, b, a, op,
            x2, y2, u2, v2, r, g, b, a, op,
            x3, y3, u3, v3, r, g, b, a, op,
        ];

        self.apply_transform_sprite(&mut verts);
        self.batcher.push_sprite(handle.id(), &verts, params.z_index, params.blend, self.current_clip);
    }

    fn load_font(&mut self, data: &[u8]) -> Result<FontHandle, RendererError> {
        self.font_system.load_font(data).map_err(RendererError::Font)
    }

    fn unload_font(&mut self, handle: FontHandle) {
        self.font_system.unload_font(handle);
    }

    fn draw_text(&mut self, text: &str, params: &TextParams) {
        let quads = self.font_system.layout_text(text, params);
        if quads.is_empty() {
            return;
        }

        // ensure atlas texture is uploaded
        if self.font_system.is_atlas_dirty() || self.font_atlas_tex_id.is_none() {
            let (data, w, h) = self.font_system.atlas_texture_data();

            // delete old atlas texture
            if let Some(old_id) = self.font_atlas_tex_id.take() {
                self.textures.remove(&old_id);
            }

            let tex_size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("font_atlas_tex"),
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
                    texture: &texture, mip_level: 0,
                    origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h),
                },
                tex_size,
            );

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("font_atlas_sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("font_atlas_bg"),
                layout: &self.sprite_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.proj_uniform_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                ],
            });

            let id = self.next_tex_id;
            self.next_tex_id += 1;
            self.textures.insert(id, TextureInfo { bind_group, width: w, height: h });
            self.font_atlas_tex_id = Some(id);
            self.font_system.clear_dirty();
        }

        let atlas_id = self.font_atlas_tex_id.unwrap();

        for q in &quads {
            let x = q.pos.x;
            let y = q.pos.y;
            let w = q.size.x;
            let h = q.size.y;
            let u0 = q.uv.pos.x;
            let v0 = q.uv.pos.y;
            let u1 = u0 + q.uv.size.x;
            let v1 = v0 + q.uv.size.y;
            let r = q.color.r;
            let g = q.color.g;
            let b = q.color.b;
            let a = q.color.a;

            #[rustfmt::skip]
            let verts: [f32; 54] = [
                x,     y,     u0, v0, r, g, b, a, 1.0,
                x + w, y,     u1, v0, r, g, b, a, 1.0,
                x + w, y + h, u1, v1, r, g, b, a, 1.0,
                x,     y,     u0, v0, r, g, b, a, 1.0,
                x + w, y + h, u1, v1, r, g, b, a, 1.0,
                x,     y + h, u0, v1, r, g, b, a, 1.0,
            ];

            self.batcher.push_sprite(atlas_id, &verts, 0, BlendMode::Alpha, self.current_clip);
        }
    }

    fn measure_text(&mut self, text: &str, params: &TextParams) -> Vec2 {
        self.font_system.measure_text(text, params)
    }

    fn end_frame(&mut self) -> Result<FrameStats, RendererError> {
        // auto-end render-to-texture if still active
        if self.active_render_target.is_some() {
            self.end_render_to_texture();
        }

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| RendererError::Surface(e.to_string()))?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let (w, h) = (self.w, self.h);
        self.flush_to_view(&view, Some(self.clear_color), w, h);

        output.present();
        Ok(FrameStats::default())
    }

    fn create_render_target(&mut self, width: u32, height: u32) -> Result<RenderTargetHandle, RendererError> {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let tex_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rt_sprite_bind_group"),
            layout: &self.sprite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.proj_uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        // register as a texture for draw_sprite
        let tex_id = self.next_tex_id;
        self.next_tex_id += 1;
        self.textures.insert(tex_id, TextureInfo {
            bind_group,
            width,
            height,
        });

        // need a second bind group for the render target info (keep it separate)
        let rt_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rt_sprite_bind_group2"),
            layout: &self.sprite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.proj_uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        let rt_id = self.next_rt_id;
        self.next_rt_id += 1;
        self.render_targets.insert(rt_id, WgpuRenderTargetInfo {
            texture,
            view: tex_view,
            bind_group: rt_bind_group,
            texture_handle_id: tex_id,
            width,
            height,
        });

        Ok(RenderTargetHandle::new(rt_id))
    }

    fn destroy_render_target(&mut self, target: RenderTargetHandle) {
        if let Some(rt) = self.render_targets.remove(&target.id()) {
            self.textures.remove(&rt.texture_handle_id);
        }
    }

    fn begin_render_to_texture(&mut self, target: RenderTargetHandle) -> Result<(), RendererError> {
        let rt_id = target.id();
        if !self.render_targets.contains_key(&rt_id) {
            return Err(RendererError::Other("invalid render target".into()));
        }

        // flush pending draws to curent target
        if let Some(view_id) = self.active_render_target {
            if self.render_targets.contains_key(&view_id) {
                // flush woud go here but begin is typicaly called after begin_frame
            }
        }
        // for simplicity, flush to a temporary - we handle this by just clearing the batcher
        // since begin is typically called right after begin_frame
        self.batcher.clear();

        self.saved_viewport = Some((self.w, self.h));
        self.saved_proj = Some(self.proj);

        let rt = self.render_targets.get(&rt_id).unwrap();
        self.proj = screen_ortho(rt.width, rt.height);
        self.active_render_target = Some(rt_id);

        // clear the render target
        let rt_view = &self.render_targets[&rt_id].view;
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rt_clear"),
        });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rt_clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: rt_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
        }
        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    fn end_render_to_texture(&mut self) {
        if let Some(rt_id) = self.active_render_target.take() {
            // flush batch to the RT view
            if let Some(rt) = self.render_targets.get(&rt_id) {
                let (rt_w, rt_h) = (rt.width, rt.height);
                // borrow workaround: we need the view but also &mut self
                // use raw pointer trick (safe because we don't modify render_targets during flush)
                let view_ptr = &rt.view as *const wgpu::TextureView;
                let view_ref = unsafe { &*view_ptr };
                self.flush_to_view(view_ref, None, rt_w, rt_h);
            }
            self.batcher.clear();

            if let Some((w, h)) = self.saved_viewport.take() {
                self.w = w;
                self.h = h;
            }
            if let Some(proj) = self.saved_proj.take() {
                self.proj = proj;
            }
        }
    }

    fn render_target_texture(&self, target: RenderTargetHandle) -> Option<TextureHandle> {
        self.render_targets.get(&target.id())
            .map(|rt| TextureHandle::new(rt.texture_handle_id))
    }

    fn apply_post_effect(&mut self, effect: &PostEffect, source: RenderTargetHandle) {
        // blur/bloom not yet implemented for wgpu backend
        match effect {
            PostEffect::Blur(_) | PostEffect::Bloom { .. } => return,
            _ => {}
        }

        let rt_id = source.id();
        let rt = match self.render_targets.get(&rt_id) {
            Some(rt) => rt,
            None => return,
        };

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("effect_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let (pipeline, bind_group) = match effect {
            PostEffect::Brightness(val) => {
                self.queue.write_buffer(&self.effect_brightness_uniform, 0, bytemuck::cast_slice(&[*val]));
                let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("effect_brightness_bg"),
                    layout: &self.effect_bgl_brightness,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&rt.view) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                        wgpu::BindGroupEntry { binding: 2, resource: self.effect_brightness_uniform.as_entire_binding() },
                    ],
                });
                (&self.effect_brightness_pipeline, bg)
            }
            _ => {
                let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("effect_bg"),
                    layout: &self.effect_bgl,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&rt.view) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                    ],
                });
                let pipe = match effect {
                    PostEffect::Grayscale => &self.effect_grayscale_pipeline,
                    PostEffect::Invert => &self.effect_invert_pipeline,
                    PostEffect::Vignette => &self.effect_vignette_pipeline,
                    _ => return, // blur/bloom handled above
                };
                (pipe, bg)
            }
        };

        // get current render target view (screen or another RT)
        // for now, draw to the active surface
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("effect_encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("effect_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn draw_sprite_instanced(
        &mut self,
        handle: TextureHandle,
        instances: &[SpriteInstance],
        blend: BlendMode,
        z_index: i32,
    ) {
        // use default fallback (individual draws)
        for inst in instances {
            self.draw_sprite(handle, SpriteParams {
                transform: inst.transform,
                tint: inst.tint,
                opacity: inst.opacity,
                src_rect: inst.src_rect,
                flip_x: inst.flip_x,
                flip_y: inst.flip_y,
                blend,
                z_index,
            });
        }
    }

    // -- sdf text --

    fn load_sdf_font(&mut self, data: &[u8]) -> Result<FontHandle, RendererError> {
        self.sdf_font_system.load_font(data).map_err(RendererError::Font)
    }

    fn unload_sdf_font(&mut self, handle: FontHandle) {
        self.sdf_font_system.unload_font(handle);
    }

    fn draw_sdf_text(&mut self, text: &str, params: &TextParams) {
        let quads = self.sdf_font_system.layout_text(text, params);
        if quads.is_empty() {
            return;
        }

        // upload sdf atlas if dirty
        if self.sdf_font_system.is_atlas_dirty() || self.sdf_atlas_tex_id.is_none() {
            let (data, w, h) = self.sdf_font_system.atlas_texture_data();
            if let Some(old_id) = self.sdf_atlas_tex_id.take() {
                self.textures.remove(&old_id);
            }

            let tex_size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sdf_atlas_tex"),
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
                    texture: &texture, mip_level: 0,
                    origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h),
                },
                tex_size,
            );

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("sdf_atlas_sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sdf_atlas_bg"),
                layout: &self.sprite_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.proj_uniform_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                ],
            });

            let id = self.next_tex_id;
            self.next_tex_id += 1;
            self.textures.insert(id, TextureInfo { bind_group, width: w, height: h });
            self.sdf_atlas_tex_id = Some(id);
            self.sdf_font_system.clear_dirty();
        }

        let atlas_id = self.sdf_atlas_tex_id.unwrap();

        for q in &quads {
            let x = q.pos.x;
            let y = q.pos.y;
            let w = q.size.x;
            let h = q.size.y;
            let u0 = q.uv.pos.x;
            let v0 = q.uv.pos.y;
            let u1 = u0 + q.uv.size.x;
            let v1 = v0 + q.uv.size.y;
            let r = q.color.r;
            let g = q.color.g;
            let b = q.color.b;
            let a = q.color.a;

            #[rustfmt::skip]
            let verts: [f32; 54] = [
                x,     y,     u0, v0, r, g, b, a, 1.0,
                x + w, y,     u1, v0, r, g, b, a, 1.0,
                x + w, y + h, u1, v1, r, g, b, a, 1.0,
                x,     y,     u0, v0, r, g, b, a, 1.0,
                x + w, y + h, u1, v1, r, g, b, a, 1.0,
                x,     y + h, u0, v1, r, g, b, a, 1.0,
            ];

            self.batcher.push_sdf_sprite(atlas_id, &verts, 0, BlendMode::Alpha, self.current_clip);
        }
    }

    fn measure_sdf_text(&mut self, text: &str, params: &TextParams) -> Vec2 {
        self.sdf_font_system.measure_text(text, params)
    }

    // -- rich text --

    fn draw_rich_text(&mut self, rich: &lite_render_2d_core::rich_text::RichText) {
        let quads = lite_render_2d_core::rich_text::layout_rich_text(rich, &mut self.font_system);
        if quads.is_empty() {
            return;
        }

        // ensure font atlas uploaded
        if self.font_system.is_atlas_dirty() || self.font_atlas_tex_id.is_none() {
            let (data, w, h) = self.font_system.atlas_texture_data();
            if let Some(old_id) = self.font_atlas_tex_id.take() {
                self.textures.remove(&old_id);
            }

            let tex_size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("font_atlas_tex"),
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
                    texture: &texture, mip_level: 0,
                    origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h),
                },
                tex_size,
            );

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("font_atlas_sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("font_atlas_bg"),
                layout: &self.sprite_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.proj_uniform_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                ],
            });

            let id = self.next_tex_id;
            self.next_tex_id += 1;
            self.textures.insert(id, TextureInfo { bind_group, width: w, height: h });
            self.font_atlas_tex_id = Some(id);
            self.font_system.clear_dirty();
        }

        let atlas_id = self.font_atlas_tex_id.unwrap();

        for q in &quads {
            let x = q.pos.x;
            let y = q.pos.y;
            let w = q.size.x;
            let h = q.size.y;
            let u0 = q.uv.pos.x;
            let v0 = q.uv.pos.y;
            let u1 = u0 + q.uv.size.x;
            let v1 = v0 + q.uv.size.y;
            let r = q.color.r;
            let g = q.color.g;
            let b = q.color.b;
            let a = q.color.a;

            #[rustfmt::skip]
            let verts: [f32; 54] = [
                x,     y,     u0, v0, r, g, b, a, 1.0,
                x + w, y,     u1, v0, r, g, b, a, 1.0,
                x + w, y + h, u1, v1, r, g, b, a, 1.0,
                x,     y,     u0, v0, r, g, b, a, 1.0,
                x + w, y + h, u1, v1, r, g, b, a, 1.0,
                x,     y + h, u0, v1, r, g, b, a, 1.0,
            ];

            self.batcher.push_sprite(atlas_id, &verts, 0, BlendMode::Alpha, self.current_clip);
        }
    }

    fn measure_rich_text(&mut self, rich: &lite_render_2d_core::rich_text::RichText) -> Vec2 {
        lite_render_2d_core::rich_text::measure_rich_text(rich, &mut self.font_system)
    }
}
