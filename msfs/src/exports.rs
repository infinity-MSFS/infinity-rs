use crate::types::*;
use crate::{
    context::Context,
    modules::{Gauge, System},
};

#[macro_export]
macro_rules! export_system {
    (name=$name:ident, state=$state:ty, ctor=$ctor:expr $(,)?) => {
        ::paste::paste! {
            static mut [<$name _SYSTEM>]: ::core::option::Option<$state> = None;

            #[inline(always)]
            unsafe fn [<$name _with>]<R>(f: impl FnOnce(&mut $state) -> R) -> Option<R> {
                [<$name _SYSTEM>].as_mut().map(f)
            }

            #[no_mangle]
            pub extern "C" fn [<$name _system_init>](
                ctx: $crate::sys::FsContext,
                p_install: *mut $crate::sys::FsSystemInstallData,
            ) -> bool {
                unsafe { [<$name _SYSTEM>] = Some($ctor); }
                unsafe {
                    let ctx = $crate::context::Context::from_raw(ctx);
                    let install = &mut *p_install;
                    [<$name _with>](|s| <$state as $crate::modules::System>::init(s, &ctx, install))
                        .unwrap_or(false)
                }
            }

            #[no_mangle]
            pub extern "C" fn [<$name _system_update>](
                ctx: $crate::sys::FsContext,
                dt: f32,
            ) -> bool {
                unsafe {
                    let ctx = $crate::context::Context::from_raw(ctx);
                    [<$name _with>](|s| <$state as $crate::modules::System>::update(s, &ctx, dt))
                        .unwrap_or(false)
                }
            }

            #[no_mangle]
            pub extern "C" fn [<$name _system_kill>](
                ctx: $crate::sys::FsContext,
            ) -> bool {
                unsafe {
                    let ctx = $crate::context::Context::from_raw(ctx);
                    let ok = [<$name _with>](|s| <$state as $crate::modules::System>::kill(s, &ctx))
                        .unwrap_or(false);
                    [<$name _SYSTEM>] = None;
                    ok
                }
            }
        }
    };
}

#[macro_export]
macro_rules! export_gauge {
    (name=$name:ident, state=$state:ty, ctor=$ctor:expr $(,)?) => {
        ::paste::paste! {
            static mut [<$name _GAUGE>]: ::core::option::Option<$state> = None;

            #[inline(always)]
            unsafe fn [<$name _with>]<R>(f: impl FnOnce(&mut $state) -> R) -> Option<R> {
                [<$name _GAUGE>].as_mut().map(f)
            }

            #[no_mangle]
            pub extern "C" fn [<$name _gauge_init>](
                ctx: $crate::sys::FsContext,
                p_install: *mut $crate::sys::FsGaugeInstallData,
            ) -> bool {
                unsafe { [<$name _GAUGE>] = Some($ctor); }
                unsafe {
                    let ctx = $crate::context::Context::from_raw(ctx);
                    let install = &mut *p_install;
                    [<$name _with>](|g| <$state as $crate::modules::Gauge>::init(g, &ctx, install))
                        .unwrap_or(false)
                }
            }

            #[no_mangle]
            pub extern "C" fn [<$name _gauge_update>](
                ctx: $crate::sys::FsContext,
                dt: f32,
            ) -> bool {
                unsafe {
                    let ctx = $crate::context::Context::from_raw(ctx);
                    [<$name _with>](|g| <$state as $crate::modules::Gauge>::update(g, &ctx, dt))
                        .unwrap_or(false)
                }
            }

            #[no_mangle]
            pub extern "C" fn [<$name _gauge_draw>](
                ctx: $crate::sys::FsContext,
                p_draw: *mut $crate::sys::FsGaugeDrawData,
            ) -> bool {
                unsafe {
                    let ctx = $crate::context::Context::from_raw(ctx);
                    let draw = &mut *p_draw;
                    [<$name _with>](|g| <$state as $crate::modules::Gauge>::draw(g, &ctx, draw))
                        .unwrap_or(false)
                }
            }

            #[no_mangle]
            pub extern "C" fn [<$name _gauge_kill>](
                ctx: $crate::sys::FsContext,
            ) -> bool {
                unsafe {
                    let ctx = $crate::context::Context::from_raw(ctx);
                    let ok = [<$name _with>](|g| <$state as $crate::modules::Gauge>::kill(g, &ctx))
                        .unwrap_or(false);
                    [<$name _GAUGE>] = None;
                    ok
                }
            }

            #[no_mangle]
            pub extern "C" fn [<$name _gauge_mouse_handler>](
                ctx: $crate::sys::FsContext,
                x: f32,
                y: f32,
                flags: i32,
            ) {
                unsafe {
                    let ctx = $crate::context::Context::from_raw(ctx);
                    let _ = [<$name _with>](|g| <$state as $crate::modules::Gauge>::mouse(g, &ctx, x, y, flags));
                }
            }
        }
    };
}
