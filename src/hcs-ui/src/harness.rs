//! Headless render harness: renders a Slint component to a PNG with the
//! software renderer, so GUI views can be tested and diffed on machines with
//! no GPU, no display and no Wayland compositor (CI, VirtualBox gates).
//!
//! Plan: docs/GUI_BUILD_PLAN.md §4 steps 2-3.

use slint::platform::{set_platform, Platform, PlatformError, WindowAdapter};
use std::path::Path;
use std::rc::Rc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("platform error: {0}")]
    Platform(String),
    #[error("invalid render size: {width}x{height}")]
    InvalidSize { width: u32, height: u32 },
    #[error("io error: {0}")]
    Io(String),
}

/// Re-export of the software renderer's window type so the render tool and
/// app crates can name it without depending on `slint`'s internals directly.
pub use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};

/// Minimal display-less platform backed by the software renderer.
pub struct HeadlessPlatform {
    window: Rc<MinimalSoftwareWindow>,
}

impl Platform for HeadlessPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }
    fn duration_since_start(&self) -> core::time::Duration {
        core::time::Duration::from_millis(0)
    }
}

impl HeadlessPlatform {
    /// Install the headless platform. Must be called at most once per process.
    pub fn install() -> Result<Rc<MinimalSoftwareWindow>, RenderError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        set_platform(Box::new(HeadlessPlatform {
            window: window.clone(),
        }))
        .map_err(|e| RenderError::Platform(format!("{e}")))?;
        Ok(window)
    }
}

/// Render any Slint component into a PNG file.
pub fn render_to_png<C: slint::ComponentHandle>(
    component: &C,
    window: &Rc<MinimalSoftwareWindow>,
    width: u32,
    height: u32,
    out: &Path,
) -> Result<(), RenderError> {
    if width == 0 || height == 0 {
        return Err(RenderError::InvalidSize { width, height });
    }
    if let Some(p) = out.parent() {
        std::fs::create_dir_all(p).map_err(|e| RenderError::Io(e.to_string()))?;
    }

    // Size must be set through the WindowAdapter trait: the inherent
    // MinimalSoftwareWindow::set_size shadows it and does not dispatch the
    // Resized event, which leaves the scene unlaid-out (blank render).
    let size = slint::WindowSize::Logical(slint::LogicalSize::new(width as f32, height as f32));
    WindowAdapter::set_size(&**window, size.clone());
    component
        .show()
        .map_err(|e| RenderError::Platform(format!("{e}")))?;
    WindowAdapter::set_size(&**window, size);

    let px = (width as usize) * (height as usize);
    let mut buf: Vec<PremultipliedRgbaColor> = vec![
        PremultipliedRgbaColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        };
        px
    ];

    // First pass lays out and paints; a second pass settles lazy bindings.
    for _ in 0..3 {
        window.draw_if_needed(|renderer| {
            renderer.render(&mut buf, width as usize);
        });
    }

    let img = image::RgbaImage::from_raw(width, height, unpremultiply(&buf))
        .ok_or(RenderError::InvalidSize { width, height })?;
    img.save(out).map_err(|e| RenderError::Io(e.to_string()))?;
    component
        .hide()
        .map_err(|e| RenderError::Platform(format!("{e}")))?;
    Ok(())
}

/// Convert the software renderer's premultiplied RGBA into straight-alpha RGBA.
/// The multiply must happen in u16: doing it in u8 saturates and yields (1,1,1).
fn unpremultiply(buf: &[PremultipliedRgbaColor]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len() * 4);
    for p in buf {
        let a = p.alpha as u16;
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let f = |c: u8| (((c as u16) * 255) / a).min(255) as u8;
            out.extend_from_slice(&[f(p.red), f(p.green), f(p.blue), p.alpha]);
        }
    }
    out
}
