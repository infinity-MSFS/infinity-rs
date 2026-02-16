use crate::sys::{self, FsContext};
use core::ptr::NonNull;

#[derive(Copy, Clone)]
pub struct Context(NonNull<FsContext>);

impl Context {
    #[inline]
    pub(crate) unsafe fn from_raw(ctx: FsContext) -> Self {
        let p = NonNull::new(ctx as *mut FsContext).expect("Fscontext ptr is null");
        Self(p)
    }

    pub fn as_ptr(&self) -> *mut FsContext {
        self.0.as_ptr()
    }
}
