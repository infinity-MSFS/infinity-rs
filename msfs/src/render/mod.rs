pub mod color;

pub mod nvg {
    use crate::sys;
    use core::{marker::PhantomData, ptr::NonNull};

    pub struct Vg {
        ctx: NonNull<sys::NVGcontext>,
    }

    impl Vg {
        pub unsafe fn from_raw(ctx: *mut sys::NVGcontext) -> Self {
            Self {
                ctx: NonNull::new(ctx).expect("NVGcontext was null"),
            }
        }

        pub fn from_msfs_context(_fs_ctx: sys::FsContext) -> Self {
            unimplemented!()
        }

        // #[inline]
    }
}
