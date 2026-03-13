use crate::sys::FsContext;

/// An opaque gauge/system context handle passed in by the MSFS host.
///
/// `FsContext` is a `u64` on all targets. In wasm32 the pointer width is only
/// 32 bits, so we must **not** store this value as a raw pointer — doing so
/// would silently truncate the high 32 bits and pass a corrupt handle to every
/// host API (including `fsRenderCreate`, causing `nvgCreateInternal` to fail).
/// We store it as the integer it is instead.
#[derive(Copy, Clone)]
pub struct Context(FsContext);

impl Context {
    /// # Safety
    /// `ctx` must be a valid `FsContext` provided by the MSFS host for the
    /// lifetime of this `Context`.
    #[inline]
    pub unsafe fn from_raw(ctx: FsContext) -> Self {
        Self(ctx)
    }

    /// Return the raw `FsContext` value.
    #[inline]
    pub fn fs_context(&self) -> FsContext {
        self.0
    }
}
