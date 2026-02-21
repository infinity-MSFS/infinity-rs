use crate::nvg::color::Color;
use crate::nvg::context::NvgContext;
use crate::sys;

/// Anything that can be used as a fill or stroke style.
///
/// Implemented for [`Color`] (solid), [`Gradient`], and [`ImagePattern`].
pub trait FillStyle {
    fn apply_fill(&self, ctx: &NvgContext);
    fn apply_stroke(&self, ctx: &NvgContext);
}

impl FillStyle for Color {
    #[inline]
    fn apply_fill(&self, ctx: &NvgContext) {
        ctx.fill_color(*self);
    }
    #[inline]
    fn apply_stroke(&self, ctx: &NvgContext) {
        ctx.stroke_color(*self);
    }
}

// Gradient

/// A gradient paint. Created via `Gradient::linear`, `Gradient::radial`, or `Gradient::box_`.
///
/// ```rust
/// let bg = Gradient::linear(0.0, 0.0, 0.0, 100.0,
///     Color::hex(0x1A237EFF), Color::hex(0x0D47A1FF));
///
/// Shape::rect(0.0, 0.0, 400.0, 100.0)
///     .fill(bg)
///     .draw(ctx);
/// ```
#[derive(Clone, Copy)]
pub struct Gradient {
    pub(crate) raw: sys::NVGpaint,
}

impl Gradient {
    /// Linear gradient from `(sx, sy)` to `(ex, ey)`.
    pub fn linear(
        ctx: &NvgContext,
        sx: f32,
        sy: f32,
        ex: f32,
        ey: f32,
        inner: Color,
        outer: Color,
    ) -> Self {
        let raw = unsafe {
            sys::nvgLinearGradient(
                ctx.raw(),
                sx,
                sy,
                ex,
                ey,
                inner.into_raw(),
                outer.into_raw(),
            )
        };
        Self { raw }
    }

    /// Radial gradient centered at `(cx, cy)`.
    pub fn radial(
        ctx: &NvgContext,
        cx: f32,
        cy: f32,
        inner_radius: f32,
        outer_radius: f32,
        inner: Color,
        outer: Color,
    ) -> Self {
        let raw = unsafe {
            sys::nvgRadialGradient(
                ctx.raw(),
                cx,
                cy,
                inner_radius,
                outer_radius,
                inner.into_raw(),
                outer.into_raw(),
            )
        };
        Self { raw }
    }

    /// Box gradient: a feathered rounded rectangle.
    /// Great for drop shadows and highlights.
    ///
    /// ```rust
    /// let shadow = Gradient::box_(ctx, x, y, w, h, 8.0, 12.0,
    ///     Color::BLACK.with_alpha(0.5), Color::TRANSPARENT);
    /// ```
    pub fn box_(
        ctx: &NvgContext,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radius: f32,
        feather: f32,
        inner: Color,
        outer: Color,
    ) -> Self {
        let raw = unsafe {
            sys::nvgBoxGradient(
                ctx.raw(),
                x,
                y,
                w,
                h,
                radius,
                feather,
                inner.into_raw(),
                outer.into_raw(),
            )
        };
        Self { raw }
    }
}

impl FillStyle for Gradient {
    #[inline]
    fn apply_fill(&self, ctx: &NvgContext) {
        unsafe { sys::nvgFillPaint(ctx.raw(), self.raw) };
    }
    #[inline]
    fn apply_stroke(&self, ctx: &NvgContext) {
        unsafe { sys::nvgStrokePaint(ctx.raw(), self.raw) };
    }
}

// Image pattern

/// An image pattern fill.
///
/// ```rust
/// let pattern = ImagePattern::new(ctx, 0.0, 0.0, 64.0, 64.0, 0.0, img_handle, 1.0);
/// Shape::rect(0.0, 0.0, 200.0, 200.0)
///     .fill(pattern)
///     .draw(ctx);
/// ```
#[derive(Clone, Copy)]
pub struct ImagePattern {
    pub(crate) raw: sys::NVGpaint,
}

impl ImagePattern {
    /// Create a repeating image pattern.
    ///
    /// - `(ox, oy)` — top-left origin of the pattern
    /// - `(ex, ey)` — size of one tile
    /// - `angle` — rotation in radians
    /// - `image` — image handle from `ctx.create_image_*`
    /// - `alpha` — opacity `[0.0, 1.0]`
    pub fn new(
        ctx: &NvgContext,
        ox: f32,
        oy: f32,
        ex: f32,
        ey: f32,
        angle: f32,
        image: i32,
        alpha: f32,
    ) -> Self {
        let raw = unsafe { sys::nvgImagePattern(ctx.raw(), ox, oy, ex, ey, angle, image, alpha) };
        Self { raw }
    }
}

impl FillStyle for ImagePattern {
    #[inline]
    fn apply_fill(&self, ctx: &NvgContext) {
        unsafe { sys::nvgFillPaint(ctx.raw(), self.raw) };
    }
    #[inline]
    fn apply_stroke(&self, ctx: &NvgContext) {
        unsafe { sys::nvgStrokePaint(ctx.raw(), self.raw) };
    }
}
