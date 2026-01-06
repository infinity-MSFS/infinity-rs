//! Smoke test for the NanoVG shim backend.
//!
//! This example doesn't depend on the simulator. It:
//! - creates a shim NanoVG context
//! - points it at an RGBA8888 framebuffer
//! - draws a simple rect
//!
//! Build/run with:
//!   cargo run -p msfs --example nanovg_shim_smoke --features nanovg-shim
//!
//! NOTE: you must have `nanovg_shim.dll` available to the linker/runtime.

#[cfg(any(target_arch = "wasm32", not(feature = "nanovg-shim")))]
fn main() {
    eprintln!("This example requires a native build with --features nanovg-shim.");
}

#[cfg(all(not(target_arch = "wasm32"), feature = "nanovg-shim"))]
fn main() {
    use msfs::render::{nanovg_api, nanovg_shim};

    // Arbitrary FsContext for the map key.
    let ctx: msfs::sys::FsContext = 1;

    let ok = nanovg_shim::init(ctx, 0);
    assert!(ok, "failed to init shim");

    let w = 256;
    let h = 256;
    let mut fb = vec![0u8; (w * h * 4) as usize];

    nanovg_shim::set_framebuffer_rgba8888(ctx, fb.as_mut_ptr() as _, w, h);

    let nvg = nanovg_shim::nvg(ctx).expect("missing NVGcontext*");

    unsafe {
        nanovg_api::begin_frame(nvg, w as f32, h as f32, 1.0);

        let red = nanovg_api::rgba_f(1.0, 0.0, 0.0, 1.0);
        nanovg_api::fill_color(nvg, red);

        nanovg_api::begin_path(nvg);
        nanovg_api::rect(nvg, 0.0, 0.0, w as f32, h as f32);
        nanovg_api::fill(nvg);

        nanovg_api::end_frame(nvg);
    }

    // Basic sanity: framebuffer should now contain non-zero bytes.
    let nonzero = fb.iter().any(|&b| b != 0);
    assert!(
        nonzero,
        "framebuffer stayed all zeros; shim may not be working"
    );

    nanovg_shim::kill(ctx);
    println!("OK: rendered into framebuffer via shim");
}
