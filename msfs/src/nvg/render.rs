use crate::sys;

#[inline(always)]
unsafe fn uptr_to_ctx(uptr: u64) -> sys::FsContext {
    uptr as sys::FsContext
}

pub(crate) unsafe fn build_nvg_params(fs_ctx: sys::FsContext) -> sys::NVGparams {
    sys::NVGparams {
        userPtr: fs_ctx as u64,
        edgeAntiAlias: 1,
        renderCreate: Some(render_create),
        renderCreateTexture: Some(render_create_texture),
        renderDeleteTexture: Some(render_delete_texture),
        renderUpdateTexture: Some(render_update_texture),
        renderGetTextureSize: Some(render_get_texture_size),
        renderViewport: Some(render_viewport),
        renderCancel: Some(render_cancel),
        renderFlush: Some(render_flush),
        renderFill: Some(render_fill),
        renderStroke: Some(render_stroke),
        renderTriangles: Some(render_triangles),
        renderClearStencil: Some(render_clear_stencil),
        renderDelete: Some(render_delete),
    }
}

unsafe extern "C" fn render_create(uptr: u64) -> i32 {
    unsafe { sys::fsRenderCreate(uptr_to_ctx(uptr)) as i32 }
}

unsafe extern "C" fn render_create_texture(
    uptr: u64,
    type_: i32,
    w: i32,
    h: i32,
    image_flags: i32,
    data: *const u8,
    debug_name: *const i8,
) -> i32 {
    unsafe {
        sys::fsRenderCreateTexture(
            uptr_to_ctx(uptr),
            type_,
            w,
            h,
            image_flags,
            data,
            debug_name,
        ) as i32
    }
}

unsafe extern "C" fn render_delete_texture(uptr: u64, image: i32) -> i32 {
    unsafe { sys::fsRenderDeleteTexture(uptr_to_ctx(uptr), image) as i32 }
}

unsafe extern "C" fn render_update_texture(
    uptr: u64,
    image: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    data: *const u8,
) -> i32 {
    unsafe { sys::fsRenderUpdateTexture(uptr_to_ctx(uptr), image, x, y, w, h, data) as i32 }
}

unsafe extern "C" fn render_get_texture_size(
    uptr: u64,
    image: i32,
    w: *mut i32,
    h: *mut i32,
) -> i32 {
    unsafe { sys::fsRenderGetTextureSize(uptr_to_ctx(uptr), image, w, h) }
}

unsafe extern "C" fn render_viewport(uptr: u64, width: f32, height: f32, device_pixel_ratio: f32) {
    unsafe { sys::fsRenderViewport(uptr_to_ctx(uptr), width, height, device_pixel_ratio) }
}

unsafe extern "C" fn render_cancel(uptr: u64) {
    unsafe {
        sys::fsRenderCancel(uptr_to_ctx(uptr));
    }
}

unsafe extern "C" fn render_flush(uptr: u64) {
    unsafe { sys::fsRenderFlush(uptr_to_ctx(uptr)) }
}

unsafe extern "C" fn render_fill(
    uptr: u64,
    paint: *mut sys::NVGpaint,
    composite_op: sys::NVGcompositeOperationState,
    scissor: *mut sys::NVGscissor,
    fringe: f32,
    bounds: *const f32,
    paths: *const sys::NVGpath,
    npaths: i32,
) {
    unsafe {
        sys::fsRenderFill(
            uptr_to_ctx(uptr),
            paint as *mut sys::FsPaint,
            composite_op,
            scissor as *mut sys::FsScissor,
            fringe,
            bounds,
            paths as *const sys::FsPath,
            npaths,
        );
    }
}

unsafe extern "C" fn render_stroke(
    uptr: u64,
    paint: *mut sys::NVGpaint,
    composite_op: sys::NVGcompositeOperationState,
    scissor: *mut sys::NVGscissor,
    fringe: f32,
    stroke_width: f32,
    paths: *const sys::NVGpath,
    npaths: i32,
) {
    unsafe {
        sys::fsRenderStroke(
            uptr_to_ctx(uptr),
            paint as *mut sys::FsPaint,
            composite_op,
            scissor as *mut sys::FsScissor,
            fringe,
            stroke_width,
            paths as *const sys::FsPath,
            npaths,
        );
    }
}

unsafe extern "C" fn render_triangles(
    uptr: u64,
    paint: *mut sys::NVGpaint,
    composite_op: sys::NVGcompositeOperationState,
    scissor: *mut sys::NVGscissor,
    verts: *const sys::NVGvertex,
    nverts: i32,
) {
    unsafe {
        sys::fsRenderTriangles(
            uptr_to_ctx(uptr),
            paint as *mut sys::FsPaint,
            composite_op,
            scissor as *mut sys::FsScissor,
            verts as *const sys::FsVertex,
            nverts,
        );
    }
}

unsafe extern "C" fn render_clear_stencil(uptr: u64) {
    unsafe { sys::fsRenderClearStencil(uptr_to_ctx(uptr)) }
}

unsafe extern "C" fn render_delete(uptr: u64) {
    unsafe { sys::fsRenderDelete(uptr_to_ctx(uptr)) }
}
