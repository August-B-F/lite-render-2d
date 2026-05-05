# Migrating from wgpu

If you're coming from raw wgpu, this guide shows you what to replace and what to delete.

## What you're replacing

| wgpu | lite-render-2d |
|------|----------------|
| `Instance`, `Adapter`, `Device`, `Queue`, `Surface` setup | `GlowRenderer::new(&window)` |
| `RenderPipeline`, `ShaderModule`, `PipelineLayout` | Nothing (baked in) |
| `BindGroup`, `BindGroupLayout` | Nothing (managed internally) |
| `Buffer` + vertex data + `write_buffer` | Nothing (batcher handles it) |
| `wgpu::Texture` + `write_texture` | `load_texture_from_file()` |
| Render pass + draw call with quad vertices | `draw_sprite()` / `draw_rect()` |
| Orthographic projection matrix uniform | `set_camera()` |
| Vertex/index buffer management | Nothing (automatic) |
| Texture atlas + UV calculations | `SpriteParams.src_rect` or `TextureAtlas` |
| Frame encoder + submit | `begin_frame()` / `end_frame()` |

## Step by step

### 1. Delete all wgpu setup code

Delete your device, queue, surface, adapter, instance creation. Delete pipeline creation, shader modules, bind groups, and layouts. Delete vertex/index buffer creation and management.

Replace with one line:

```rust
let mut renderer = GlowRenderer::new(&window)?;
```

### 2. Replace texture loading

**Before (wgpu):**
```rust
let image = image::open("sprite.png")?.to_rgba8();
let size = wgpu::Extent3d {
    width: image.width(),
    height: image.height(),
    depth_or_array_layers: 1,
};
let texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some("sprite"),
    size,
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8UnormSrgb,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    view_formats: &[],
});
queue.write_texture(
    wgpu::ImageCopyTexture { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
    &image,
    wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * image.width()), rows_per_image: Some(image.height()) },
    size,
);
let view = texture.create_view(&Default::default());
let sampler = device.create_sampler(&wgpu::SamplerDescriptor { /* ... */ });
let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { /* ... */ });
```

**After (lite-render-2d):**
```rust
let tex = renderer.load_texture_from_file("sprite.png".as_ref(), TextureParams::default())?;
```

### 3. Replace drawing

**Before (wgpu):**
```rust
let mut encoder = device.create_command_encoder(&Default::default());
{
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("render"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &surface_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    render_pass.set_pipeline(&pipeline);
    render_pass.set_bind_group(0, &bind_group, &[]);
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    render_pass.draw_indexed(0..6, 0, 0..1);
}
queue.submit(std::iter::once(encoder.finish()));
surface.get_current_texture()?.present();
```

**After (lite-render-2d):**
```rust
renderer.begin_frame()?;
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(100.0, 200.0)));
renderer.draw_rect(Rect::new(50.0, 50.0, 200.0, 100.0), DrawParams::fill(Color::RED));
renderer.end_frame()?;
```

### 4. Replace camera/projection

**Before (wgpu):**
```rust
let proj = cgmath::ortho(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);
queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[proj]));
render_pass.set_bind_group(1, &camera_bind_group, &[]);
```

**After (lite-render-2d):**
```rust
let camera = Camera2D::new(width as f32, height as f32);
renderer.set_camera(&camera);
```

### 5. Delete vertex/index buffer management

You no longer need to:
- Define vertex structs
- Create vertex/index buffers
- Calculate UVs manually
- Manage buffer capacity
- Track draw call batching

The renderer handles all of this internally.

## Before (complete wgpu example, ~80 lines)

```rust
// Setup: ~30 lines of instance/adapter/device/queue/surface
// Pipeline: ~20 lines of shader/pipeline/bindgroup
// Buffers: ~15 lines of vertex/index buffer creation
// Texture: ~15 lines of image load + write_texture + view + sampler + bind_group

// Per frame: ~15 lines
let output = surface.get_current_texture()?;
let view = output.texture.create_view(&Default::default());
let mut encoder = device.create_command_encoder(&Default::default());
{
    let mut pass = encoder.begin_render_pass(/* ... */);
    pass.set_pipeline(&pipeline);
    pass.set_bind_group(0, &texture_bind, &[]);
    pass.set_bind_group(1, &camera_bind, &[]);
    pass.set_vertex_buffer(0, vbo.slice(..));
    pass.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint16);
    pass.draw_indexed(0..6, 0, 0..1);
}
queue.submit([encoder.finish()]);
output.present();
```

## After (complete lite-render-2d equivalent, ~10 lines)

```rust
// Setup: 2 lines
let mut renderer = GlowRenderer::new(&window)?;
let tex = renderer.load_texture_from_file("sprite.png".as_ref(), TextureParams::default())?;

// Per frame: 4 lines
renderer.begin_frame()?;
renderer.set_camera(&Camera2D::new(800.0, 600.0));
renderer.draw_sprite(tex, SpriteParams::new(Transform2D::new(100.0, 200.0)));
renderer.end_frame()?;
```

## What if you need wgpu's power?

lite-render-2d also has a wgpu backend:

```rust
use lite_render_2d_wgpu::WgpuRenderer;
let renderer = WgpuRenderer::new(&window)?;
```

Same API, but backed by wgpu internally. You get the simple API with wgpu's broader GPU support (Vulkan, Metal, DX12).

For truly custom rendering (compute shaders, custom pipelines, advanced GPU features), wgpu is still the right choice. lite-render-2d is designed for 2D games and apps where you want to draw shapes, sprites, and text without managing GPU state.
