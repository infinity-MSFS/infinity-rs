//! Minimal NanoVG API layer.
//!
//! - On wasm32: use the MSFS-provided NanoVG symbols from `msfs::sys`.
//! - On native with `nanovg-shim`: resolve NanoVG symbols from `nanovg_shim.dll`/`.so`.

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

#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim", windows))]
mod imp {
    use super::*;

    // We already depend on kernel32 in nanovg_shim.rs; duplicate minimal load logic here
    // so we can resolve NanoVG functions from the shim DLL instead of MSFS.

    type HMODULE = *mut core::ffi::c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(lpLibFileName: *const u16) -> HMODULE;
        fn GetProcAddress(hModule: HMODULE, lpProcName: *const u8) -> *mut core::ffi::c_void;
    }

    fn load_symbol<T>(h: HMODULE, name: &'static [u8]) -> T {
        debug_assert_eq!(name.last().copied(), Some(0));
        let p = unsafe { GetProcAddress(h, name.as_ptr()) };
        assert!(
            !p.is_null(),
            "missing NanoVG export in shim: {}",
            std::str::from_utf8(&name[..name.len() - 1]).unwrap()
        );
        unsafe { std::mem::transmute_copy(&p) }
    }

    struct Fns {
        begin_frame: unsafe extern "C" fn(*mut NVGcontext, f32, f32, f32),
        end_frame: unsafe extern "C" fn(*mut NVGcontext),
        rgba_f: unsafe extern "C" fn(f32, f32, f32, f32) -> NVGcolor,
        fill_color: unsafe extern "C" fn(*mut NVGcontext, NVGcolor),
        begin_path: unsafe extern "C" fn(*mut NVGcontext),
        rect: unsafe extern "C" fn(*mut NVGcontext, f32, f32, f32, f32),
        fill: unsafe extern "C" fn(*mut NVGcontext),
    }

    fn fns() -> &'static Fns {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use std::sync::OnceLock;

        static FNS: OnceLock<Fns> = OnceLock::new();
        FNS.get_or_init(|| {
            let dll_name =
                std::env::var("NANOVG_SHIM_DLL").unwrap_or_else(|_| "nanovg_shim.dll".to_string());
            let wide: Vec<u16> = OsStr::new(&dll_name)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let h = unsafe { LoadLibraryW(wide.as_ptr()) };
            assert!(
                !h.is_null(),
                "failed to load {} (set NANOVG_SHIM_DLL or put it on PATH/next to exe)",
                dll_name
            );

            Fns {
                begin_frame: load_symbol(h, b"nvgBeginFrame\0"),
                end_frame: load_symbol(h, b"nvgEndFrame\0"),
                rgba_f: load_symbol(h, b"nvgRGBAf\0"),
                fill_color: load_symbol(h, b"nvgFillColor\0"),
                begin_path: load_symbol(h, b"nvgBeginPath\0"),
                rect: load_symbol(h, b"nvgRect\0"),
                fill: load_symbol(h, b"nvgFill\0"),
            }
        })
    }

    #[inline]
    pub unsafe fn begin_frame(ctx: *mut NVGcontext, w: f32, h: f32, px_ratio: f32) {
        (fns().begin_frame)(ctx, w, h, px_ratio)
    }
    #[inline]
    pub unsafe fn end_frame(ctx: *mut NVGcontext) {
        (fns().end_frame)(ctx)
    }
    #[inline]
    pub unsafe fn rgba_f(r: f32, g: f32, b: f32, a: f32) -> NVGcolor {
        (fns().rgba_f)(r, g, b, a)
    }
    #[inline]
    pub unsafe fn fill_color(ctx: *mut NVGcontext, color: NVGcolor) {
        (fns().fill_color)(ctx, color)
    }
    #[inline]
    pub unsafe fn begin_path(ctx: *mut NVGcontext) {
        (fns().begin_path)(ctx)
    }
    #[inline]
    pub unsafe fn rect(ctx: *mut NVGcontext, x: f32, y: f32, w: f32, h: f32) {
        (fns().rect)(ctx, x, y, w, h)
    }
    #[inline]
    pub unsafe fn fill(ctx: *mut NVGcontext) {
        (fns().fill)(ctx)
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim", not(windows)))]
mod imp {
    use super::*;
    compile_error!(
        "nanovg_api native shim backend is only implemented for Windows right now; add a Linux dlopen backend (or link-time .so) if needed."
    );

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

#[cfg(all(not(target_arch = "wasm32"), not(feature = "nanovg-shim")))]
mod imp {
    use super::*;
    compile_error!(
        "nanovg_api requires either target_arch=wasm32 (MSFS NanoVG) or the 'nanovg-shim' feature (native shim backend)"
    );

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
