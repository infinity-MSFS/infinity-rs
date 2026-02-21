use crate::context::Context;
use crate::nvg::color::Color;
use crate::nvg::enums::*;
use crate::nvg::path::PathBuilder;
use crate::nvg::render;
use crate::nvg::text::{TextBounds, TextMetrics};
use crate::nvg::transform::Transform;
use crate::sys;

use std::ffi::CString;

/// Safe, owned wrapper around an `NVGcontext*`.
///
/// Created from a [`Context`] (or raw `FsContext`) during gauge init,
/// destroyed automatically when dropped.
///
/// # Lifecycle
///
/// ```rust
/// pub struct MyGauge {
///     nvg: Option<NvgContext>,
///     font: Option<i32>,
/// }
///
/// impl Gauge for MyGauge {
///     fn init(&mut self, ctx: &Context, install: &mut GaugeInstall) -> bool {
///         let nvg = NvgContext::new(ctx).expect("NVG init failed");
///         self.font = nvg.create_font("sans", "./data/Roboto-Regular.ttf");
///         self.nvg = Some(nvg);
///         true
///     }
///
///     fn draw(&mut self, ctx: &Context, draw: &mut GaugeDraw) -> bool {
///         let nvg = self.nvg.as_ref().unwrap();
///         let px_ratio = draw.fb_width() as f32 / draw.win_width();
///
///         nvg.frame(draw.win_width(), draw.win_height(), px_ratio, |nvg| {
///             Shape::circle(100.0, 100.0, 40.0)
///                 .fill(Color::RED)
///                 .draw(nvg);
///         });
///         true
///     }
///
///     fn kill(&mut self, _ctx: &Context) -> bool {
///         self.nvg = None; // calls nvgDeleteInternal via Drop
///         true
///     }
/// }
/// ```
pub struct NvgContext {
    raw: *mut sys::NVGcontext,
}

unsafe impl Send for NvgContext {} // Not needed since the wasm module is single threaded, but this allows it to be used in global states that require Send (poor coding practices, but we can allow it)

// Lifecycle
impl NvgContext {
    /// Create a new NanoVG context from a [`Context`].
    ///
    /// This is the primary constructor. It calls `nvgCreateInternal` with render
    /// callbacks routed through the MSFS `fsRender*` functions.
    ///
    /// ```rust
    /// let nvg = NvgContext::new(ctx).expect("NVG init failed");
    /// ```
    pub fn new(ctx: &Context) -> Option<Self> {
        unsafe { Self::from_fs_context(ctx.fs_context()) }
    }

    /// Create a new NanoVG context with edge anti-aliasing disabled.
    ///
    /// Useful when performance matters more than smooth edges.
    pub fn new_no_aa(ctx: &Context) -> Option<Self> {
        unsafe {
            let mut params = render::build_nvg_params(ctx.fs_context());
            params.edgeAntiAlias = 0;
            let raw = sys::nvgCreateInternal(&mut params);
            if raw.is_null() {
                None
            } else {
                Some(Self { raw })
            }
        }
    }

    /// Create from a raw `FsContext` pointer directly.
    ///
    /// Prefer [`new`](Self::new) when you have a [`Context`].
    ///
    /// # Safety
    /// `fs_ctx` must be a valid `FsContext` that outlives this `NvgContext`.
    pub unsafe fn from_fs_context(fs_ctx: sys::FsContext) -> Option<Self> {
        unsafe {
            let mut params = render::build_nvg_params(fs_ctx);
            let raw = sys::nvgCreateInternal(&mut params);
            if raw.is_null() {
                None
            } else {
                Some(Self { raw })
            }
        }
    }

    #[inline]
    pub fn raw(&self) -> *mut sys::NVGcontext {
        self.raw
    }
}

impl Drop for NvgContext {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { sys::nvgDeleteInternal(self.raw) };
            self.raw = std::ptr::null_mut();
        }
    }
}

// Frame
impl NvgContext {
    pub fn begin_frame(&self, width: f32, height: f32, device_pixel_ratio: f32) {
        unsafe { sys::nvgBeginFrame(self.raw, width, height, device_pixel_ratio) }
    }

    pub fn cancel_frame(&self) {
        unsafe { sys::nvgCancelFrame(self.raw) }
    }

    pub fn end_frame(&self) {
        unsafe { sys::nvgEndFrame(self.raw) }
    }

    /// Execute a closure within a begin/end frame pair.
    ///
    /// ```rust
    /// nvg.frame(win_w, win_h, px_ratio, |nvg| {
    ///     // all drawing here
    /// });
    /// ```
    pub fn frame<F: FnOnce(&Self)>(&self, w: f32, h: f32, dpr: f32, f: F) {
        self.begin_frame(w, h, dpr);
        f(self);
        self.end_frame();
    }
}

