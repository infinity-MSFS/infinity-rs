use crate::sys::{NVGcolor, NVGcontext};

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;

    #[inline]
    pub unsafe fn begin_frame(ctx: *mut NVGcontext, w: f32, h: f32, px_ratio: f32) {
        crate::sys::nvgBeginFrame(ctx, w, h, px_ratio)
    }

    #[inline]
    pub unsafe fn end_frame(ctx: *mut NVGcontext) {
        crate::sys::nvgEndFrame(ctx)
    }

    #[inline]
    pub unsafe fn rgba_f(r: f32, g: f32, b: f32, a: f32) -> NVGcolor {
        crate::sys::nvgRGBAf(r, g, b, a)
    }

    #[inline]
    pub unsafe fn fill_color(ctx: *mut NVGcontext, color: NVGcolor) {
        crate::sys::nvgFillColor(ctx, color)
    }

    #[inline]
    pub unsafe fn begin_path(ctx: *mut NVGcontext) {
        crate::sys::nvgBeginPath(ctx)
    }

    #[inline]
    pub unsafe fn rect(ctx: *mut NVGcontext, x: f32, y: f32, w: f32, h: f32) {
        crate::sys::nvgRect(ctx, x, y, w, h)
    }

    #[inline]
    pub unsafe fn fill(ctx: *mut NVGcontext) {
        crate::sys::nvgFill(ctx)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
    compile_error!("nanovg_api requires target_arch=wasm32 (MSFS NanoVG)");

    pub unsafe fn begin_frame(_ctx: *mut NVGcontext, _w: f32, _h: f32, _px_ratio: f32) {
        unreachable!()
    }
    pub unsafe fn end_frame(_ctx: *mut NVGcontext) {
        unreachable!()
    }
    pub unsafe fn rgba_f(_r: f32, _g: f32, _b: f32, _a: f32) -> NVGcolor {
        unreachable!()
    }
    pub unsafe fn fill_color(_ctx: *mut NVGcontext, _color: NVGcolor) {
        unreachable!()
    }
    pub unsafe fn begin_path(_ctx: *mut NVGcontext) {
        unreachable!()
    }
    pub unsafe fn rect(_ctx: *mut NVGcontext, _x: f32, _y: f32, _w: f32, _h: f32) {
        unreachable!()
    }
    pub unsafe fn fill(_ctx: *mut NVGcontext) {
        unreachable!()
    }
}

pub use imp::*;
