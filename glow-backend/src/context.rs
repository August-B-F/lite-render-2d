use std::num::NonZeroU32;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, Version};
use glutin::display::GlDisplay;
use glutin::surface::{GlSurface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use lite_render_2d_core::RendererError;

pub type Surface = glutin::surface::Surface<WindowSurface>;
pub type GlContext = glutin::context::PossiblyCurrentContext;

pub fn create_gl_context(
    window: &Window,
) -> Result<(glow::Context, Surface, GlContext), RendererError> {
    let raw_window = window
        .window_handle()
        .expect("window handle")
        .as_raw();

    // platform-specifc display preference
    #[cfg(windows)]
    let pref = glutin::display::DisplayApiPreference::Wgl(Some(raw_window));
    #[cfg(target_os = "linux")]
    let pref = glutin::display::DisplayApiPreference::Egl;
    #[cfg(target_os = "macos")]
    let pref = glutin::display::DisplayApiPreference::Cgl;

    let raw_display = window
        .display_handle()
        .expect("display handle")
        .as_raw();

    let display = unsafe { glutin::display::Display::new(raw_display, pref) }
        .map_err(|e| RendererError::ContextCreation(e.to_string()))?;

    // no depth/stencil for 2d stuf
    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_depth_size(0)
        .with_stencil_size(0)
        .compatible_with_native_window(raw_window)
        .build();

    let config = unsafe { display.find_configs(template) }
        .map_err(|e| RendererError::ContextCreation(e.to_string()))?
        .next()
        .ok_or_else(|| RendererError::ContextCreation("no suitable gl config".into()))?;

    // try es 3.0 first, fallback to gl 3.3 if es isnt availble
    let es3 = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
        .build(Some(raw_window));

    let gl33 = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
        .build(Some(raw_window));

    let ctx = unsafe { display.create_context(&config, &es3) }
        .or_else(|_| unsafe { display.create_context(&config, &gl33) })
        .map_err(|e| RendererError::ContextCreation(e.to_string()))?;

    let size = window.inner_size();
    let w = NonZeroU32::new(size.width.max(1)).unwrap();
    let h = NonZeroU32::new(size.height.max(1)).unwrap();

    let surface_attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(raw_window, w, h);
    let surface = unsafe { display.create_window_surface(&config, &surface_attrs) }
        .map_err(|e| RendererError::Surface(e.to_string()))?;

    let ctx = ctx
        .make_current(&surface)
        .expect("make gl context current");

    // vsync on, dont fail if it doesnt work
    let _ = surface.set_swap_interval(&ctx, SwapInterval::Wait(NonZeroU32::new(1).unwrap()));

    let gl = unsafe {
        glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s))
    };

    Ok((gl, surface, ctx))
}