// State
impl NvgContext {
    pub fn save(&self) {
        unsafe { sys::nvgSave(self.raw) }
    }

    pub fn restore(&self) {
        unsafe { sys::nvgRestore(self.raw) };
    }

    pub fn reset(&self) {
        unsafe { sys::nvgReset(self.raw) };
    }

    /// Execute a closure with automatic save/restore.
    ///
    /// ```rust
    /// ctx.scoped(|ctx| {
    ///     ctx.translate(100.0, 50.0);
    ///     ctx.rotate(0.5);
    ///     // state restored when closure returns
    /// });
    /// ```
    pub fn scoped<F: FnOnce(&Self)>(&self, f: F) {
        self.save();
        f(self);
        self.restore();
    }
}

// Composite operations
impl NvgContext {
    pub fn global_composite_operation(&self, op: CompositeOp) {
        unsafe { sys::nvgGlobalCompositeOperation(self.raw, op as i32) };
    }

    pub fn global_composite_blend_func(&self, src: BlendFactor, dst: BlendFactor) {
        unsafe { sys::nvgGlobalCompositeBlendFunc(self.raw, src as i32, dst as i32) };
    }

    pub fn global_composite_blend_func_separate(
        &self,
        src_rgb: BlendFactor,
        dst_rgb: BlendFactor,
        src_alpha: BlendFactor,
        dst_alpha: BlendFactor,
    ) {
        unsafe {
            sys::nvgGlobalCompositeBlendFuncSeparate(
                self.raw,
                src_rgb as i32,
                dst_rgb as i32,
                src_alpha as i32,
                dst_alpha as i32,
            )
        };
    }
}

// Render style
impl NvgContext {
    pub fn shape_anti_alias(&self, enabled: bool) {
        unsafe { sys::nvgShapeAntiAlias(self.raw, enabled as i32) };
    }

    pub fn stroke_color(&self, color: Color) {
        unsafe { sys::nvgStrokeColor(self.raw, color.into_raw()) };
    }

    pub fn stroke_paint(&self, paint: sys::NVGpaint) {
        unsafe { sys::nvgStrokePaint(self.raw, paint) };
    }

    pub fn fill_color(&self, color: Color) {
        unsafe { sys::nvgFillColor(self.raw, color.into_raw()) };
    }

    pub fn fill_paint(&self, paint: sys::NVGpaint) {
        unsafe { sys::nvgFillPaint(self.raw, paint) };
    }

    pub fn miter_limit(&self, limit: f32) {
        unsafe { sys::nvgMiterLimit(self.raw, limit) };
    }

    pub fn stroke_width(&self, width: f32) {
        unsafe { sys::nvgStrokeWidth(self.raw, width) };
    }

    pub fn line_cap(&self, cap: LineCap) {
        unsafe { sys::nvgLineCap(self.raw, cap as i32) };
    }

    pub fn line_join(&self, join: LineJoin) {
        unsafe { sys::nvgLineJoin(self.raw, join as i32) };
    }

    pub fn global_alpha(&self, alpha: f32) {
        unsafe { sys::nvgGlobalAlpha(self.raw, alpha) };
    }
}

// Transforms
impl NvgContext {
    pub fn reset_transform(&self) {
        unsafe { sys::nvgResetTransform(self.raw) };
    }

    pub fn set_transform(&self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        unsafe { sys::nvgTransform(self.raw, a, b, c, d, e, f) };
    }

    /// Apply a [`Transform`] to the current coordinate system.
    pub fn apply_transform(&self, t: &Transform) {
        self.set_transform(t.m[0], t.m[1], t.m[2], t.m[3], t.m[4], t.m[5]);
    }

    pub fn translate(&self, x: f32, y: f32) {
        unsafe { sys::nvgTranslate(self.raw, x, y) };
    }

    pub fn rotate(&self, angle: f32) {
        unsafe { sys::nvgRotate(self.raw, angle) };
    }

    pub fn skew_x(&self, angle: f32) {
        unsafe { sys::nvgSkewX(self.raw, angle) };
    }

    pub fn skew_y(&self, angle: f32) {
        unsafe { sys::nvgSkewY(self.raw, angle) };
    }

    pub fn scale(&self, x: f32, y: f32) {
        unsafe { sys::nvgScale(self.raw, x, y) };
    }

    pub fn current_transform(&self) -> Transform {
        let mut m = [0.0f32; 6];
        unsafe { sys::nvgCurrentTransform(self.raw, m.as_mut_ptr()) };
        Transform { m }
    }
}

// Scissor
impl NvgContext {
    pub fn scissor(&self, x: f32, y: f32, w: f32, h: f32) {
        unsafe { sys::nvgScissor(self.raw, x, y, w, h) };
    }

