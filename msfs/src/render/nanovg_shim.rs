//! Native-only NanoVG backend that links to your `nanovg_shim` shared library.
//!
//! This mirrors the C++ pattern:
//! - create a `ShimCtx` per `FsContext`
//! - point it at the host-provided RGBA8888 framebuffer each frame
//! - fetch `NVGcontext*` from the shim and use NanoVG normally

use crate::sys::FsContext;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[allow(non_camel_case_types)]
pub enum ShimCtx {}

#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim", not(windows)))]
#[link(name = "nanovg_shim")]
unsafe extern "C" {
    fn shim_create(flags: i32) -> *mut ShimCtx;
    fn shim_delete(s: *mut ShimCtx);
    fn shim_nvg(s: *mut ShimCtx) -> *mut crate::sys::NVGcontext;
    fn shim_set_framebuffer_rgba8888(s: *mut ShimCtx, dest: *mut core::ffi::c_void, w: i32, h: i32);
}

// On Windows, requiring an import library (nanovg_shim.lib) is often inconvenient.
// Instead, load nanovg_shim.dll with LoadLibraryW and resolve required exports.
#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim", windows))]
mod win {
    use super::ShimCtx;

    type ShimCreate = unsafe extern "C" fn(flags: i32) -> *mut ShimCtx;
    type ShimDelete = unsafe extern "C" fn(s: *mut ShimCtx);
    type ShimNvg = unsafe extern "C" fn(s: *mut ShimCtx) -> *mut crate::sys::NVGcontext;
    type ShimSetFramebufferRgba8888 =
        unsafe extern "C" fn(s: *mut ShimCtx, dest: *mut core::ffi::c_void, w: i32, h: i32);

    #[repr(C)]
    struct ShimFns {
        create: ShimCreate,
        delete: ShimDelete,
        nvg: ShimNvg,
        set_fb_rgba8888: ShimSetFramebufferRgba8888,
    }

    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::OnceLock;

    type HMODULE = *mut core::ffi::c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(lpLibFileName: *const u16) -> HMODULE;
        fn GetProcAddress(hModule: HMODULE, lpProcName: *const u8) -> *mut core::ffi::c_void;
    }

    fn load_symbol<T>(h: HMODULE, name: &'static [u8]) -> T {
        // name must be null-terminated for GetProcAddress.
        debug_assert_eq!(name.last().copied(), Some(0));
        let p = unsafe { GetProcAddress(h, name.as_ptr()) };
        assert!(
            !p.is_null(),
            "missing export: {}",
            std::str::from_utf8(&name[..name.len() - 1]).unwrap()
        );
        unsafe { std::mem::transmute_copy(&p) }
    }

    fn shim() -> &'static ShimFns {
        static SHIM: OnceLock<ShimFns> = OnceLock::new();
        SHIM.get_or_init(|| {
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

            ShimFns {
                create: load_symbol(h, b"shim_create\0"),
                delete: load_symbol(h, b"shim_delete\0"),
                nvg: load_symbol(h, b"shim_nvg\0"),
                set_fb_rgba8888: load_symbol(h, b"shim_set_framebuffer_rgba8888\0"),
            }
        })
    }

    pub(super) unsafe fn shim_create(flags: i32) -> *mut ShimCtx {
        (shim().create)(flags)
    }
    pub(super) unsafe fn shim_delete(s: *mut ShimCtx) {
        (shim().delete)(s)
    }
    pub(super) unsafe fn shim_nvg(s: *mut ShimCtx) -> *mut crate::sys::NVGcontext {
        (shim().nvg)(s)
    }
    pub(super) unsafe fn shim_set_framebuffer_rgba8888(
        s: *mut ShimCtx,
        dest: *mut core::ffi::c_void,
        w: i32,
        h: i32,
    ) {
        (shim().set_fb_rgba8888)(s, dest, w, h)
    }
}

// Re-export the shim FFI calls behind a common name.
#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim", windows))]
use win::{shim_create, shim_delete, shim_nvg, shim_set_framebuffer_rgba8888};

fn map() -> &'static Mutex<HashMap<FsContext, usize>> {
    static MAP: OnceLock<Mutex<HashMap<FsContext, usize>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Create and store a shim context for this gauge instance.
///
/// Safe wrapper around `shim_create`.
#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim"))]
pub fn init(ctx: FsContext, flags: i32) -> bool {
    let mut m = map().lock().unwrap();
    if m.contains_key(&ctx) {
        return true;
    }
    let s = unsafe { shim_create(flags) };
    if s.is_null() {
        return false;
    }
    m.insert(ctx, s as usize);
    true
}

/// Fetch the `NVGcontext*` for the given `FsContext`.
#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim"))]
pub fn nvg(ctx: FsContext) -> Option<*mut crate::sys::NVGcontext> {
    let m = map().lock().unwrap();
    let s = *m.get(&ctx)? as *mut ShimCtx;
    let nvg = unsafe { shim_nvg(s) };
    if nvg.is_null() { None } else { Some(nvg) }
}

/// Point the software renderer at an RGBA8888 framebuffer.
#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim"))]
pub fn set_framebuffer_rgba8888(
    ctx: FsContext,
    dest: *mut core::ffi::c_void,
    w: i32,
    h: i32,
) -> bool {
    let m = map().lock().unwrap();
    let s = match m.get(&ctx) {
        Some(s) => *s as *mut ShimCtx,
        None => return false,
    };
    unsafe {
        shim_set_framebuffer_rgba8888(s, dest, w, h);
    }
    true
}

/// Destroy and remove the shim context for this gauge instance.
#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim"))]
pub fn kill(ctx: FsContext) {
    let mut m = map().lock().unwrap();
    if let Some(s) = m.remove(&ctx) {
        unsafe { shim_delete(s as *mut ShimCtx) };
    }
}

// Stubs when not available.
#[cfg(any(target_arch = "wasm32", not(feature = "nanovg-shim")))]
pub fn init(_ctx: FsContext, _flags: i32) -> bool {
    false
}
#[cfg(any(target_arch = "wasm32", not(feature = "nanovg-shim")))]
pub fn nvg(_ctx: FsContext) -> Option<*mut crate::sys::NVGcontext> {
    None
}
#[cfg(any(target_arch = "wasm32", not(feature = "nanovg-shim")))]
pub fn set_framebuffer_rgba8888(
    _ctx: FsContext,
    _dest: *mut core::ffi::c_void,
    _w: i32,
    _h: i32,
) -> bool {
    false
}
#[cfg(any(target_arch = "wasm32", not(feature = "nanovg-shim")))]
pub fn kill(_ctx: FsContext) {}