    pub fn intersect_scissor(&self, x: f32, y: f32, w: f32, h: f32) {
        unsafe { sys::nvgIntersectScissor(self.raw, x, y, w, h) };
    }

    pub fn reset_scissor(&self) {
        unsafe { sys::nvgResetScissor(self.raw) };
    }

    pub fn reset_stencil(&self) {
        unsafe { sys::nvgResetStencil(self.raw) };
    }
}

// Asobo extenstions
impl NvgContext {
    pub fn select_path(&self, index: i32) {
        unsafe { sys::nvgSelectPath(self.raw, index) };
    }

    pub fn current_path_index(&self) -> i32 {
        unsafe { sys::nvgCurrentPath(self.raw) }
    }

    pub fn set_buffer(&self, buffer: i32) {
        unsafe { sys::nvgSetBuffer(self.raw, buffer) };
    }

    pub fn set_clip_mode(&self, mode: ClipMode) {
        unsafe { sys::nvgSetClipMode(self.raw, mode as i32) };
    }

    pub fn set_clipped(&self, clipped: bool) {
        unsafe { sys::nvgSetClipped(self.raw, clipped) };
    }
}

// Paths
impl NvgContext {
    pub fn begin_path(&self) {
        unsafe { sys::nvgBeginPath(self.raw) };
    }

    pub fn move_to(&self, x: f32, y: f32) {
        unsafe { sys::nvgMoveTo(self.raw, x, y) };
    }

    pub fn line_to(&self, x: f32, y: f32) {
        unsafe { sys::nvgLineTo(self.raw, x, y) };
    }

    pub fn bezier_to(&self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        unsafe { sys::nvgBezierTo(self.raw, c1x, c1y, c2x, c2y, x, y) };
    }

    pub fn quad_to(&self, cx: f32, cy: f32, x: f32, y: f32) {
        unsafe { sys::nvgQuadTo(self.raw, cx, cy, x, y) };
    }

    pub fn arc_to(&self, x1: f32, y1: f32, x2: f32, y2: f32, radius: f32) {
        unsafe { sys::nvgArcTo(self.raw, x1, y1, x2, y2, radius) };
    }

    pub fn close_path(&self) {
        unsafe { sys::nvgClosePath(self.raw) };
    }

    pub fn path_winding(&self, dir: Winding) {
        unsafe { sys::nvgPathWinding(self.raw, dir as i32) };
    }

    pub fn arc(&self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32, dir: Winding) {
        unsafe { sys::nvgArc(self.raw, cx, cy, r, a0, a1, dir as i32) };
    }

    pub fn elliptical_arc(
        &self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        a0: f32,
        a1: f32,
        dir: Winding,
    ) {
        unsafe { sys::nvgEllipticalArc(self.raw, cx, cy, rx, ry, a0, a1, dir as i32) };
    }

    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) {
        unsafe { sys::nvgRect(self.raw, x, y, w, h) };
    }

    pub fn rounded_rect(&self, x: f32, y: f32, w: f32, h: f32, r: f32) {
        unsafe { sys::nvgRoundedRect(self.raw, x, y, w, h, r) };
    }

    pub fn rounded_rect_varying(
        &self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        tl: f32,
        tr: f32,
        br: f32,
        bl: f32,
    ) {
        unsafe { sys::nvgRoundedRectVarying(self.raw, x, y, w, h, tl, tr, br, bl) };
    }

    pub fn ellipse(&self, cx: f32, cy: f32, rx: f32, ry: f32) {
        unsafe { sys::nvgEllipse(self.raw, cx, cy, rx, ry) };
    }

    pub fn circle(&self, cx: f32, cy: f32, r: f32) {
        unsafe { sys::nvgCircle(self.raw, cx, cy, r) };
    }

    pub fn fill(&self) {
        unsafe { sys::nvgFill(self.raw) };
    }

    pub fn stroke(&self) {
        unsafe { sys::nvgStroke(self.raw) };
    }

    /// Start building a path with chainable methods.
    pub fn path(&self) -> PathBuilder<'_> {
        PathBuilder::new(self)
    }
}

// Images
impl NvgContext {
    pub fn create_image(&self, filename: &str, flags: ImageFlags) -> Option<i32> {
        let c = CString::new(filename).ok()?;
        let id = unsafe { sys::nvgCreateImage(self.raw, c.as_ptr(), flags.0) };
        if id > 0 { Some(id) } else { None }
    }

    pub fn create_image_mem(&self, flags: ImageFlags, data: &mut [u8]) -> Option<i32> {
        let id = unsafe {
            sys::nvgCreateImageMem(self.raw, flags.0, data.as_mut_ptr(), data.len() as i32)
        };
        if id > 0 { Some(id) } else { None }
    }

    pub fn create_image_rgba(&self, w: i32, h: i32, flags: ImageFlags, data: &[u8]) -> Option<i32> {
        let id = unsafe { sys::nvgCreateImageRGBA(self.raw, w, h, flags.0, data.as_ptr()) };
        if id > 0 { Some(id) } else { None }
    }

    pub fn update_image(&self, image: i32, data: &[u8]) {
        unsafe { sys::nvgUpdateImage(self.raw, image, data.as_ptr()) };
    }

    pub fn image_size(&self, image: i32) -> (i32, i32) {
        let (mut w, mut h) = (0i32, 0i32);
        unsafe { sys::nvgImageSize(self.raw, image, &mut w, &mut h) };
        (w, h)
    }

    pub fn delete_image(&self, image: i32) {
        unsafe { sys::nvgDeleteImage(self.raw, image) };
    }
}

// Fonts and Text
impl NvgContext {
    pub fn create_font(&self, name: &str, filename: &str) -> Option<i32> {
        let cname = CString::new(name).ok()?;
        let cfile = CString::new(filename).ok()?;
        let id = unsafe { sys::nvgCreateFont(self.raw, cname.as_ptr(), cfile.as_ptr()) };
        if id >= 0 { Some(id) } else { None }
    }

    pub fn find_font(&self, name: &str) -> Option<i32> {
        let cname = CString::new(name).ok()?;
        let id = unsafe { sys::nvgFindFont(self.raw, cname.as_ptr()) };
        if id >= 0 { Some(id) } else { None }
    }

    pub fn add_fallback_font(&self, base: &str, fallback: &str) -> bool {
        let cb = CString::new(base).unwrap();
        let cf = CString::new(fallback).unwrap();
        unsafe { sys::nvgAddFallbackFont(self.raw, cb.as_ptr(), cf.as_ptr()) != 0 }
    }

    pub fn font_size(&self, size: f32) {
        unsafe { sys::nvgFontSize(self.raw, size) };
    }

    pub fn font_blur(&self, blur: f32) {
        unsafe { sys::nvgFontBlur(self.raw, blur) };
    }

    pub fn text_letter_spacing(&self, spacing: f32) {
        unsafe { sys::nvgTextLetterSpacing(self.raw, spacing) };
    }

    pub fn text_line_height(&self, line_height: f32) {
        unsafe { sys::nvgTextLineHeight(self.raw, line_height) };
    }

    pub fn text_align(&self, align: Align) {
        unsafe { sys::nvgTextAlign(self.raw, align.0) };
    }

    pub fn font_face_id(&self, font: i32) {
        unsafe { sys::nvgFontFaceId(self.raw, font) };
    }

    pub fn font_face(&self, name: &str) {
        let c = CString::new(name).unwrap();
        unsafe { sys::nvgFontFace(self.raw, c.as_ptr()) };
    }

    /// Draw text at `(x, y)`. Returns the horizontal advance.
    pub fn text(&self, x: f32, y: f32, text: &str) -> f32 {
        let ptr = text.as_ptr() as *const i8;
        let end = unsafe { ptr.add(text.len()) };
        unsafe { sys::nvgText(self.raw, x, y, ptr, end) }
    }

    /// Draw word-wrapped text within `break_width`.
    pub fn text_box(&self, x: f32, y: f32, break_width: f32, text: &str) {
        let ptr = text.as_ptr() as *const i8;
        let end = unsafe { ptr.add(text.len()) };
        unsafe { sys::nvgTextBox(self.raw, x, y, break_width, ptr, end) };
    }

    /// Measure text. Returns bounding box and horizontal advance.
    pub fn text_bounds(&self, x: f32, y: f32, text: &str) -> TextBounds {
        let ptr = text.as_ptr() as *const i8;
        let end = unsafe { ptr.add(text.len()) };
        let mut bounds = [0.0f32; 4];
        let advance = unsafe { sys::nvgTextBounds(self.raw, x, y, ptr, end, bounds.as_mut_ptr()) };
        TextBounds { advance, bounds }
    }

    /// Measure word-wrapped text bounds.
    pub fn text_box_bounds(&self, x: f32, y: f32, break_width: f32, text: &str) -> [f32; 4] {
        let ptr = text.as_ptr() as *const i8;
        let end = unsafe { ptr.add(text.len()) };
        let mut bounds = [0.0f32; 4];
        unsafe {
            sys::nvgTextBoxBounds(self.raw, x, y, break_width, ptr, end, bounds.as_mut_ptr())
        };
        bounds
    }

    /// Get vertical text metrics for the current font/size.
    pub fn text_metrics(&self) -> TextMetrics {
        let (mut asc, mut desc, mut lh) = (0.0f32, 0.0f32, 0.0f32);
        unsafe { sys::nvgTextMetrics(self.raw, &mut asc, &mut desc, &mut lh) };
        TextMetrics {
            ascender: asc,
            descender: desc,
            line_height: lh,
        }
    }
}
